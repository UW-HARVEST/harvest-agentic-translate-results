#[allow(unused_imports)]
use libpgn::comments::{PgnComment, PgnCommentPosition, PgnComments};

#[test]
fn test_comment_new_default() {
    let c = PgnComment::new();
    assert_eq!(c.position, PgnCommentPosition::Unknown);
    assert_eq!(c.value, "");
    assert_eq!(c.value(), "");
}

#[test]
fn test_comment_from_string_simple() {
    let mut consumed = 0;
    let c = PgnComment::from_string("{hello}", &mut consumed);
    assert_eq!(c.value, "hello");
    assert_eq!(c.position, PgnCommentPosition::Unknown);
    assert_eq!(consumed, 7);
}

#[test]
fn test_comment_from_string_with_spaces() {
    let mut consumed = 0;
    let c = PgnComment::from_string("{ a comment }", &mut consumed);
    assert_eq!(c.value, " a comment ");
    assert_eq!(consumed, 13);
}

#[test]
fn test_comment_from_string_nested() {
    let mut consumed = 0;
    // From C tests: "{ a comment {inside a comment} ? }"
    let c = PgnComment::from_string("{ a comment {inside a comment} ? }", &mut consumed);
    assert_eq!(c.value, " a comment {inside a comment} ? ");
    assert_eq!(consumed, 34);
}

#[test]
fn test_comment_from_string_empty_braces() {
    let mut consumed = 0;
    let c = PgnComment::from_string("{}", &mut consumed);
    assert_eq!(c.value, "");
    assert_eq!(consumed, 2);
}

#[test]
fn test_comments_init_empty() {
    let comments = PgnComments::new();
    assert_eq!(comments.values.len(), 0);
    assert_eq!(comments.get_first_after_alternative_index(), None);
}

#[test]
fn test_comments_push() {
    let mut comments = PgnComments::new();
    let mut c1 = PgnComment::new();
    c1.position = PgnCommentPosition::BeforeMove;
    c1.value = "first".to_string();
    let mut c2 = PgnComment::new();
    c2.position = PgnCommentPosition::AfterMove;
    c2.value = "second".to_string();

    comments.push(c1);
    comments.push(c2);

    assert_eq!(comments.values.len(), 2);
    assert_eq!(comments.values[0].position, PgnCommentPosition::BeforeMove);
    assert_eq!(comments.values[0].value, "first");
    assert_eq!(comments.values[1].position, PgnCommentPosition::AfterMove);
    assert_eq!(comments.values[1].value, "second");
}

#[test]
fn test_comments_get_first_after_alternative_index() {
    let mut comments = PgnComments::new();

    let mut c1 = PgnComment::new();
    c1.position = PgnCommentPosition::AfterMove;
    let mut c2 = PgnComment::new();
    c2.position = PgnCommentPosition::AfterAlternative;
    let mut c3 = PgnComment::new();
    c3.position = PgnCommentPosition::AfterAlternative;

    comments.push(c1);
    comments.push(c2);
    comments.push(c3);

    assert_eq!(comments.get_first_after_alternative_index(), Some(1));
}

#[test]
fn test_comments_get_first_after_alternative_index_none() {
    let mut comments = PgnComments::new();
    let mut c1 = PgnComment::new();
    c1.position = PgnCommentPosition::BeforeMove;
    let mut c2 = PgnComment::new();
    c2.position = PgnCommentPosition::AfterMove;
    comments.push(c1);
    comments.push(c2);
    assert_eq!(comments.get_first_after_alternative_index(), None);
}

#[test]
fn test_comments_poll_basic() {
    let mut comments = PgnComments::new();
    // Should consume "{hello} " (8 chars) leaving rest
    let consumed = comments.poll(PgnCommentPosition::BeforeMove, "{hello} 1.e4");
    assert_eq!(comments.values.len(), 1);
    assert_eq!(comments.values[0].value, "hello");
    assert_eq!(comments.values[0].position, PgnCommentPosition::BeforeMove);
    assert_eq!(consumed, 8); // "{hello} " == 8 chars
}

#[test]
fn test_comments_poll_multiple() {
    let mut comments = PgnComments::new();
    // From C tests: "{ hello} {libpgn} 1.e4 e5"
    let consumed = comments.poll(PgnCommentPosition::BeforeMove, "{ hello} {libpgn} 1.e4");
    assert_eq!(comments.values.len(), 2);
    assert_eq!(comments.values[0].value, " hello");
    assert_eq!(comments.values[0].position, PgnCommentPosition::BeforeMove);
    assert_eq!(comments.values[1].value, "libpgn");
    assert_eq!(comments.values[1].position, PgnCommentPosition::BeforeMove);
    assert_eq!(consumed, 18); // "{ hello} {libpgn} " == 18 chars
}

#[test]
fn test_comments_poll_no_brace() {
    let mut comments = PgnComments::new();
    let consumed = comments.poll(PgnCommentPosition::BeforeMove, "1.e4");
    assert_eq!(comments.values.len(), 0);
    assert_eq!(consumed, 0);
}

#[test]
fn test_comment_position_repr() {
    assert_eq!(PgnCommentPosition::Unknown as i32, 0);
    assert_eq!(PgnCommentPosition::BeforeMove as i32, 1);
    assert_eq!(PgnCommentPosition::BetweenMove as i32, 2);
    assert_eq!(PgnCommentPosition::AfterMove as i32, 3);
    assert_eq!(PgnCommentPosition::AfterAlternative as i32, 4);
}

fn main() {}
