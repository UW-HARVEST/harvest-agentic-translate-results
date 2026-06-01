use libpgn::coordinate::{pgn_coordinate_file_as_index, PgnCoordinate};

#[test]
fn test_coordinate_lower() {
    assert_eq!(pgn_coordinate_file_as_index('a'), 0);
    assert_eq!(pgn_coordinate_file_as_index('b'), 1);
    assert_eq!(pgn_coordinate_file_as_index('c'), 2);
    assert_eq!(pgn_coordinate_file_as_index('d'), 3);
    assert_eq!(pgn_coordinate_file_as_index('e'), 4);
    assert_eq!(pgn_coordinate_file_as_index('f'), 5);
    assert_eq!(pgn_coordinate_file_as_index('g'), 6);
    assert_eq!(pgn_coordinate_file_as_index('h'), 7);
}

#[test]
fn test_coordinate_upper() {
    assert_eq!(pgn_coordinate_file_as_index('A'), 0);
    assert_eq!(pgn_coordinate_file_as_index('H'), 7);
}

#[test]
fn test_coordinate_struct() {
    let c = PgnCoordinate { file: Some('e'), rank: Some(4) };
    assert_eq!(c.file, Some('e'));
    assert_eq!(c.rank, Some(4));

    let c2 = PgnCoordinate { file: None, rank: None };
    assert_eq!(c2.file, None);
    assert_eq!(c2.rank, None);
}

fn main() {}
