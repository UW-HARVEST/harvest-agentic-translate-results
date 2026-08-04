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
    assert_eq!(PgnPiece::from('1'), PgnPiece::Unknown);
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

#[test]
fn test_piece_repr_value() {
    assert_eq!(PgnPiece::Pawn as u8, b'P');
    assert_eq!(PgnPiece::Rook as u8, b'R');
    assert_eq!(PgnPiece::Knight as u8, b'N');
    assert_eq!(PgnPiece::Bishop as u8, b'B');
    assert_eq!(PgnPiece::Queen as u8, b'Q');
    assert_eq!(PgnPiece::King as u8, b'K');
    assert_eq!(PgnPiece::Unknown as u8, 0);
}

fn main() {}
