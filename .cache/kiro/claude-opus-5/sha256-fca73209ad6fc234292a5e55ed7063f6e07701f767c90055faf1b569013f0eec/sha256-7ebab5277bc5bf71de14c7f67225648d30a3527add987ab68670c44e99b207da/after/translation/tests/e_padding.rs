//! Raw-memory parity for the whole node table, including the `Node` struct's
//! padding bytes. The C compiles `Node new_node = {.id = ...}` into a full
//! 80-byte zero-fill followed by member stores, then copies the entire object
//! into `node_storage`, so padding is observably zero to any caller that
//! memcpy's a `Node` returned by `find_node_by_id`.
mod harness;

use harness::{impls, Api, Node};
use std::ffi::{c_char, c_double, c_int};

const SZ: usize = std::mem::size_of::<Node>();

fn table_bytes(api: &Api, first_id: c_int, n: usize) -> Vec<u8> {
    unsafe {
        let base = (api.find_node_by_id)(first_id) as *const u8;
        assert!(!base.is_null(), "{} could not find node {first_id}", api.label);
        std::slice::from_raw_parts(base, n * SZ).to_vec()
    }
}

fn add(api: &Api, id: c_int, pid: c_int, name: &[u8], v: c_double) -> c_int {
    let mut buf: Vec<c_char> = name.iter().map(|&b| b as c_char).collect();
    buf.push(0);
    unsafe { (api.add_node)(id, pid, buf.as_ptr(), v) }
}

#[test]
fn node_struct_layout_is_80_bytes() {
    // guards the offsets the padding assertions below depend on
    assert_eq!(SZ, 80);
    assert_eq!(std::mem::align_of::<Node>(), 8);
}

/// Fill the table with a wide variety of names (so the tail of `name` and the
/// padding after it vary) and compare all 100 slots byte-for-byte.
#[test]
fn full_table_bytes_match_including_padding() {
    let i = impls();
    let names: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"ab".to_vec(),
        vec![b'q'; 7],
        vec![b'q'; 48],
        vec![b'q'; 49],
        vec![b'q'; 50],
        vec![b'q'; 130],
        vec![0xff, 0xfe, 0x80],
        (1u8..=49).collect(),
        (1u8..=90).collect(),
    ];

    for api in std::iter::once(&i.c).chain(i.rust.iter()) {
        api.reset();
    }

    // 6 baseline nodes are already present; add up to and past capacity.
    for k in 0..110usize {
        let name = &names[k % names.len()];
        let id = 300 + k as c_int;
        let pid = (k % 9) as c_int;
        let v = (k as f64) * -1.375;
        let expected = add(&i.c, id, pid, name, v);
        for r in &i.rust {
            assert_eq!(expected, add(r, id, pid, name, v), "add_node #{k} in {}", r.label);
        }

        // compare every occupied slot after each insertion
        let occupied = (6 + k + 1).min(harness::MAX_NODES);
        let cb = table_bytes(&i.c, 1, occupied);
        for r in &i.rust {
            assert_eq!(
                cb,
                table_bytes(r, 1, occupied),
                "table bytes differ after add #{k} in {}",
                r.label
            );
        }
    }
}

/// The padding bytes specifically: offsets 58..64 (after `name`) and 76..80
/// (after `active`) must be zero in both implementations.
#[test]
fn padding_bytes_are_zero_in_both() {
    let i = impls();
    for api in std::iter::once(&i.c).chain(i.rust.iter()) {
        api.reset();
        for k in 0..20i32 {
            add(api, 400 + k, k, &vec![b'p'; (k as usize * 3) % 60], k as f64);
        }
        let bytes = table_bytes(api, 1, 26);
        for slot in 0..26usize {
            for off in (58..64).chain(76..80) {
                assert_eq!(
                    bytes[slot * SZ + off],
                    0,
                    "{}: slot {slot} padding byte {off} is non-zero",
                    api.label
                );
            }
        }
    }
}

/// Stale slots beyond `node_count` are never rewritten by `maxnmin`'s reset, so
/// their raw bytes must also agree.
#[test]
fn stale_slot_bytes_match_after_reset() {
    let i = impls();
    for api in std::iter::once(&i.c).chain(i.rust.iter()) {
        api.reset();
        for k in 0..40i32 {
            add(api, 500 + k, k, format!("stale{k}").as_bytes(), k as f64 * 0.5);
        }
        unsafe { (api.maxnmin)(11, 22, 33, 44) };
    }
    let cb = table_bytes(&i.c, 1, harness::MAX_NODES);
    for r in &i.rust {
        assert_eq!(
            cb,
            table_bytes(r, 1, harness::MAX_NODES),
            "stale storage bytes differ in {}",
            r.label
        );
    }
}
