use libpgn::piece::PgnPiece;

#[test]
fn test_piece_from_char_valid() {
    assert_eq!(PgnPiece::from('P'), PgnPiece::Pawn);
    assert_eq!(PgnPiece::from('R'), PgnPiece::Rook);
    assert_eq!(PgnPiece::from('N'), PgnPiece::Knight);
    assert_eq!(PgnPiece::from('B'), PgnPiece::Bishop);
    assert_eq!(PgnPiece::from('Q'), PgnPiece::Queen);
    assert_eq!(PgnPiece::from('K'), PgnPiece::King);
}

#[test]
fn test_piece_from_char_unknown() {
    assert_eq!(PgnPiece::from('x'), PgnPiece::Unknown);
    assert_eq!(PgnPiece::from('a'), PgnPiece::Unknown);
    assert_eq!(PgnPiece::from('0'), PgnPiece::Unknown);
}

#[test]
fn test_piece_to_string() {
    assert_eq!(PgnPiece::Pawn.to_string(), "Pawn");
    assert_eq!(PgnPiece::Rook.to_string(), "Rook");
    assert_eq!(PgnPiece::Knight.to_string(), "Knight");
    assert_eq!(PgnPiece::Bishop.to_string(), "Bishop");
    assert_eq!(PgnPiece::Queen.to_string(), "Queen");
    assert_eq!(PgnPiece::King.to_string(), "King");
}

#[test]
fn test_piece_unknown_to_string_empty() {
    // C returns NULL for unknown; Rust Display returns empty string
    assert_eq!(PgnPiece::Unknown.to_string(), "");
}

fn main() {}
