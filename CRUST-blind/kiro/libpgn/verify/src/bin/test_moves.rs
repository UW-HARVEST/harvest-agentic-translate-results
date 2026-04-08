use libpgn::moves::{PgnMove, PgnMoves, PGN_CASTLING_NONE, PGN_CASTLING_KINGSIDE, PGN_CASTLING_QUEENSIDE};
use libpgn::piece::PgnPiece;
use libpgn::check::PgnCheck;
use libpgn::annotation::PgnAnnotation;
use libpgn::comments::PgnCommentPosition;

#[test]
fn test_parse_pawn_move_with_blunder() {
    let mv = PgnMove::from("e4??");
    assert_eq!(mv.piece, PgnPiece::Pawn);
    assert_eq!(mv.captures, false);
    assert_eq!(mv.from.file, None);
    assert_eq!(mv.from.rank, None);
    assert_eq!(mv.dest.file, Some('e'));
    assert_eq!(mv.dest.rank, Some(4));
    assert_eq!(mv.check, PgnCheck::None);
    assert_eq!(mv.notation, "e4??");
    assert_eq!(mv.annotation, PgnAnnotation::Blunder);
}

#[test]
fn test_parse_knight_ambiguous_with_check() {
    let mv = PgnMove::from("Nb6d5+");
    assert_eq!(mv.piece, PgnPiece::Knight);
    assert_eq!(mv.captures, false);
    assert_eq!(mv.from.file, Some('b'));
    assert_eq!(mv.from.rank, Some(6));
    assert_eq!(mv.dest.file, Some('d'));
    assert_eq!(mv.dest.rank, Some(5));
    assert_eq!(mv.check, PgnCheck::Single);
    assert_eq!(mv.notation, "Nb6d5+");
    assert_eq!(mv.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_parse_queen_dubious() {
    let mv = PgnMove::from("Qf1?!");
    assert_eq!(mv.piece, PgnPiece::Queen);
    assert_eq!(mv.captures, false);
    assert_eq!(mv.from.file, None);
    assert_eq!(mv.from.rank, None);
    assert_eq!(mv.dest.file, Some('f'));
    assert_eq!(mv.dest.rank, Some(1));
    assert_eq!(mv.check, PgnCheck::None);
    assert_eq!(mv.notation, "Qf1?!");
    assert_eq!(mv.annotation, PgnAnnotation::DubiousMove);
}

#[test]
fn test_parse_pawn_capture_promotion() {
    let mv = PgnMove::from("bxa8=Q??");
    assert_eq!(mv.piece, PgnPiece::Pawn);
    assert_eq!(mv.captures, true);
    assert_eq!(mv.from.file, Some('b'));
    assert_eq!(mv.from.rank, None);
    assert_eq!(mv.dest.file, Some('a'));
    assert_eq!(mv.dest.rank, Some(8));
    assert_eq!(mv.check, PgnCheck::None);
    assert_eq!(mv.promoted_to, PgnPiece::Queen);
    assert_eq!(mv.notation, "bxa8=Q??");
    assert_eq!(mv.annotation, PgnAnnotation::Blunder);
}

#[test]
fn test_parse_bishop_capture() {
    let mv = PgnMove::from("Bxg2");
    assert_eq!(mv.piece, PgnPiece::Bishop);
    assert_eq!(mv.captures, true);
    assert_eq!(mv.from.file, None);
    assert_eq!(mv.from.rank, None);
    assert_eq!(mv.dest.file, Some('g'));
    assert_eq!(mv.dest.rank, Some(2));
    assert_eq!(mv.check, PgnCheck::None);
    assert_eq!(mv.promoted_to, PgnPiece::Unknown);
    assert_eq!(mv.notation, "Bxg2");
    assert_eq!(mv.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_parse_pawn_capture_check_good() {
    let mv = PgnMove::from("exf2+!");
    assert_eq!(mv.piece, PgnPiece::Pawn);
    assert_eq!(mv.captures, true);
    assert_eq!(mv.from.file, Some('e'));
    assert_eq!(mv.from.rank, None);
    assert_eq!(mv.dest.file, Some('f'));
    assert_eq!(mv.dest.rank, Some(2));
    assert_eq!(mv.check, PgnCheck::Single);
    assert_eq!(mv.promoted_to, PgnPiece::Unknown);
    assert_eq!(mv.notation, "exf2+!");
    assert_eq!(mv.annotation, PgnAnnotation::GoodMove);
}

#[test]
fn test_parse_kingside_castle_check_brilliant() {
    let mv = PgnMove::from("O-O+!!");
    assert_eq!(mv.castles, PGN_CASTLING_KINGSIDE);
    assert_eq!(mv.check, PgnCheck::Single);
    assert_eq!(mv.notation, "O-O+!!");
    assert_eq!(mv.annotation, PgnAnnotation::BrilliantMove);
}

#[test]
fn test_parse_queenside_castle() {
    let mv = PgnMove::from("O-O-O");
    assert_eq!(mv.castles, PGN_CASTLING_QUEENSIDE);
    assert_eq!(mv.check, PgnCheck::None);
    assert_eq!(mv.notation, "O-O-O");
    assert_eq!(mv.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_parse_en_passant() {
    let mv = PgnMove::from("exd6 e.p.");
    assert_eq!(mv.piece, PgnPiece::Pawn);
    assert_eq!(mv.en_passant, true);
    assert_eq!(mv.from.file, Some('e'));
    assert_eq!(mv.dest.file, Some('d'));
    assert_eq!(mv.dest.rank, Some(6));
    assert_eq!(mv.captures, true);
    assert_eq!(mv.notation, "exd6 e.p.");
}

#[test]
fn test_moves_basic() {
    let moves = PgnMoves::from("1.e4 e5");
    assert_eq!(moves.values[0].white.notation, "e4");
    assert_eq!(moves.values[0].white.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].white.dest.file, Some('e'));
    assert_eq!(moves.values[0].white.dest.rank, Some(4));
    assert_eq!(moves.values[0].white.captures, false);
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Unknown);
    assert_eq!(moves.values[0].black.notation, "e5");
    assert_eq!(moves.values[0].black.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].black.dest.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.rank, Some(5));
    assert_eq!(moves.values[0].black.captures, false);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_moves_with_annotations() {
    let moves = PgnMoves::from("1. a4?? a5!");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Blunder);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::GoodMove);
    assert_eq!(moves.values[0].white.notation, "a4??");
    assert_eq!(moves.values[0].black.notation, "a5!");
}

#[test]
fn test_moves_with_nag_annotations() {
    // NAG $69 and $420 map to Null in Rust (C stores raw int)
    let moves = PgnMoves::from("1. a4 $69 a5 $420");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Null);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::Null);
    assert_eq!(moves.values[0].white.notation, "a4 $69");
    assert_eq!(moves.values[0].black.notation, "a5 $420");
}

