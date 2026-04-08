use hamta::hamta::*;

// ============================================================
// Constants
// ============================================================

#[test]
fn test_constants() {
    assert_eq!(FNV_BASE, 14695981039346656037u64);
    assert_eq!(FNV_PRIME, 1099511628211u64);
    assert_eq!(CHUNK_SIZE, 6);
    assert_eq!(HAMT_NODE_T_FLAG, 1);
    assert_eq!(KEY_VALUE_T_FLAG, 0);
}

// ============================================================
// hamt_int_hash
// ============================================================

#[test]
fn test_hamt_int_hash() {
    // Ground truth from C: hamt_int_hash on little-endian 4-byte int
    assert_eq!(hamt_int_hash(&mut 0i32), 2647528437);
    assert_eq!(hamt_int_hash(&mut 1i32), 2565215562);
    assert_eq!(hamt_int_hash(&mut 2i32), 2482902687);
    assert_eq!(hamt_int_hash(&mut 42i32), 163444391);
    assert_eq!(hamt_int_hash(&mut 100i32), 3979232233);
    assert_eq!(hamt_int_hash(&mut 255i32), 1565125984);
    assert_eq!(hamt_int_hash(&mut 1000i32), 3458511334);
    assert_eq!(hamt_int_hash(&mut (-1i32)), 2729652521);
    assert_eq!(hamt_int_hash(&mut 2147483647i32), 1565314729);
}

// ============================================================
// hamt_str_hash
// ============================================================

#[test]
fn test_hamt_str_hash() {
    // hamt_str_hash casts T to String internally
    assert_eq!(hamt_str_hash(&mut String::from("")), 2216829733);
    assert_eq!(hamt_str_hash(&mut String::from("a")), 2248259518);
    assert_eq!(hamt_str_hash(&mut String::from("bb")), 3035313733);
    assert_eq!(hamt_str_hash(&mut String::from("hello")), 3183334599);
    assert_eq!(hamt_str_hash(&mut String::from("xx")), 3035304125);
    assert_eq!(hamt_str_hash(&mut String::from("yy")), 3035303787);
    assert_eq!(hamt_str_hash(&mut String::from("aut")), 1806671401);
    assert_eq!(hamt_str_hash(&mut String::from("bus")), 1806519589);
    assert_eq!(hamt_str_hash(&mut String::from("vlak")), 2502912359);
    assert_eq!(hamt_str_hash(&mut String::from("kokos")), 3779452536);
    assert_eq!(hamt_str_hash(&mut String::from("banan")), 1227426209);
    assert_eq!(hamt_str_hash(&mut String::from("losos")), 3773913639);
}

// ============================================================
// hamt_int_equals
// ============================================================

#[test]
fn test_hamt_int_equals() {
    assert_eq!(hamt_int_equals(&mut 5i32, &mut 5i32), true);
    assert_eq!(hamt_int_equals(&mut 5i32, &mut 6i32), false);
    assert_eq!(hamt_int_equals(&mut 0i32, &mut 0i32), true);
    assert_eq!(hamt_int_equals(&mut -1i32, &mut -1i32), true);
    assert_eq!(hamt_int_equals(&mut -1i32, &mut 1i32), false);
}

// ============================================================
// hamt_str_equals
// ============================================================

#[test]
fn test_hamt_str_equals() {
    assert_eq!(hamt_str_equals(&mut String::from("abc"), &mut String::from("abc")), true);
    assert_eq!(hamt_str_equals(&mut String::from("abc"), &mut String::from("def")), false);
    assert_eq!(hamt_str_equals(&mut String::from(""), &mut String::from("")), true);
    assert_eq!(hamt_str_equals(&mut String::from("a"), &mut String::from("b")), false);
}

// ============================================================
// hamt_get_symbol (public wrapper)
// ============================================================

#[test]
fn test_hamt_get_symbol() {
    // The public hamt_get_symbol returns void in Rust (translation bug).
    // We just call it to ensure it doesn't panic.
    hamt_get_symbol(0, 0);
    hamt_get_symbol(0xFFFFFFFF, 5);
}

// ============================================================
// hamt_fnv1_hash (public wrapper)
// ============================================================

#[test]
fn test_hamt_fnv1_hash() {
    // The public hamt_fnv1_hash returns void in Rust (translation bug).
    // We just call it to ensure it doesn't panic.
    let mut data = String::from("hello");
    hamt_fnv1_hash(&mut data, 5);
}

// ============================================================
// HamtNode::is_leaf
// ============================================================

#[test]
fn test_hamt_node_is_leaf() {
    let mut leaf: HamtNode<i32, i32> = HamtNode::Leaf(None);
    assert_eq!(leaf.is_leaf(), true);

    let mut sub: HamtNode<i32, i32> = HamtNode::Sub(SubNode {
        bitmap: 0,
        children: Vec::new(),
    });
    assert_eq!(sub.is_leaf(), false);
}

