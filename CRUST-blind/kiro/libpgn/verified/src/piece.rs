use std::fmt::Display;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PgnPiece {
    Unknown = 0,
    Pawn = b'P',
    Rook = b'R',
    Knight = b'N',
    Bishop = b'B',
    Queen = b'Q',
    King = b'K',
}
impl From<char> for PgnPiece {
    fn from(ch: char) -> Self {
        match ch {
            'P' => PgnPiece::Pawn,
            'R' => PgnPiece::Rook,
            'N' => PgnPiece::Knight,
            'B' => PgnPiece::Bishop,
            'Q' => PgnPiece::Queen,
            'K' => PgnPiece::King,
            _ => PgnPiece::Unknown,
        }
    }
}
impl Display for PgnPiece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PgnPiece::Pawn => write!(f, "Pawn"),
            PgnPiece::Rook => write!(f, "Rook"),
            PgnPiece::Knight => write!(f, "Knight"),
            PgnPiece::Bishop => write!(f, "Bishop"),
            PgnPiece::Queen => write!(f, "Queen"),
            PgnPiece::King => write!(f, "King"),
            PgnPiece::Unknown => Ok(()),
        }
    }
}