#[test]
fn test_moves_nag_multi() {
    // "6. Nce2 $2 e5 $0 $19 {}" - white gets $2=Mistake, black gets last NAG $19=Null
    let moves = PgnMoves::from("6. Nce2 $2 e5 $0 $19 {}");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Mistake);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::Null);
    assert_eq!(moves.values[0].white.notation, "Nce2 $2");
    assert_eq!(moves.values[0].black.notation, "e5 $0 $19");
}

#[test]
fn test_moves_with_black_move_number() {
    let moves = PgnMoves::from("69.Be4 69... Rxe5?!");
    assert_eq!(moves.values[0].white.piece, PgnPiece::Bishop);
    assert_eq!(moves.values[0].white.dest.file, Some('e'));
    assert_eq!(moves.values[0].white.dest.rank, Some(4));
    assert_eq!(moves.values[0].white.captures, false);
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Unknown);
    assert_eq!(moves.values[0].black.piece, PgnPiece::Rook);
    assert_eq!(moves.values[0].black.dest.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.rank, Some(5));
    assert_eq!(moves.values[0].black.captures, true);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::DubiousMove);
}

#[test]
fn test_moves_with_comment() {
    let moves = PgnMoves::from("9.e4 { This is a comment :O } e5 10. Nc3 Nc6");
    let comments = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(comments.values.len(), 1);
    assert_eq!(comments.values[0].value(), " This is a comment :O ");
    assert_eq!(comments.values[0].position, PgnCommentPosition::AfterMove);
    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].white.dest.file, Some('c'));
    assert_eq!(moves.values[1].white.dest.rank, Some(3));
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.dest.file, Some('c'));
    assert_eq!(moves.values[1].black.dest.rank, Some(6));
}

#[test]
fn test_moves_with_white_alternatives() {
    let moves = PgnMoves::from("69.Be4 ( 69. Be2 69... e4 ) 69... Rxe5?!");
    assert_eq!(moves.values.len(), 1);
    let alt = moves.values[0].white.alternatives.as_ref().unwrap();
    assert_eq!(alt.values.len(), 1);
    assert_eq!(alt.values[0].values[0].white.piece, PgnPiece::Bishop);
    assert_eq!(alt.values[0].values[0].white.dest.file, Some('e'));
    assert_eq!(alt.values[0].values[0].white.dest.rank, Some(2));
    assert_eq!(alt.values[0].values[0].black.piece, PgnPiece::Pawn);
    assert_eq!(alt.values[0].values[0].black.dest.file, Some('e'));
    assert_eq!(alt.values[0].values[0].black.dest.rank, Some(4));
}

