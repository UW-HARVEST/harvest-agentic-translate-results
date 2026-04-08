use libpgn::coordinate::{pgn_coordinate_file_as_index, PgnCoordinate, UNKNOWN};

#[test]
fn test_unknown_constant() {
    assert_eq!(UNKNOWN, 0);
}

#[test]
fn test_file_as_index_lowercase() {
    assert_eq!(pgn_coordinate_file_as_index('a'), 0);
    assert_eq!(pgn_coordinate_file_as_index('b'), 1);
    assert_eq!(pgn_coordinate_file_as_index('h'), 7);
}

#[test]
fn test_file_as_index_uppercase() {
    assert_eq!(pgn_coordinate_file_as_index('A'), 0);
    assert_eq!(pgn_coordinate_file_as_index('B'), 1);
    assert_eq!(pgn_coordinate_file_as_index('H'), 7);
}

#[test]
fn test_coordinate_struct() {
    let c = PgnCoordinate { file: Some('e'), rank: Some(4) };
    assert_eq!(c.file, Some('e'));
    assert_eq!(c.rank, Some(4));
}

#[test]
fn test_coordinate_default_none() {
    let c = PgnCoordinate { file: None, rank: None };
    assert_eq!(c.file, None);
    assert_eq!(c.rank, None);
}

fn main() {}
