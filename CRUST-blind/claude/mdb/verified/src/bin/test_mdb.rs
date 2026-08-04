#![allow(dead_code, unused_imports)]

use mdb::mdb::{Mdb, MdbError, MdbOptions, MdbStatus, MdbStatusCode, mdb_status};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_dir(label: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("mdb_test_{}_{}_{}_{}", label, pid, nanos, n));
    std::fs::create_dir_all(&dir).expect("create test dir");
    dir
}

fn db_path(dir: &PathBuf, name: &str) -> PathBuf {
    dir.join(name)
}

fn make_options(name: &str, key_max: u16, data_max: u32, buckets: u32, items: u32) -> MdbOptions {
    MdbOptions {
        db_name: name.to_string(),
        key_size_max: key_max,
        data_size_max: data_max,
        hash_buckets: buckets,
        items_max: items,
    }
}

fn read_value(db: &mut Mdb, key: &str) -> String {
    let mut buf = vec![0u8; 1024];
    let n = db.read(key, &mut buf).expect("read");
    String::from_utf8(buf[..n].to_vec()).expect("utf8")
}

#[test]
fn test_create_and_get_options() {
    let dir = unique_dir("opts");
    let path = db_path(&dir, "mydb");
    let options = make_options("mydb", 64, 256, 128, 10000);
    let db = Mdb::create(&path, options.clone()).expect("create");

    let got = db.get_options();
    assert_eq!(got.db_name, "mydb");
    assert_eq!(got.key_size_max, 64);
    assert_eq!(got.data_size_max, 256);
    assert_eq!(got.hash_buckets, 128);
    assert_eq!(got.items_max, 10000);
}

#[test]
fn test_simple_write_and_read() {
    let dir = unique_dir("simple_rw");
    let path = db_path(&dir, "simple");
    let mut db = Mdb::create(&path, make_options("simple", 64, 256, 128, 10000)).expect("create");

    db.write("hello", "world").expect("write");
    let mut buf = vec![0u8; 257];
    let n = db.read("hello", &mut buf).expect("read");
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"world");
    // Should be null-terminated within the buffer (mirrors C valbuf[valsize] = '\0')
    assert_eq!(buf[n], 0);
}

#[test]
fn test_read_missing_key() {
    let dir = unique_dir("missing");
    let path = db_path(&dir, "missdb");
    let mut db = Mdb::create(&path, make_options("missdb", 64, 256, 128, 10000)).expect("create");

    db.write("hello", "world").expect("write");

    let mut buf = vec![0u8; 257];
    match db.read("missing", &mut buf) {
        Err(MdbError::KeyNotFound) => {}
        other => panic!("expected KeyNotFound, got {:?}", other),
    }
}

#[test]
fn test_read_bufsize_too_small() {
    let dir = unique_dir("bufsize");
    let path = db_path(&dir, "bsdb");
    let mut db = Mdb::create(&path, make_options("bsdb", 64, 256, 128, 10000)).expect("create");

    db.write("hello", "world").expect("write");

    // Need at least valsize+1 = 6 bytes per C semantics.
    let mut small = vec![0u8; 5];
    match db.read("hello", &mut small) {
        Err(MdbError::BufferSizeTooSmall) => {}
        other => panic!("expected BufferSizeTooSmall, got {:?}", other),
    }

    let mut just_right = vec![0u8; 6];
    let n = db.read("hello", &mut just_right).expect("read just-right");
    assert_eq!(n, 5);
    assert_eq!(&just_right[..n], b"world");
    assert_eq!(just_right[5], 0);
}

#[test]
fn test_write_key_too_large() {
    let dir = unique_dir("keytoolarge");
    let path = db_path(&dir, "ktl");
    let mut db = Mdb::create(&path, make_options("ktl", 64, 256, 128, 10000)).expect("create");

    let bigkey = "a".repeat(199);
    match db.write(&bigkey, "x") {
        Err(MdbError::KeySizeTooLarge) => {}
        other => panic!("expected KeySizeTooLarge, got {:?}", other),
    }
}

#[test]
fn test_write_key_at_max_allowed() {
    let dir = unique_dir("keyatmax");
    let path = db_path(&dir, "kam");
    let mut db = Mdb::create(&path, make_options("kam", 64, 256, 128, 10000)).expect("create");

    // strlen == key_size_max should be allowed (C uses strict >).
    let key = "k".repeat(64);
    db.write(&key, "vmax").expect("write at max");
    let mut buf = vec![0u8; 257];
    let n = db.read(&key, &mut buf).expect("read at max");
    assert_eq!(n, 4);
    assert_eq!(&buf[..n], b"vmax");
}