#[test]
fn test_moves_with_black_alternatives() {
    let moves = PgnMoves::from("69.Be4 69... Rxe5?! ( 69... e5 70. e4 )");
    let alt = moves.values[0].black.alternatives.as_ref().unwrap();
    assert_eq!(alt.values.len(), 1);
    assert_eq!(alt.values[0].values[0].black.piece, PgnPiece::Pawn);
    assert_eq!(alt.values[0].values[0].black.dest.file, Some('e'));
    assert_eq!(alt.values[0].values[0].black.dest.rank, Some(5));
    assert_eq!(alt.values[0].values[1].white.piece, PgnPiece::Pawn);
    assert_eq!(alt.values[0].values[1].white.dest.file, Some('e'));
    assert_eq!(alt.values[0].values[1].white.dest.rank, Some(4));
}

#[test]
fn test_moves_alternatives_with_continuation() {
    let moves = PgnMoves::from("1. e4 (1. f4? e5! 2. g4?? Qh4#) e5 2. Nc6 Nf4");
    assert_eq!(moves.values.len(), 2);
    let alt = moves.values[0].white.alternatives.as_ref().unwrap();
    assert_eq!(alt.values.len(), 1);
    assert_eq!(alt.values[0].values[0].white.notation, "f4?");
    assert_eq!(alt.values[0].values[0].black.notation, "e5!");
    assert_eq!(alt.values[0].values[1].white.notation, "g4??");
    assert_eq!(alt.values[0].values[1].black.notation, "Qh4#");
    assert_eq!(moves.values[1].white.notation, "Nc6");
    assert_eq!(moves.values[1].black.notation, "Nf4");
}

#[test]
fn test_moves_two_alternatives() {
    let moves = PgnMoves::from("1. e4 (1. f4? e5! 2. g4?? Qh4#) (1. e4 f6? 2. d4 g5?? 3. Qh5#) e5");
    assert_eq!(moves.values.len(), 1);
    let alt = moves.values[0].white.alternatives.as_ref().unwrap();
    assert_eq!(alt.values.len(), 2);
    assert_eq!(alt.values[0].values[0].white.notation, "f4?");
    assert_eq!(alt.values[0].values[0].black.notation, "e5!");
    assert_eq!(alt.values[1].values[0].white.notation, "e4");
    assert_eq!(alt.values[1].values[0].black.notation, "f6?");
    assert_eq!(alt.values[1].values[2].white.notation, "Qh5#");
}

#[test]
fn test_ambiguate_moves() {
    let moves = PgnMoves::from("1. Rdf8 R1a3");
    assert_eq!(moves.values[0].white.notation, "Rdf8");
    assert_eq!(moves.values[0].white.from.file, Some('d'));
    assert_eq!(moves.values[0].white.from.rank, None);
    assert_eq!(moves.values[0].white.dest.file, Some('f'));
    assert_eq!(moves.values[0].white.dest.rank, Some(8));

    assert_eq!(moves.values[0].black.notation, "R1a3");
    assert_eq!(moves.values[0].black.from.file, None);
    assert_eq!(moves.values[0].black.from.rank, Some(1));
    assert_eq!(moves.values[0].black.dest.file, Some('a'));
    assert_eq!(moves.values[0].black.dest.rank, Some(3));
}

#[test]
fn test_ambiguate_full() {
    let moves = PgnMoves::from("1. Qh4e1");
    assert_eq!(moves.values[0].white.notation, "Qh4e1");
    assert_eq!(moves.values[0].white.from.file, Some('h'));
    assert_eq!(moves.values[0].white.from.rank, Some(4));
    assert_eq!(moves.values[0].white.dest.file, Some('e'));
    assert_eq!(moves.values[0].white.dest.rank, Some(1));
}

#[test]
fn test_move_display_basic() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::Unknown;
    mv.piece = PgnPiece::Pawn;
    mv.dest.file = Some('e');
    mv.dest.rank = Some(4);
    assert_eq!(format!("{}", mv), "e4");
}

#[test]
fn test_move_display_rook() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::Unknown;
    mv.piece = PgnPiece::Rook;
    mv.dest.file = Some('e');
    mv.dest.rank = Some(4);
    assert_eq!(format!("{}", mv), "Re4");
}

