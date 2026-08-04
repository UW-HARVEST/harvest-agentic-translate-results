use dict::dict::{
    dict_create, dict_create_args, dict_free_key, dict_free_node, dict_free_val, dict_get,
    dict_get_hash, dict_key, dict_key_equals, dict_len, dict_new, dict_remove, dict_reshape,
    DictAlloc, DictArgs, DictBucket, DictElem, DictKeyAttr, DictType, DictValAttr, DEFAULT_MOD,
    DEFAULT_STEP,
};

fn write_i32(slot: &mut [u8], v: i32) {
    slot[..4].copy_from_slice(&v.to_le_bytes());
}

fn read_i32(slot: &[u8]) -> i32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&slot[..4]);
    i32::from_le_bytes(buf)
}

fn args_for(key_type: DictType, key_size: usize, val_size: usize) -> DictArgs {
    DictArgs {
        key: DictKeyAttr {
            type_: key_type,
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
fn test_dict_create_with_args() {
    let mut d = dict_create(args_for(DictType::I32, 4, 4));
    assert_eq!(d.key.size, 4);
    assert_eq!(d.val.size, 8); // padded to ptr size
    assert_eq!(d.mod_, DEFAULT_MOD);
    assert_eq!(dict_len(&d), 0);
    write_i32(dict_get(&mut d, &10i32.to_le_bytes()).unwrap(), 200);
    assert_eq!(read_i32(dict_get(&mut d, &10i32.to_le_bytes()).unwrap()), 200);
}

#[test]
fn test_dict_create_args_alias_returns_same() {
    let mut d = dict_create_args(args_for(DictType::I32, 4, 4));
    write_i32(dict_get(&mut d, &1i32.to_le_bytes()).unwrap(), 11);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_dict_create_str_key() {
    let mut d = dict_create(args_for(DictType::Str, 0, 4));
    assert_eq!(d.key.size, std::mem::size_of::<usize>());
    write_i32(dict_get(&mut d, b"hello").unwrap(), 50);
    assert_eq!(read_i32(dict_get(&mut d, b"hello").unwrap()), 50);
}

#[test]
fn test_dict_key_returns_none_for_empty() {
    let d = dict_new(DictType::I32, 4, 4);
    let mut s = 99usize;
    let result = dict_key(&d, &mut s);
    assert!(result.is_none());
    assert_eq!(s, 0);
}

#[test]
fn test_dict_key_returns_concatenated_bytes() {
    let mut d = dict_new(DictType::I32, 4, 4);
    write_i32(dict_get(&mut d, &10i32.to_le_bytes()).unwrap(), 0);
    write_i32(dict_get(&mut d, &20i32.to_le_bytes()).unwrap(), 0);
    write_i32(dict_get(&mut d, &30i32.to_le_bytes()).unwrap(), 0);

    let mut s = 0usize;
    let buf = dict_key(&d, &mut s).unwrap();
    assert_eq!(s, 3);
    // 3 keys * 4 bytes = 12 bytes
    assert_eq!(buf.len(), 12);

    // collect the keys; they may be in any order due to hash bucketing.
    let mut keys: Vec<i32> = Vec::with_capacity(3);
    for chunk in buf.chunks(4) {
        let mut t = [0u8; 4];
        t.copy_from_slice(chunk);
        keys.push(i32::from_le_bytes(t));
    }
    keys.sort();
    assert_eq!(keys, vec![10, 20, 30]);
}

#[test]
fn test_dict_key_equals_for_i32_dict() {
    let d = dict_new(DictType::I32, 4, 4);
    let a = 5i32.to_le_bytes();
    let b = 5i32.to_le_bytes();
    let c = 6i32.to_le_bytes();
    assert!(dict_key_equals(&d, &a, &b));
    assert!(!dict_key_equals(&d, &a, &c));
}

#[test]
fn test_dict_get_hash_matches_dict_internal() {
    let d = dict_new(DictType::I32, 4, 4);
    // For an i32 dict, the hash is the value sign-extended to u64.
    assert_eq!(dict_get_hash(&d, &7i32.to_le_bytes()), 7u64);
    assert_eq!(dict_get_hash(&d, &(-1i32).to_le_bytes()), u64::MAX);
}

#[test]
fn test_dict_reshape_increases_buckets() {
    let mut d = dict_new(DictType::I32, 4, 4);
    let original_size = d.mod_;
    let success = dict_reshape(&mut d, 1);
    assert!(success);
    // After reshape with step=1: new size = old_size * max(1,1) * DEFAULT_STEP
    assert_eq!(d.mod_, original_size * DEFAULT_STEP);
    assert_eq!(d.buckets.len(), original_size * DEFAULT_STEP);
}

#[test]
fn test_dict_reshape_with_step_2() {
    let mut d = dict_new(DictType::I32, 4, 4);
    let original_size = d.mod_;
    let success = dict_reshape(&mut d, 2);
    assert!(success);
    // step=2: new size = old_size * 2 * DEFAULT_STEP = old_size * 4
    assert_eq!(d.mod_, original_size * 2 * DEFAULT_STEP);
}

#[test]
fn test_dict_reshape_preserves_entries() {
    let mut d = dict_new(DictType::I32, 4, 4);
    for i in 0i32..20 {
        write_i32(dict_get(&mut d, &i.to_le_bytes()).unwrap(), i * 10);
    }
    assert_eq!(dict_len(&d), 20);

    let success = dict_reshape(&mut d, 1);
    assert!(success);

    assert_eq!(dict_len(&d), 20);
    for i in 0i32..20 {
        let v = dict_get(&mut d, &i.to_le_bytes()).unwrap();
        assert_eq!(read_i32(v), i * 10, "lost entry {} after reshape", i);
    }
}

#[test]
fn test_dict_free_node_is_noop() {
    let d = dict_new(DictType::I32, 4, 4);
    let mut elem = DictElem {
        code: 42,
        key: vec![0, 0, 0, 0],
        val: vec![0; 4],
    };
    dict_free_node(&d, &mut elem);
    // Sanity: still readable after the no-op.
    assert_eq!(elem.code, 42);
}

#[test]
fn test_dict_free_key_no_copy_set_is_noop() {
    let d = dict_new(DictType::I32, 4, 4);
    let mut buf = vec![1u8, 2, 3, 4];
    dict_free_key(&d, &mut buf);
    // Buf shouldn't be mutated when key.copy is None.
    assert_eq!(buf, vec![1, 2, 3, 4]);
}

#[test]
fn test_dict_free_key_calls_free_when_copy_set() {
    fn marker_free(buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = 0xFE;
        }
    }
    fn marker_copy(_dest: &mut [u8], _src: &[u8]) {}

    let mut args = args_for(DictType::Struct, 4, 4);
    args.key.copy = Some(marker_copy);
    args.key.free = Some(marker_free);
    let d = dict_create(args);

    let mut buf = vec![0u8; 4];
    dict_free_key(&d, &mut buf);
    assert_eq!(buf, vec![0xFE; 4]);
}

#[test]
fn test_dict_free_val_calls_free_when_set() {
    fn marker_free(buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = 0xAA;
        }
    }

    let mut args = args_for(DictType::I32, 4, 4);
    args.val.free = Some(marker_free);
    let d = dict_create(args);

    let mut buf = vec![0u8; 4];
    dict_free_val(&d, &mut buf);
    assert_eq!(buf, vec![0xAA; 4]);
}

#[test]
fn test_dict_free_val_noop_when_unset() {
    let d = dict_new(DictType::I32, 4, 4);
    let mut buf = vec![1u8, 2, 3, 4];
    dict_free_val(&d, &mut buf);
    assert_eq!(buf, vec![1, 2, 3, 4]);
}

#[test]
fn test_custom_hash_function_used() {
    fn custom_hash(_data: &[u8]) -> u64 {
        42
    }
    let mut args = args_for(DictType::I32, 4, 4);
    args.key.hash = Some(custom_hash);
    let d = dict_create(args);
    assert_eq!(dict_get_hash(&d, &123i32.to_le_bytes()), 42);
    assert_eq!(dict_get_hash(&d, &456i32.to_le_bytes()), 42);
}

#[test]
fn test_custom_compare_function_used() {
    fn always_equal(_a: &[u8], _b: &[u8]) -> i32 {
        0
    }
    let mut args = args_for(DictType::I32, 4, 4);
    args.key.cmpr = Some(always_equal);
    let d = dict_create(args);
    assert!(dict_key_equals(&d, &1i32.to_le_bytes(), &2i32.to_le_bytes()));
}

#[test]
fn test_remove_then_reinsert_same_key_works() {
    let mut d = dict_new(DictType::I32, 4, 4);
    write_i32(dict_get(&mut d, &7i32.to_le_bytes()).unwrap(), 70);
    assert!(dict_remove(&mut d, &7i32.to_le_bytes()));
    write_i32(dict_get(&mut d, &7i32.to_le_bytes()).unwrap(), 71);
    assert_eq!(read_i32(dict_get(&mut d, &7i32.to_le_bytes()).unwrap()), 71);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_reshape_with_step_zero_uses_min_one() {
    // When step = 0, max(1, 0) = 1, so new size = old_size * 1 * DEFAULT_STEP.
    let mut d = dict_new(DictType::I32, 4, 4);
    let original_size = d.mod_;
    let success = dict_reshape(&mut d, 0);
    assert!(success);
    assert_eq!(d.mod_, original_size * DEFAULT_STEP);
}

#[test]
fn test_dict_bucket_struct_initially_empty() {
    let d = dict_new(DictType::I32, 4, 4);
    for bucket in &d.buckets {
        assert!(bucket.elements.is_empty());
    }
}

#[test]
fn test_dict_dictbucket_constructable() {
    // Ensure DictBucket and DictElem exposed types are usable.
    let _b = DictBucket {
        elements: Vec::<DictElem>::new(),
    };
}

fn main() {}
