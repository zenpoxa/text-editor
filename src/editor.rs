// Saisie utilisateur
use crossterm::event::{read, Event, Event::Key, KeyCode::Char, KeyEvent, KeyModifiers};
use std::io::Error;

mod terminal;
use terminal::{Terminal, Size, Position};

pub struct Editor {
    should_quit : bool
}

impl Editor {

    pub const fn default() -> Self {
        Self { should_quit : false }
    }

    pub fn run(&mut self) {
        Terminal::initialize().unwrap();
        let result = self.repl();
        Terminal::terminate().unwrap();
        result.unwrap(); // Peut importe s'il y a une erreur, on la vérifie à la fin (d'abord, faire initialize & terminate)
    }

    fn repl(&mut self) -> Result<(), Error> {

        loop {
            self.refresh_screen()?;
            if self.should_quit {
                break;
            }

            let event = read()?;
            self.evaluate_event(&event);
        }

        Ok(())
    }

    // Check if we must quit
    fn evaluate_event(&mut self, event : &Event) {
        if let Key(KeyEvent {
            code: Char('w'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) = event
        {
            self.should_quit = true;
        }
    }

    fn refresh_screen(&self) -> Result<(), Error> {
        Terminal::hide_cursor()?;
        
        if self.should_quit {
            Terminal::clear_screen()?;
            Terminal::print("Goodbye !")?;
        }
        else {
            Self::draw_rows()?;
            Terminal::move_cursor_to(Position{x: 0, y: 0})?;
        }

        Terminal::show_cursor()?;
        Terminal::execute()?;
        Ok(())
    }

    fn draw_rows() -> Result<(), Error> {
        let Size{height, ..} = Terminal::size()?;

        for current_row in 0..height {
            Terminal::clear_line()?;
            Terminal::print("~")?;
            if current_row + 1 < height {
                Terminal::print("\r\n")?;
            }
        }

        Ok(())
    }
}