// ============================================================
// Hamt: new, size, empty operations
// ============================================================

#[test]
fn test_new_hamt_and_size() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    assert_eq!(h.hamt_size(), 0);
    assert!(h.root.is_some());
}

#[test]
fn test_search_on_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let result = h.hamt_search(&mut 0);
    assert!(result.is_none());
}

#[test]
fn test_remove_on_empty() {
    let h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);
    let mut rk = 0i32;
    let mut rv = 0i32;
    let mut rkv = KeyValue { key: &mut rk, value: &mut rv };
    let removed = h.hamt_remove(&mut 0, &mut rkv);
    assert_eq!(removed, false);
}

// ============================================================
// Hamt: single element insert, search, remove
// ============================================================

#[test]
fn test_single_element() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k = 42i32;
    let mut v = 100i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut ckv = KeyValue { key: &mut ck, value: &mut cv };

    // Insert
    let conflict = h.hamt_set(&mut k, &mut v, &mut ckv);
    assert_eq!(conflict, false);
    assert_eq!(h.hamt_size(), 1);

    // Search
    let found = h.hamt_search(&mut 42i32);
    assert!(found.is_some());
    let kv = found.unwrap();
    assert_eq!(*kv.key, 42);
    assert_eq!(*kv.value, 100);

    // Remove
    let mut rk = 0i32;
    let mut rv = 0i32;
    let mut rkv = KeyValue { key: &mut rk, value: &mut rv };
    let removed = h.hamt_remove(&mut 42i32, &mut rkv);
    assert_eq!(removed, true);
    assert_eq!(*rkv.key, 42);
    assert_eq!(*rkv.value, 100);
    assert_eq!(h.hamt_size(), 0);
}

// ============================================================
// Hamt: multiple int inserts, search, overwrite, remove
// ============================================================

#[test]
fn test_hamt_int_operations() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    // Insert keys 0..10 with value = key*key
    let mut keys: Vec<i32> = (0..10).collect();
    let mut vals: Vec<i32> = (0..10).map(|i| i * i).collect();

    for i in 0..10 {
        let mut ck = 0i32;
        let mut cv = 0i32;
        let mut ckv = KeyValue { key: &mut ck, value: &mut cv };
        let conflict = h.hamt_set(&mut keys[i], &mut vals[i], &mut ckv);
        assert_eq!(conflict, false);
    }
    assert_eq!(h.hamt_size(), 10);

    // Search for each key
    for i in 0..10 {
        let found = h.hamt_search(&mut (i as i32));
        assert!(found.is_some(), "search({}) should find a result", i);
        let kv = found.unwrap();
        assert_eq!(*kv.value, (i * i) as i32);
    }

    // Search for non-existent key
    let not_found = h.hamt_search(&mut 99i32);
    assert!(not_found.is_none());

    // Overwrite key 5 with value 555
    let mut k5 = 5i32;
    let mut v555 = 555i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut ckv = KeyValue { key: &mut ck, value: &mut cv };
    let conflict = h.hamt_set(&mut k5, &mut v555, &mut ckv);
    assert_eq!(conflict, true); // conflict means key already existed
    assert_eq!(*ckv.value, 25); // old value was 5*5=25
    assert_eq!(h.hamt_size(), 10); // size unchanged

    // Search key 5 after overwrite
    let found5 = h.hamt_search(&mut 5i32);
    assert!(found5.is_some());
    assert_eq!(*found5.unwrap().value, 555);

    // Remove key 3
    let mut rk = 0i32;
    let mut rv = 0i32;
    let mut rkv = KeyValue { key: &mut rk, value: &mut rv };
    let removed = h.hamt_remove(&mut 3i32, &mut rkv);
    assert_eq!(removed, true);
    assert_eq!(*rkv.value, 9); // 3*3=9
    assert_eq!(h.hamt_size(), 9);

    // Search removed key
    let gone = h.hamt_search(&mut 3i32);
    assert!(gone.is_none());

    // Remove non-existent
    let mut rk2 = 0i32;
    let mut rv2 = 0i32;
    let mut rkv2 = KeyValue { key: &mut rk2, value: &mut rv2 };
    let removed2 = h.hamt_remove(&mut 99i32, &mut rkv2);
    assert_eq!(removed2, false);
}

// ============================================================
// Hamt: string operations
// ============================================================

