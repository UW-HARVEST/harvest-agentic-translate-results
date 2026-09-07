use std::ffi::{c_int, c_ulonglong, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
pub struct SpritebatchSprite {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

unsafe extern "C" {
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
}

const SPRITE_SIZE: usize = size_of::<SpritebatchSprite>();

unsafe fn sprite_at(sprites: *mut SpritebatchSprite, index: c_int) -> *mut SpritebatchSprite {
    unsafe { sprites.offset(index as isize) }
}

unsafe fn sprite_less_than_or_equal(
    a: *const SpritebatchSprite,
    b: *const SpritebatchSprite,
) -> bool {
    unsafe {
        if (*a).sort_bits <= (*b).sort_bits {
            return true;
        }
        if (*a).sort_bits == (*b).sort_bits && (*a).texture_id <= (*b).texture_id {
            return true;
        }
    }
    false
}

unsafe fn copy_sprite(source: *const SpritebatchSprite, destination: *mut SpritebatchSprite) {
    unsafe {
        ptr::copy_nonoverlapping(source.cast::<u8>(), destination.cast::<u8>(), SPRITE_SIZE);
    }
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
            && (j >= hi || unsafe { sprite_less_than_or_equal(sprite_at(a, i), sprite_at(a, j)) });

        if take_left {
            unsafe { copy_sprite(sprite_at(a, i), sprite_at(b, k)) };
            i = i.wrapping_add(1);
        } else {
            unsafe { copy_sprite(sprite_at(a, j), sprite_at(b, k)) };
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
    let byte_count = (size as isize as usize).wrapping_mul(SPRITE_SIZE);
    unsafe {
        memcpy(b.cast(), a.cast(), byte_count);
        merge_sort_recurse(b, 0, size, a);
    }
}
