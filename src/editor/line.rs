use std::{fmt, ops::Range};

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy)]
enum GraphemeWidth {
    Half,
    Full,
}

impl GraphemeWidth {
    const fn saturating_add(self, other: usize) -> usize {
        match self {
            Self::Half => other.saturating_add(1),
            Self::Full => other.saturating_add(2),
        }
    }
}

struct TextFragment {
    grapheme: String,
    rendered_width: GraphemeWidth,
    replacement: Option<char>,
}

#[derive(Default)]
pub struct Line {
    fragments: Vec<TextFragment>,
}

impl Line {
    pub fn from(line_str: &str) -> Self {
        let fragments = Self::str_to_fragments(line_str);
        Self { fragments }
    }

    fn str_to_fragments(line_str: &str) -> Vec<TextFragment> {

        line_str
        .graphemes(true)
        .map(|grapheme| {
            
            let (replacement, rendered_width) = Self::replacement_character(grapheme)
                .map_or_else(
                    || {
                        let unicode_width = grapheme.width();
                        let rendered_width = match unicode_width {
                            0 | 1 => GraphemeWidth::Half,
                            _ => GraphemeWidth::Full,
                        };
                        (None, rendered_width)
                    },
                    |replacement| (Some(replacement), GraphemeWidth::Half)
                );

            TextFragment {
                grapheme: grapheme.to_string(),
                rendered_width,
                replacement,
            }

        }).collect()
        
    }

    fn replacement_character(for_str: &str) -> Option<char> {
        let width = for_str.width();
        match for_str {
            " " => Some(' '),
            "\t" => Some(' '),
            _ if width > 0 && for_str.trim().is_empty() => Some('␣'),
            _ if width == 0 => {
                let mut chars = for_str.chars();
                if let Some(char) = chars.next() {
                    if char.is_control() && chars.next().is_none() {
                        return Some('▯');
                    }
                }
                return Some('·');
            }
            _ => None,
        }
    }

    pub fn get_visible_graphemes(&self, range: Range<usize>) -> String {
        if range.start >= range.end {
            return String::new();
        }
        
        let mut result = String::new();
        let mut current_pos = 0;

        for fragment in &self.fragments {
            let fragment_end = fragment.rendered_width.saturating_add(current_pos);
            
            // condition d'arrêt -> fin de la range voulue
            if current_pos >= range.end {
                break;
            }

            // seulement considérer les 'fragments' dans la range
            if fragment_end > range.start {
                // Edge case
                if fragment_end > range.end || current_pos < range.start {
                    result.push('⋯');
                } 
                else if let Some(char) = fragment.replacement {
                    // Besoin de mettre un caractère de remplacement
                    result.push(char);
                } else {
                    result.push_str(&fragment.grapheme);
                }
            }
            current_pos = fragment_end;
        }
        result
    }

    pub fn grapheme_count(&self) -> usize {
        self.fragments.len()
    }

    pub fn width_until(&self, grapheme_index: usize) -> usize {
        self.fragments
            .iter()
            .take(grapheme_index)
            .map(|fragment|{
                match fragment.rendered_width {
                    GraphemeWidth::Half => 1,
                    GraphemeWidth::Full => 2,
                }
            })
            .sum()
    }

    pub fn width(&self) -> usize {
        self.width_until(self.grapheme_count())
    }

    pub fn insert_char(&mut self, character: char, at: usize) {
        let mut result = String::new();

        for (index, fragment) in self.fragments.iter().enumerate() {
            if index == at {
                result.push(character)
            }
            result.push_str(&fragment.grapheme);
        }
        if at >= self.fragments.len() {
            result.push(character);
        }

        self.fragments = Self::str_to_fragments(&result);
    }
    pub fn append_char(&mut self, character: char) {
        self.insert_char(character, self.grapheme_count());
    }

    pub fn delete(&mut self, at: usize) {
        
        // nothing to do if deleting empty space at the end of the line
        if at >= self.grapheme_count() {
            return;
        }
        
        // rewriting everythin but the deleted character
        let mut result = String::new();
        for (index, fragment) in self.fragments.iter().enumerate() {
            if index != at {
                result.push_str(&fragment.grapheme);
            }
        }
        self.fragments = Self::str_to_fragments(&result);
    }
    pub fn delete_last(&mut self) {
        self.delete(self.grapheme_count().saturating_sub(1));
    }

    pub fn append(&mut self, other: &Self) {
        let mut concat = self.to_string();
        concat.push_str(&other.to_string());
        self.fragments = Self::str_to_fragments(&concat);
    }

    pub fn split(&mut self, at: usize) -> Self {
        if at > self.fragments.len() {
            return Self::default();
        }
        let remainder = self.fragments.split_off(at);
        Self {
            fragments: remainder,
        }

    }
}

impl fmt::Display for Line {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let result: String = self
            .fragments
            .iter()
            .map(|fragment| fragment.grapheme.clone())
            .collect();
        write!(formatter, "{result}")
    }
}