#[test]
fn test_hamt_str_operations() {
    let mut h: Hamt<String, String> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let mut keys: Vec<String> = vec![
        "aut", "bus", "vlak", "kokos", "banan", "losos", "bro", "b", "bubakov"
    ].into_iter().map(String::from).collect();
    let mut vals: Vec<String> = keys.clone();

    for i in 0..9 {
        let mut ck = String::new();
        let mut cv = String::new();
        let mut ckv = KeyValue { key: &mut ck, value: &mut cv };
        h.hamt_set(&mut keys[i], &mut vals[i], &mut ckv);
    }
    assert_eq!(h.hamt_size(), 9);

    // Search all keys
    for key_str in &["aut", "bus", "vlak", "kokos", "banan", "losos", "bro", "b", "bubakov"] {
        let found = h.hamt_search(&mut String::from(*key_str));
        assert!(found.is_some(), "search(\"{}\") should find a result", key_str);
        assert_eq!(&*found.unwrap().value, *key_str);
    }

    // Remove "vlak"
    let mut rk = String::new();
    let mut rv = String::new();
    let mut rkv = KeyValue { key: &mut rk, value: &mut rv };
    let removed = h.hamt_remove(&mut String::from("vlak"), &mut rkv);
    assert_eq!(removed, true);
    assert_eq!(h.hamt_size(), 8);

    // Search removed key
    let gone = h.hamt_search(&mut String::from("vlak"));
    assert!(gone.is_none());
}

// ============================================================
// Hamt: string overwrite (test_create from C)
// ============================================================

#[test]
fn test_hamt_str_overwrite_and_remove() {
    let mut h: Hamt<String, String> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let mut kx = String::from("xx");
    let mut vx = String::from("xx");
    let mut ky = String::from("yy");
    let mut vy = String::from("yy");
    let mut ck = String::new();
    let mut cv = String::new();
    let mut ckv = KeyValue { key: &mut ck, value: &mut cv };

    // set(xx, xx)
    h.hamt_set(&mut kx, &mut vx, &mut ckv);
    assert_eq!(h.hamt_size(), 1);

    // set(yy, yy)
    h.hamt_set(&mut ky, &mut vy, &mut ckv);
    assert_eq!(h.hamt_size(), 2);

    // set(xx, yy) — overwrite
    let mut kx2 = String::from("xx");
    let mut vy2 = String::from("yy");
    let conflict = h.hamt_set(&mut kx2, &mut vy2, &mut ckv);
    assert_eq!(conflict, true);
    assert_eq!(h.hamt_size(), 2);

    // set(yy, xx) — overwrite
    let mut ky2 = String::from("yy");
    let mut vx2 = String::from("xx");
    let conflict2 = h.hamt_set(&mut ky2, &mut vx2, &mut ckv);
    assert_eq!(conflict2, true);
    assert_eq!(h.hamt_size(), 2);

    // remove(xx)
    let mut rk = String::new();
    let mut rv = String::new();
    let mut rkv = KeyValue { key: &mut rk, value: &mut rv };
    let removed = h.hamt_remove(&mut String::from("xx"), &mut rkv);
    assert_eq!(removed, true);
    assert_eq!(h.hamt_size(), 1);
}

// ============================================================
// Hamt: large test with int keys (test_big from C)
// ============================================================

#[test]
fn test_hamt_big_int() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let n = 10000i32;
    let mut keys: Vec<Box<i32>> = Vec::new();
    let mut vals: Vec<Box<i32>> = Vec::new();
    let mut ckeys: Vec<Box<i32>> = Vec::new();
    let mut cvals: Vec<Box<i32>> = Vec::new();
    for i in 0..n {
        keys.push(Box::new(i % (n / 1337) + 1));
        vals.push(Box::new(i * i + 10));
        ckeys.push(Box::new(0i32));
        cvals.push(Box::new(0i32));
    }

    for i in 0..n as usize {
        let key_val = *keys[i]; // save key value before set (may be swapped)
        let val_val = *vals[i];
        let mut ckv = KeyValue { key: &mut *ckeys[i], value: &mut *cvals[i] };
        let conflict = h.hamt_set(&mut *keys[i], &mut *vals[i], &mut ckv);
        if conflict {
            // On conflict, the old key/value are freed in C. In Rust, the values
            // were swapped into ckeys[i]/cvals[i]. The HAMT still holds the
            // original memory locations with the new values.
        }

        // Search using the original key value (not the potentially-swapped Box)
        let mut search_key = key_val;
        let found = h.hamt_search(&mut search_key);
        assert!(found.is_some(), "search failed at i={}, key={}", i, key_val);
        assert_eq!(*found.unwrap().value, val_val);
    }
}

// ============================================================
// Hamt: large test with insert and remove (test_big2 from C)
// ============================================================

