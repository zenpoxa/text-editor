use crossterm::event::{read, Event, KeyEvent, KeyEventKind};
use std::{
    env,
    io::Error,
    panic::{set_hook, take_hook},
};
mod documentstatus;
mod editorcommand;
mod terminal;
mod view;
mod fileinfo;
mod statusbar;
use documentstatus::DocumentStatus;
use terminal::Terminal;
use view::View;
use editorcommand::EditorCommand;
use statusbar::StatusBar;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
pub struct Editor {
    should_quit: bool,
    view: View,
    status_bar: StatusBar,
    title: String,
}

impl Editor {
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        Terminal::initialize()?;

        let mut editor = Self {
            should_quit: false,
            view: View::new(2),
            status_bar: StatusBar::new(1),
            title: String::new(),
        };

        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            editor.view.load(file_name);
        }

        editor.refresh_status();

        Ok(editor)
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

    // needless_pass_by_value: Event is not huge, so there is not a
    // performance overhead in passing by value, and pattern matching in this
    // function would be needlessly complicated if we pass by reference here.
    #[allow(clippy::needless_pass_by_value)]
    fn evaluate_event(&mut self, event: Event) {
        
        let should_process = match &event {
            Event::Key(KeyEvent {kind, ..}) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if should_process {

            // Ici, seulement traiter les touches avec des commandes. Les touches sans commande associée (Err{}) ne feront rien
            if let Ok(command) = EditorCommand::try_from(event) {
                // devoir quitter
                if matches!(command, EditorCommand::Quit) {
                    self.should_quit = true;
                }
                // toute autre commande
                else {
                    self.view.handle_command(command);
                    if let EditorCommand::Resize(size) = command {
                        self.status_bar.resize(size);
                    }
                }
            }
        }
        // A EVITER POUR WINDOWS (car l'événement 'Release' n'est pas pris en compte)
        else if !(cfg!(windows)) {
            #[cfg(debug_assertions)]
            {
                panic!("Received and discarded unsupported or non-press event.");
            }
        }

    }
    fn refresh_screen(&mut self) {
        let _ = Terminal::hide_caret();
        self.view.render();
        self.status_bar.render();
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
