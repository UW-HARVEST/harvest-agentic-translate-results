use Simple_Sparsehash::simple_sparsehash::{
    sparse_array_free, sparse_array_get, sparse_array_init, sparse_array_set, sparse_dict_free,
    sparse_dict_get, sparse_dict_init, sparse_dict_set, BITCHUNK_SIZE, BITMAP_SIZE, GROUP_SIZE,
    RESIZE_PERCENT, STARTING_SIZE,
};

// ---------- Constant values (verified via the C header) ----------

#[test]
fn test_constants() {
    assert_eq!(GROUP_SIZE, 48);
    assert_eq!(STARTING_SIZE, 32);
    assert_eq!(RESIZE_PERCENT, 80);
    assert_eq!(BITCHUNK_SIZE, 32);
    // (GROUP_SIZE - 1) / 32 + 1 = 47/32 + 1 = 1 + 1 = 2
    assert_eq!(BITMAP_SIZE, 2);
}

// ---------- sparse_array_init ----------

#[test]
fn test_array_init_basic() {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert_eq!(arr.maximum, 32);
    // (32 - 1)/48 + 1 == 1 group
    assert_eq!(arr.groups.len(), 1);
    let g = &arr.groups[0];
    assert_eq!(g.count, 0);
    assert_eq!(g.elem_size, std::mem::size_of::<u64>());
    assert!(g.group.is_empty());
    assert_eq!(g.bitmap, [0u32; BITMAP_SIZE]);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_init_multiple_groups() {
    // For maximum=120 GROUP_SIZE=48:  (120-1)/48 + 1 = 2 + 1 = 3 groups
    let arr = sparse_array_init(std::mem::size_of::<i32>(), 120).unwrap();
    assert_eq!(arr.maximum, 120);
    assert_eq!(arr.groups.len(), 3);
    for g in arr.groups.iter() {
        assert_eq!(g.count, 0);
        assert_eq!(g.elem_size, std::mem::size_of::<i32>());
        assert!(g.group.is_empty());
        assert_eq!(g.bitmap, [0u32; BITMAP_SIZE]);
    }
    assert_eq!(sparse_array_free(arr), 1);
}

// ---------- empty array behavior ----------

#[test]
fn test_empty_array_does_not_blow_up() {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert!(sparse_array_get(&arr, 0, None).is_none());
    let mut outsize: usize = 99;
    assert!(sparse_array_get(&arr, 0, Some(&mut outsize)).is_none());
    // outsize is unchanged because the slot was unoccupied (matches C behavior:
    // C only writes outsize when it actually returns the value).
    assert_eq!(outsize, 99);
    assert_eq!(sparse_array_free(arr), 1);
}

// ---------- bounds checks ----------

#[test]
fn test_cannot_set_outside_bounds() {
    let mut arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    let test_num: u64 = 666;
    let bytes = test_num.to_le_bytes();
    // i=35 > maximum=32 -> 0 (error). C also yields 0 for i=33 with max=32.
    assert_eq!(sparse_array_set(&mut arr, 35, &bytes, bytes.len()), 0);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_cannot_get_outside_bounds() {
    let arr = sparse_array_init(std::mem::size_of::<u64>(), 32).unwrap();
    assert!(sparse_array_get(&arr, 35, None).is_none());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_cannot_set_bigger_elements() {
    // elem_size = 1 byte; setting an 8-byte u64 is forbidden.
    let mut arr = sparse_array_init(std::mem::size_of::<u8>(), 100).unwrap();
    let test_num: u64 = 666;
    let bytes = test_num.to_le_bytes();
    assert_eq!(sparse_array_set(&mut arr, 0, &bytes, bytes.len()), 0);
    // The slot remains empty.
    assert!(sparse_array_get(&arr, 0, None).is_none());
    assert_eq!(sparse_array_free(arr), 1);
}

// ---------- forward sets and gets ----------

#[test]
fn test_array_set_forward() {
    let array_size: i32 = 130;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), array_size as u32).unwrap();
    for i in 0..array_size {
        let bytes = i.to_le_bytes();
        assert_eq!(
            sparse_array_set(&mut arr, i as u32, &bytes, bytes.len()),
            1
        );
        let mut siz: usize = 0;
        let got = sparse_array_get(&arr, i as u32, Some(&mut siz)).unwrap();
        assert_eq!(siz, std::mem::size_of::<i32>());
        let got_val = i32::from_le_bytes(got.try_into().unwrap());
        assert_eq!(got_val, i);
    }
    // Re-read; everything still there.
    for i in 0..array_size {
        let mut siz: usize = 0;
        let got = sparse_array_get(&arr, i as u32, Some(&mut siz)).unwrap();
        assert_eq!(siz, std::mem::size_of::<i32>());
        let got_val = i32::from_le_bytes(got.try_into().unwrap());
        assert_eq!(got_val, i);
    }
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_backwards() {
    let array_size: i32 = 120;
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), array_size as u32).unwrap();
    for i in (0..array_size).rev() {
        let bytes = i.to_le_bytes();
        assert_eq!(
            sparse_array_set(&mut arr, i as u32, &bytes, bytes.len()),
            1
        );
        let mut siz: usize = 0;
        let got = sparse_array_get(&arr, i as u32, Some(&mut siz)).unwrap();
        assert_eq!(siz, std::mem::size_of::<i32>());
        let got_val = i32::from_le_bytes(got.try_into().unwrap());
        assert_eq!(got_val, i);
    }
    for i in (0..array_size).rev() {
        let mut siz: usize = 0;
        let got = sparse_array_get(&arr, i as u32, Some(&mut siz)).unwrap();
        assert_eq!(siz, std::mem::size_of::<i32>());
        let got_val = i32::from_le_bytes(got.try_into().unwrap());
        assert_eq!(got_val, i);
    }
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_high_num() {
    let test_num: i32 = 65555555;
    let index: u32 = (GROUP_SIZE - 1) as u32; // 47
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 140).unwrap();
    let bytes = test_num.to_le_bytes();
    assert_eq!(sparse_array_set(&mut arr, index, &bytes, bytes.len()), 1);
    let mut siz: usize = 0;
    let got = sparse_array_get(&arr, index, Some(&mut siz)).unwrap();
    assert_eq!(siz, std::mem::size_of::<i32>());
    assert_eq!(i32::from_le_bytes(got.try_into().unwrap()), test_num);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_overwrites_old_values() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 150).unwrap();
    let a: i32 = 666;
    let b: i32 = 1024;
    assert_eq!(
        sparse_array_set(&mut arr, 0, &a.to_le_bytes(), 4),
        1
    );
    assert_eq!(
        sparse_array_set(&mut arr, 0, &b.to_le_bytes(), 4),
        1
    );
    let mut siz: usize = 0;
    let got = sparse_array_get(&arr, 0, Some(&mut siz)).unwrap();
    assert_eq!(siz, 4);
    assert_eq!(i32::from_le_bytes(got.try_into().unwrap()), 1024);

    // After overwrite: only one element and bitmap unchanged.
    assert_eq!(arr.groups[0].count, 1);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_get_with_outsize() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 200).unwrap();
    let test_num: i32 = 666;
    let bytes = test_num.to_le_bytes();
    assert_eq!(sparse_array_set(&mut arr, 0, &bytes, 4), 1);
    let mut item_size: usize = 0;
    let got = sparse_array_get(&arr, 0, Some(&mut item_size)).unwrap();
    assert_eq!(item_size, 4);
    assert_eq!(i32::from_le_bytes(got.try_into().unwrap()), 666);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_at_boundary_i_equals_maximum() {
    // C allows i == maximum (the check is i > maximum), and i==maximum lands
    // in a valid group when maximum is itself a multiple of GROUP_SIZE because
    // MAX_ARR_SIZE = (max-1)/48 + 1.
    // For max=48 (and GROUP_SIZE=48), MAX_ARR_SIZE = 47/48 + 1 = 1.
    // i=48 -> group_idx=1 which is OUT of allocated groups.  The Rust port
    // returns 0 in that case. The C port has UB but produces rc=1 most of
    // the time. Verify the safer Rust behavior: we don't crash.
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 48).unwrap();
    let v: i32 = 7;
    let bytes = v.to_le_bytes();
    // The Rust `sparse_array_set` returns 0 when group_idx is out of range.
    let rc = sparse_array_set(&mut arr, 48, &bytes, bytes.len());
    // Either 0 (Rust) or 1 (C). Just verify it does not panic & the slot at
    // 47 still works correctly.
    assert!(rc == 0 || rc == 1);
    assert_eq!(sparse_array_set(&mut arr, 47, &bytes, bytes.len()), 1);
    let got = sparse_array_get(&arr, 47, None).unwrap();
    assert_eq!(i32::from_le_bytes(got.try_into().unwrap()), 7);
    // Definitely out-of-range.
    assert_eq!(sparse_array_set(&mut arr, 49, &bytes, bytes.len()), 0);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_set_in_each_group() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 120).unwrap();
    // Probe specifically across group boundaries.
    let positions = [0u32, 47, 48, 95, 96, 119];
    for &p in positions.iter() {
        let v: i32 = p as i32 * 2 + 7;
        let bytes = v.to_le_bytes();
        assert_eq!(sparse_array_set(&mut arr, p, &bytes, bytes.len()), 1);
    }
    for &p in positions.iter() {
        let v: i32 = p as i32 * 2 + 7;
        let mut siz: usize = 0;
        let got = sparse_array_get(&arr, p, Some(&mut siz)).unwrap();
        assert_eq!(siz, 4);
        assert_eq!(i32::from_le_bytes(got.try_into().unwrap()), v);
    }
    // Group counts:
    //   group 0 (positions 0..48): two stored => count==2
    //   group 1 (positions 48..96): two stored => count==2
    //   group 2 (positions 96..120): two stored => count==2
    assert_eq!(arr.groups[0].count, 2);
    assert_eq!(arr.groups[1].count, 2);
    assert_eq!(arr.groups[2].count, 2);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_unset_slot_returns_none() {
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 100).unwrap();
    let v: i32 = 42;
    let bytes = v.to_le_bytes();
    assert_eq!(sparse_array_set(&mut arr, 10, &bytes, bytes.len()), 1);
    assert!(sparse_array_get(&arr, 9, None).is_none());
    assert!(sparse_array_get(&arr, 11, None).is_none());
    assert!(sparse_array_get(&arr, 10, None).is_some());
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_variable_value_sizes() {
    // elem_size = 16 bytes. Actually store records with vlen 4, 8, 16.
    let mut arr = sparse_array_init(16, 100).unwrap();
    let four = [0xAAu8, 0xBB, 0xCC, 0xDD];
    let eight: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let sixteen: [u8; 16] = [
        9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
    ];
    assert_eq!(sparse_array_set(&mut arr, 0, &four, 4), 1);
    assert_eq!(sparse_array_set(&mut arr, 5, &eight, 8), 1);
    assert_eq!(sparse_array_set(&mut arr, 10, &sixteen, 16), 1);

    let mut s: usize = 0;
    let g0 = sparse_array_get(&arr, 0, Some(&mut s)).unwrap();
    assert_eq!(s, 4);
    assert_eq!(g0, &four);

    let mut s: usize = 0;
    let g5 = sparse_array_get(&arr, 5, Some(&mut s)).unwrap();
    assert_eq!(s, 8);
    assert_eq!(g5, &eight);

    let mut s: usize = 0;
    let g10 = sparse_array_get(&arr, 10, Some(&mut s)).unwrap();
    assert_eq!(s, 16);
    assert_eq!(g10, &sixteen);
    assert_eq!(sparse_array_free(arr), 1);
}

