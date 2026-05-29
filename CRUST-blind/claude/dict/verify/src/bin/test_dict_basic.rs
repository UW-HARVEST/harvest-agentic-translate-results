use dict::dict::{
    dict_destroy, dict_get, dict_has, dict_len, dict_new, dict_remove, DictType, DEFAULT_MOD,
    DEFAULT_STEP, HASH_BASE, HASH_MOD,
};

/// Helper to write a 4-byte i32 into the first 4 bytes of an 8-byte (padded) value slot.
fn write_i32(slot: &mut [u8], v: i32) {
    let bytes = v.to_le_bytes();
    slot[..4].copy_from_slice(&bytes);
    // higher bytes remain zero (initialised by dict_get).
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

#[test]
fn test_dict_new_creates_empty_i32_dict() {
    let d = dict_new(DictType::I32, 4, 8);
    assert_eq!(d.key.size, 4);
    assert_eq!(d.val.size, 8);
    assert_eq!(d.mod_, DEFAULT_MOD);
    assert_eq!(d.buckets.len(), DEFAULT_MOD);
    assert_eq!(d.count, 0);
    assert_eq!(dict_len(&d), 0);
}

#[test]
fn test_dict_new_with_str_key() {
    let d = dict_new(DictType::Str, 0, 4);
    // ptr-sized key, val rounded up to 8.
    assert_eq!(d.key.size, std::mem::size_of::<usize>());
    assert_eq!(d.val.size, 8);
    assert_eq!(dict_len(&d), 0);
}

#[test]
fn test_dict_new_struct_alignment() {
    // a 5-byte struct should be padded up to 8 (align_up to ptr size).
    let d = dict_new(DictType::Struct, 5, 1);
    assert_eq!(d.key.size, 8);
    assert_eq!(d.val.size, 8);
}

#[test]
fn test_dict_new_char_key() {
    let d = dict_new(DictType::Char, 1, 4);
    assert_eq!(d.key.size, 1);
    assert_eq!(d.val.size, 8); // 4 padded to 8
}

#[test]
fn test_dict_new_wchar_key() {
    let d = dict_new(DictType::WChar, 4, 4);
    assert_eq!(d.key.size, 4);
}

#[test]
fn test_dict_new_u32_key() {
    let d = dict_new(DictType::U32, 4, 4);
    assert_eq!(d.key.size, 4);
}

#[test]
fn test_dict_new_f32_key() {
    let d = dict_new(DictType::F32, 4, 4);
    assert_eq!(d.key.size, 4);
}

#[test]
fn test_dict_new_i64_u64_f64_key() {
    let d = dict_new(DictType::I64, 8, 4);
    assert_eq!(d.key.size, 8);
    let d = dict_new(DictType::U64, 8, 4);
    assert_eq!(d.key.size, 8);
    let d = dict_new(DictType::F64, 8, 4);
    assert_eq!(d.key.size, 8);
}

#[test]
fn test_dict_new_ptr_key() {
    let d = dict_new(DictType::Ptr, 8, 4);
    assert_eq!(d.key.size, std::mem::size_of::<usize>());
}

#[test]
fn test_default_constants() {
    assert_eq!(HASH_BASE, 256);
    assert_eq!(HASH_MOD, 1_000_000_007);
    assert_eq!(DEFAULT_STEP, 2);
    assert_eq!(DEFAULT_MOD, 8);
}

#[test]
fn test_dict_get_creates_then_retrieves_i32_to_double() {
    // Mirrors C tests/test0.c: keys 0..30 => values (double)i.
    let mut d = dict_new(DictType::I32, 4, 8);
    for i in 0i32..30 {
        let v = dict_get(&mut d, &i.to_le_bytes()).unwrap();
        write_f64(v, i as f64);
    }
    assert_eq!(dict_len(&d), 30);
    for i in 0i32..30 {
        let v = dict_get(&mut d, &i.to_le_bytes()).unwrap();
        assert_eq!(read_f64(v), i as f64, "mismatch at key {}", i);
    }
}

#[test]
fn test_dict_get_initializes_value_to_zero() {
    // val.size of 4 gets padded to 8.
    let mut d = dict_new(DictType::I32, 4, 4);
    let v = dict_get(&mut d, &99i32.to_le_bytes()).unwrap();
    assert_eq!(v, &[0u8; 8]);
}

#[test]
fn test_dict_has_yes_and_no() {
    let mut d = dict_new(DictType::I32, 4, 4);
    let v = dict_get(&mut d, &5i32.to_le_bytes()).unwrap();
    write_i32(v, 100);
    assert!(dict_has(&d, &5i32.to_le_bytes()));
    assert!(!dict_has(&d, &999i32.to_le_bytes()));
    assert!(!dict_has(&d, &(-7i32).to_le_bytes()));
}

#[test]
fn test_dict_remove_returns_true_for_present_and_false_for_missing() {
    let mut d = dict_new(DictType::I32, 4, 4);
    let v = dict_get(&mut d, &7i32.to_le_bytes()).unwrap();
    write_i32(v, 700);
    assert_eq!(dict_len(&d), 1);
    assert!(dict_has(&d, &7i32.to_le_bytes()));

    let removed = dict_remove(&mut d, &7i32.to_le_bytes());
    assert!(removed);
    assert_eq!(dict_len(&d), 0);
    assert!(!dict_has(&d, &7i32.to_le_bytes()));

    let removed_missing = dict_remove(&mut d, &999i32.to_le_bytes());
    assert!(!removed_missing);

    // re-add reuses logical entry slot
    let v = dict_get(&mut d, &7i32.to_le_bytes()).unwrap();
    write_i32(v, 701);
    let v = dict_get(&mut d, &7i32.to_le_bytes()).unwrap();
    assert_eq!(read_i32(v), 701);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_dict_get_idempotent_reuses_existing_entry() {
    let mut d = dict_new(DictType::I32, 4, 4);
    {
        let v = dict_get(&mut d, &10i32.to_le_bytes()).unwrap();
        write_i32(v, 111);
    }
    assert_eq!(dict_len(&d), 1);
    {
        let v = dict_get(&mut d, &10i32.to_le_bytes()).unwrap();
        assert_eq!(read_i32(v), 111);
    }
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_dict_destroy_clears_buckets_and_count() {
    let mut d = dict_new(DictType::I32, 4, 4);
    let v = dict_get(&mut d, &1i32.to_le_bytes()).unwrap();
    write_i32(v, 10);
    let v = dict_get(&mut d, &2i32.to_le_bytes()).unwrap();
    write_i32(v, 20);
    assert_eq!(dict_len(&d), 2);

    dict_destroy(&mut d);
    assert_eq!(dict_len(&d), 0);
    assert_eq!(d.count, 0);
    assert_eq!(d.buckets.len(), 0);
}

#[test]
fn test_dict_len_empty() {
    let d = dict_new(DictType::I32, 4, 4);
    assert_eq!(dict_len(&d), 0);
}

#[test]
fn test_dict_remove_on_empty_returns_false() {
    let mut d = dict_new(DictType::I32, 4, 4);
    assert!(!dict_remove(&mut d, &1i32.to_le_bytes()));
}

#[test]
fn test_dict_has_on_empty_returns_false() {
    let d = dict_new(DictType::I32, 4, 4);
    assert!(!dict_has(&d, &1i32.to_le_bytes()));
}

#[test]
fn test_dict_growth_to_100_entries() {
    let mut d = dict_new(DictType::I32, 4, 4);
    for i in 0i32..100 {
        let v = dict_get(&mut d, &i.to_le_bytes()).unwrap();
        write_i32(v, i * 10);
    }
    assert_eq!(dict_len(&d), 100);
    for i in 0i32..100 {
        let v = dict_get(&mut d, &i.to_le_bytes()).unwrap();
        assert_eq!(read_i32(v), i * 10, "mismatch at {}", i);
    }
    // After enough inserts, the table should have grown beyond DEFAULT_MOD buckets.
    assert!(d.mod_ > DEFAULT_MOD);
}

fn main() {}
