use libpgn::coordinate::pgn_coordinate_file_as_index;

#[test]
fn test_file_as_index_lowercase() {
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
fn test_file_as_index_uppercase() {
    assert_eq!(pgn_coordinate_file_as_index('A'), 0);
    assert_eq!(pgn_coordinate_file_as_index('B'), 1);
    assert_eq!(pgn_coordinate_file_as_index('C'), 2);
    assert_eq!(pgn_coordinate_file_as_index('D'), 3);
    assert_eq!(pgn_coordinate_file_as_index('E'), 4);
    assert_eq!(pgn_coordinate_file_as_index('F'), 5);
    assert_eq!(pgn_coordinate_file_as_index('G'), 6);
    assert_eq!(pgn_coordinate_file_as_index('H'), 7);
}

fn main() {}
