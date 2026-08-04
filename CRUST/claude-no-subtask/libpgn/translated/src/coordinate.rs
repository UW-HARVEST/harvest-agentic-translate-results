pub const UNKNOWN: usize = 0;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PgnCoordinate {
    pub file: Option<char>,
    pub rank: Option<i32>,
}
pub fn pgn_coordinate_file_as_index(file: char) -> i32 {
    if file.is_ascii_uppercase() {
        (file as i32) - ('A' as i32)
    } else {
        (file as i32) - ('a' as i32)
    }
}
