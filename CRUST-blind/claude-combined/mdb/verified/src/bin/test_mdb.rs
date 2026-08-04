#![allow(dead_code, unused_imports)]

use mdb::mdb::{Mdb, MdbError, MdbOptions, mdb_status};
use std::path::PathBuf;

/// Helper: pick a unique temp directory inside the system temp dir to avoid
/// collisions when multiple tests run in parallel from the same `cargo test`
/// invocation.
fn unique_tmp(name: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut p = std::env::temp_dir();
    p.push(format!("rust_mdb_test_{}_{}_{}", name, pid, nanos));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn options_for(name: &str) -> MdbOptions {
    MdbOptions {
        db_name: name.to_string(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 4,
        items_max: 1000,
    }
}

#[test]
fn test_mdb_status_ok() {
    let s = mdb_status().unwrap();
    assert_eq!(s.code, 0);
    assert_eq!(s.desc, "");
}

#[test]
fn test_create_basic() {
    let dir = unique_tmp("create_basic");
    let path = dir.join("sizetest");
    let opts = options_for("sizetest");
    let mut db = Mdb::create(&path, opts.clone()).unwrap();

    // After create: index = MDB_PTR_SIZE (freeptr) + 4 buckets * MDB_PTR_SIZE
    // = 4 + 4*4 = 20. Data file = 0.
    assert_eq!(db.index_size().unwrap(), 20);
    assert_eq!(db.data_size().unwrap(), 0);

    let got = db.get_options();
    assert_eq!(got.db_name, opts.db_name);
    assert_eq!(got.key_size_max, opts.key_size_max);
    assert_eq!(got.data_size_max, opts.data_size_max);
    assert_eq!(got.hash_buckets, opts.hash_buckets);
    assert_eq!(got.items_max, opts.items_max);
}

#[test]
fn test_write_then_read() {
    let dir = unique_tmp("write_read");
    let path = dir.join("sizetest");
    let mut db = Mdb::create(&path, options_for("sizetest")).unwrap();

    db.write("k1", "value1").unwrap();
    // Index record size = key_size_max + MDB_PTR_SIZE*2 + MDB_DATALEN_SIZE
    // = 8 + 8 + 4 = 20. Initial 20 + 1 record = 40.
    assert_eq!(db.index_size().unwrap(), 40);
    assert_eq!(db.data_size().unwrap(), 6);

    let mut buf = [0u8; 300];
    let n = db.read("k1", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value1");
    // Null terminator written.
    assert_eq!(buf[6], 0);
}

#[test]
fn test_write_two_then_read_back() {
    let dir = unique_tmp("write_two");
    let path = dir.join("sizetest");
    let mut db = Mdb::create(&path, options_for("sizetest")).unwrap();

    db.write("k1", "value1").unwrap();
    db.write("k2", "value2").unwrap();
    // Two index records.
    assert_eq!(db.index_size().unwrap(), 60);
    assert_eq!(db.data_size().unwrap(), 12);

    let mut buf = [0u8; 300];
    let n = db.read("k1", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value1");

    let mut buf2 = [0u8; 300];
    let n = db.read("k2", &mut buf2).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf2[..6], b"value2");
}

#[test]
fn test_update_existing_key() {
    let dir = unique_tmp("update_key");
    let path = dir.join("sizetest");
    let mut db = Mdb::create(&path, options_for("sizetest")).unwrap();

    db.write("k1", "value1").unwrap();
    db.write("k2", "value2").unwrap();
    // Update k1.
    db.write("k1", "value1updated").unwrap();
    // Index size unchanged: 60. Data extended (matches C: 25).
    assert_eq!(db.index_size().unwrap(), 60);
    assert_eq!(db.data_size().unwrap(), 25);

    let mut buf = [0u8; 300];
    let n = db.read("k1", &mut buf).unwrap();
    assert_eq!(n, 13);
    assert_eq!(&buf[..13], b"value1updated");

    let n = db.read("k2", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value2");
}

#[test]
fn test_delete_then_reuse() {
    let dir = unique_tmp("delete_reuse");
    let path = dir.join("sizetest");
    let mut db = Mdb::create(&path, options_for("sizetest")).unwrap();

    db.write("k1", "value1").unwrap();
    db.write("k2", "value2").unwrap();
    db.write("k1", "value1updated").unwrap();
    db.delete("k2").unwrap();
    // Sizes unchanged (free list reuse, not truncate).
    assert_eq!(db.index_size().unwrap(), 60);
    assert_eq!(db.data_size().unwrap(), 25);

    // Reading k2 returns key-not-found.
    let mut buf = [0u8; 300];
    let err = db.read("k2", &mut buf).unwrap_err();
    assert!(matches!(err, MdbError::KeyNotFound));

    // Reading a never-existed key returns key-not-found.
    let err = db.read("nokey", &mut buf).unwrap_err();
    assert!(matches!(err, MdbError::KeyNotFound));

    // Now write k3 — should reuse the freed index slot.
    db.write("k3", "v3").unwrap();
    // Sizes still unchanged (matches C: 60, 25).
    assert_eq!(db.index_size().unwrap(), 60);
    assert_eq!(db.data_size().unwrap(), 25);

    let n = db.read("k3", &mut buf).unwrap();
    assert_eq!(n, 2);
    assert_eq!(&buf[..2], b"v3");
}

#[test]
fn test_delete_nonexistent_key() {
    let dir = unique_tmp("del_none");
    let path = dir.join("dnone");
    let mut db = Mdb::create(&path, options_for("dnone")).unwrap();
    let err = db.delete("nope").unwrap_err();
    assert!(matches!(err, MdbError::KeyNotFound));
}

#[test]
fn test_oversized_key() {
    let dir = unique_tmp("oversized_key");
    let path = dir.join("edgetest");
    let opts = MdbOptions {
        db_name: "edgetest".to_string(),
        key_size_max: 4,
        data_size_max: 8,
        hash_buckets: 2,
        items_max: 100,
    };
    let mut db = Mdb::create(&path, opts).unwrap();
    let err = db.write("abcdefg", "ok").unwrap_err();
    assert!(matches!(err, MdbError::KeySizeTooLarge));
}

#[test]
fn test_oversized_value() {
    let dir = unique_tmp("oversized_val");
    let path = dir.join("edgetest");
    let opts = MdbOptions {
        db_name: "edgetest".to_string(),
        key_size_max: 4,
        data_size_max: 8,
        hash_buckets: 2,
        items_max: 100,
    };
    let mut db = Mdb::create(&path, opts).unwrap();
    let err = db.write("ab", "012345678").unwrap_err();
    assert!(matches!(err, MdbError::ValueSizeTooLarge));
}

#[test]
fn test_max_sized_key() {
    let dir = unique_tmp("max_key");
    let path = dir.join("edgetest");
    let opts = MdbOptions {
        db_name: "edgetest".to_string(),
        key_size_max: 4,
        data_size_max: 8,
        hash_buckets: 2,
        items_max: 100,
    };
    let mut db = Mdb::create(&path, opts).unwrap();
    db.write("abcd", "v").unwrap();
    let mut buf = [0u8; 20];
    let n = db.read("abcd", &mut buf).unwrap();
    assert_eq!(n, 1);
    assert_eq!(&buf[..1], b"v");
}

#[test]
fn test_buffer_too_small() {
    let dir = unique_tmp("bufsmall");
    let path = dir.join("edgetest");
    let opts = MdbOptions {
        db_name: "edgetest".to_string(),
        key_size_max: 4,
        data_size_max: 8,
        hash_buckets: 2,
        items_max: 100,
    };
    let mut db = Mdb::create(&path, opts).unwrap();
    db.write("abcd", "v").unwrap();
    let mut tiny = [0u8; 1];
    let err = db.read("abcd", &mut tiny).unwrap_err();
    assert!(matches!(err, MdbError::BufferSizeTooSmall));
    let mut just_enough = [0u8; 2];
    let n = db.read("abcd", &mut just_enough).unwrap();
    assert_eq!(n, 1);
    assert_eq!(just_enough[0], b'v');
    assert_eq!(just_enough[1], 0);
}

#[test]
fn test_create_then_open_roundtrip() {
    let dir = unique_tmp("reopen");
    let path = dir.join("sizetest");
    {
        let mut db = Mdb::create(&path, options_for("sizetest")).unwrap();
        db.write("k1", "value1").unwrap();
        db.write("k2", "value2").unwrap();
        db.write("k1", "value1updated").unwrap();
        db.delete("k2").unwrap();
        db.write("k3", "v3").unwrap();
        // db drops here, file handles flushed/closed.
    }

    let mut db2 = Mdb::open(&path).unwrap();
    let opts = db2.get_options().clone();
    assert_eq!(opts.db_name, "sizetest");
    assert_eq!(opts.key_size_max, 8);
    assert_eq!(opts.data_size_max, 256);
    assert_eq!(opts.hash_buckets, 4);
    assert_eq!(opts.items_max, 1000);

    let mut buf = [0u8; 300];
    let n = db2.read("k1", &mut buf).unwrap();
    assert_eq!(n, 13);
    assert_eq!(&buf[..13], b"value1updated");

    let n = db2.read("k3", &mut buf).unwrap();
    assert_eq!(n, 2);
    assert_eq!(&buf[..2], b"v3");

    let err = db2.read("k2", &mut buf).unwrap_err();
    assert!(matches!(err, MdbError::KeyNotFound));
}

#[test]
fn test_collision_chaining() {
    // Use a single-bucket hash table to force all keys to collide.
    let dir = unique_tmp("collision");
    let path = dir.join("collide");
    let opts = MdbOptions {
        db_name: "collide".to_string(),
        key_size_max: 8,
        data_size_max: 64,
        hash_buckets: 1,
        items_max: 100,
    };
    let mut db = Mdb::create(&path, opts).unwrap();

    // Write several keys, all collide.
    db.write("a", "1").unwrap();
    db.write("b", "22").unwrap();
    db.write("c", "333").unwrap();
    db.write("d", "4444").unwrap();

    let mut buf = [0u8; 64];
    let n = db.read("a", &mut buf).unwrap();
    assert_eq!(n, 1);
    assert_eq!(&buf[..1], b"1");

    let n = db.read("b", &mut buf).unwrap();
    assert_eq!(n, 2);
    assert_eq!(&buf[..2], b"22");

    let n = db.read("c", &mut buf).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&buf[..3], b"333");

    let n = db.read("d", &mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"4444");

    // Delete from the middle and re-read.
    db.delete("b").unwrap();
    let err = db.read("b", &mut buf).unwrap_err();
    assert!(matches!(err, MdbError::KeyNotFound));

    let n = db.read("a", &mut buf).unwrap();
    assert_eq!(n, 1);
    assert_eq!(&buf[..1], b"1");
    let n = db.read("c", &mut buf).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&buf[..3], b"333");
    let n = db.read("d", &mut buf).unwrap();
    assert_eq!(n, 4);
    assert_eq!(&buf[..4], b"4444");
}

#[test]
fn test_load_many_keys() {
    // Mirrors the C "load_test" behavior: write 1000 keys, then read all back.
    let dir = unique_tmp("load_many");
    let path = dir.join("accelerator");
    let opts = MdbOptions {
        db_name: "accelerator".to_string(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 128,
        items_max: 166716,
    };
    let mut db = Mdb::create(&path, opts).unwrap();

    let preset_values = [
        "misakawa", "kamijou", "accelerator", "index", "lyzh", "chuigda",
        "de_nuke", "de_mirage", "de_cache", "de_vertigo", "de_inferno",
        "de_cbble", "de_dust2", "fy_iceworld", "cs_assault", "komakan",
        "scarlet", "flandre",
    ];

    let mut keys: Vec<String> = Vec::new();
    let mut values: Vec<&str> = Vec::new();
    let mut i: usize = 0;
    for c1 in b'0'..=b'9' {
        for c2 in b'0'..=b'9' {
            for c3 in b'0'..=b'9' {
                let key = format!(
                    "{}{}{}",
                    c1 as char, c2 as char, c3 as char
                );
                let val = preset_values[i % 18];
                db.write(&key, val).unwrap();
                keys.push(key);
                values.push(val);
                i += 1;
            }
        }
    }

    let mut buf = [0u8; 300];
    for (k, v) in keys.iter().zip(values.iter()) {
        let n = db.read(k, &mut buf).unwrap();
        assert_eq!(n, v.len());
        assert_eq!(&buf[..n], v.as_bytes());
    }
}

fn main() {}
