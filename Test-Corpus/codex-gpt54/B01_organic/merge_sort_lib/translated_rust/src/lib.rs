#![allow(non_camel_case_types)]

use std::ffi::{c_int, c_ulonglong};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

fn spritebatch_internal_sprite_less_than_or_equal(
    a: &spritebatch_sprite_t,
    b: &spritebatch_sprite_t,
) -> c_int {
    if a.sort_bits <= b.sort_bits {
        return 1;
    }
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return 1;
    }
    0
}

unsafe fn spritebatch_internal_merge_sort_iteration(
    a: *mut spritebatch_sprite_t,
    lo: c_int,
    split: c_int,
    hi: c_int,
    b: *mut spritebatch_sprite_t,
) {
    let mut i = lo;
    let mut j = split;
    let mut k = lo;

    while k < hi {
        if i < split
            && (j >= hi
                || spritebatch_internal_sprite_less_than_or_equal(
                    &*a.add(i as usize),
                    &*a.add(j as usize),
                ) != 0)
        {
            *b.add(k as usize) = *a.add(i as usize);
            i += 1;
        } else {
            *b.add(k as usize) = *a.add(j as usize);
            j += 1;
        }
        k += 1;
    }
}

unsafe fn spritebatch_internal_merge_sort_recurse(
    b: *mut spritebatch_sprite_t,
    lo: c_int,
    hi: c_int,
    a: *mut spritebatch_sprite_t,
) {
    if hi - lo <= 1 {
        return;
    }

    let split = (lo + hi) / 2;
    spritebatch_internal_merge_sort_recurse(a, lo, split, b);
    spritebatch_internal_merge_sort_recurse(a, split, hi, b);
    spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    ptr::copy_nonoverlapping(a, b, size as usize);
    spritebatch_internal_merge_sort_recurse(b, 0, size, a);
}
