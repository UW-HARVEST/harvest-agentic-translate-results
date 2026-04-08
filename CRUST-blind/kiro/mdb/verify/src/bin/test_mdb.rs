use mdb::mdb::{Mdb, MdbError, MdbOptions};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn unique_db_name(prefix: &str) -> String {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("/tmp/mdb_test_{}_{}", prefix, id)
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

fn cleanup(name: &str) {
    let _ = std::fs::remove_file(format!("{}.db.super", name));
    let _ = std::fs::remove_file(format!("{}.db.index", name));
    let _ = std::fs::remove_file(format!("{}.db.data", name));
}

// ---- Hash function tests (via observable bucket behavior) ----

#[test]
fn test_hash_empty_key() {
    // hash("") = 0, bucket = 0 % 128 = 0
    // Writing empty key should work (key_size 0 <= 64)
    let name = unique_db_name("hash_empty");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("", "value").unwrap();
    let mut buf = [0u8; 257];
    let n = db.read("", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"value");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_happy_path() {
    // Mirrors C happy_test0: create, write, read, delete, read-after-delete
    let name = unique_db_name("happy");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();

    db.write("misakawa", "mikoto").unwrap();

    let mut buf = [0u8; 257];
    let n = db.read("misakawa", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"mikoto");

    db.delete("misakawa").unwrap();

    let err = db.read("misakawa", &mut buf);
    assert!(matches!(err, Err(MdbError::KeyNotFound)));

    drop(db);
    cleanup(&name);
}

#[test]
fn test_reopen() {
    // Mirrors C reopen_test1: create, write, close, open, verify options + read
    let name = unique_db_name("reopen");
    let opts = default_options(&name);
    {
        let mut db = Mdb::create(&name, opts.clone()).unwrap();
        db.write("Lisp", "LambdaExpression").unwrap();
    }

    let mut db = Mdb::open(&name).unwrap();
    let ro = db.get_options();
    assert_eq!(ro.db_name, name);
    assert_eq!(ro.key_size_max, 64);
    assert_eq!(ro.data_size_max, 256);
    assert_eq!(ro.hash_buckets, 128);
    assert_eq!(ro.items_max, 166716);

    let mut buf = [0u8; 257];
    let n = db.read("Lisp", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"LambdaExpression");

    drop(db);
    cleanup(&name);
}

#[test]
fn test_overwrite_value() {
    // Write same key twice, second value should replace first
    let name = unique_db_name("overwrite");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();

    db.write("key1", "first").unwrap();
    db.write("key1", "second").unwrap();

    let mut buf = [0u8; 257];
    let n = db.read("key1", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"second");

    drop(db);
    cleanup(&name);
}

#[test]
fn test_multiple_keys() {
    let name = unique_db_name("multi");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();

    db.write("alpha", "one").unwrap();
    db.write("beta", "two").unwrap();
    db.write("gamma", "three").unwrap();

    let mut buf = [0u8; 257];
    let n = db.read("alpha", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"one");
    let n = db.read("beta", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"two");
    let n = db.read("gamma", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"three");

    drop(db);
    cleanup(&name);
}

#[test]
fn test_delete_nonexistent_key() {
    let name = unique_db_name("del_nokey");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    let err = db.delete("nonexistent");
    assert!(matches!(err, Err(MdbError::KeyNotFound)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_read_nonexistent_key() {
    let name = unique_db_name("read_nokey");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    let mut buf = [0u8; 257];
    let err = db.read("nonexistent", &mut buf);
    assert!(matches!(err, Err(MdbError::KeyNotFound)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_key_size_too_large() {
    let name = unique_db_name("keysize");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 4,
        data_size_max: 256,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    // Key "hello" is 5 bytes, max is 4
    let err = db.write("hello", "world");
    assert!(matches!(err, Err(MdbError::KeySizeTooLarge)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_value_size_too_large() {
    let name = unique_db_name("valsize");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 64,
        data_size_max: 4,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    // Value "hello" is 5 bytes, max is 4
    let err = db.write("k", "hello");
    assert!(matches!(err, Err(MdbError::ValueSizeTooLarge)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_buffer_too_small() {
    let name = unique_db_name("bufsiz");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key", "longvalue").unwrap();
    // C checks bufsiz < valsize + 1, so buf of len 9 is too small for "longvalue" (9 chars + null)
    let mut buf = [0u8; 9];
    let err = db.read("key", &mut buf);
    assert!(matches!(err, Err(MdbError::BufferSizeTooSmall)));
    // buf of len 10 should work
    let mut buf = [0u8; 10];
    let n = db.read("key", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"longvalue");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_index_and_data_size_after_create() {
    // After create: index = 4 + 128*4 = 516, data = 0
    let name = unique_db_name("sizes_init");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    assert_eq!(db.index_size().unwrap(), 4 + 128 * 4);
    assert_eq!(db.data_size().unwrap(), 0);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_index_and_data_size_after_write() {
    // After one write: index grows by index_record_size (64+8+4=76)
    // data grows by value length
    let name = unique_db_name("sizes_write");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("misakawa", "mikoto").unwrap();
    // index: 516 + 76 = 592
    assert_eq!(db.index_size().unwrap(), 516 + 76);
    // data: "mikoto" = 6 bytes
    assert_eq!(db.data_size().unwrap(), 6);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_get_options() {
    let name = unique_db_name("getopts");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 32,
        data_size_max: 512,
        hash_buckets: 64,
        items_max: 1000,
    };
    let db = Mdb::create(&name, opts).unwrap();
    let ro = db.get_options();
    assert_eq!(ro.db_name, name);
    assert_eq!(ro.key_size_max, 32);
    assert_eq!(ro.data_size_max, 512);
    assert_eq!(ro.hash_buckets, 64);
    assert_eq!(ro.items_max, 1000);
    drop(db);
    cleanup(&name);
}

#[test]
fn test_write_delete_write_same_key() {
    // Write, delete, then write same key again
    let name = unique_db_name("wdw");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();

    db.write("key", "val1").unwrap();
    db.delete("key").unwrap();
    db.write("key", "val2").unwrap();

    let mut buf = [0u8; 257];
    let n = db.read("key", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"val2");

    drop(db);
    cleanup(&name);
}

#[test]
fn test_many_keys_same_bucket() {
    // Use hash_buckets=1 to force all keys into same bucket (chain collisions)
    let name = unique_db_name("collision");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 64,
        data_size_max: 256,
        hash_buckets: 1,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();

    for i in 0..10 {
        let key = format!("key{}", i);
        let val = format!("val{}", i);
        db.write(&key, &val).unwrap();
    }

    let mut buf = [0u8; 257];
    for i in 0..10 {
        let key = format!("key{}", i);
        let val = format!("val{}", i);
        let n = db.read(&key, &mut buf).unwrap();
        assert_eq!(&buf[..n], val.as_bytes());
    }

    // Delete middle key and verify others still work
    db.delete("key5").unwrap();
    assert!(matches!(db.read("key5", &mut buf), Err(MdbError::KeyNotFound)));
    let n = db.read("key4", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"val4");
    let n = db.read("key6", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"val6");

    drop(db);
    cleanup(&name);
}

#[test]
fn test_key_at_max_size() {
    // Key exactly at key_size_max should succeed
    let name = unique_db_name("key_exact");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    // 8-byte key is exactly at limit
    db.write("12345678", "val").unwrap();
    let mut buf = [0u8; 257];
    let n = db.read("12345678", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"val");
    // 9-byte key exceeds limit
    let err = db.write("123456789", "val");
    assert!(matches!(err, Err(MdbError::KeySizeTooLarge)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_value_at_max_size() {
    // Value exactly at data_size_max should succeed
    let name = unique_db_name("val_exact");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 64,
        data_size_max: 10,
        hash_buckets: 16,
        items_max: 100,
    };
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("k", "1234567890").unwrap();
    let mut buf = [0u8; 257];
    let n = db.read("k", &mut buf).unwrap();
    assert_eq!(&buf[..n], b"1234567890");
    // 11-byte value exceeds limit
    let err = db.write("k2", "12345678901");
    assert!(matches!(err, Err(MdbError::ValueSizeTooLarge)));
    drop(db);
    cleanup(&name);
}

#[test]
fn test_empty_value() {
    let name = unique_db_name("empty_val");
    let opts = default_options(&name);
    let mut db = Mdb::create(&name, opts).unwrap();
    db.write("key", "").unwrap();
    let mut buf = [0u8; 257];
    let n = db.read("key", &mut buf).unwrap();
    assert_eq!(n, 0);
    assert_eq!(&buf[..n], b"");
    drop(db);
    cleanup(&name);
}

#[test]
fn test_load_many_entries() {
    // Similar to C load_test2: write 100 entries, read them all back
    let name = unique_db_name("load");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 128,
        items_max: 166716,
    };
    let mut db = Mdb::create(&name, opts).unwrap();

    let preset = [
        "misakawa", "kamijou", "accelerator", "index", "lyzh", "chuigda",
        "de_nuke", "de_mirage", "de_cache",
    ];

    let mut entries = Vec::new();
    for i in 0..100u32 {
        let key = format!("{:03}", i);
        let val = preset[(i as usize) % preset.len()];
        db.write(&key, val).unwrap();
        entries.push((key, val.to_string()));
    }

    let mut buf = [0u8; 257];
    for (key, val) in &entries {
        let n = db.read(key, &mut buf).unwrap();
        assert_eq!(&buf[..n], val.as_bytes(), "mismatch for key {}", key);
    }

    drop(db);
    cleanup(&name);
}

#[test]
fn test_delete_and_reinsert() {
    // Write several, delete some, write new ones, verify all
    let name = unique_db_name("del_reins");
    let opts = MdbOptions {
        db_name: name.clone(),
        key_size_max: 8,
        data_size_max: 256,
        hash_buckets: 128,
        items_max: 166716,
    };
    let mut db = Mdb::create(&name, opts).unwrap();

    for i in 0..20 {
        db.write(&format!("{:03}", i), &format!("v{}", i)).unwrap();
    }

    // Delete even entries
    for i in (0..20).step_by(2) {
        db.delete(&format!("{:03}", i)).unwrap();
    }

    // Write new entries in the freed slots
    for i in 20..30 {
        db.write(&format!("{:03}", i), &format!("v{}", i)).unwrap();
    }

    let mut buf = [0u8; 257];
    // Odd originals should still be there
    for i in (1..20).step_by(2) {
        let key = format!("{:03}", i);
        let n = db.read(&key, &mut buf).unwrap();
        assert_eq!(&buf[..n], format!("v{}", i).as_bytes());
    }
    // Even originals should be gone
    for i in (0..20).step_by(2) {
        let key = format!("{:03}", i);
        assert!(matches!(db.read(&key, &mut buf), Err(MdbError::KeyNotFound)));
    }
    // New entries should be there
    for i in 20..30 {
        let key = format!("{:03}", i);
        let n = db.read(&key, &mut buf).unwrap();
        assert_eq!(&buf[..n], format!("v{}", i).as_bytes());
    }

    drop(db);
    cleanup(&name);
}

#[test]
fn test_open_nonexistent() {
    let err = Mdb::open("/tmp/mdb_test_nonexistent_db_xyz");
    assert!(err.is_err());
}

fn main() {}