#[test]
fn test_write_value_too_large() {
    let dir = unique_dir("valtoolarge");
    let path = db_path(&dir, "vtl");
    let mut db = Mdb::create(&path, make_options("vtl", 64, 256, 128, 10000)).expect("create");

    let bigval = "b".repeat(299);
    match db.write("key2", &bigval) {
        Err(MdbError::ValueSizeTooLarge) => {}
        other => panic!("expected ValueSizeTooLarge, got {:?}", other),
    }
}

#[test]
fn test_write_value_at_max_allowed() {
    let dir = unique_dir("valatmax");
    let path = db_path(&dir, "vam");
    let mut db = Mdb::create(&path, make_options("vam", 64, 256, 128, 10000)).expect("create");

    // strlen == data_size_max should be allowed (C uses strict >).
    let val = "v".repeat(256);
    db.write("k", &val).expect("write val at max");
    let mut buf = vec![0u8; 257];
    let n = db.read("k", &mut buf).expect("read val at max");
    assert_eq!(n, 256);
    assert_eq!(&buf[..n], val.as_bytes());
}

#[test]
fn test_update_existing_key() {
    let dir = unique_dir("update");
    let path = db_path(&dir, "upddb");
    let mut db = Mdb::create(&path, make_options("upddb", 64, 256, 128, 10000)).expect("create");

    db.write("hello", "world").expect("write1");
    db.write("hello", "everyone").expect("write2");
    let mut buf = vec![0u8; 257];
    let n = db.read("hello", &mut buf).expect("read");
    assert_eq!(n, 8);
    assert_eq!(&buf[..n], b"everyone");
}

#[test]
fn test_delete_then_read() {
    let dir = unique_dir("del");
    let path = db_path(&dir, "deldb");
    let mut db = Mdb::create(&path, make_options("deldb", 64, 256, 128, 10000)).expect("create");

    db.write("hello", "world").expect("write");
    db.delete("hello").expect("delete");

    let mut buf = vec![0u8; 257];
    match db.read("hello", &mut buf) {
        Err(MdbError::KeyNotFound) => {}
        other => panic!("expected KeyNotFound after delete, got {:?}", other),
    }
}

#[test]
fn test_delete_missing_key() {
    let dir = unique_dir("delmiss");
    let path = db_path(&dir, "dmdb");
    let mut db = Mdb::create(&path, make_options("dmdb", 64, 256, 128, 10000)).expect("create");

    // Delete with empty db.
    match db.delete("absent") {
        Err(MdbError::KeyNotFound) => {}
        other => panic!("expected KeyNotFound, got {:?}", other),
    }

    db.write("hello", "world").expect("write");
    db.delete("hello").expect("delete");
    // Delete again on now-missing key.
    match db.delete("hello") {
        Err(MdbError::KeyNotFound) => {}
        other => panic!("expected KeyNotFound, got {:?}", other),
    }
}

#[test]
fn test_multiple_keys_in_same_bucket() {
    // Force collisions via tiny bucket count.
    let dir = unique_dir("collide");
    let path = db_path(&dir, "cdb");
    let mut db = Mdb::create(&path, make_options("cdb", 32, 256, 4, 10000)).expect("create");

    db.write("alpha", "one").expect("w1");
    db.write("beta", "two").expect("w2");
    db.write("gamma", "three").expect("w3");
    db.write("delta", "four").expect("w4");
    db.write("epsilon", "five").expect("w5");

    assert_eq!(read_value(&mut db, "alpha"), "one");
    assert_eq!(read_value(&mut db, "beta"), "two");
    assert_eq!(read_value(&mut db, "gamma"), "three");
    assert_eq!(read_value(&mut db, "delta"), "four");
    assert_eq!(read_value(&mut db, "epsilon"), "five");

    // Delete the middle of a chain and ensure others still readable.
    db.delete("gamma").expect("del gamma");
    assert_eq!(read_value(&mut db, "alpha"), "one");
    assert_eq!(read_value(&mut db, "beta"), "two");
    assert_eq!(read_value(&mut db, "delta"), "four");
    assert_eq!(read_value(&mut db, "epsilon"), "five");
    match db.read("gamma", &mut [0u8; 32]) {
        Err(MdbError::KeyNotFound) => {}
        other => panic!("expected KeyNotFound for gamma, got {:?}", other),
    }
}

#[test]
fn test_reopen_persists_options_and_data() {
    let dir = unique_dir("reopen");
    let path = db_path(&dir, "lambda");
    {
        let mut db = Mdb::create(&path, make_options("lambda", 64, 256, 128, 166716))
            .expect("create");
        db.write("Lisp", "LambdaExpression").expect("write");
        // Drop closes files.
    }

    let mut db = Mdb::open(&path).expect("reopen");
    let opts = db.get_options();
    assert_eq!(opts.db_name, "lambda");
    assert_eq!(opts.key_size_max, 64);
    assert_eq!(opts.data_size_max, 256);
    assert_eq!(opts.hash_buckets, 128);
    assert_eq!(opts.items_max, 166716);

    let mut buf = vec![0u8; 257];
    let n = db.read("Lisp", &mut buf).expect("read after reopen");
    assert_eq!(n, "LambdaExpression".len());
    assert_eq!(&buf[..n], b"LambdaExpression");
}

