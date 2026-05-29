#[allow(unused_imports)]
use libpgn::annotation::PgnAnnotation;
#[allow(unused_imports)]
use libpgn::check::PgnCheck;
#[allow(unused_imports)]
use libpgn::moves::{
    PgnMove, PgnMoves, PGN_CASTLING_KINGSIDE, PGN_CASTLING_NONE, PGN_CASTLING_QUEENSIDE,
};
#[allow(unused_imports)]
use libpgn::piece::PgnPiece;

#[test]
fn test_parse_pawn_move() {
    let m = PgnMove::from("e4??");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, false);
    assert_eq!(m.en_passant, false);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(4));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.castles, PGN_CASTLING_NONE);
    assert_eq!(m.promoted_to, PgnPiece::Unknown);
    assert_eq!(m.annotation, PgnAnnotation::Blunder);
    assert_eq!(m.notation, "e4??");
}

#[test]
fn test_parse_knight_disambig_check() {
    let m = PgnMove::from("Nb6d5+");
    assert_eq!(m.piece, PgnPiece::Knight);
    assert_eq!(m.captures, false);
    assert_eq!(m.from.file, Some('b'));
    assert_eq!(m.from.rank, Some(6));
    assert_eq!(m.dest.file, Some('d'));
    assert_eq!(m.dest.rank, Some(5));
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
    assert_eq!(m.notation, "Nb6d5+");
}

#[test]
fn test_parse_queen_dubious() {
    let m = PgnMove::from("Qf1?!");
    assert_eq!(m.piece, PgnPiece::Queen);
    assert_eq!(m.captures, false);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(1));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.annotation, PgnAnnotation::DubiousMove);
    assert_eq!(m.notation, "Qf1?!");
}

#[test]
fn test_parse_pawn_promotion_capture() {
    let m = PgnMove::from("bxa8=Q??");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, true);
    assert_eq!(m.from.file, Some('b'));
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('a'));
    assert_eq!(m.dest.rank, Some(8));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.promoted_to, PgnPiece::Queen);
    assert_eq!(m.annotation, PgnAnnotation::Blunder);
    assert_eq!(m.notation, "bxa8=Q??");
}

#[test]
fn test_parse_bishop_capture() {
    let m = PgnMove::from("Bxg2");
    assert_eq!(m.piece, PgnPiece::Bishop);
    assert_eq!(m.captures, true);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('g'));
    assert_eq!(m.dest.rank, Some(2));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.promoted_to, PgnPiece::Unknown);
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
    assert_eq!(m.notation, "Bxg2");
}

#[test]
fn test_parse_pawn_disambig_capture_check_good() {
    let m = PgnMove::from("exf2+!");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, true);
    assert_eq!(m.from.file, Some('e'));
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(2));
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.promoted_to, PgnPiece::Unknown);
    assert_eq!(m.annotation, PgnAnnotation::GoodMove);
    assert_eq!(m.notation, "exf2+!");
}

#[test]
fn test_parse_kingside_castle() {
    let m = PgnMove::from("O-O+!!");
    assert_eq!(m.castles, PGN_CASTLING_KINGSIDE);
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.annotation, PgnAnnotation::BrilliantMove);
    assert_eq!(m.notation, "O-O+!!");
    assert_eq!(m.piece, PgnPiece::Unknown);
}

#[test]
fn test_parse_queenside_castle() {
    let m = PgnMove::from("O-O-O");
    assert_eq!(m.castles, PGN_CASTLING_QUEENSIDE);
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
    assert_eq!(m.notation, "O-O-O");
    assert_eq!(m.piece, PgnPiece::Unknown);
}

#[test]
fn test_parse_simple_pawn() {
    let m = PgnMove::from("e4");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(4));
    assert_eq!(m.captures, false);
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.notation, "e4");
}

#[test]
fn test_parse_en_passant() {
    let m = PgnMove::from("exd6 e.p.");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, true);
    assert_eq!(m.en_passant, true);
    assert_eq!(m.from.file, Some('e'));
    assert_eq!(m.dest.file, Some('d'));
    assert_eq!(m.dest.rank, Some(6));
    assert_eq!(m.notation, "exd6 e.p.");
}

#[test]
fn test_parse_disambig_file_only() {
    let m = PgnMove::from("Rdf8");
    assert_eq!(m.piece, PgnPiece::Rook);
    assert_eq!(m.from.file, Some('d'));
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(8));
    assert_eq!(m.notation, "Rdf8");
}

#[test]
fn test_parse_disambig_rank_only() {
    let m = PgnMove::from("R1a3");
    assert_eq!(m.piece, PgnPiece::Rook);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, Some(1));
    assert_eq!(m.dest.file, Some('a'));
    assert_eq!(m.dest.rank, Some(3));
    assert_eq!(m.notation, "R1a3");
}

