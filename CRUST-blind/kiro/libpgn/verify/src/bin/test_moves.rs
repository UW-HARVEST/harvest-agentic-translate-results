use libpgn::moves::{PgnMove, PgnMoves, PGN_CASTLING_NONE, PGN_CASTLING_KINGSIDE, PGN_CASTLING_QUEENSIDE};
use libpgn::piece::PgnPiece;
use libpgn::check::PgnCheck;
use libpgn::annotation::PgnAnnotation;

#[test]
fn test_move_simple_pawn() {
    let m = PgnMove::from("e4");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(4));
    assert!(!m.captures);
    assert_eq!(m.castles, PGN_CASTLING_NONE);
    assert_eq!(m.check, PgnCheck::None);
}

#[test]
fn test_move_piece_move() {
    let m = PgnMove::from("Nf3");
    assert_eq!(m.piece, PgnPiece::Knight);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(3));
}

#[test]
fn test_move_capture() {
    let m = PgnMove::from("Bxe5");
    assert_eq!(m.piece, PgnPiece::Bishop);
    assert!(m.captures);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(5));
}

#[test]
fn test_move_pawn_capture() {
    let m = PgnMove::from("exd5");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert!(m.captures);
    assert_eq!(m.from.file, Some('e'));
    assert_eq!(m.dest.file, Some('d'));
    assert_eq!(m.dest.rank, Some(5));
}

#[test]
fn test_move_kingside_castle() {
    let m = PgnMove::from("O-O");
    assert_eq!(m.castles, PGN_CASTLING_KINGSIDE);
}

#[test]
fn test_move_queenside_castle() {
    let m = PgnMove::from("O-O-O");
    assert_eq!(m.castles, PGN_CASTLING_QUEENSIDE);
}

#[test]
fn test_move_check() {
    let m = PgnMove::from("Qh5+");
    assert_eq!(m.piece, PgnPiece::Queen);
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.dest.file, Some('h'));
    assert_eq!(m.dest.rank, Some(5));
}

#[test]
fn test_move_checkmate() {
    let m = PgnMove::from("Qf7#");
    assert_eq!(m.check, PgnCheck::Mate);
}

#[test]
fn test_move_promotion_equals() {
    let m = PgnMove::from("e8=Q");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.promoted_to, PgnPiece::Queen);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(8));
}

#[test]
fn test_move_promotion_paren() {
    let m = PgnMove::from("e8(Q)");
    assert_eq!(m.promoted_to, PgnPiece::Queen);
}

#[test]
fn test_move_promotion_slash() {
    let m = PgnMove::from("e8/Q");
    assert_eq!(m.promoted_to, PgnPiece::Queen);
}

#[test]
fn test_move_disambiguation_file() {
    let m = PgnMove::from("Rae1");
    assert_eq!(m.piece, PgnPiece::Rook);
    assert_eq!(m.from.file, Some('a'));
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(1));
}

#[test]
fn test_move_disambiguation_rank() {
    let m = PgnMove::from("R1e1");
    assert_eq!(m.piece, PgnPiece::Rook);
    assert_eq!(m.from.rank, Some(1));
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(1));
}

#[test]
fn test_move_annotation() {
    let m = PgnMove::from("e4!");
    assert_eq!(m.annotation, PgnAnnotation::GoodMove);
}

#[test]
fn test_move_display_simple() {
    let m = PgnMove::from("e4");
    assert_eq!(m.to_string(), "e4");
}

#[test]
fn test_move_display_piece() {
    let m = PgnMove::from("Nf3");
    assert_eq!(m.to_string(), "Nf3");
}

#[test]
fn test_move_display_capture() {
    let m = PgnMove::from("Bxe5");
    assert_eq!(m.to_string(), "Bxe5");
}

#[test]
fn test_move_display_castle_kingside() {
    let m = PgnMove::from("O-O");
    assert_eq!(m.to_string(), "O-O");
}

#[test]
fn test_move_display_castle_queenside() {
    let m = PgnMove::from("O-O-O");
    assert_eq!(m.to_string(), "O-O-O");
}

#[test]
fn test_move_display_check() {
    let m = PgnMove::from("Qh5+");
    assert_eq!(m.to_string(), "Qh5+");
}

#[test]
fn test_move_display_promotion() {
    let m = PgnMove::from("e8=Q");
    assert_eq!(m.to_string(), "e8=Q");
}

#[test]
fn test_moves_from_string_simple() {
    let moves = PgnMoves::from("1. e4 e5 1-0");
    assert_eq!(moves.values.len(), 1);
    assert_eq!(moves.values[0].white.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].white.dest.file, Some('e'));
    assert_eq!(moves.values[0].white.dest.rank, Some(4));
    assert_eq!(moves.values[0].black.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].black.dest.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.rank, Some(5));
}

#[test]
fn test_moves_from_string_multiple() {
    let moves = PgnMoves::from("1. e4 e5 2. Nf3 Nc6 1-0");
    assert_eq!(moves.values.len(), 2);
    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
}

#[test]
fn test_moves_white_only_last() {
    // Game ends after white's move
    let moves = PgnMoves::from("1. e4 e5 2. Qh5 1-0");
    assert_eq!(moves.values.len(), 2);
    assert_eq!(moves.values[1].white.piece, PgnPiece::Queen);
}

#[test]
fn test_move_colon_capture() {
    // C code supports ':' as capture indicator
    let m = PgnMove::from("B:e5");
    assert!(m.captures);
    assert_eq!(m.piece, PgnPiece::Bishop);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(5));
}

#[test]
fn test_move_double_check() {
    let m = PgnMove::from("Nf7++");
    assert_eq!(m.check, PgnCheck::Double);
}

#[test]
fn test_move_castle_with_check() {
    let m = PgnMove::from("O-O+");
    assert_eq!(m.castles, PGN_CASTLING_KINGSIDE);
    assert_eq!(m.check, PgnCheck::Single);
}

fn main() {}