#[test]
fn test_load_test_3digit_keys() {
    // Mirrors C `load_test2`: write and read 1000 unique numeric keys.
    let dir = unique_dir("load3");
    let path = db_path(&dir, "accel");
    let mut db = Mdb::create(&path, make_options("accel", 8, 256, 128, 166716)).expect("create");

    let presets = [
        "misakawa", "kamijou", "accelerator", "index", "lyzh", "chuigda",
        "de_nuke", "de_mirage", "de_cache", "de_vertigo", "de_inferno",
        "de_cbble", "de_dust2", "fy_iceworld", "cs_assault", "komakan",
        "scarlet", "flandre",
    ];

    let mut values: Vec<String> = Vec::with_capacity(1000);
    let mut i = 0usize;
    for c1 in b'0'..=b'9' {
        for c2 in b'0'..=b'9' {
            for c3 in b'0'..=b'9' {
                let key = String::from_utf8(vec![c1, c2, c3]).unwrap();
                let v = presets[i % presets.len()];
                db.write(&key, v).expect("write");
                values.push(v.to_string());
                i += 1;
            }
        }
    }
    assert_eq!(values.len(), 1000);

    i = 0;
    for c1 in b'0'..=b'9' {
        for c2 in b'0'..=b'9' {
            for c3 in b'0'..=b'9' {
                let key = String::from_utf8(vec![c1, c2, c3]).unwrap();
                let mut buf = vec![0u8; 257];
                let n = db.read(&key, &mut buf).expect("read");
                let got = std::str::from_utf8(&buf[..n]).expect("utf8");
                assert_eq!(got, values[i], "key {}", key);
                i += 1;
            }
        }
    }
}

#[test]
fn test_empty_key() {
    let dir = unique_dir("emptykey");
    let path = db_path(&dir, "ekdb");
    let mut db = Mdb::create(&path, make_options("ekdb", 16, 16, 4, 100)).expect("create");

    db.write("", "v1").expect("write empty key");
    let mut buf = vec![0u8; 32];
    let n = db.read("", &mut buf).expect("read empty key");
    assert_eq!(n, 2);
    assert_eq!(&buf[..n], b"v1");
}

#[test]
fn test_empty_value() {
    let dir = unique_dir("emptyval");
    let path = db_path(&dir, "evdb");
    let mut db = Mdb::create(&path, make_options("evdb", 16, 16, 4, 100)).expect("create");

    db.write("k", "").expect("write empty val");
    let mut buf = vec![0u8; 32];
    let n = db.read("k", &mut buf).expect("read empty val");
    assert_eq!(n, 0);
    // C writes valbuf[0] = '\0' and returns OK.
    assert_eq!(buf[0], 0);
}

#[test]
fn test_empty_value_buf_too_small() {
    // Empty value: bufsiz must be at least 1 (valsize+1=1).
    let dir = unique_dir("emptyval_buf0");
    let path = db_path(&dir, "evdb0");
    let mut db = Mdb::create(&path, make_options("evdb0", 16, 16, 4, 100)).expect("create");

    db.write("k", "").expect("write empty val");
    let mut empty_buf: [u8; 0] = [];
    match db.read("k", &mut empty_buf) {
        Err(MdbError::BufferSizeTooSmall) => {}
        other => panic!("expected BufferSizeTooSmall, got {:?}", other),
    }

    // bufsiz=1 should succeed.
    let mut tiny = [0u8; 1];
    let n = db.read("k", &mut tiny).expect("read tiny");
    assert_eq!(n, 0);
    assert_eq!(tiny[0], 0);
}

#[test]
fn test_index_size_grows_with_create() {
    // C: file size after create = MDB_PTR_SIZE * (hash_buckets + 1)
    let dir = unique_dir("idxsize_create");
    let path = db_path(&dir, "isdb");
    let buckets = 128u32;
    let mut db = Mdb::create(&path, make_options("isdb", 64, 256, buckets, 10000))
        .expect("create");

    let expected = 4u64 * (buckets as u64 + 1);
    assert_eq!(db.index_size().expect("index_size"), expected);
}

#[test]
fn test_data_size_initially_zero() {
    let dir = unique_dir("datasize_zero");
    let path = db_path(&dir, "dsdb");
    let mut db = Mdb::create(&path, make_options("dsdb", 64, 256, 128, 10000))
        .expect("create");
    assert_eq!(db.data_size().expect("data_size"), 0);
}

