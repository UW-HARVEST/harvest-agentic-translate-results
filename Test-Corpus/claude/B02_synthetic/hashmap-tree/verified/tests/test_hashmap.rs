// Compare hashmap functions between the C and Rust shared libraries.

mod common;

use common::*;
use std::os::raw::c_void;

fn run_pair<F>(name: &str, f: F)
where
    F: Fn(&DriverLib) -> Vec<u8>,
{
    let c = unsafe { load_c() };
    let r = unsafe { load_rust() };
    let c_out = f(&c);
    let r_out = f(&r);
    assert_eq!(
        c_out, r_out,
        "Mismatch in test '{}'\nC:    {:?}\nRust: {:?}",
        name, c_out, r_out
    );
}

#[test]
fn test_hashmap_create_destroy() {
    run_pair("create_destroy", |lib| unsafe {
        let m = (lib.hashmap_create)();
        assert!(!m.is_null());
        let mut out = Vec::new();
        out.extend_from_slice(&(lib.hashmap_size)(m).to_le_bytes());
        // also peek capacity / size / deleted_count from the struct
        let map = &*m;
        out.extend_from_slice(&(map.capacity as u64).to_le_bytes());
        out.extend_from_slice(&(map.size as u64).to_le_bytes());
        out.extend_from_slice(&(map.deleted_count as u64).to_le_bytes());
        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_put_get_basic() {
    run_pair("put_get_basic", |lib| unsafe {
        let m = (lib.hashmap_create)();

        // Use static-like storage; keep values alive via Box::leak in the test only.
        // Use sentinel pointer values (unique per key) so we can compare addresses.
        let v1 = Box::leak(Box::new(42i64)) as *mut i64 as *mut c_void;
        let v2 = Box::leak(Box::new(100i64)) as *mut i64 as *mut c_void;
        let v3 = Box::leak(Box::new(200i64)) as *mut i64 as *mut c_void;

        let mut out = Vec::new();
        out.push((lib.hashmap_put)(m, 1, v1) as u8);
        out.push((lib.hashmap_put)(m, 2, v2) as u8);
        out.push((lib.hashmap_put)(m, 3, v3) as u8);
        out.extend_from_slice(&(lib.hashmap_size)(m).to_le_bytes());

        // For pointer compare: just check whether returned pointer == sentinel
        out.push(if (lib.hashmap_get)(m, 1) == v1 { 1 } else { 0 });
        out.push(if (lib.hashmap_get)(m, 2) == v2 { 1 } else { 0 });
        out.push(if (lib.hashmap_get)(m, 3) == v3 { 1 } else { 0 });
        out.push(if (lib.hashmap_get)(m, 99).is_null() { 1 } else { 0 });

        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_update() {
    run_pair("update", |lib| unsafe {
        let m = (lib.hashmap_create)();
        let v1 = 0x1111usize as *mut c_void;
        let v2 = 0x2222usize as *mut c_void;
        (lib.hashmap_put)(m, 1, v1);
        (lib.hashmap_put)(m, 1, v2);
        let mut out = Vec::new();
        out.extend_from_slice(&((lib.hashmap_size)(m) as u64).to_le_bytes());
        out.extend_from_slice(&((lib.hashmap_get)(m, 1) as usize as u64).to_le_bytes());
        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_remove() {
    run_pair("remove", |lib| unsafe {
        let m = (lib.hashmap_create)();
        let v1 = 0x1111usize as *mut c_void;
        let v2 = 0x2222usize as *mut c_void;
        (lib.hashmap_put)(m, 1, v1);
        (lib.hashmap_put)(m, 2, v2);

        let mut out = Vec::new();
        out.extend_from_slice(&((lib.hashmap_remove)(m, 2) as usize as u64).to_le_bytes());
        out.extend_from_slice(&((lib.hashmap_size)(m) as u64).to_le_bytes());
        out.push((lib.hashmap_contains)(m, 1) as u8);
        out.push((lib.hashmap_contains)(m, 2) as u8);
        out.extend_from_slice(&((lib.hashmap_remove)(m, 99) as usize as u64).to_le_bytes());

        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_collisions() {
    run_pair("collisions", |lib| unsafe {
        let m = (lib.hashmap_create)();

        let mut out = Vec::new();
        for i in 0..100u64 {
            let v = (i * 10 + 1) as *mut c_void;
            out.push((lib.hashmap_put)(m, i, v) as u8);
        }
        out.extend_from_slice(&((lib.hashmap_size)(m) as u64).to_le_bytes());

        // After 100 inserts, capacity should have grown several times
        let map = &*m;
        out.extend_from_slice(&(map.capacity as u64).to_le_bytes());
        out.extend_from_slice(&(map.size as u64).to_le_bytes());
        out.extend_from_slice(&(map.deleted_count as u64).to_le_bytes());

        for i in 0..100u64 {
            let got = (lib.hashmap_get)(m, i) as usize as u64;
            out.extend_from_slice(&got.to_le_bytes());
        }

        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_remove_then_insert() {
    run_pair("remove_then_insert", |lib| unsafe {
        let m = (lib.hashmap_create)();

        // Insert keys, remove some, re-insert different ones to exercise the
        // tombstone-reuse path in put.
        for i in 0..20u64 {
            (lib.hashmap_put)(m, i, (i + 1) as *mut c_void);
        }
        for i in 0..10u64 {
            (lib.hashmap_remove)(m, i * 2);
        }
        for i in 0..10u64 {
            (lib.hashmap_put)(m, 1000 + i, (1000 + i + 1) as *mut c_void);
        }

        let mut out = Vec::new();
        let map = &*m;
        out.extend_from_slice(&(map.capacity as u64).to_le_bytes());
        out.extend_from_slice(&(map.size as u64).to_le_bytes());
        out.extend_from_slice(&(map.deleted_count as u64).to_le_bytes());

        for i in 0..20u64 {
            let got = (lib.hashmap_get)(m, i) as usize as u64;
            out.extend_from_slice(&got.to_le_bytes());
        }
        for i in 0..10u64 {
            let got = (lib.hashmap_get)(m, 1000 + i) as usize as u64;
            out.extend_from_slice(&got.to_le_bytes());
        }

        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_clear() {
    run_pair("clear", |lib| unsafe {
        let m = (lib.hashmap_create)();
        for i in 0..30u64 {
            (lib.hashmap_put)(m, i, (i + 1) as *mut c_void);
        }
        (lib.hashmap_clear)(m);

        let mut out = Vec::new();
        out.extend_from_slice(&((lib.hashmap_size)(m) as u64).to_le_bytes());
        let map = &*m;
        out.extend_from_slice(&(map.capacity as u64).to_le_bytes());
        out.extend_from_slice(&(map.size as u64).to_le_bytes());
        out.extend_from_slice(&(map.deleted_count as u64).to_le_bytes());

        // After clear, all gets should be null.
        for i in 0..30u64 {
            out.push(if (lib.hashmap_get)(m, i).is_null() { 1 } else { 0 });
        }

        (lib.hashmap_destroy)(m);
        out
    });
}

#[test]
fn test_hashmap_null_inputs() {
    run_pair("null_inputs", |lib| unsafe {
        let mut out = Vec::new();
        out.push((lib.hashmap_put)(std::ptr::null_mut(), 1, std::ptr::null_mut()) as u8);
        out.push(if (lib.hashmap_get)(std::ptr::null_mut(), 1).is_null() {
            1
        } else {
            0
        });
        out.push(
            if (lib.hashmap_remove)(std::ptr::null_mut(), 1).is_null() {
                1
            } else {
                0
            },
        );
        out.extend_from_slice(&((lib.hashmap_size)(std::ptr::null_mut()) as u64).to_le_bytes());
        // Clear should not crash
        (lib.hashmap_clear)(std::ptr::null_mut());
        out
    });
}

#[test]
fn test_hashmap_struct_layout() {
    // Verify capacity and other internal fields evolve identically.
    run_pair("struct_layout", |lib| unsafe {
        let m = (lib.hashmap_create)();
        let mut out = Vec::new();

        let snapshot = |m: *mut hashmap_t, out: &mut Vec<u8>| {
            let map = &*m;
            out.extend_from_slice(&(map.capacity as u64).to_le_bytes());
            out.extend_from_slice(&(map.size as u64).to_le_bytes());
            out.extend_from_slice(&(map.deleted_count as u64).to_le_bytes());
        };

        snapshot(m, &mut out);
        for i in 0..50u64 {
            (lib.hashmap_put)(m, i, (i + 1) as *mut c_void);
            snapshot(m, &mut out);
        }
        for i in 0..25u64 {
            (lib.hashmap_remove)(m, i);
            snapshot(m, &mut out);
        }
        for i in 100..150u64 {
            (lib.hashmap_put)(m, i, (i + 1) as *mut c_void);
            snapshot(m, &mut out);
        }

        (lib.hashmap_destroy)(m);
        out
    });
}
