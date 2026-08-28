//! Sanity checks for the harness itself plus the `driver` end-to-end entry
//! point from `c_src/test.c`.
#![allow(non_snake_case)]

mod harness;

use harness::*;
#[allow(unused_imports)]
use std::ffi::{c_char, c_int};

#[test]
fn harness_loads_both_libraries() {
    let (c, r) = both();
    unsafe {
        let cv = cstr((c.cJSON_Version)());
        let rv = cstr((r.cJSON_Version)());
        assert_eq!(cv, rv, "cJSON_Version mismatch");
        assert_eq!(cv, Some(b"1.7.19".to_vec()));
    }
}

#[test]
fn struct_layout_matches_c() {
    // 3 pointers + int(+pad) + pointer + int(+pad) + double + pointer
    assert_eq!(std::mem::size_of::<CJson>(), 64);
    assert_eq!(std::mem::align_of::<CJson>(), 8);
}

