// Saisie utilisateur
use crossterm::event::{read, Event::Key, KeyCode::Char};
use crossterm::terminal::disable_raw_mode;

pub struct Editor {

}

impl Editor {

    pub fn default() -> Self {
        Editor{}
    }

    pub fn run(&self) {
        loop {
            match read() {
                // Cas où key pressed
                Ok(Key(event)) => {
                    // Afficher ce sur quoi on vient de cliquer
                    println!("{event:?} \r");

                    // Condition d'arrêt
                    if let Char(c) = event.code {
                        if c == 'q' {
                            break;
                        }
                    }   
                },

                // Cas d'erreur
                Err(err) => println!("Error: {err}"),

                // Tous les autres événements (pas traités pour l'instant)
                _ => ()
            }
        }
        disable_raw_mode().unwrap(); 
    }
}
