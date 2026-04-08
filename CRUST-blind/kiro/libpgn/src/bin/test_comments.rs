use libpgn::comments::{PgnComment, PgnComments, PgnCommentPosition};

#[test]
fn test_comment_new() {
    let c = PgnComment::new();
    assert_eq!(c.position, PgnCommentPosition::Unknown);
    assert_eq!(c.value(), "");
}

#[test]
fn test_comment_from_string_simple() {
    let mut consumed = 0;
    let c = PgnComment::from_string("{hello}", &mut consumed);
    assert_eq!(c.value(), "hello");
    assert_eq!(consumed, 7);
}

#[test]
fn test_comment_from_string_nested_braces() {
    let mut consumed = 0;
    let c = PgnComment::from_string("{a{b}c}", &mut consumed);
    assert_eq!(c.value(), "a{b}c");
    assert_eq!(consumed, 7);
}

#[test]
fn test_comment_from_string_empty() {
    let mut consumed = 0;
    let c = PgnComment::from_string("{}", &mut consumed);
    assert_eq!(c.value(), "");
    assert_eq!(consumed, 2);
}

#[test]
fn test_comments_new() {
    let cs = PgnComments::new();
    assert_eq!(cs.values.len(), 0);
}

#[test]
fn test_comments_push() {
    let mut cs = PgnComments::new();
    let mut c = PgnComment::new();
    c.value = "test".to_string();
    c.position = PgnCommentPosition::AfterMove;
    cs.push(c);
    assert_eq!(cs.values.len(), 1);
    assert_eq!(cs.values[0].value(), "test");
    assert_eq!(cs.values[0].position, PgnCommentPosition::AfterMove);
}

#[test]
fn test_comments_poll_single() {
    let mut cs = PgnComments::new();
    let consumed = cs.poll(PgnCommentPosition::BeforeMove, "{comment} rest");
    assert!(consumed > 0);
    assert_eq!(cs.values.len(), 1);
    assert_eq!(cs.values[0].value(), "comment");
    assert_eq!(cs.values[0].position, PgnCommentPosition::BeforeMove);
}

#[test]
fn test_comments_poll_multiple() {
    let mut cs = PgnComments::new();
    let consumed = cs.poll(PgnCommentPosition::AfterMove, "{first} {second} rest");
    assert!(consumed > 0);
    assert_eq!(cs.values.len(), 2);
    assert_eq!(cs.values[0].value(), "first");
    assert_eq!(cs.values[1].value(), "second");
}

#[test]
fn test_comments_poll_no_comment() {
    let mut cs = PgnComments::new();
    let consumed = cs.poll(PgnCommentPosition::BeforeMove, "no comment here");
    assert_eq!(consumed, 0);
    assert_eq!(cs.values.len(), 0);
}

#[test]
fn test_comments_get_first_after_alternative_index_found() {
    let mut cs = PgnComments::new();
    cs.push(PgnComment { position: PgnCommentPosition::BeforeMove, value: "a".to_string() });
    cs.push(PgnComment { position: PgnCommentPosition::AfterAlternative, value: "b".to_string() });
    assert_eq!(cs.get_first_after_alternative_index(), Some(1));
}

#[test]
fn test_comments_get_first_after_alternative_index_not_found() {
    let mut cs = PgnComments::new();
    cs.push(PgnComment { position: PgnCommentPosition::BeforeMove, value: "a".to_string() });
    assert_eq!(cs.get_first_after_alternative_index(), None);
}

#[test]
fn test_comments_get_first_after_alternative_index_empty() {
    let cs = PgnComments::new();
    assert_eq!(cs.get_first_after_alternative_index(), None);
}

fn main() {}
