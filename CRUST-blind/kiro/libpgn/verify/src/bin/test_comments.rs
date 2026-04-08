use libpgn::comments::{PgnComment, PgnComments, PgnCommentPosition};

#[test]
fn test_comment_from_string() {
    let mut consumed = 0;
    let comment = PgnComment::from_string("{hello}", &mut consumed);
    assert_eq!(comment.value(), "hello");
    assert_eq!(consumed, 7);
    assert_eq!(comment.position, PgnCommentPosition::Unknown);
}

#[test]
fn test_comment_nested_braces() {
    let mut consumed = 0;
    let comment = PgnComment::from_string("{ a comment {inside a comment} ? }", &mut consumed);
    assert_eq!(comment.value(), " a comment {inside a comment} ? ");
    assert_eq!(consumed, 34);
}

#[test]
fn test_comments_poll_single() {
    let mut comments = PgnComments::new();
    let consumed = comments.poll(PgnCommentPosition::AfterMove, "{hello}");
    assert_eq!(consumed, 7);
    assert_eq!(comments.values.len(), 1);
    assert_eq!(comments.values[0].position, PgnCommentPosition::AfterMove);
    assert_eq!(comments.values[0].value(), "hello");
}

#[test]
fn test_comments_poll_multiple() {
    let mut comments = PgnComments::new();
    let consumed = comments.poll(PgnCommentPosition::AfterMove, "{hello} {from} {libpgn}");
    assert!(consumed > 0);
    assert_eq!(comments.values.len(), 3);
    assert_eq!(comments.values[0].value(), "hello");
    assert_eq!(comments.values[1].value(), "from");
    assert_eq!(comments.values[2].value(), "libpgn");
}

#[test]
fn test_comments_poll_no_comment() {
    let mut comments = PgnComments::new();
    let consumed = comments.poll(PgnCommentPosition::AfterMove, "e4 e5");
    assert_eq!(consumed, 0);
    assert_eq!(comments.values.len(), 0);
}

#[test]
fn test_comments_get_first_after_alternative_index() {
    let mut comments = PgnComments::new();
    comments.push(PgnComment { position: PgnCommentPosition::AfterMove, value: "x".to_string() });
    comments.push(PgnComment { position: PgnCommentPosition::AfterAlternative, value: "y".to_string() });
    comments.push(PgnComment { position: PgnCommentPosition::AfterAlternative, value: "z".to_string() });
    assert_eq!(comments.get_first_after_alternative_index(), Some(1));
}

#[test]
fn test_comments_no_after_alternative() {
    let mut comments = PgnComments::new();
    comments.push(PgnComment { position: PgnCommentPosition::AfterMove, value: "x".to_string() });
    assert_eq!(comments.get_first_after_alternative_index(), None);
}

fn main() {}
