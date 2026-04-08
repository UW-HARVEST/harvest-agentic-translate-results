use libpgn::piece::PgnPiece;

#[test]
fn test_piece_from_char() {
    assert_eq!(PgnPiece::from('P'), PgnPiece::Pawn);
    assert_eq!(PgnPiece::from('R'), PgnPiece::Rook);
    assert_eq!(PgnPiece::from('N'), PgnPiece::Knight);
    assert_eq!(PgnPiece::from('B'), PgnPiece::Bishop);
    assert_eq!(PgnPiece::from('Q'), PgnPiece::Queen);
    assert_eq!(PgnPiece::from('K'), PgnPiece::King);
    assert_eq!(PgnPiece::from('x'), PgnPiece::Unknown);
    assert_eq!(PgnPiece::from('a'), PgnPiece::Unknown);
}

#[test]
fn test_piece_display() {
    assert_eq!(format!("{}", PgnPiece::Pawn), "Pawn");
    assert_eq!(format!("{}", PgnPiece::Rook), "Rook");
    assert_eq!(format!("{}", PgnPiece::Knight), "Knight");
    assert_eq!(format!("{}", PgnPiece::Bishop), "Bishop");
    assert_eq!(format!("{}", PgnPiece::Queen), "Queen");
    assert_eq!(format!("{}", PgnPiece::King), "King");
    assert_eq!(format!("{}", PgnPiece::Unknown), "");
}

fn main() {}
