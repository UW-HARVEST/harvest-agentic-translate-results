use std::ffi::c_int;

#[repr(C)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
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

fn merge_iteration(a: &[spritebatch_sprite_t], lo: usize, split: usize, hi: usize, b: &mut [spritebatch_sprite_t]) {
    let mut i = lo;
    let mut j = split;
    for k in lo..hi {
        if i < split && (j >= hi || sprite_less_than_or_equal(&a[i], &a[j])) {
            b[k].texture_id = a[i].texture_id;
            b[k].sort_bits = a[i].sort_bits;
            i += 1;
        } else {
            b[k].texture_id = a[j].texture_id;
            b[k].sort_bits = a[j].sort_bits;
            j += 1;
        }
    }
}

fn merge_sort_recurse(b: &mut [spritebatch_sprite_t], lo: usize, hi: usize, a: &mut [spritebatch_sprite_t]) {
    if hi - lo <= 1 {
        return;
    }
    let split = (lo + hi) / 2;
    merge_sort_recurse(a, lo, split, b);
    merge_sort_recurse(a, split, hi, b);
    merge_iteration(b, lo, split, hi, a);
}

/// # Safety
/// `a` and `b` must point to valid arrays of at least `size` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn merge_sort(a: *mut spritebatch_sprite_t, b: *mut spritebatch_sprite_t, size: c_int) {
    let n = size as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, n) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, n) };
    // memcpy(b, a, ...)
    for i in 0..n {
        b_slice[i].texture_id = a_slice[i].texture_id;
        b_slice[i].sort_bits = a_slice[i].sort_bits;
    }
    merge_sort_recurse(b_slice, 0, n, a_slice);
}
