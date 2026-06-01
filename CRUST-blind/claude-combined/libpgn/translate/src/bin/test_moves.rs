use libpgn::annotation::PgnAnnotation;
use libpgn::check::PgnCheck;
use libpgn::moves::{
    PgnMove, PgnMoves, PGN_CASTLING_KINGSIDE, PGN_CASTLING_NONE, PGN_CASTLING_QUEENSIDE,
};
use libpgn::piece::PgnPiece;

#[test]
fn test_parse_simple_move() {
    let m = PgnMove::from("e4");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, false);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(4));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.notation, "e4");
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
    assert_eq!(m.castles, PGN_CASTLING_NONE);
    assert_eq!(m.en_passant, false);
}

#[test]
fn test_parse_blunder() {
    let m = PgnMove::from("e4??");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, false);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('e'));
    assert_eq!(m.dest.rank, Some(4));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.notation, "e4??");
    assert_eq!(m.annotation, PgnAnnotation::Blunder);
}

#[test]
fn test_parse_knight_move_disambiguated() {
    let m = PgnMove::from("Nb6d5+");
    assert_eq!(m.piece, PgnPiece::Knight);
    assert_eq!(m.captures, false);
    assert_eq!(m.from.file, Some('b'));
    assert_eq!(m.from.rank, Some(6));
    assert_eq!(m.dest.file, Some('d'));
    assert_eq!(m.dest.rank, Some(5));
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.notation, "Nb6d5+");
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
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
    assert_eq!(m.notation, "Qf1?!");
    assert_eq!(m.annotation, PgnAnnotation::DubiousMove);
}

#[test]
fn test_parse_promotion_blunder() {
    let m = PgnMove::from("bxa8=Q??");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, true);
    assert_eq!(m.from.file, Some('b'));
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('a'));
    assert_eq!(m.dest.rank, Some(8));
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.promoted_to, PgnPiece::Queen);
    assert_eq!(m.notation, "bxa8=Q??");
    assert_eq!(m.annotation, PgnAnnotation::Blunder);
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
    assert_eq!(m.notation, "Bxg2");
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_parse_pawn_capture_check_good() {
    let m = PgnMove::from("exf2+!");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.captures, true);
    assert_eq!(m.from.file, Some('e'));
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, Some('f'));
    assert_eq!(m.dest.rank, Some(2));
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.promoted_to, PgnPiece::Unknown);
    assert_eq!(m.notation, "exf2+!");
    assert_eq!(m.annotation, PgnAnnotation::GoodMove);
}

#[test]
fn test_parse_kingside_castle_check_brilliant() {
    let m = PgnMove::from("O-O+!!");
    assert_eq!(m.castles, PGN_CASTLING_KINGSIDE);
    assert_eq!(m.check, PgnCheck::Single);
    assert_eq!(m.notation, "O-O+!!");
    assert_eq!(m.annotation, PgnAnnotation::BrilliantMove);
}

#[test]
fn test_parse_queenside_castle() {
    let m = PgnMove::from("O-O-O");
    assert_eq!(m.castles, PGN_CASTLING_QUEENSIDE);
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.notation, "O-O-O");
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
}

#[test]
fn test_parse_en_passant() {
    let m = PgnMove::from("exd6 e.p.");
    assert_eq!(m.piece, PgnPiece::Pawn);
    assert_eq!(m.en_passant, true);
    assert_eq!(m.from.file, Some('e'));
    assert_eq!(m.dest.file, Some('d'));
    assert_eq!(m.dest.rank, Some(6));
    assert_eq!(m.captures, true);
}

#[test]
fn test_parse_moves_simple() {
    let moves = PgnMoves::from("1.e4 e5");
    assert_eq!(moves.values.len(), 1);
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
fn test_parse_moves_with_annotations() {
    let moves = PgnMoves::from("1. a4?? a5!");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Blunder);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::GoodMove);
    assert_eq!(moves.values[0].white.notation, "a4??");
    assert_eq!(moves.values[0].black.notation, "a5!");

    let moves = PgnMoves::from("1. a4!! a5?!");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::BrilliantMove);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::DubiousMove);
}

#[test]
fn test_parse_moves_with_nag() {
    let moves = PgnMoves::from("1. a4 $4 a5 $1");
    assert_eq!(moves.values[0].white.annotation, PgnAnnotation::Blunder);
    assert_eq!(moves.values[0].black.annotation, PgnAnnotation::GoodMove);
    assert_eq!(moves.values[0].white.notation, "a4 $4");
    assert_eq!(moves.values[0].black.notation, "a5 $1");
}

