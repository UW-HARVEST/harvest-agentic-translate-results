use libpgn::metadata::PgnMetadata;

#[test]
fn test_metadata_new_empty() {
    let m = PgnMetadata::new();
    assert_eq!(m.get("Event"), None);
}

#[test]
fn test_metadata_insert_and_get() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "Test");
    assert_eq!(m.get("Event"), Some("Test"));
}

#[test]
fn test_metadata_get_missing_key() {
    let m = PgnMetadata::new();
    assert_eq!(m.get("NonExistent"), None);
}

#[test]
fn test_metadata_delete() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "Test");
    m.delete("Event");
    assert_eq!(m.get("Event"), None);
}

#[test]
fn test_metadata_delete_nonexistent() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "Test");
    m.delete("NonExistent");
    assert_eq!(m.get("Event"), Some("Test"));
}

#[test]
fn test_metadata_from_string_single() {
    let m = PgnMetadata::from_string("[Event \"Test\"]\n");
    assert_eq!(m.get("Event"), Some("Test"));
}

#[test]
fn test_metadata_from_string_multiple() {
    let input = "[Event \"Test\"]\n[Site \"Here\"]\n";
    let m = PgnMetadata::from_string(input);
    assert_eq!(m.get("Event"), Some("Test"));
    assert_eq!(m.get("Site"), Some("Here"));
}

#[test]
fn test_metadata_from_string_no_bracket() {
    let m = PgnMetadata::from_string("no metadata");
    assert_eq!(m.get("Event"), None);
}

#[test]
fn test_metadata_overwrite() {
    let mut m = PgnMetadata::new();
    m.insert("Event", "First");
    m.insert("Event", "Second");
    // HashMap overwrites, so we get the latest
    assert_eq!(m.get("Event"), Some("Second"));
}

#[test]
fn test_metadata_with_consumption() {
    let input = "[Event \"Test\"]\n1. e4 e5";
    let mut consumed = 0;
    let m = PgnMetadata::from_string_with_consumption(input, &mut consumed);
    assert_eq!(m.get("Event"), Some("Test"));
    assert!(consumed > 0);
    assert!(consumed <= input.len());
}

fn main() {}
