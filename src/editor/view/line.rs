use std::{cmp, ops::Range};
use unicode_segmentation::UnicodeSegmentation;

pub struct Line {
    pub string: String,
}

impl Line {
    pub fn from(line_str: &str) -> Self {
        Self {
            string: String::from(line_str),
        }
    }

    pub fn get(&self, range: Range<usize>) -> String {
        let start = range.start;
        let end = cmp::min(range.end, self.string.len());
        
        self.string.get(start..end).unwrap_or_default().to_string()
    }

    pub fn len(&self) -> usize {
        let graphemes = self.string.graphemes(true).collect::<Vec<&str>>();
        graphemes.len()
    }
}