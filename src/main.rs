#![warn(clippy::all, clippy::pedantic, clippy::print_stdout)]

// Charger un module
mod editor;

// Charger une structure de ce module
use editor::Editor;

fn main() {
    print!("\x1b[2J");
    Editor::default().run();
}
