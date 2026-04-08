use libpgn::metadata::PgnMetadata;

#[test]
fn test_metadata_from_string() {
    let md = PgnMetadata::from_string(
        "[Event \"F/S Return Match\"]\n\
         [Site \"Belgrade, Serbia JUG\"]\n\
         [Date \"1992.11.04\"]\n\
         [Round \"29\"]\n\
         [White \"Fischer, Robert J.\"]\n\
         [Black \"Spassky, Boris V.\"]\n\
         [Result \"1/2-1/2\"]\n"
    );
    assert_eq!(md.get("Event"), Some("F/S Return Match"));
    assert_eq!(md.get("Site"), Some("Belgrade, Serbia JUG"));
    assert_eq!(md.get("Date"), Some("1992.11.04"));
    assert_eq!(md.get("Round"), Some("29"));
    assert_eq!(md.get("White"), Some("Fischer, Robert J."));
    assert_eq!(md.get("Black"), Some("Spassky, Boris V."));
    assert_eq!(md.get("Result"), Some("1/2-1/2"));
}

#[test]
fn test_metadata_get_missing_key() {
    let md = PgnMetadata::from_string("[Event \"test\"]\n");
    assert_eq!(md.get("Missing"), None);
}

#[test]
fn test_metadata_insert_and_get() {
    let mut md = PgnMetadata::new();
    md.insert("Key", "Value");
    assert_eq!(md.get("Key"), Some("Value"));
}

#[test]
fn test_metadata_delete() {
    let mut md = PgnMetadata::new();
    md.insert("Key", "Value");
    md.delete("Key");
    assert_eq!(md.get("Key"), None);
}

#[test]
fn test_metadata_with_consumption() {
    let mut consumed = 0;
    let md = PgnMetadata::from_string_with_consumption(
        "[Event \"test\"]\n[Site \"here\"]\n",
        &mut consumed,
    );
    assert_eq!(md.get("Event"), Some("test"));
    assert_eq!(md.get("Site"), Some("here"));
    assert!(consumed > 0);
}

#[test]
fn test_metadata_empty_string() {
    let md = PgnMetadata::from_string("");
    assert_eq!(md.get("Event"), None);
}

fn main() {}
