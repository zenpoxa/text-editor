use crossterm::event::{read, Event, KeyEvent, KeyEventKind};
use std::{
    env, io::Error, panic::{set_hook, take_hook}
};
mod command;
mod commandbar;
mod messagebar;
mod uicomponent;
mod documentstatus;
mod terminal;
mod view;
mod line;
mod position;
mod size;
mod statusbar;

use commandbar::CommandBar;
use documentstatus::DocumentStatus;
use line::Line;
use messagebar::MessageBar;
use position::Position;
use size::Size;
use terminal::Terminal;
use view::View;
use statusbar::StatusBar;
use uicomponent::UIComponent;

use self::command::{
    Command::{self, Edit, Move, System},
    Edit::InsertNewline,
    System::{Dismiss, Quit, Resize, Save},
};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    view: View,
    status_bar: StatusBar,
    title: String,
    message_bar: MessageBar,
    command_bar: Option<CommandBar>,
    terminal_size: Size,
    quit_times: u8,
}

impl Editor {
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        Terminal::initialize()?;

        let mut editor = Self::default();
        let size = Terminal::size().unwrap_or_default();
        editor.resize(size);
        editor
            .message_bar
            .update_message("HELP: Ctrl-S = save | Ctrl-Q = quit");

        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            if editor.view.load(file_name).is_err() {
                editor
                    .message_bar
                    .update_message(&format!("ERR: Could not open file: {file_name}"));
            }
        }

        editor.refresh_status();

        Ok(editor)
    }

    pub fn resize(&mut self, size: Size) {
        self.terminal_size = size;
        self.view.resize(Size {
            height: size.height.saturating_sub(2), // status bar & message_bar heights combined
            width: size.width,
        });
        self.message_bar.resize(Size {
            height: 1,
            width: size.width,
        });
        self.status_bar.resize(Size {
            height: 1,
            width: size.width,
        });

        if let Some(command_bar) = &mut self.command_bar {
            command_bar.resize(Size{
                height: 1,
                width: size.width,
            });
        }
    }

    pub fn refresh_status(&mut self) {
        let status = self.view.get_status();
        let title = format!("{} - {NAME}", status.file_name);
        self.status_bar.update_status(status);

        if title != self.title && matches!(Terminal::set_title(&title), Ok(())) {
            self.title = title;
        }
    }

    pub fn run(&mut self) {
        loop {
            self.refresh_screen();
            if self.should_quit {
                break;
            }
            match read() {
                Ok(event) => self.evaluate_event(event),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not read event: {err:?}");
                    }
                }
            }
            let status = self.view.get_status();
            self.status_bar.update_status(status);
        }
    }

    fn evaluate_event(&mut self, event: Event) {
        let should_process = match &event {
            Event::Key(KeyEvent {kind, ..}) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if should_process {
            // Ici, seulement traiter les touches avec des commandes. Les touches sans commande associée (Err{}) ne feront rien
            if let Ok(command) = Command::try_from(event) {
                self.process_command(command);
            }
        }

        // PANIC A EVITER POUR WINDOWS (car l'événement 'Release' n'est pas pris en compte)
        else if !(cfg!(windows)) {
            #[cfg(debug_assertions)]
            {
                panic!("Received and discarded unsupported or non-press event.");
            }
        }
    }

    fn process_command(&mut self, command: Command) {
        // Handle quit times to reset or continue
        match command {
            System(Quit) => {
                if self.command_bar.is_none() {
                    self.handle_quit();
                }   
            }
            System(Resize(size)) => self.resize(size),
            _ => self.reset_quit_times(),
        }
        // Other commands
        match command {
            System(Quit | Resize(_)) => {}
            System(Save) => {
                if self.command_bar.is_none() {
                    self.handle_save();
                }
            }
            System(Dismiss) => {
                if self.command_bar.is_some() {
                    self.dismiss_prompt();
                    self.message_bar.update_message("Save aborted.");
                }
            }
            Edit(edit_command) => {
                if let Some(command_bar) = &mut self.command_bar {
                    if matches!(edit_command, InsertNewline) {
                        let file_name = command_bar.value();
                        self.dismiss_prompt();
                        self.save(Some(&file_name));
                        // MANQUE 'FICHIER SAUVEGARDE' ??
                    } else {
                        command_bar.handle_edit_command(edit_command);
                    }
                } else {
                    self.view.handle_edit_command(edit_command);
                }
            }
            Move(move_command) => {
                if self.command_bar.is_none() {
                    self.view.handle_move_command(move_command);
                }
            }
        }
    }
    
    fn dismiss_prompt(&mut self) {
        self.command_bar = None;
        self.message_bar.set_needs_redraw(true);
    }

    fn show_prompt(&mut self) {
        let mut command_bar = CommandBar::default();
        command_bar.set_prompt("Enregistrer sous: ");
        command_bar.resize(Size {
            height: 1,
            width: self.terminal_size.width,
        });
        command_bar.set_needs_redraw(true);
        self.command_bar = Some(command_bar);
    }

    fn handle_save(&mut self) {
        if self.view.is_file_loaded() {
            self.save(None);
        }
        else {
            self.show_prompt();
        }
    }
    fn save(&mut self, file_name: Option<&str>) {
        let result = if let Some(name) = file_name {
            self.view.save_as(name)
        } else {
            self.view.save()
        };
        if result.is_ok() {
            self.message_bar.update_message("File saved successfully.");
        } else {
            self.message_bar.update_message("Error writing file!");
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn handle_quit(&mut self) {
        self.quit_times += 1;
        if !self.view.get_status().is_modified || self.quit_times == QUIT_TIMES {
            self.should_quit = true;
        }
        else {
            self.message_bar.update_message(&format!(
                "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                QUIT_TIMES - self.quit_times
            ));

        }
    }
    fn reset_quit_times(&mut self) {
        if self.quit_times > 0 {
            self.quit_times = 0;
            self.message_bar.update_message("");
        }
    }
    fn refresh_screen(&mut self) {
        if self.terminal_size.height == 0 || self.terminal_size.width == 0 {
            return;
        }
        let bottom_bar_row = self.terminal_size.height.saturating_sub(1);
        let _ = Terminal::hide_caret();
        
        // height at least 1 -> Render message bar
        if let Some(command_bar) = &mut self.command_bar {
            command_bar.render(self.terminal_size.height.saturating_sub(1));
        } else {
            self.message_bar.render(bottom_bar_row)
        }
        // height at least 2 -> Render status bar
        if self.terminal_size.height > 1 {
            self.status_bar.render(self.terminal_size.height.saturating_sub(2));
        }
        // height at least 3 -> Render view also
        if self.terminal_size.height > 2 {
            self.view.render(0);
        }

        let new_caret_pos = if let Some(command_bar) = &self.command_bar {
            Position {
                row: bottom_bar_row,
                col: command_bar.caret_position_col(),
            }
        } else {
            self.view.caret_position()
        };

        let _ = Terminal::move_caret_to(new_caret_pos);
        let _ = Terminal::show_caret();
        let _ = Terminal::execute();
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("Goodbye.\r\n");
        }
    }
}
