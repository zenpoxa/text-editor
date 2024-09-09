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
    System::{Dismiss, Quit, Resize, Save, Search},
};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUIT_TIMES: u8 = 3;

#[derive(Eq, PartialEq, Default)]
enum PromptType {
    Search,
    Save,
    #[default]
    None,
}

impl PromptType {
    fn is_none(&self) -> bool {
        *self == Self::None
    }
}

#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    view: View,
    status_bar: StatusBar,
    title: String,
    message_bar: MessageBar,
    command_bar: CommandBar,
    prompt_type: PromptType,
    terminal_size: Size,
    quit_times: u8,
}

impl Editor {

    // region: struct lifestyle
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        Terminal::initialize()?;

        let mut editor = Self::default();
        let size = Terminal::size().unwrap_or_default();
        
        editor.handle_resize_command(size);
        editor.update_message("HELP: Ctrl-F = Search | Ctrl-S = save | Ctrl-Q = quit");

        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            if editor.view.load(file_name).is_err() {
                editor.update_message(&format!("ERR: Could not open file: {file_name}"));
            }
        }
        editor.refresh_status();
        Ok(editor)
    }
    // endregion

    // region: Event loop
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
            self.refresh_status();
        }
    }

    fn refresh_screen(&mut self) {
        if self.terminal_size.height == 0 || self.terminal_size.width == 0 {
            return;
        }
        let bottom_bar_row = self.terminal_size.height.saturating_sub(1);
        let _ = Terminal::hide_caret();
        
        
        if self.in_prompt() {
            self.command_bar.render(bottom_bar_row);
        } else {
            self.message_bar.render(bottom_bar_row)
        }

        if self.terminal_size.height > 1 {
            self.status_bar.render(self.terminal_size.height.saturating_sub(2));
        }

        if self.terminal_size.height > 2 {
            self.view.render(0);
        }

        let new_caret_pos = if self.in_prompt() {
            Position {
                row: bottom_bar_row,
                col: self.command_bar.caret_position_col(),
            }
        } else {
            self.view.caret_position()
        };

        let _ = Terminal::move_caret_to(new_caret_pos);
        let _ = Terminal::show_caret();
        let _ = Terminal::execute();
    }
    pub fn refresh_status(&mut self) {
        let status = self.view.get_status();
        let title = format!("{} - {NAME}", status.file_name);
        self.status_bar.update_status(status);

        if title != self.title && matches!(Terminal::set_title(&title), Ok(())) {
            self.title = title;
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
    // endregion

    // region: Command Handling
    fn process_command(&mut self, command: Command) {
        if let System(Resize(size)) = command {
            self.handle_resize_command(size);
            return;
        }
        match self.prompt_type {
            PromptType::Search => self.process_command_during_search(command),
            PromptType::Save => self.process_command_during_save(command),
            PromptType::None => self.process_command_no_prompt(command),
        }
    }

    fn process_command_no_prompt(&mut self, command: Command) {
        if matches!(command, System(Quit)) {
            self.handle_quit_command();
            return;
        }

        self.reset_quit_times();
        
        match command {
            System(Quit | Resize(_) | Dismiss) => {}
            System(Search) => self.set_prompt(PromptType::Search),
            System(Save) => self.handle_save_command(),
            Edit(edit_command) => self.view.handle_edit_command(edit_command),
            Move(move_command) => self.view.handle_move_command(move_command),
        }

    }
    // endregion

    // region: resize command handling
    pub fn handle_resize_command(&mut self, size: Size) {
        self.terminal_size = size;
        self.view.resize(Size {
            height: size.height.saturating_sub(2), // status bar & message_bar heights combined
            width: size.width,
        });
        let bar_size = Size {
            height: 1,
            width: size.width,
        };
        self.message_bar.resize(bar_size);
        self.status_bar.resize(bar_size);
        self.command_bar.resize(bar_size);
    }
    // endregion

    // region: quit command handling
    #[allow(clippy::arithmetic_side_effects)]
    fn handle_quit_command(&mut self) {
        self.quit_times += 1;
        if !self.view.get_status().is_modified || self.quit_times == QUIT_TIMES {
            self.should_quit = true;
        }
        else {
            self.update_message(&format!(
                "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                QUIT_TIMES - self.quit_times
            ));
        }
    }
    fn reset_quit_times(&mut self) {
        if self.quit_times > 0 {
            self.quit_times = 0;
            self.update_message("");
        }
    }
    // endregion

    // region save command & prompt handling
    fn handle_save_command(&mut self) {
        if self.view.is_file_loaded() {
            self.save(None);
        } else {
            self.set_prompt(PromptType::Save);
        }
    }
    fn process_command_during_save(&mut self, command: Command) {
        match command {
            System(Quit | Resize(_) | Search | Save) | Move(_) => {} // Not applicable during save, Resize already handled at this stage
            System(Dismiss) => {
                self.set_prompt(PromptType::None);
                self.update_message("Save aborted.");
            }
            Edit(InsertNewline) => {
                let file_name = self.command_bar.value();
                self.save(Some(&file_name));
                self.set_prompt(PromptType::None);
            }
            Edit(edit_command) => self.command_bar.handle_edit_command(edit_command),
        }
    }
    fn save(&mut self, file_name: Option<&str>) {
        let result = if let Some(name) = file_name {
            self.view.save_as(name)
        } else {
            self.view.save()
        };
    
        if result.is_ok() {
            self.update_message("Fichier sauvegardé correctement.");
        } else {
            self.update_message("Impossible de sauvegarder le fichier.");
        }
    }
    // endregion

    // region search command & prompt handling
    fn process_command_during_search(&mut self, command: Command) {
        match command {
            System(Quit | Resize(_) | Search | Save) | Move(_) => {} // Not applicable during save, Resize already handled at this stage
            System(Dismiss) => {
                self.set_prompt(PromptType::None);
                self.view.dismiss_search();
            }
            Edit(InsertNewline) => {
                self.set_prompt(PromptType::None);
                self.view.exit_search();
            }
            Edit(edit_command) => {
                self.command_bar.handle_edit_command(edit_command);
                let query = self.command_bar.value();
                self.view.search(&query);
            }
        }
    }
    // endregion

    // region: message & command bar
    fn update_message(&mut self, new_message: &str) {
        self.message_bar.update_message(new_message);
    }
    // endregion    

    // region: prompt handling
    fn in_prompt(&self) -> bool {
        !self.prompt_type.is_none()
    }
    fn set_prompt(&mut self, prompt_type: PromptType) {
        match prompt_type {
            PromptType::None => self.message_bar.set_needs_redraw(true), // Prompt closed, needs to redraw
            PromptType::Save => self.command_bar.set_prompt("Enregistrer sous : "),
            PromptType::Search => {
                self.view.enter_search();
                self.command_bar.set_prompt("Rechercher (Esc pour annuler) : ");
            }
        }
        self.command_bar.clear_value();
        self.prompt_type = prompt_type;
    }
    // endregion
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("Goodbye.\r\n");
        }
    }
}
