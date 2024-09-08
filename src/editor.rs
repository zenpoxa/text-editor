use crossterm::event::{read, Event, KeyEvent, KeyEventKind};
use std::{
    env, io::Error, panic::{set_hook, take_hook}
};
mod command;
mod messagebar;
mod uicomponent;
mod documentstatus;
mod terminal;
mod view;
mod fileinfo;
mod statusbar;
use documentstatus::DocumentStatus;
use terminal::Terminal;
use view::View;
use statusbar::StatusBar;
use uicomponent::UIComponent;

use self:: {
    command::{
        Command::{self, Edit, Move, System},
        System::{Quit, Resize, Save}
    },
    messagebar::MessageBar,
    terminal::Size,
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
            System(Quit) => self.handle_quit(),
            System(Resize(size)) => self.resize(size),
            _ => self.reset_quit_times(),
        }
        // Other commands
        match command {
            System(Quit | Resize(_)) => {}
            System(Save) => self.handle_save(),
            Edit(edit_command) => self.view.handle_edit_command(edit_command),
            Move(move_command) => self.view.handle_move_command(move_command),            
        }
    }

    fn handle_save(&mut self) {
        if self.view.save().is_ok() {
            self.message_bar.update_message("Fichier sauvegardé correctement");
        }
        else {
            self.message_bar.update_message("Impossible de sauvegarder dans ce fichier");
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

        let _ = Terminal::hide_caret();

        // height at least 1 -> Render message bar
        self.message_bar.render(self.terminal_size.height.saturating_sub(1));
        // height at least 2 -> Render status bar
        if self.terminal_size.height > 1 {
            self.status_bar.render(self.terminal_size.height.saturating_sub(2));
        }
        // height at least 3 -> Render view also
        if self.terminal_size.height > 2 {
            self.view.render(0);
        }

        let _ = Terminal::move_caret_to(self.view.caret_position());
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
