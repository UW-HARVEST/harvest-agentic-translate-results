use lambda_calculus_eval::io::{
    close_file, create_file, delete_file, get_file, next, write_to_file,
};
use std::io::{Read, Seek, SeekFrom, Write};

const TEMP_PATH_1: &str = "/tmp/test_io_lambda_test_1.txt";
const TEMP_PATH_2: &str = "/tmp/test_io_lambda_test_2.txt";
const TEMP_PATH_3: &str = "/tmp/test_io_lambda_test_3.txt";
const TEMP_PATH_4: &str = "/tmp/test_io_lambda_test_4.txt";
const TEMP_PATH_5: &str = "/tmp/test_io_lambda_test_5.txt";

#[test]
fn test_create_file_creates_file() {
    let _ = std::fs::remove_file(TEMP_PATH_1);
    let f = create_file(TEMP_PATH_1).expect("create_file failed");
    drop(f);
    assert!(std::path::Path::new(TEMP_PATH_1).exists());
    let _ = std::fs::remove_file(TEMP_PATH_1);
}

#[test]
fn test_get_file_round_trip() {
    let _ = std::fs::remove_file(TEMP_PATH_2);
    let mut writer = create_file(TEMP_PATH_2).expect("create_file failed");
    writer.write_all(b"Hello").expect("write failed");
    writer.flush().expect("flush failed");
    drop(writer);

    let mut reader = get_file(TEMP_PATH_2, "r").expect("get_file failed");
    let mut buf = [0u8; 100];
    let n = reader.read(&mut buf).expect("read failed");
    let read_str = std::str::from_utf8(&buf[..n]).expect("utf8");
    assert_eq!(read_str, "Hello");

    let _ = std::fs::remove_file(TEMP_PATH_2);
}

#[test]
fn test_delete_file() {
    let _ = std::fs::remove_file(TEMP_PATH_3);
    let f = create_file(TEMP_PATH_3).expect("create_file failed");
    drop(f);
    assert!(std::path::Path::new(TEMP_PATH_3).exists());
    delete_file(TEMP_PATH_3).expect("delete failed");
    assert!(!std::path::Path::new(TEMP_PATH_3).exists());
}

#[test]
fn test_close_file_drops_handle() {
    let _ = std::fs::remove_file(TEMP_PATH_4);
    let f = create_file(TEMP_PATH_4).expect("create_file failed");
    close_file(f).expect("close failed");
    assert!(std::path::Path::new(TEMP_PATH_4).exists());
    let _ = std::fs::remove_file(TEMP_PATH_4);
}

#[test]
fn test_next_reads_first_chars() {
    let _ = std::fs::remove_file(TEMP_PATH_5);
    let mut w = create_file(TEMP_PATH_5).expect("create_file failed");
    w.write_all(b"Hello World").expect("write failed");
    w.flush().expect("flush failed");
    drop(w);

    let mut r = get_file(TEMP_PATH_5, "r").expect("get_file failed");
    let c1 = next(&mut r).expect("next failed");
    let c2 = next(&mut r).expect("next failed");
    let c3 = next(&mut r).expect("next failed");
    assert_eq!(c1, 'H');
    assert_eq!(c2, 'e');
    assert_eq!(c3, 'l');

    let _ = std::fs::remove_file(TEMP_PATH_5);
}

#[test]
fn test_next_eof_returns_sentinel() {
    let path = "/tmp/test_io_lambda_test_eof.txt";
    let _ = std::fs::remove_file(path);
    let mut w = create_file(path).expect("create_file failed");
    w.write_all(b"x").expect("write failed");
    w.flush().expect("flush failed");
    drop(w);

    let mut r = get_file(path, "r").expect("get_file failed");
    let c = next(&mut r).expect("next failed");
    assert_eq!(c, 'x');
    let c2 = next(&mut r).expect("next failed");
    // EOF sentinel
    assert_eq!(c2, '\u{FFFF}');

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_write_to_file_rewinds() {
    let path = "/tmp/test_io_lambda_test_rewind.txt";
    let _ = std::fs::remove_file(path);
    // open with r+ to allow read/write
    let mut f = get_file(path, "w+").expect("create rw");
    write_to_file(&mut f, "abc").expect("write_to_file failed");

    // After write_to_file the C version rewinds the file. We should now be
    // able to read from the start.
    let mut buf = [0u8; 10];
    let n = f.read(&mut buf).expect("read");
    let s = std::str::from_utf8(&buf[..n]).expect("utf8");
    assert_eq!(s, "abc");

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_get_file_for_writing_does_not_clobber_in_r_plus() {
    // Verify "r+" keeps existing content (does not truncate)
    let path = "/tmp/test_io_lambda_test_rplus.txt";
    let _ = std::fs::remove_file(path);
    let mut w = create_file(path).expect("create");
    w.write_all(b"hello").expect("write");
    drop(w);

    let mut f = get_file(path, "r+").expect("r+ open");
    let mut buf = [0u8; 5];
    let n = f.read(&mut buf).expect("read");
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"hello");

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_get_file_w_truncates() {
    // "w" mode truncates existing content
    let path = "/tmp/test_io_lambda_test_wtrunc.txt";
    let _ = std::fs::remove_file(path);
    let mut w = create_file(path).expect("create");
    w.write_all(b"hello").expect("write");
    drop(w);

    let f = get_file(path, "w").expect("w open");
    drop(f);
    // file is now empty
    let meta = std::fs::metadata(path).expect("metadata");
    assert_eq!(meta.len(), 0);

    let _ = std::fs::remove_file(path);
}

#[test]
fn test_get_file_seek_and_read() {
    let path = "/tmp/test_io_lambda_test_seek.txt";
    let _ = std::fs::remove_file(path);
    let mut w = create_file(path).expect("create");
    w.write_all(b"abcdef").expect("write");
    drop(w);

    let mut r = get_file(path, "r").expect("r open");
    r.seek(SeekFrom::Start(2)).expect("seek");
    let c = next(&mut r).expect("next");
    assert_eq!(c, 'c');

    let _ = std::fs::remove_file(path);
}

fn main() {}
