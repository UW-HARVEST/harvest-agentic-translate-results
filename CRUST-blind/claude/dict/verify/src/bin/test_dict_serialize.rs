use dict::dict::{
    dict_deserialize, dict_get, dict_has, dict_len, dict_new, dict_serialize, DictAlloc,
    DictArgs, DictKeyAttr, DictType, DictValAttr,
};

fn write_i32(slot: &mut [u8], v: i32) {
    slot[..4].copy_from_slice(&v.to_le_bytes());
}

fn read_i32(slot: &[u8]) -> i32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&slot[..4]);
    i32::from_le_bytes(buf)
}

fn write_f64(slot: &mut [u8], v: f64) {
    slot.copy_from_slice(&v.to_le_bytes());
}

fn read_f64(slot: &[u8]) -> f64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&slot[..8]);
    f64::from_le_bytes(buf)
}

fn args_for(key: DictType, key_size: usize, val_size: usize) -> DictArgs {
    DictArgs {
        key: DictKeyAttr {
            type_: key,
            size: key_size,
            copy: None,
            free: None,
            hash: None,
            cmpr: None,
        },
        val: DictValAttr {
            size: val_size,
            free: None,
        },
        alloc: DictAlloc {
            malloc: None,
            free: None,
        },
    }
}

#[test]
fn test_serialize_empty_dict() {
    // Empty I32 -> i32 dict produces 12-byte header (key_size=4, val_size=8, count=0).
    let d = dict_new(DictType::I32, 4, 4);
    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    assert_eq!(bytes, 12);
    assert_eq!(data.len(), 12);

    let key_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let val_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    assert_eq!(key_size, 4);
    assert_eq!(val_size, 8);
    assert_eq!(count, 0);
}

#[test]
fn test_serialize_single_i32_double() {
    // Verified against C: I32 keys (4 byte) + double values (8 byte). Single
    // entry (key=7, val=7.5) yields 24 bytes total.
    let mut d = dict_new(DictType::I32, 4, 8);
    write_f64(dict_get(&mut d, &7i32.to_le_bytes()).unwrap(), 7.5);
    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    assert_eq!(bytes, 24);

    let key_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let val_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    assert_eq!(key_size, 4);
    assert_eq!(val_size, 8);
    assert_eq!(count, 1);

    // The element record (Rust serializes key+val with key.size = 4, val.size = 8 = 12 bytes per entry)
    assert_eq!(data.len(), 12 + 12);
}

#[test]
fn test_serialize_i32_i32_two_entries_exact_layout() {
    // Compare against the C implementation's 36-byte output.
    let mut d = dict_new(DictType::I32, 4, 4);
    write_i32(dict_get(&mut d, &1i32.to_le_bytes()).unwrap(), 100);
    write_i32(dict_get(&mut d, &2i32.to_le_bytes()).unwrap(), 200);

    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    // header (12) + 2 records of 12 bytes each (4-byte key + 8-byte val) = 36
    assert_eq!(bytes, 36);

    let count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    assert_eq!(count, 2);
}

#[test]
fn test_serialize_str_layout() {
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"ab").unwrap(), 10);
    write_i32(dict_get(&mut d, b"cd").unwrap(), 20);

    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    // header (12) + 2 records (4 + 8 = 12 bytes each) + str area (4 bytes total) = 40
    assert_eq!(bytes, 40);
    let count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    assert_eq!(count, 2);
    let key_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    assert_eq!(key_size, std::mem::size_of::<usize>() as u32);

    // string area at offset 12 + 2*12 = 36 to 40 must contain the chars 'a','b','c','d' in some order.
    let str_area = &data[36..40];
    let s = std::str::from_utf8(str_area).unwrap();
    assert!(s.contains('a') && s.contains('b') && s.contains('c') && s.contains('d'));
}

#[test]
fn test_roundtrip_i32_dict() {
    // Build, serialize, deserialize, verify all the entries.
    let mut d = dict_new(DictType::I32, 4, 4);
    write_i32(dict_get(&mut d, &5i32.to_le_bytes()).unwrap(), 50);
    write_i32(dict_get(&mut d, &10i32.to_le_bytes()).unwrap(), 100);
    write_i32(dict_get(&mut d, &15i32.to_le_bytes()).unwrap(), 150);

    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();

    let args = args_for(DictType::I32, 4, 4);
    let mut d2 = dict_deserialize(args, &data);
    assert_eq!(dict_len(&d2), 3);
    assert_eq!(read_i32(dict_get(&mut d2, &5i32.to_le_bytes()).unwrap()), 50);
    assert_eq!(read_i32(dict_get(&mut d2, &10i32.to_le_bytes()).unwrap()), 100);
    assert_eq!(read_i32(dict_get(&mut d2, &15i32.to_le_bytes()).unwrap()), 150);
    assert!(dict_has(&d2, &5i32.to_le_bytes()));
    assert!(!dict_has(&d2, &99i32.to_le_bytes()));
}

#[test]
fn test_roundtrip_str_dict_test5_pattern() {
    // Mirrors the data laid out in tests/test5.c.
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"1").unwrap(), 1);
    write_i32(dict_get(&mut d, b"2").unwrap(), 2);
    write_i32(dict_get(&mut d, b"0").unwrap(), 0);
    write_i32(dict_get(&mut d, b"-1").unwrap(), -1);
    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    let args = args_for(DictType::Str, 0, 4);
    let mut d2 = dict_deserialize(args, &data);
    assert_eq!(dict_len(&d2), 4);
    assert_eq!(read_i32(dict_get(&mut d2, b"1").unwrap()), 1);
    assert_eq!(read_i32(dict_get(&mut d2, b"2").unwrap()), 2);
    assert_eq!(read_i32(dict_get(&mut d2, b"0").unwrap()), 0);
    assert_eq!(read_i32(dict_get(&mut d2, b"-1").unwrap()), -1);
}

#[test]
fn test_roundtrip_500_entries_triggers_reshape() {
    // Mirrors tests/test3.c: insert 500 entries, serialize/deserialize.
    let mut d = dict_new(DictType::I32, 4, 8);
    for i in 0i32..500 {
        write_f64(dict_get(&mut d, &i.to_le_bytes()).unwrap(), i as f64);
    }
    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    let args = args_for(DictType::I32, 4, 8);
    let mut d2 = dict_deserialize(args, &data);
    assert_eq!(dict_len(&d2), 500);
    for i in 0i32..500 {
        let v = dict_get(&mut d2, &i.to_le_bytes()).unwrap();
        assert_eq!(read_f64(v), i as f64);
    }
}

#[test]
fn test_deserialize_with_wrong_key_size_returns_empty_dict() {
    // The C code prints an error and returns NULL; the Rust translation
    // returns an empty dict.
    let mut d = dict_new(DictType::I32, 4, 4);
    write_i32(dict_get(&mut d, &1i32.to_le_bytes()).unwrap(), 10);
    let mut bytes = 0usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();

    // Try to deserialize as if I64 keys, which would be a key_size mismatch.
    let args = args_for(DictType::I64, 8, 4);
    let d_bad = dict_deserialize(args, &data);
    assert_eq!(dict_len(&d_bad), 0);
}

#[test]
fn test_serialize_writes_size_into_bytes_param() {
    let d = dict_new(DictType::I32, 4, 4);
    let mut bytes = 12345usize;
    let data = dict_serialize(&d, &mut bytes).unwrap();
    assert_eq!(bytes, data.len());
}

fn main() {}