#[test]
fn test_parse_disambig_full() {
    let m = PgnMove::from("Qh4e1");
    assert_eq!(m.piece, PgnPiece::Queen);
    assert_eq!(m.from.file, Some('h'));
    assert_eq!(m.from.rank, Some(4));
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(1));
    assert_eq!(m.notation, "Qh4e1");
}

#[test]
fn test_parse_nag_annotation_after_move() {
    let m = PgnMove::from("f3 $9");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(3));
    assert_eq!(m.annotation, PgnAnnotation(9));
    assert_eq!(m.notation, "f3 $9");
}

#[test]
fn test_parse_nag_annotation_after_ep() {
    let m = PgnMove::from("f3 e.p. $9");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(3));
    assert_eq!(m.en_passant, true);
    assert_eq!(m.annotation, PgnAnnotation(9));
    assert_eq!(m.notation, "f3 e.p. $9");
}

#[test]
fn test_parse_moves_simple() {
    let moves = PgnMoves::from("1.e4 e5");
    assert_eq!(moves.values.len(), 1);
    let w = &moves.values[0].white;
    let b = &moves.values[0].black;
    assert_eq!(w.notation, "e4");
    assert_eq!(w.piece, PgnPiece::Pawn);
    assert_eq!(w.dest.file, Some('e'));
    assert_eq!(w.dest.rank, Some(4));
    assert_eq!(w.captures, false);
    assert_eq!(w.annotation, PgnAnnotation::Unknown);
    assert_eq!(b.notation, "e5");
    assert_eq!(b.piece, PgnPiece::Pawn);
    assert_eq!(b.dest.file, Some('e'));
    assert_eq!(b.dest.rank, Some(5));
    assert_eq!(b.captures, false);
    assert_eq!(b.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_parse_moves_with_nag() {
    let moves = PgnMoves::from("1.e4 $2 e5 $1");
    let w = &moves.values[0].white;
    let b = &moves.values[0].black;
    assert_eq!(w.piece, PgnPiece::Pawn);
    assert_eq!(w.dest.file, Some('e'));
    assert_eq!(w.dest.rank, Some(4));
    assert_eq!(w.annotation, PgnAnnotation(2));
    assert_eq!(b.piece, PgnPiece::Pawn);
    assert_eq!(b.dest.file, Some('e'));
    assert_eq!(b.dest.rank, Some(5));
    assert_eq!(b.annotation, PgnAnnotation(1));
}

#[test]
fn test_parse_moves_arbitrary_nag() {
    // From C tests/annotation.c
    let moves = PgnMoves::from("1. a4 $69 a5 $420");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation(69));
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation(420));
    assert_eq!(moves.values[0].white.notation, "a4 $69");
    assert_eq!(moves.values[0].black.notation, "a5 $420");
}

#[test]
fn test_parse_moves_with_alternatives() {
    let moves = PgnMoves::from("1. e4 (1. f4? e5! 2. g4?? Qh4#) e5");
    assert_eq!(moves.values.len(), 1);
    let alt = moves.values[0].white.alternatives.as_ref().unwrap();
    assert_eq!(alt.values.len(), 1);
    assert_eq!(alt.values[0].values[0].white.notation, "f4?");
    assert_eq!(alt.values[0].values[0].black.notation, "e5!");
    assert_eq!(alt.values[0].values[1].white.notation, "g4??");
    assert_eq!(alt.values[0].values[1].black.notation, "Qh4#");
}

#[test]
fn test_parse_moves_skip_with_dot_dot_dot() {
    let moves = PgnMoves::from("69.Be4 69... Rxe5?!");
    let w = &moves.values[0].white;
    let b = &moves.values[0].black;
    assert_eq!(w.piece, PgnPiece::Bishop);
    assert_eq!(w.dest.file, Some('e'));
    assert_eq!(w.dest.rank, Some(4));
    assert_eq!(w.captures, false);
    assert_eq!(b.piece, PgnPiece::Rook);
    assert_eq!(b.dest.file, Some('e'));
    assert_eq!(b.dest.rank, Some(5));
    assert_eq!(b.captures, true);
    assert_eq!(b.annotation, PgnAnnotation::DubiousMove);
}

#[test]
fn test_parse_moves_with_comment_after_white() {
    let moves = PgnMoves::from("9.e4 { This is a comment :O } e5 10. Nc3 Nc6");
    let w_comments = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(w_comments.values.len(), 1);
    assert_eq!(w_comments.values[0].value, " This is a comment :O ");
    assert_eq!(
        w_comments.values[0].position,
        libpgn::comments::PgnCommentPosition::AfterMove
    );
    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].white.dest.file, Some('c'));
    assert_eq!(moves.values[1].white.dest.rank, Some(3));
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.dest.file, Some('c'));
    assert_eq!(moves.values[1].black.dest.rank, Some(6));
}

#[test]
fn test_dump_move_simple_pawn() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Pawn;
    m.dest.file = Some('e');
    m.dest.rank = Some(4);
    assert_eq!(format!("{}", m), "e4");
}