#[test]
fn test_parse_moves_ambiguate() {
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
fn test_parse_qh4e1() {
    let moves = PgnMoves::from("1. Qh4e1");
    assert_eq!(moves.values[0].white.notation, "Qh4e1");
    assert_eq!(moves.values[0].white.from.file, Some('h'));
    assert_eq!(moves.values[0].white.from.rank, Some(4));
    assert_eq!(moves.values[0].white.dest.file, Some('e'));
    assert_eq!(moves.values[0].white.dest.rank, Some(1));
}

#[test]
fn test_parse_moves_with_alternatives() {
    let moves = PgnMoves::from("1. e4 (1. f4? e5! 2. g4?? Qh4#) e5");
    assert_eq!(moves.values.len(), 1);
    let alts = moves.values[0].white.alternatives.as_ref().unwrap();
    assert_eq!(alts.values.len(), 1);
    assert_eq!(alts.values[0].values[0].white.notation, "f4?");
    assert_eq!(alts.values[0].values[0].black.notation, "e5!");
    assert_eq!(alts.values[0].values[1].white.notation, "g4??");
    assert_eq!(alts.values[0].values[1].black.notation, "Qh4#");
}

#[test]
fn test_dump_move_e4() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Pawn;
    m.dest.file = Some('e');
    m.dest.rank = Some(4);
    let s = format!("{}", m);
    assert_eq!(s, "e4");
}

#[test]
fn test_dump_move_re4() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Rook;
    m.dest.file = Some('e');
    m.dest.rank = Some(4);
    let s = format!("{}", m);
    assert_eq!(s, "Re4");
}

#[test]
fn test_dump_move_re4_mate() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Rook;
    m.check = PgnCheck::Mate;
    m.dest.file = Some('e');
    m.dest.rank = Some(4);
    let s = format!("{}", m);
    assert_eq!(s, "Re4#");
}

#[test]
fn test_dump_move_kingside_castle_mate() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.castles = PGN_CASTLING_KINGSIDE;
    m.check = PgnCheck::Mate;
    let s = format!("{}", m);
    assert_eq!(s, "O-O#");
}

#[test]
fn test_dump_move_ra3_interesting() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::InterestingMove;
    m.piece = PgnPiece::Rook;
    m.dest.file = Some('a');
    m.dest.rank = Some(3);
    let s = format!("{}", m);
    assert_eq!(s, "Ra3!?");
}

#[test]
fn test_dump_move_promotion_double_check() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Pawn;
    m.captures = true;
    m.dest.file = Some('f');
    m.dest.rank = Some(8);
    m.promoted_to = PgnPiece::Queen;
    m.check = PgnCheck::Double;
    let s = format!("{}", m);
    assert_eq!(s, "xf8=Q++");
}

#[test]
fn test_dump_move_pawn_capture() {
    let mut m = PgnMove::default();
    m.annotation = PgnAnnotation::Unknown;
    m.piece = PgnPiece::Pawn;
    m.captures = true;
    m.dest.file = Some('f');
    m.dest.rank = Some(3);
    let s = format!("{}", m);
    assert_eq!(s, "xf3");
}

#[test]
fn test_pgn_move_default() {
    let m = PgnMove::default();
    assert_eq!(m.piece, PgnPiece::Unknown);
    assert_eq!(m.promoted_to, PgnPiece::Unknown);
    assert_eq!(m.notation, "");
    assert_eq!(m.castles, PGN_CASTLING_NONE);
    assert_eq!(m.captures, false);
    assert_eq!(m.en_passant, false);
    assert_eq!(m.check, PgnCheck::None);
    assert_eq!(m.from.file, None);
    assert_eq!(m.from.rank, None);
    assert_eq!(m.dest.file, None);
    assert_eq!(m.dest.rank, None);
    assert_eq!(m.annotation, PgnAnnotation::Unknown);
    assert!(m.comments.is_none());
    assert!(m.alternatives.is_none());
}

#[test]
fn test_parse_en_passant_in_moves() {
    let moves = PgnMoves::from("1. d4 exd3 e.p. 2.  Nc4 Nc6 3. cxb3 e.p. 3... Be4");
    assert_eq!(moves.values[0].white.notation, "d4");
    assert_eq!(moves.values[0].white.en_passant, false);
    assert_eq!(moves.values[0].black.from.file, Some('e'));
    assert_eq!(moves.values[0].black.dest.file, Some('d'));
    assert_eq!(moves.values[0].black.dest.rank, Some(3));
    assert_eq!(moves.values[0].black.notation, "exd3 e.p.");
    assert_eq!(moves.values[0].black.en_passant, true);

    assert_eq!(moves.values[2].white.notation, "cxb3 e.p.");
    assert_eq!(moves.values[2].white.en_passant, true);
    assert_eq!(moves.values[2].black.notation, "Be4");
    assert_eq!(moves.values[2].black.en_passant, false);
}

fn main() {}
