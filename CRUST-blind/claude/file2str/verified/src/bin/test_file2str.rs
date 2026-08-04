use std::io::Write;

// The crate is named `file2str` and contains module `file2str` which exports
// functions `file2strl` and `file2str`. Use fully-qualified paths everywhere
// to avoid ambiguity between the crate, module, and function names.

fn write_temp_file(name: &str, bytes: &[u8]) -> String {
    let mut path = std::env::temp_dir();
    path.push(name);
    let mut f = std::fs::File::create(&path).expect("create");
    f.write_all(bytes).expect("write");
    path.to_str().unwrap().to_string()
}

#[test]
fn test_file2strl_text_file() {
    // Mirrors the original C test:
    //   buf = file2strl("tests/test.txt", &len)
    //   len == 11; strcmp(buf, "Text file\n") == 0
    let path = write_temp_file("rust_file2str_text.txt", b"Text file\n");
    let mut len: u32 = 12345;
    let result = ::file2str::file2str::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    // C reports len = file_len + 1 = 10 + 1 = 11
    assert_eq!(len, 11);
    // The Rust string holds the file's bytes (no trailing NUL)
    assert_eq!(s, "Text file\n");
    assert_eq!(s.len(), 10);
}

#[test]
fn test_file2strl_empty_file() {
    let path = write_temp_file("rust_file2str_empty.txt", b"");
    let mut len: u32 = 99;
    let result = ::file2str::file2str::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    // C: file_len = 0, len = 0 + 1 = 1
    assert_eq!(len, 1);
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
}

#[test]
fn test_file2strl_no_trailing_newline() {
    let path = write_temp_file("rust_file2str_hello.txt", b"Hello");
    let mut len: u32 = 0;
    let result = ::file2str::file2str::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    // C: file_len = 5, len = 6
    assert_eq!(len, 6);
    assert_eq!(s, "Hello");
    assert_eq!(s.len(), 5);
}

#[test]
fn test_file2strl_nonexistent_returns_none() {
    let mut len: u32 = 7777;
    let path = "/tmp/this_path_should_not_exist_for_file2str_8a7b6c5d.txt";
    let _ = std::fs::remove_file(path);
    let result = ::file2str::file2str::file2strl(path, &mut len);
    assert!(result.is_none());
}

#[test]
fn test_file2strl_single_byte() {
    let path = write_temp_file("rust_file2str_single.txt", b"X");
    let mut len: u32 = 0;
    let result = ::file2str::file2str::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    // C: file_len = 1, len = 2
    assert_eq!(len, 2);
    assert_eq!(s, "X");
    assert_eq!(s.len(), 1);
}

#[test]
fn test_file2strl_multiline() {
    let bytes = b"line1\nline2\nline3\n";
    let path = write_temp_file("rust_file2str_ml.txt", bytes);
    let mut len: u32 = 0;
    let result = ::file2str::file2str::file2strl(&path, &mut len);
    assert!(result.is_some());
    let s = result.unwrap();
    // C: file_len = 18, len = 19
    assert_eq!(len, 19);
    assert_eq!(s, "line1\nline2\nline3\n");
    assert_eq!(s.len(), 18);
}

#[test]
fn test_file2strl_does_not_overwrite_len_on_error() {
    // C code never assigns to file_len_out on error paths. The Rust API takes
    // `&mut u32`, but on error it should also leave the value alone.
    let path = "/tmp/file2str_doesnt_exist_zzzz_qqqq_pppp.txt";
    let _ = std::fs::remove_file(path);
    let mut len: u32 = 0xdead_beef;
    let result = ::file2str::file2str::file2strl(path, &mut len);
    assert!(result.is_none());
    assert_eq!(len, 0xdead_beef);
}

#[test]
fn test_file2str_text_file() {
    let path = write_temp_file("rust_file2str_wrap.txt", b"Text file\n");
    let result = ::file2str::file2str::file2str(&path);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(s, "Text file\n");
    assert_eq!(s.len(), 10);
}

#[test]
fn test_file2str_empty_file() {
    let path = write_temp_file("rust_file2str_wrap_empty.txt", b"");
    let result = ::file2str::file2str::file2str(&path);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
}

#[test]
fn test_file2str_nonexistent_returns_none() {
    let path = "/tmp/nope_file2str_no_exist_aabbccdd_ee.txt";
    let _ = std::fs::remove_file(path);
    let result = ::file2str::file2str::file2str(path);
    assert!(result.is_none());
}

#[test]
fn test_file2str_multiline() {
    let bytes = b"line1\nline2\nline3\n";
    let path = write_temp_file("rust_file2str_wrap_ml.txt", bytes);
    let result = ::file2str::file2str::file2str(&path);
    assert!(result.is_some());
    let s = result.unwrap();
    assert_eq!(s, "line1\nline2\nline3\n");
    assert_eq!(s.len(), 18);
}

fn main() {}
