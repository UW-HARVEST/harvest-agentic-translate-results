use std::ffi::{c_int, c_ulonglong};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpritebatchSprite {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

unsafe fn less_than_or_equal(a: *const SpritebatchSprite, b: *const SpritebatchSprite) -> bool {
    unsafe { (*a).sort_bits <= (*b).sort_bits }
}

unsafe fn merge_sort_iteration(
    a: *mut SpritebatchSprite,
    lo: c_int,
    split: c_int,
    hi: c_int,
    b: *mut SpritebatchSprite,
) {
    let mut i = lo;
    let mut j = split;
    let mut k = lo;

    while k < hi {
        let take_left = i < split
            && (j >= hi
                || unsafe {
                    less_than_or_equal(a.wrapping_offset(i as isize), a.wrapping_offset(j as isize))
                });

        if take_left {
            unsafe {
                ptr::copy_nonoverlapping(
                    a.wrapping_offset(i as isize),
                    b.wrapping_offset(k as isize),
                    1,
                );
            }
            i = i.wrapping_add(1);
        } else {
            unsafe {
                ptr::copy_nonoverlapping(
                    a.wrapping_offset(j as isize),
                    b.wrapping_offset(k as isize),
                    1,
                );
            }
            j = j.wrapping_add(1);
        }
        k = k.wrapping_add(1);
    }
}

unsafe fn merge_sort_recurse(
    b: *mut SpritebatchSprite,
    lo: c_int,
    hi: c_int,
    a: *mut SpritebatchSprite,
) {
    if hi.wrapping_sub(lo) <= 1 {
        return;
    }

    let split = lo.wrapping_add(hi) / 2;
    unsafe {
        merge_sort_recurse(a, lo, split, b);
        merge_sort_recurse(a, split, hi, b);
        merge_sort_iteration(b, lo, split, hi, a);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut SpritebatchSprite,
    b: *mut SpritebatchSprite,
    size: c_int,
) {
    let byte_count = size_of::<SpritebatchSprite>().wrapping_mul(size as usize);
    unsafe {
        ptr::copy_nonoverlapping(a.cast::<u8>(), b.cast::<u8>(), byte_count);
        merge_sort_recurse(b, 0, size, a);
    }
}
