#[allow(unused_imports)]
use libpgn::metadata::PgnMetadata;

#[test]
fn test_metadata_new_empty() {
    let m = PgnMetadata::new();
    assert_eq!(m.get("Event"), None);
}

#[test]
fn test_metadata_insert_get() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "F/S Return Match");
    m.insert("Site", "Belgrade, Serbia JUG");

    assert_eq!(m.get("Event"), Some("F/S Return Match"));
    assert_eq!(m.get("Site"), Some("Belgrade, Serbia JUG"));
    assert_eq!(m.get("DoesNotExist"), None);
}

#[test]
fn test_metadata_delete() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "Foo");
    m.insert("Site", "Bar");
    m.insert("Date", "Baz");

    m.delete("Site");

    assert_eq!(m.get("Event"), Some("Foo"));
    assert_eq!(m.get("Site"), None);
    assert_eq!(m.get("Date"), Some("Baz"));
}

#[test]
fn test_metadata_delete_missing() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "Foo");
    m.delete("Nope"); // should not panic
    assert_eq!(m.get("Event"), Some("Foo"));
}

#[test]
fn test_metadata_from_string() {
    // Built from C tests/metadata.c
    let s = "[Event \"F/S Return Match\"]\n\
             [Site \"Belgrade, Serbia JUG\"]\n\
             [Date \"1992.11.04\"]\n\
             [Round \"29\"]\n\
             [White \"Fischer, Robert J.\"]\n\
             [Black \"Spassky, Boris V.\"]\n\
             [Result \"1/2-1/2\"]\n";

    let m = PgnMetadata::from_string(s);
    assert_eq!(m.get("Event"), Some("F/S Return Match"));
    assert_eq!(m.get("Site"), Some("Belgrade, Serbia JUG"));
    assert_eq!(m.get("Date"), Some("1992.11.04"));
    assert_eq!(m.get("Round"), Some("29"));
    assert_eq!(m.get("White"), Some("Fischer, Robert J."));
    assert_eq!(m.get("Black"), Some("Spassky, Boris V."));
    assert_eq!(m.get("Result"), Some("1/2-1/2"));
}

#[test]
fn test_metadata_from_string_empty_returns_empty() {
    let m = PgnMetadata::from_string("");
    assert_eq!(m.get("Event"), None);
}

#[test]
fn test_metadata_from_string_no_brackets() {
    let m = PgnMetadata::from_string("nothing here");
    assert_eq!(m.get("Event"), None);
}

#[test]
fn test_metadata_with_consumption() {
    let s = "[Event \"Foo\"]\n[Site \"Bar\"]\n";
    let mut consumed = 0usize;
    let m = PgnMetadata::from_string_with_consumption(s, &mut consumed);
    assert_eq!(m.get("Event"), Some("Foo"));
    assert_eq!(m.get("Site"), Some("Bar"));
    assert_eq!(consumed, s.len());
}

fn main() {}
