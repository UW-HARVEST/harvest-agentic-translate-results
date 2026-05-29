use dict::dict::{
    dict_get, dict_has, dict_len, dict_new, DictType,
};

fn write_i32(slot: &mut [u8], v: i32) {
    slot[..4].copy_from_slice(&v.to_le_bytes());
}

fn read_i32(slot: &[u8]) -> i32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&slot[..4]);
    i32::from_le_bytes(buf)
}

#[test]
fn test_u32_keys() {
    let mut d = dict_new(DictType::U32, 4, 4);
    write_i32(dict_get(&mut d, &(42u32).to_le_bytes()).unwrap(), 99);
    assert_eq!(read_i32(dict_get(&mut d, &(42u32).to_le_bytes()).unwrap()), 99);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_i64_keys() {
    let mut d = dict_new(DictType::I64, 8, 4);
    write_i32(dict_get(&mut d, &(9999i64).to_le_bytes()).unwrap(), 12345);
    assert_eq!(read_i32(dict_get(&mut d, &(9999i64).to_le_bytes()).unwrap()), 12345);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_u64_keys() {
    let mut d = dict_new(DictType::U64, 8, 8);
    let key1: u64 = 42;
    let key0: u64 = 0;
    let key_one: u64 = 1;
    let mut tmp = [0u8; 8];

    tmp[..8].copy_from_slice(&99u64.to_le_bytes());
    dict_get(&mut d, &key1.to_le_bytes()).unwrap().copy_from_slice(&tmp);
    tmp[..8].copy_from_slice(&0u64.to_le_bytes());
    dict_get(&mut d, &key0.to_le_bytes()).unwrap().copy_from_slice(&tmp);
    tmp[..8].copy_from_slice(&1u64.to_le_bytes());
    dict_get(&mut d, &key_one.to_le_bytes()).unwrap().copy_from_slice(&tmp);

    assert_eq!(dict_len(&d), 3);
    let v = dict_get(&mut d, &key1.to_le_bytes()).unwrap();
    assert_eq!(u64::from_le_bytes(v[..8].try_into().unwrap()), 99);
    let v = dict_get(&mut d, &key0.to_le_bytes()).unwrap();
    assert_eq!(u64::from_le_bytes(v[..8].try_into().unwrap()), 0);
    let v = dict_get(&mut d, &key_one.to_le_bytes()).unwrap();
    assert_eq!(u64::from_le_bytes(v[..8].try_into().unwrap()), 1);
}

#[test]
fn test_f64_keys() {
    let mut d = dict_new(DictType::F64, 8, 4);
    write_i32(dict_get(&mut d, &(1.5f64).to_le_bytes()).unwrap(), 100);
    write_i32(dict_get(&mut d, &(2.5f64).to_le_bytes()).unwrap(), 200);

    assert_eq!(dict_len(&d), 2);
    assert_eq!(read_i32(dict_get(&mut d, &(1.5f64).to_le_bytes()).unwrap()), 100);
    assert_eq!(read_i32(dict_get(&mut d, &(2.5f64).to_le_bytes()).unwrap()), 200);
}

#[test]
fn test_f32_keys() {
    let mut d = dict_new(DictType::F32, 4, 4);
    write_i32(dict_get(&mut d, &(3.14f32).to_le_bytes()).unwrap(), 314);
    assert_eq!(dict_len(&d), 1);
    assert_eq!(read_i32(dict_get(&mut d, &(3.14f32).to_le_bytes()).unwrap()), 314);
}

#[test]
fn test_char_keys() {
    let mut d = dict_new(DictType::Char, 1, 4);
    write_i32(dict_get(&mut d, &[b'a']).unwrap(), 65);
    write_i32(dict_get(&mut d, &[b'A']).unwrap(), 1);

    assert_eq!(dict_len(&d), 2);
    assert!(dict_has(&d, &[b'a']));
    assert!(dict_has(&d, &[b'A']));
    assert!(!dict_has(&d, &[b'z']));
    assert_eq!(read_i32(dict_get(&mut d, &[b'a']).unwrap()), 65);
    assert_eq!(read_i32(dict_get(&mut d, &[b'A']).unwrap()), 1);
}

#[test]
fn test_wchar_keys() {
    let mut d = dict_new(DictType::WChar, 4, 4);
    let k_a = (b'a' as u32).to_le_bytes();
    let k_omega = (0x03A9u32).to_le_bytes();
    write_i32(dict_get(&mut d, &k_a).unwrap(), 1);
    write_i32(dict_get(&mut d, &k_omega).unwrap(), 2);
    assert_eq!(dict_len(&d), 2);
    assert_eq!(read_i32(dict_get(&mut d, &k_a).unwrap()), 1);
    assert_eq!(read_i32(dict_get(&mut d, &k_omega).unwrap()), 2);
}

#[test]
fn test_ptr_keys() {
    let mut d = dict_new(DictType::Ptr, 8, 4);
    let p1: usize = 0xCAFE;
    let p2: usize = 0xBEEF;
    write_i32(dict_get(&mut d, &p1.to_le_bytes()).unwrap(), 1);
    write_i32(dict_get(&mut d, &p2.to_le_bytes()).unwrap(), 2);
    assert_eq!(dict_len(&d), 2);
    assert_eq!(read_i32(dict_get(&mut d, &p1.to_le_bytes()).unwrap()), 1);
    assert_eq!(read_i32(dict_get(&mut d, &p2.to_le_bytes()).unwrap()), 2);
}

#[test]
fn test_struct_keys() {
    // Struct keys: caller passes raw bytes of the struct, padded to ptr-size.
    let mut d = dict_new(DictType::Struct, 5, 4);
    assert_eq!(d.key.size, 8);

    let key1 = [1u8, 0, 0, 0, 0, 0, 0, 0];
    let key2 = [2u8, 0, 0, 0, 0, 0, 0, 0];
    write_i32(dict_get(&mut d, &key1).unwrap(), 100);
    write_i32(dict_get(&mut d, &key2).unwrap(), 200);

    assert_eq!(dict_len(&d), 2);
    assert!(dict_has(&d, &key1));
    assert!(dict_has(&d, &key2));
    assert_eq!(read_i32(dict_get(&mut d, &key1).unwrap()), 100);
    assert_eq!(read_i32(dict_get(&mut d, &key2).unwrap()), 200);
}

#[test]
fn test_zero_value_size() {
    // val.size = 0 still inserts a key (matches C dict_get behavior).
    let mut d = dict_new(DictType::I32, 4, 0);
    let v = dict_get(&mut d, &5i32.to_le_bytes()).unwrap();
    assert_eq!(v.len(), 0);
    assert_eq!(dict_len(&d), 1);
    assert!(dict_has(&d, &5i32.to_le_bytes()));
}

fn main() {}