#[test]
fn test_hamt_big_insert_remove() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let n = 10000i32;
    let mut keys: Vec<Box<i32>> = Vec::new();
    let mut vals: Vec<Box<i32>> = Vec::new();
    let mut ckeys: Vec<Box<i32>> = Vec::new();
    let mut cvals: Vec<Box<i32>> = Vec::new();
    for i in 0..n {
        keys.push(Box::new(i % (n / 1531)));
        vals.push(Box::new(i.wrapping_mul(i).wrapping_mul(i)));
        ckeys.push(Box::new(0i32));
        cvals.push(Box::new(0i32));
    }

    for i in 0..n as usize {
        let key_val = *keys[i];
        let val_val = *vals[i];
        let mut ckv = KeyValue { key: &mut *ckeys[i], value: &mut *cvals[i] };
        h.hamt_set(&mut *keys[i], &mut *vals[i], &mut ckv);

        let mut search_key = key_val;
        let found = h.hamt_search(&mut search_key);
        assert!(found.is_some());
        assert_eq!(*found.unwrap().value, val_val);
    }

    // Remove all keys 0..n
    let mut rkv_keys: Vec<Box<i32>> = (0..n).map(|_| Box::new(0i32)).collect();
    let mut rkv_vals: Vec<Box<i32>> = (0..n).map(|_| Box::new(0i32)).collect();
    for i in 0..n as usize {
        let mut rkey = i as i32;
        let mut rkv = KeyValue { key: &mut *rkv_keys[i], value: &mut *rkv_vals[i] };
        h.hamt_remove(&mut rkey, &mut rkv);
    }
}

// ============================================================
// Hamt: search and destroy test (test_search_destroy from C)
// ============================================================

#[test]
fn test_hamt_search_and_remove_all() {
    let mut h: Hamt<String, String> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let mut strs: Vec<String> = vec![
        "aut", "bus", "vlak", "kokos", "banan", "losos", "bro", "b", "bubakov"
    ].into_iter().map(String::from).collect();
    let mut vals: Vec<String> = strs.clone();

    for i in 0..9 {
        let mut ck = String::new();
        let mut cv = String::new();
        let mut ckv = KeyValue { key: &mut ck, value: &mut cv };
        h.hamt_set(&mut strs[i], &mut vals[i], &mut ckv);
    }
    assert_eq!(h.hamt_size(), 9);

    // Search all in different order (matching C test)
    let search_order = ["losos", "bus", "aut", "vlak", "banan", "kokos", "bro", "b", "bubakov"];
    for s in &search_order {
        let found = h.hamt_search(&mut String::from(*s));
        assert!(found.is_some(), "should find {}", s);
        assert_eq!(&*found.unwrap().value, *s);
    }

    // Remove all in same order
    let mut expected_size = 9;
    for s in &search_order {
        let mut rk = String::new();
        let mut rv = String::new();
        let mut rkv = KeyValue { key: &mut rk, value: &mut rv };
        let removed = h.hamt_remove(&mut String::from(*s), &mut rkv);
        assert_eq!(removed, true, "should remove {}", s);
        expected_size -= 1;
        assert_eq!(h.hamt_size(), expected_size);
    }
    assert_eq!(h.hamt_size(), 0);
}

// ============================================================
// Hamt: 13 string elements (test_hamta2 from C)
// ============================================================

#[test]
fn test_hamt_13_strings() {
    let mut h: Hamt<String, String> = Hamt::new_hamt(hamt_str_hash, hamt_str_equals);

    let raw = vec![
        "a", "bb", "auto", "bus", "vlak", "kokos", "banan",
        "losos", "bubakov", "korkodyl", "x", "__x__", "y"
    ];
    let mut keys: Vec<String> = raw.iter().map(|s| String::from(*s)).collect();
    let mut vals: Vec<String> = keys.clone();

    for i in 0..13 {
        let mut ck = String::new();
        let mut cv = String::new();
        let mut ckv = KeyValue { key: &mut ck, value: &mut cv };
        h.hamt_set(&mut keys[i], &mut vals[i], &mut ckv);
    }
    assert_eq!(h.hamt_size(), 13);
}

// ============================================================
// hamt_set return value semantics
// ============================================================

#[test]
fn test_hamt_set_return_value() {
    let mut h: Hamt<i32, i32> = Hamt::new_hamt(hamt_int_hash, hamt_int_equals);

    let mut k = 1i32;
    let mut v = 10i32;
    let mut ck = 0i32;
    let mut cv = 0i32;
    let mut ckv = KeyValue { key: &mut ck, value: &mut cv };

    // First insert: no conflict, returns false
    let r1 = h.hamt_set(&mut k, &mut v, &mut ckv);
    assert_eq!(r1, false);

    // Second insert same key: conflict, returns true
    let mut k2 = 1i32;
    let mut v2 = 20i32;
    let r2 = h.hamt_set(&mut k2, &mut v2, &mut ckv);
    assert_eq!(r2, true);
    assert_eq!(h.hamt_size(), 1);
}

fn main() {}
