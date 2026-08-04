pub const UNKNOWN: usize = 0;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgnCoordinate {
    pub file: Option<char>,
    pub rank: Option<i32>,
}

impl Default for PgnCoordinate {
    fn default() -> Self {
        PgnCoordinate {
            file: None,
            rank: None,
        }
    }
}

pub fn pgn_coordinate_file_as_index(file: char) -> i32 {
    let base = if file.is_ascii_uppercase() { 'A' } else { 'a' };
    (file as i32) - (base as i32)
}
