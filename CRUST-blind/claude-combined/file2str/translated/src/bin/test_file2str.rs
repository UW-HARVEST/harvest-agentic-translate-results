use file2str::file2str as f2s_mod;
use std::fs;
use std::io::Write;

fn write_temp(name: &str, contents: &[u8]) -> String {
    let dir = std::env::temp_dir();
    let path = dir.join(name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(contents).unwrap();
    path.to_str().unwrap().to_string()
}

#[test]
fn test_file2strl_text_file() {
    // Matches C test: tests/test.txt with "Text file\n"
    let path = write_temp("file2str_test_textfile.txt", b"Text file\n");
    let mut len: u32 = 0;
    let result = f2s_mod::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(len, 11);
    assert_eq!(s, "Text file\n");
    assert_eq!(s.len(), 10);
}

#[test]
fn test_file2strl_hello_world() {
    // C confirms: printf "Hello, World!" -> len=14, strlen=13
    let path = write_temp("file2str_test_hello.txt", b"Hello, World!");
    let mut len: u32 = 0;
    let result = f2s_mod::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(len, 14);
    assert_eq!(s, "Hello, World!");
    assert_eq!(s.len(), 13);
}

#[test]
fn test_file2strl_empty_file() {
    // C confirms: empty file -> len=1, contents=""
    let path = write_temp("file2str_test_empty.txt", b"");
    let mut len: u32 = 0;
    let result = f2s_mod::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(len, 1);
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
}

#[test]
fn test_file2strl_missing_file_returns_none() {
    // C confirms: missing file -> NULL, returns None
    let mut len: u32 = 999;
    let result = f2s_mod::file2strl("/tmp/this_file_does_not_exist_zzz_42.txt", &mut len);
    assert!(result.is_none());
}

#[test]
fn test_file2str_text_file() {
    let path = write_temp("file2str_test_text2.txt", b"Text file\n");
    let result = f2s_mod::file2str(&path);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(s, "Text file\n");
    assert_eq!(s.len(), 10);
}

#[test]
fn test_file2str_empty_file() {
    let path = write_temp("file2str_test_empty2.txt", b"");
    let result = f2s_mod::file2str(&path);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
}

#[test]
fn test_file2str_missing_file_returns_none() {
    let result = f2s_mod::file2str("/tmp/this_file_does_not_exist_zzz_77.txt");
    assert!(result.is_none());
}

#[test]
fn test_file2strl_multiline_content() {
    let content = "line one\nline two\nline three\n";
    let path = write_temp("file2str_test_multi.txt", content.as_bytes());
    let mut len: u32 = 0;
    let result = f2s_mod::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(len, content.len() as u32 + 1);
    assert_eq!(s, content);
    assert_eq!(s.len(), content.len());
}

fn main() {}
