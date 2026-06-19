use std::ffi::{c_int, c_ulonglong, c_void};
use std::mem;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

unsafe extern "C" {
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
}

unsafe fn spritebatch_internal_sprite_less_than_or_equal(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
) -> c_int {
    if unsafe { (*a).sort_bits <= (*b).sort_bits } {
        return 1;
    }
    if unsafe { (*a).sort_bits == (*b).sort_bits && (*a).texture_id <= (*b).texture_id } {
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
                || unsafe {
                    spritebatch_internal_sprite_less_than_or_equal(
                        a.offset(i as isize),
                        a.offset(j as isize),
                    ) != 0
                })
        {
            unsafe {
                *b.offset(k as isize) = *a.offset(i as isize);
            }
            i += 1;
        } else {
            unsafe {
                *b.offset(k as isize) = *a.offset(j as isize);
            }
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
    unsafe {
        spritebatch_internal_merge_sort_recurse(a, lo, split, b);
        spritebatch_internal_merge_sort_recurse(a, split, hi, b);
        spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    let byte_count = mem::size_of::<spritebatch_sprite_t>().wrapping_mul(size as usize);
    unsafe {
        memcpy(b.cast::<c_void>(), a.cast::<c_void>(), byte_count);
        spritebatch_internal_merge_sort_recurse(b, 0, size, a);
    }
}
