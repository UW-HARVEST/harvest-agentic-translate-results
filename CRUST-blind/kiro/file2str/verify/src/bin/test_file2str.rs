use file2str::file2str::{file2str, file2strl};

#[test]
fn test_file2strl_reads_file() {
    let mut len = 0u32;
    let result = file2strl("tests/test.txt", &mut len);
    assert_eq!(result, Some("Text file\n".to_string()));
    assert_eq!(len, 11);
}

#[test]
fn test_file2str_reads_file() {
    let result = file2str("tests/test.txt");
    assert_eq!(result, Some("Text file\n".to_string()));
}

#[test]
fn test_nonexistent_file_returns_none() {
    let mut len = 0u32;
    assert_eq!(file2strl("nonexistent_file.txt", &mut len), None);
    assert_eq!(file2str("nonexistent_file.txt"), None);
}

#[test]
fn test_empty_file() {
    let path = "/tmp/test_empty_file2str.txt";
    std::fs::File::create(path).unwrap();
    let mut len = 0u32;
    let result = file2strl(path, &mut len);
    assert_eq!(result, Some("".to_string()));
    assert_eq!(len, 1);
    std::fs::remove_file(path).ok();
}

#[test]
fn test_file2strl_len_not_modified_on_error() {
    let mut len = 999u32;
    let result = file2strl("nonexistent.txt", &mut len);
    assert!(result.is_none());
    // In C, len is not modified when file open fails (stays at caller's value).
    // Rust returns None early before setting len, so len stays unchanged.
    assert_eq!(len, 999);
}

#[test]
fn test_binary_content_file() {
    let path = "/tmp/test_binary_file2str.txt";
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(b"hello\x00world").unwrap();
    drop(f);
    // C reads raw bytes including null; Rust read_to_string fails on embedded null
    // since it's not valid UTF-8 in the middle of a string... actually \x00 IS valid UTF-8.
    // But read_to_string should handle it. Let's verify.
    let mut len = 0u32;
    let result = file2strl(path, &mut len);
    assert_eq!(result, Some("hello\x00world".to_string()));
    assert_eq!(len, 12); // 11 bytes + 1
    std::fs::remove_file(path).ok();
}

fn main() {}