#[test]
fn test_index_and_data_size_after_writes() {
    // Mirrors probe2.c results:
    //   create (buckets=128): idx=516 data=0
    //   after write hello=world: idx=592 data=5
    //   after write foo=bar:    idx=668 data=8
    //   after update hello=everyone: idx=668 data=16
    //   after delete foo:       idx=668 data=16
    //   after write baz=qux:    idx=668 data=16
    let dir = unique_dir("sizes_progress");
    let path = db_path(&dir, "probe2");
    let mut db = Mdb::create(&path, make_options("probe2", 64, 256, 128, 10000))
        .expect("create");
    assert_eq!(db.index_size().expect("idx"), 516);
    assert_eq!(db.data_size().expect("data"), 0);

    db.write("hello", "world").expect("w1");
    assert_eq!(db.index_size().expect("idx"), 592);
    assert_eq!(db.data_size().expect("data"), 5);

    db.write("foo", "bar").expect("w2");
    assert_eq!(db.index_size().expect("idx"), 668);
    assert_eq!(db.data_size().expect("data"), 8);

    db.write("hello", "everyone").expect("update");
    assert_eq!(db.index_size().expect("idx"), 668);
    assert_eq!(db.data_size().expect("data"), 16);

    db.delete("foo").expect("delete foo");
    assert_eq!(db.index_size().expect("idx"), 668);
    assert_eq!(db.data_size().expect("data"), 16);

    db.write("baz", "qux").expect("w3");
    // baz/qux reuses freed index slot and freed data slot.
    assert_eq!(db.index_size().expect("idx"), 668);
    assert_eq!(db.data_size().expect("data"), 16);

    // And ensure reads still work.
    let mut buf = vec![0u8; 300];
    let n = db.read("hello", &mut buf).expect("read hello");
    assert_eq!(&buf[..n], b"everyone");
    let n = db.read("baz", &mut buf).expect("read baz");
    assert_eq!(&buf[..n], b"qux");
    match db.read("foo", &mut buf) {
        Err(MdbError::KeyNotFound) => {}
        other => panic!("expected KeyNotFound for foo, got {:?}", other),
    }
}

#[test]
fn test_status_code_values() {
    // Verify enum discriminants match the C values in the header.
    assert_eq!(MdbStatusCode::MDB_OK as u8, 0);
    assert_eq!(MdbStatusCode::MDB_NO_KEY as u8, 1);
    assert_eq!(MdbStatusCode::MDB_ERR_CRITICAL as u8, 2);
    assert_eq!(MdbStatusCode::MDB_ERR_LOGIC as u8, 3);
    assert_eq!(MdbStatusCode::MDB_ERR_FLUSH as u8, 4);
    assert_eq!(MdbStatusCode::MDB_ERR_OPEN_FILE as u8, 5);
    assert_eq!(MdbStatusCode::MDB_ERR_READ as u8, 6);
    assert_eq!(MdbStatusCode::MDB_ERR_WRITE as u8, 7);
    assert_eq!(MdbStatusCode::MDB_ERR_ALLOC as u8, 8);
    assert_eq!(MdbStatusCode::MDB_ERR_SEEK as u8, 9);
    assert_eq!(MdbStatusCode::MDB_ERR_BUFSIZ as u8, 10);
    assert_eq!(MdbStatusCode::MDB_ERR_KEY_SIZE as u8, 11);
    assert_eq!(MdbStatusCode::MDB_ERR_VALUE_SIZE as u8, 12);
    assert_eq!(MdbStatusCode::MDB_ERR_UNIMPLEMENTED as u8, 100);
}

#[test]
fn test_mdb_status_helper() {
    let s: MdbStatus = mdb_status().expect("mdb_status");
    assert_eq!(s.code, 0);
    assert_eq!(s.desc, "");
}

#[test]
fn test_open_nonexistent_fails() {
    let dir = unique_dir("open_missing");
    let path = db_path(&dir, "no_such_db");
    match Mdb::open(&path) {
        Err(_) => {}
        Ok(_) => panic!("opening nonexistent db should fail"),
    }
}

#[test]
fn test_options_clone_independence() {
    // The MdbOptions struct must be Clone'able.
    let opts = make_options("foo", 8, 32, 4, 10);
    let cloned = opts.clone();
    assert_eq!(opts.db_name, cloned.db_name);
    assert_eq!(opts.key_size_max, cloned.key_size_max);
    assert_eq!(opts.data_size_max, cloned.data_size_max);
    assert_eq!(opts.hash_buckets, cloned.hash_buckets);
    assert_eq!(opts.items_max, cloned.items_max);
}

fn main() {}
