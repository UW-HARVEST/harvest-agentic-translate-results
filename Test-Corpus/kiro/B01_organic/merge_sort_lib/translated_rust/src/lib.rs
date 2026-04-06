use std::ffi::c_int;
use std::os::raw::c_ulonglong;
use std::ptr;

#[repr(C)]
pub struct spritebatch_sprite_t {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

fn sprite_less_than_or_equal(a: &spritebatch_sprite_t, b: &spritebatch_sprite_t) -> bool {
    if a.sort_bits <= b.sort_bits {
        return true;
    }
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return true;
    }
    false
}

unsafe fn merge_sort_iteration(
    a: *const spritebatch_sprite_t,
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
            && (j >= hi || sprite_less_than_or_equal(&*a.offset(i as isize), &*a.offset(j as isize)))
        {
            ptr::copy_nonoverlapping(a.offset(i as isize), b.offset(k as isize), 1);
            i += 1;
        } else {
            ptr::copy_nonoverlapping(a.offset(j as isize), b.offset(k as isize), 1);
            j += 1;
        }
        k += 1;
    }
}

unsafe fn merge_sort_recurse(
    b: *mut spritebatch_sprite_t,
    lo: c_int,
    hi: c_int,
    a: *mut spritebatch_sprite_t,
) {
    if hi - lo <= 1 {
        return;
    }
    let split = (lo + hi) / 2;
    merge_sort_recurse(a, lo, split, b);
    merge_sort_recurse(a, split, hi, b);
    merge_sort_iteration(b, lo, split, hi, a);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    unsafe {
        ptr::copy_nonoverlapping(a, b, size as usize);
        merge_sort_recurse(b, 0, size, a);
    }
}
