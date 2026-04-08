use mdb::mdb::{Mdb, MdbError, MdbOptions};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_db_name() -> String {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let name = format!("/tmp/mdb_test_{}_{}", pid, id);
    // Clean up any leftover files
    let _ = fs::remove_file(format!("{}.db.super", name));
    let _ = fs::remove_file(format!("{}.db.index", name));
    let _ = fs::remove_file(format!("{}.db.data", name));
    name
}

fn cleanup(name: &str) {
    let _ = fs::remove_file(format!("{}.db.super", name));
    let _ = fs::remove_file(format!("{}.db.index", name));
    let _ = fs::remove_file(format!("{}.db.data", name));
}

fn default_options(name: &str) -> MdbOptions {
    MdbOptions {
        db_name: name.to_string(),
        key_size_max: 64,
        data_size_max: 256,
        hash_buckets: 128,
        items_max: 166716,
    }
}

// ---- Hash function tests (via write/read with known bucket behavior) ----

#[test]
fn test_create_and_get_options() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let db = Mdb::create(&name, opts).unwrap();
    let got = db.get_options();
    assert_eq!(got.db_name, name);
    assert_eq!(got.key_size_max, 64);
    assert_eq!(got.data_size_max, 256);
    assert_eq!(got.hash_buckets, 128);
    assert_eq!(got.items_max, 166716);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_write_and_read_single_key() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key1", "value1").unwrap();
    let mut buf = vec![0u8; 257];
    let n = db.read("key1", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value1");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_write_and_read_two_keys() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key1", "value1").unwrap();
    db.write("key2", "value2").unwrap();

    let mut buf = vec![0u8; 257];
    let n = db.read("key1", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value1");

    let n = db.read("key2", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value2");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_overwrite_key() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key1", "value1").unwrap();
    db.write("key1", "updated1").unwrap();

    let mut buf = vec![0u8; 257];
    let n = db.read("key1", &mut buf).unwrap();
    assert_eq!(n, 8);
    assert_eq!(&buf[..8], b"updated1");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_delete_key() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key1", "value1").unwrap();
    db.delete("key1").unwrap();

    let mut buf = vec![0u8; 257];
    let result = db.read("key1", &mut buf);
    assert!(matches!(result, Err(MdbError::KeyNotFound)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_delete_nonexistent_key() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    let result = db.delete("nonexistent");
    assert!(matches!(result, Err(MdbError::KeyNotFound)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_read_nonexistent_key() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    let mut buf = vec![0u8; 257];
    let result = db.read("nonexistent", &mut buf);
    assert!(matches!(result, Err(MdbError::KeyNotFound)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_delete_then_other_key_still_readable() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key1", "value1").unwrap();
    db.write("key2", "value2").unwrap();
    db.delete("key1").unwrap();

    let mut buf = vec![0u8; 257];
    let n = db.read("key2", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value2");

    let result = db.read("key1", &mut buf);
    assert!(matches!(result, Err(MdbError::KeyNotFound)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_reopen_database() {
    let name = temp_db_name();
    let opts = default_options(&name);
    {
        let mut db = Mdb::create(&name, opts).unwrap();
        db.write("Lisp", "LambdaExpression").unwrap();
        // db dropped here, files closed
    }

    let mut db2 = Mdb::open(&name).unwrap();
    let opts2 = db2.get_options();
    assert_eq!(opts2.key_size_max, 64);
    assert_eq!(opts2.data_size_max, 256);
    assert_eq!(opts2.hash_buckets, 128);
    assert_eq!(opts2.items_max, 166716);

    let mut buf = vec![0u8; 257];
    let n = db2.read("Lisp", &mut buf).unwrap();
    assert_eq!(n, 16);
    assert_eq!(&buf[..16], b"LambdaExpression");
    drop(db2);
    cleanup(&name);
}

#[test]
fn test_reopen_deleted_key_not_found() {
    let name = temp_db_name();
    let opts = default_options(&name);
    {
        let mut db = Mdb::create(&name, opts).unwrap();
        db.write("key1", "value1").unwrap();
        db.write("key2", "value2").unwrap();
        db.delete("key1").unwrap();
    }

    let mut db2 = Mdb::open(&name).unwrap();
    let mut buf = vec![0u8; 257];
    let n = db2.read("key2", &mut buf).unwrap();
    assert_eq!(n, 6);
    assert_eq!(&buf[..6], b"value2");

    let result = db2.read("key1", &mut buf);
    assert!(matches!(result, Err(MdbError::KeyNotFound)));
    drop(db2);
    cleanup(&name);
}

#[test]
fn test_buffer_too_small() {
    let name = temp_db_name();
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 64,
        data_size_max: 256,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("k", "longvalue").unwrap(); // 9 bytes

    let mut buf = vec![0u8; 5]; // too small: need 10 (9+1)
    let result = db.read("k", &mut buf);
    assert!(matches!(result, Err(MdbError::BufferSizeTooSmall)));

    let mut buf = vec![0u8; 10]; // exact: 9+1=10
    let n = db.read("k", &mut buf).unwrap();
    assert_eq!(n, 9);
    assert_eq!(&buf[..9], b"longvalue");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_key_size_too_large() {
    let name = temp_db_name();
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 4,
        data_size_max: 256,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    let result = db.write("toolong", "val");
    assert!(matches!(result, Err(MdbError::KeySizeTooLarge)));

    // Key within limit should work
    db.write("ok", "val").unwrap();
    drop(db);
    cleanup(&name);
}

#[test]
fn test_value_size_too_large() {
    let name = temp_db_name();
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 64,
        data_size_max: 4,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    let result = db.write("k", "toolong");
    assert!(matches!(result, Err(MdbError::ValueSizeTooLarge)));

    // Value within limit should work
    db.write("k", "ok").unwrap();
    drop(db);
    cleanup(&name);
}

#[test]
fn test_multiple_keys_small_buckets() {
    let name = temp_db_name();
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 4,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("a", "alpha").unwrap();
    db.write("b", "beta").unwrap();
    db.write("c", "gamma").unwrap();
    db.write("d", "delta").unwrap();
    db.write("e", "epsilon").unwrap();

    let mut buf = vec![0u8; 257];
    let n = db.read("a", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"alpha");
    let n = db.read("b", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"beta");
    let n = db.read("c", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"gamma");
    let n = db.read("d", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"delta");
    let n = db.read("e", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"epsilon");

    // Delete middle key
    db.delete("c").unwrap();
    let result = db.read("c", &mut buf);
    assert!(matches!(result, Err(MdbError::KeyNotFound)));

    // Others still readable
    let n = db.read("a", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"alpha");
    let n = db.read("e", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"epsilon");

    // Write new key after delete (reuses freed index)
    db.write("f", "foxtrot").unwrap();
    let n = db.read("f", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"foxtrot");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_index_size_after_create() {
    let name = temp_db_name();
    let opts = default_options(&name);
    // index_size after create = freeptr(4) + 128 buckets * 4 = 516
    let mut db = Mdb::create(&name, opts).unwrap();
    let sz = db.index_size().unwrap();
    assert_eq!(sz, 4 + 128 * 4); // 516
    drop(db);
    cleanup(&name);
}

#[test]
fn test_data_size_after_create() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    let sz = db.data_size().unwrap();
    assert_eq!(sz, 0);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_index_size_grows_after_write() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    let sz_before = db.index_size().unwrap();
    db.write("key1", "value1").unwrap();
    let sz_after = db.index_size().unwrap();
    // index_record_size = key_size_max(64) + 2*PTR_SIZE(8) + DATALEN_SIZE(4) = 76
    assert_eq!(sz_after, sz_before + 76);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_empty_value() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("k", "").unwrap();
    let mut buf = vec![0u8; 257];
    let n = db.read("k", &mut buf).unwrap();
    assert_eq!(n, 0);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_overwrite_with_different_length() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("k", "short").unwrap();
    db.write("k", "a_much_longer_value").unwrap();
    let mut buf = vec![0u8; 257];
    let n = db.read("k", &mut buf).unwrap();
    assert_eq!(n, 19);
    assert_eq!(&buf[..19], b"a_much_longer_value");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_many_writes_and_reads() {
    let name = temp_db_name();
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 16,
        items_max: 1000,
    };
    let mut db = Mdb::create(&name, opts).unwrap();

    // Write 50 keys
    let mut expected = Vec::new();
    for i in 0..50u32 {
        let key = format!("k{}", i);
        let val = format!("v{}", i * 7);
        db.write(&key, &val).unwrap();
        expected.push((key, val));
    }

    // Read all back
    let mut buf = vec![0u8; 257];
    for (key, val) in &expected {
        let n = db.read(key, &mut buf).unwrap();
        assert_eq!(&buf[..n], val.as_bytes(), "mismatch for key {}", key);
    }
    drop(db);
    cleanup(&name);
}

#[test]
fn test_delete_and_reinsert() {
    let name = temp_db_name();
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("k", "first").unwrap();
    db.delete("k").unwrap();
    db.write("k", "second").unwrap();

    let mut buf = vec![0u8; 257];
    let n = db.read("k", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"second");
    drop(db);
    cleanup(&name);
}

fn main() {}
