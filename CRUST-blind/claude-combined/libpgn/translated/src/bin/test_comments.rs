use libpgn::comments::{PgnComment, PgnCommentPosition, PgnComments};
use libpgn::moves::PgnMoves;

#[test]
fn test_comment_new() {
    let c = PgnComment::new();
    assert_eq!(c.position, PgnCommentPosition::Unknown);
    assert_eq!(c.value, "");
    assert_eq!(c.value(), "");
}

#[test]
fn test_comment_from_string_simple() {
    let mut consumed = 0usize;
    let c = PgnComment::from_string("{hello}", &mut consumed);
    assert_eq!(c.value, "hello");
    assert_eq!(consumed, 7);
    assert_eq!(c.position, PgnCommentPosition::Unknown);
}

#[test]
fn test_comment_from_string_nested() {
    let mut consumed = 0usize;
    let c = PgnComment::from_string("{ a comment {inside a comment} ? }", &mut consumed);
    assert_eq!(c.value, " a comment {inside a comment} ? ");
    assert_eq!(consumed, 34);
}

#[test]
fn test_comments_new() {
    let c = PgnComments::new();
    assert_eq!(c.values.len(), 0);
    assert!(c.is_empty());
}

#[test]
fn test_comments_push() {
    let mut comments = PgnComments::new();
    let mut c = PgnComment::new();
    c.position = PgnCommentPosition::AfterMove;
    c.value = String::from("dump");
    comments.push(c);
    assert_eq!(comments.values.len(), 1);
    assert_eq!(comments.values[0].value, "dump");
    assert_eq!(comments.values[0].position, PgnCommentPosition::AfterMove);
}

#[test]
fn test_comments_poll_after_move() {
    let mut comments = PgnComments::new();
    let consumed = comments.poll(PgnCommentPosition::AfterMove, "{a} {b}");
    // Both comments should be parsed
    assert_eq!(comments.values.len(), 2);
    assert_eq!(comments.values[0].value, "a");
    assert_eq!(comments.values[0].position, PgnCommentPosition::AfterMove);
    assert_eq!(comments.values[1].value, "b");
    assert_eq!(comments.values[1].position, PgnCommentPosition::AfterMove);
    assert!(consumed >= 7);
}

#[test]
fn test_comments_get_first_after_alternative_index_none() {
    let comments = PgnComments::new();
    assert_eq!(comments.get_first_after_alternative_index(), None);
}

#[test]
fn test_comments_get_first_after_alternative_index() {
    let mut comments = PgnComments::new();
    let mut c1 = PgnComment::new();
    c1.position = PgnCommentPosition::AfterMove;
    c1.value = String::from("first");
    comments.push(c1);
    let mut c2 = PgnComment::new();
    c2.position = PgnCommentPosition::AfterAlternative;
    c2.value = String::from("second");
    comments.push(c2);

    assert_eq!(comments.get_first_after_alternative_index(), Some(1));
}

#[test]
fn test_parsing_moves_with_nested_comment() {
    let moves = PgnMoves::from("1.e4 { a comment {inside a comment} ? } e5");
    let c = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(c.values.len(), 1);
    assert_eq!(c.values[0].position, PgnCommentPosition::AfterMove);
    assert_eq!(c.values[0].value, " a comment {inside a comment} ? ");
}

#[test]
fn test_parsing_comments_before_move() {
    let moves = PgnMoves::from("{ hello} 1.e4 e5");
    let c = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(c.values.len(), 1);
    assert_eq!(c.values[0].position, PgnCommentPosition::BeforeMove);
    assert_eq!(c.values[0].value, " hello");
}

#[test]
fn test_parsing_comments_multiple_before() {
    let moves = PgnMoves::from("{ hello} {libpgn} 1.e4 e5");
    let c = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(c.values.len(), 2);
    assert_eq!(c.values[0].position, PgnCommentPosition::BeforeMove);
    assert_eq!(c.values[0].value, " hello");
    assert_eq!(c.values[1].position, PgnCommentPosition::BeforeMove);
    assert_eq!(c.values[1].value, "libpgn");
}

#[test]
fn test_parsing_comments_between() {
    let moves = PgnMoves::from("1. {jackalope:w} e4 e5");
    let c = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(c.values.len(), 1);
    assert_eq!(c.values[0].position, PgnCommentPosition::BetweenMove);
    assert_eq!(c.values[0].value, "jackalope:w");
}

#[test]
fn test_parsing_comments_after_multiple() {
    let moves = PgnMoves::from("1.e4 {hello} {from} {libpgn} e5");
    let c = moves.values[0].white.comments.as_ref().unwrap();
    assert_eq!(c.values.len(), 3);
    assert_eq!(c.values[0].position, PgnCommentPosition::AfterMove);
    assert_eq!(c.values[0].value, "hello");
    assert_eq!(c.values[1].position, PgnCommentPosition::AfterMove);
    assert_eq!(c.values[1].value, "from");
    assert_eq!(c.values[2].position, PgnCommentPosition::AfterMove);
    assert_eq!(c.values[2].value, "libpgn");
}

#[test]
fn test_parsing_comments_after_alt() {
    let moves = PgnMoves::from("1. e4 e5 {IGNORE MEEE} (1... f5) {hello} {again}");
    let c = moves.values[0].black.comments.as_ref().unwrap();
    assert_eq!(c.values.len(), 3);
    assert_eq!(c.values[0].position, PgnCommentPosition::AfterMove);
    assert_eq!(c.values[0].value, "IGNORE MEEE");
    assert_eq!(c.values[1].position, PgnCommentPosition::AfterAlternative);
    assert_eq!(c.values[1].value, "hello");
    assert_eq!(c.values[2].position, PgnCommentPosition::AfterAlternative);
    assert_eq!(c.values[2].value, "again");
    assert_eq!(c.get_first_after_alternative_index(), Some(1));
}

fn main() {}