#[test]
fn test_dump_move_rook() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Rook;
    m.dest.file = Some('e');
    m.dest.rank = Some(4);
    assert_eq!(format!("{}", m), "Re4");
}

#[test]
fn test_dump_move_rook_mate() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Rook;
    m.check = PgnCheck::Mate;
    m.dest.file = Some('e');
    m.dest.rank = Some(4);
    assert_eq!(format!("{}", m), "Re4#");
}

#[test]
fn test_dump_move_kingside_castle_mate() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.castles = PGN_CASTLING_KINGSIDE;
    m.check = PgnCheck::Mate;
    assert_eq!(format!("{}", m), "O-O#");
}

#[test]
fn test_dump_move_rook_interesting() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::InterestingMove;
    m.piece = PgnPiece::Rook;
    m.dest.file = Some('a');
    m.dest.rank = Some(3);
    assert_eq!(format!("{}", m), "Ra3!?");
}

#[test]
fn test_dump_move_pawn_ep_interesting() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::InterestingMove;
    m.en_passant = true;
    m.piece = PgnPiece::Pawn;
    m.dest.file = Some('f');
    m.dest.rank = Some(3);
    assert_eq!(format!("{}", m), "f3!? e.p.");
}

#[test]
fn test_dump_move_pawn_nag() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation(9);
    m.piece = PgnPiece::Pawn;
    m.dest.file = Some('f');
    m.dest.rank = Some(3);
    assert_eq!(format!("{}", m), "f3 $9");
}

#[test]
fn test_dump_move_pawn_ep_nag() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation(9);
    m.en_passant = true;
    m.piece = PgnPiece::Pawn;
    m.dest.file = Some('f');
    m.dest.rank = Some(3);
    assert_eq!(format!("{}", m), "f3 e.p. $9");
}

#[test]
fn test_dump_move_pawn_capture() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Pawn;
    m.captures = true;
    m.dest.file = Some('f');
    m.dest.rank = Some(3);
    assert_eq!(format!("{}", m), "xf3");
}

#[test]
fn test_dump_move_rook_capture() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Rook;
    m.captures = true;
    m.dest.file = Some('f');
    m.dest.rank = Some(3);
    assert_eq!(format!("{}", m), "Rxf3");
}

#[test]
fn test_dump_move_pawn_capture_promotion_double_check() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Pawn;
    m.captures = true;
    m.dest.file = Some('f');
    m.dest.rank = Some(8);
    m.promoted_to = PgnPiece::Queen;
    m.check = PgnCheck::Double;
    assert_eq!(format!("{}", m), "xf8=Q++");
}

#[test]
fn test_parse_en_passant_full_game() {
    let moves = PgnMoves::from("1. d4 exd3 e.p. 2.  Nc4 Nc6 3. cxb3 e.p. 3... Be4");
    assert_eq!(moves.values.len(), 3);
    assert_eq!(moves.values[0].white.notation, "d4");
    assert_eq!(moves.values[0].white.en_passant, false);
    assert_eq!(moves.values[0].black.notation, "exd3 e.p.");
    assert_eq!(moves.values[0].black.en_passant, true);
    assert_eq!(moves.values[0].black.from.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.file, Some('d'));
    assert_eq!(moves.values[0].black.dest.rank, Some(3));

    assert_eq!(moves.values[1].white.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].white.dest.file, Some('c'));
    assert_eq!(moves.values[1].white.dest.rank, Some(4));
    assert_eq!(moves.values[1].white.notation, "Nc4");
    assert_eq!(moves.values[1].white.en_passant, false);
    assert_eq!(moves.values[1].black.piece, PgnPiece::Knight);
    assert_eq!(moves.values[1].black.dest.file, Some('c'));
    assert_eq!(moves.values[1].black.dest.rank, Some(6));
    assert_eq!(moves.values[1].black.notation, "Nc6");

    assert_eq!(moves.values[2].white.piece, PgnPiece::Pawn);
    assert_eq!(moves.values[2].white.from.file, Some('c'));
    assert_eq!(moves.values[2].white.dest.file, Some('b'));
    assert_eq!(moves.values[2].white.dest.rank, Some(3));
    assert_eq!(moves.values[2].white.notation, "cxb3 e.p.");
    assert_eq!(moves.values[2].white.en_passant, true);
    assert_eq!(moves.values[2].black.piece, PgnPiece::Bishop);
    assert_eq!(moves.values[2].black.dest.file, Some('e'));
    assert_eq!(moves.values[2].black.dest.rank, Some(4));
    assert_eq!(moves.values[2].black.notation, "Be4");
    assert_eq!(moves.values[2].black.en_passant, false);
}

#[test]
fn test_castling_constants() {
    assert_eq!(PGN_CASTLING_NONE, 0);
    assert_eq!(PGN_CASTLING_KINGSIDE, 2);
    assert_eq!(PGN_CASTLING_QUEENSIDE, 3);
}

fn main() {}
