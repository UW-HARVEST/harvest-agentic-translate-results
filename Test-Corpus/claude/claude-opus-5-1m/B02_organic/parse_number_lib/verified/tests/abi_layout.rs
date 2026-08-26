//! Sanity checks: both `.so`s are loadable, export `parse_number`, and the ABI
//! mirrors used by the harness have the x86-64 SysV layout the C compiler picks.

mod common;

use common::*;
use std::mem::{align_of, size_of};

#[test]
fn abi_cjson_layout() {
    assert_eq!(size_of::<CJson>(), 16);
    assert_eq!(align_of::<CJson>(), 8);
    assert_eq!(std::mem::offset_of!(CJson, type_), 0);
    assert_eq!(std::mem::offset_of!(CJson, valueint), 4);
    assert_eq!(std::mem::offset_of!(CJson, valuedouble), 8);
}

#[test]
fn abi_parse_buffer_layout() {
    assert_eq!(size_of::<ParseBuffer>(), 32);
    assert_eq!(align_of::<ParseBuffer>(), 8);
    assert_eq!(std::mem::offset_of!(ParseBuffer, content), 0);
    assert_eq!(std::mem::offset_of!(ParseBuffer, length), 8);
    assert_eq!(std::mem::offset_of!(ParseBuffer, offset), 16);
    assert_eq!(std::mem::offset_of!(ParseBuffer, depth), 24);
}

#[test]
fn both_shared_objects_export_parse_number() {
    // Panics inside the loader if either `dlopen` or `dlsym` fails.
    let c = c_parse_number();
    let r = rust_parse_number();
    assert_ne!(c as usize, 0);
    assert_ne!(r as usize, 0);
    assert_ne!(
        c as usize, r as usize,
        "the two symbols must come from two distinct shared objects"
    );
}

#[test]
fn smoke_simple_number() {
    let out = assert_same_str("123");
    assert_eq!(out.ret, C_TRUE);
    assert_eq!(out.item_type, CJSON_NUMBER);
    assert_eq!(out.item_valueint, 123);
    assert_eq!(f64::from_bits(out.item_double_bits), 123.0);
    assert_eq!(out.buf_offset, 3);
}
