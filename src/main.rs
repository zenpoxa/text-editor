// Charger un module
mod editor;

// Charger une structure de ce module
use editor::Editor;

fn main() {

    let editor = Editor::default();
    editor.run();
}
