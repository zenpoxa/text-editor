use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType};
use crossterm::queue;
use crossterm::cursor::{MoveTo, Hide, Show};
use crossterm::style::Print;

use std::io::{stdout, Error, Write};

pub struct Terminal {}

impl Terminal {

    pub fn initialize() -> Result<(), Error> {
        enable_raw_mode()?;
        Self::clear_screen()
    }
    pub fn terminate() -> Result<(), Error> {
        disable_raw_mode()
    }
    pub fn clear_screen() -> Result<(), Error> {
        let mut stdout = stdout();
        queue!(stdout, MoveTo(0,0))?;
        queue!(stdout, Clear(ClearType::All))
    }
    pub fn clear_line() -> Result<(), Error> {
        queue!(stdout(), Clear(ClearType::CurrentLine))
    }

    pub fn size() -> Result<Size, Error> {
        let (w, h) = size()?;
        Ok(
            Size {
                width: w,
                height: h
            }
        )
    }

    pub fn print(string: &str) -> Result<(), Error> {
        queue!(stdout(), Print(string))?;
        Ok(())
    }
    pub fn execute() -> Result<(), Error> {
        stdout().flush()?;
        Ok(())
    }
    
    pub fn move_cursor_to(pos: Position) -> Result<(), Error> {
        let Position{x, y} = pos;
        queue!(stdout(), MoveTo(x, y))?;
        Ok(())
    }
    pub fn hide_cursor() -> Result<(), Error>{
        queue!(stdout(), Hide)?;
        Ok(())
    }
    pub fn show_cursor() -> Result<(), Error>{
        queue!(stdout(), Show)?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct Size {
    pub width: u16,
    pub height: u16
}

#[derive(Clone, Copy)]

pub struct Position {
    pub x: u16,
    pub y: u16
}