#[test]
fn test_move_display_mate() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::Unknown;
    mv.piece = PgnPiece::Rook;
    mv.check = PgnCheck::Mate;
    mv.dest.file = Some('e');
    mv.dest.rank = Some(4);
    assert_eq!(format!("{}", mv), "Re4#");
}

#[test]
fn test_move_display_castle_mate() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::Unknown;
    mv.castles = PGN_CASTLING_KINGSIDE;
    mv.check = PgnCheck::Mate;
    assert_eq!(format!("{}", mv), "O-O#");
}

#[test]
fn test_move_display_interesting() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::InterestingMove;
    mv.piece = PgnPiece::Rook;
    mv.dest.file = Some('a');
    mv.dest.rank = Some(3);
    assert_eq!(format!("{}", mv), "Ra3!?");
}

#[test]
fn test_move_display_interesting_ep() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::InterestingMove;
    mv.en_passant = true;
    mv.piece = PgnPiece::Pawn;
    mv.dest.file = Some('f');
    mv.dest.rank = Some(3);
    assert_eq!(format!("{}", mv), "f3!? e.p.");
}

#[test]
fn test_move_display_capture() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::Unknown;
    mv.piece = PgnPiece::Pawn;
    mv.captures = true;
    mv.dest.file = Some('f');
    mv.dest.rank = Some(3);
    assert_eq!(format!("{}", mv), "xf3");
}

#[test]
fn test_move_display_capture_promotion_double_check() {
    let mut mv = PgnMove::default();
    mv.annotation = PgnAnnotation::Unknown;
    mv.piece = PgnPiece::Pawn;
    mv.captures = true;
    mv.dest.file = Some('f');
    mv.dest.rank = Some(8);
    mv.promoted_to = PgnPiece::Queen;
    mv.check = PgnCheck::Double;
    assert_eq!(format!("{}", mv), "xf8=Q++");
}

#[test]
fn test_moves_en_passant_sequence() {
    let moves = PgnMoves::from("1. d4 exd3 e.p. 2.  Nc4 Nc6 3. cxb3 e.p. 3... Be4");
    assert_eq!(moves.values[0].white.notation, "d4");
    assert_eq!(moves.values[0].white.en_passant, false);
    assert_eq!(moves.values[0].black.notation, "exd3 e.p.");
    assert_eq!(moves.values[0].black.en_passant, true);
    assert_eq!(moves.values[0].black.from.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.file, Some('d'));
    assert_eq!(moves.values[0].black.dest.rank, Some(3));

    assert_eq!(moves.values[1].white.notation, "Nc4");
    assert_eq!(moves.values[1].white.en_passant, false);
    assert_eq!(moves.values[1].black.notation, "Nc6");
    assert_eq!(moves.values[1].black.en_passant, false);

    assert_eq!(moves.values[2].white.notation, "cxb3 e.p.");
    assert_eq!(moves.values[2].white.en_passant, true);
    assert_eq!(moves.values[2].white.from.file, Some('c'));
    assert_eq!(moves.values[2].white.dest.file, Some('b'));
    assert_eq!(moves.values[2].white.dest.rank, Some(3));
    assert_eq!(moves.values[2].black.notation, "Be4");
    assert_eq!(moves.values[2].black.en_passant, false);
}

#[test]
fn test_moves_with_nag_standard() {
    let moves = PgnMoves::from("1.e4 $2 e5 $1");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Mistake);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::GoodMove);
}

#[test]
fn test_moves_alternative_with_white_only() {
    let moves = PgnMoves::from("1.c4 ( 1.e4  ) c5 2.Nc3 Nc6");
    assert_eq!(moves.values[0].white.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[0].white.dest.file, Some('c'));
    assert_eq!(moves.values[0].white.dest.rank, Some(4));
    let alt = moves.values[0].white.alternatives.as_ref().unwrap();
    assert_eq!(alt.values[0].values[0].white.piece, PgnPiece::Pawn);
    assert_eq!(alt.values[0].values[0].white.dest.file, Some('e'));
    assert_eq!(alt.values[0].values[0].white.dest.rank, Some(4));
    assert_eq!(moves.values[0].black.dest.file, Some('c'));
    assert_eq!(moves.values[0].black.dest.rank, Some(5));
    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].white.dest.file, Some('c'));
    assert_eq!(moves.values[1].white.dest.rank, Some(3));
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.dest.file, Some('c'));
    assert_eq!(moves.values[1].black.dest.rank, Some(6));
}

fn main() {}