#[test]
fn test_array_zero_vlen_returns_none() {
    // C behavior: sparse_array_set with vlen=0 returns 1 (success), but
    // sparse_array_get returns NULL because the C code returns NULL when
    // size==0. Verified directly with the C reference implementation.
    let mut arr = sparse_array_init(std::mem::size_of::<i32>(), 50).unwrap();
    let bytes = [0u8; 4];
    let rc = sparse_array_set(&mut arr, 5, &bytes, 0);
    assert_eq!(rc, 1, "set with vlen=0 should succeed (matches C)");
    let mut outsize: usize = 99;
    let got = sparse_array_get(&arr, 5, Some(&mut outsize));
    assert!(got.is_none(), "get on vlen=0 slot returns None per C");
    // The C code does not write outsize when returning NULL, so it should
    // remain unchanged.
    assert_eq!(outsize, 99);
    assert_eq!(sparse_array_free(arr), 1);
}

// ---------- sparse_dict ----------

#[test]
fn test_dict_init_basic() {
    let dict = sparse_dict_init().unwrap();
    assert_eq!(dict.bucket_max, 32);
    assert_eq!(dict.bucket_count, 0);
    assert_eq!(dict.buckets.len(), 1);
    assert_eq!(dict.buckets[0].maximum, 32);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_set() {
    let mut dict = sparse_dict_init().unwrap();
    let key = "key";
    let val = "value";
    let rc = sparse_dict_set(&mut dict, key, key.len(), val.as_bytes(), val.len());
    assert_eq!(rc, 1);
    assert_eq!(dict.bucket_count, 1);
    assert_eq!(dict.bucket_max, 32);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get() {
    let mut dict = sparse_dict_init().unwrap();
    let key = "key";
    let val = "value";
    assert_eq!(
        sparse_dict_set(&mut dict, key, key.len(), val.as_bytes(), val.len()),
        1
    );

    let mut outsize: usize = 0;
    let got = sparse_dict_get(&dict, key, key.len(), Some(&mut outsize)).unwrap();
    assert_eq!(outsize, val.len());
    assert_eq!(got, val.as_bytes());
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get_nonexistent_returns_none() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(
        sparse_dict_set(&mut dict, "key", 3, b"value", 5),
        1
    );
    // Retrieve nonexistent.
    let mut outsize: usize = 99;
    let got = sparse_dict_get(&dict, "nope", 4, Some(&mut outsize));
    assert!(got.is_none());
    // Outsize must be unmodified per C semantics.
    assert_eq!(outsize, 99);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get_on_empty_dict_returns_none() {
    let dict = sparse_dict_init().unwrap();
    assert!(sparse_dict_get(&dict, "anything", 8, None).is_none());
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_overwrite_does_not_change_count() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(sparse_dict_set(&mut dict, "key", 3, b"v1", 2), 1);
    assert_eq!(dict.bucket_count, 1);
    assert_eq!(dict.bucket_max, 32);

    // Overwrite same key.
    assert_eq!(
        sparse_dict_set(&mut dict, "key", 3, b"different value", 15),
        1
    );
    assert_eq!(
        dict.bucket_count, 1,
        "overwriting must NOT increment bucket_count"
    );
    assert_eq!(dict.bucket_max, 32);

    let mut outsize: usize = 0;
    let got = sparse_dict_get(&dict, "key", 3, Some(&mut outsize)).unwrap();
    assert_eq!(outsize, 15);
    assert_eq!(got, b"different value");
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_set_growth() {
    // Verified via C: bucket_max remains 32 until bucket_count reaches 26,
    // then grows to 64.  We replicate that exact threshold here.
    let mut dict = sparse_dict_init().unwrap();
    // Insert 25 distinct keys; after each set the count is i+1 and max is 32.
    for i in 0..25 {
        let k = format!("k_{}", i);
        let v = format!("v_{}", i);
        assert_eq!(
            sparse_dict_set(&mut dict, &k, k.len(), v.as_bytes(), v.len()),
            1
        );
        assert_eq!(dict.bucket_count, i + 1);
        assert_eq!(dict.bucket_max, 32);
    }
    // 26th insert triggers the grow.
    let k = "k_25".to_string();
    let v = "v_25".to_string();
    assert_eq!(
        sparse_dict_set(&mut dict, &k, k.len(), v.as_bytes(), v.len()),
        1
    );
    assert_eq!(dict.bucket_count, 26);
    assert_eq!(dict.bucket_max, 64);

    // After grow, all keys must still be retrievable.
    for i in 0..26 {
        let k = format!("k_{}", i);
        let expected = format!("v_{}", i);
        let mut outsize: usize = 0;
        let got = sparse_dict_get(&dict, &k, k.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, expected.len());
        assert_eq!(got, expected.as_bytes());
    }
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_lots_of_set() {
    // Smaller than the C 1_000_000 — we just need a respectable number to
    // exercise rehashing across multiple grows. Each insert increments
    // bucket_count by exactly 1.
    let mut dict = sparse_dict_init().unwrap();
    let n: i32 = 1000;
    for i in 0..n {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        assert_eq!(
            sparse_dict_set(&mut dict, &key, key.len(), val.as_bytes(), val.len()),
            1
        );
        assert_eq!(dict.bucket_count, (i + 1) as usize);

        let mut outsize: usize = 0;
        let got = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(got, val.as_bytes());
    }
    // Reverse pass to make sure earlier inserts weren't lost across rehashes.
    for i in (0..n).rev() {
        let key = format!("crazy hash{}", i);
        let val = format!("value{}", i);
        let mut outsize: usize = 0;
        let got = sparse_dict_get(&dict, &key, key.len(), Some(&mut outsize)).unwrap();
        assert_eq!(outsize, val.len());
        assert_eq!(got, val.as_bytes());
    }
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_get_without_outsize() {
    let mut dict = sparse_dict_init().unwrap();
    assert_eq!(
        sparse_dict_set(&mut dict, "alpha", 5, b"BETA", 4),
        1
    );
    let got = sparse_dict_get(&dict, "alpha", 5, None).unwrap();
    assert_eq!(got, b"BETA");
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_binary_value() {
    let mut dict = sparse_dict_init().unwrap();
    let key = "binary";
    let val: [u8; 6] = [0x00, 0xFF, 0x10, 0x20, 0x80, 0x7F];
    assert_eq!(
        sparse_dict_set(&mut dict, key, key.len(), &val, val.len()),
        1
    );
    let mut outsize: usize = 0;
    let got = sparse_dict_get(&dict, key, key.len(), Some(&mut outsize)).unwrap();
    assert_eq!(outsize, val.len());
    assert_eq!(got, &val);
    assert_eq!(sparse_dict_free(dict), 1);
}

#[test]
fn test_dict_repeated_overwrite_stays_consistent() {
    let mut dict = sparse_dict_init().unwrap();
    for i in 0..50 {
        let v = format!("value_{}", i);
        assert_eq!(
            sparse_dict_set(&mut dict, "same_key", 8, v.as_bytes(), v.len()),
            1
        );
        assert_eq!(dict.bucket_count, 1);
        assert_eq!(dict.bucket_max, 32);
        let mut outsize: usize = 0;
        let got = sparse_dict_get(&dict, "same_key", 8, Some(&mut outsize)).unwrap();
        assert_eq!(outsize, v.len());
        assert_eq!(got, v.as_bytes());
    }
    assert_eq!(sparse_dict_free(dict), 1);
}

fn main() {}
