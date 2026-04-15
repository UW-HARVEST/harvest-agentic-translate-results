use std::os::raw::{c_int, c_ulonglong};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct spritebatch_sprite_t {
    pub texture_id: c_ulonglong,
    pub sort_bits: c_int,
}

fn spritebatch_internal_sprite_less_than_or_equal(
    a: &spritebatch_sprite_t,
    b: &spritebatch_sprite_t,
) -> bool {
    if a.sort_bits <= b.sort_bits {
        return true;
    }
    if a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id {
        return true;
    }
    false
}

fn spritebatch_internal_merge_sort_iteration(
    a: &[spritebatch_sprite_t],
    lo: usize,
    split: usize,
    hi: usize,
    b: &mut [spritebatch_sprite_t],
) {
    let mut i = lo;
    let mut j = split;
    for k in lo..hi {
        if i < split && (j >= hi || spritebatch_internal_sprite_less_than_or_equal(&a[i], &a[j])) {
            b[k] = a[i];
            i += 1;
        } else {
            b[k] = a[j];
            j += 1;
        }
    }
}

fn spritebatch_internal_merge_sort_recurse(
    b: &mut [spritebatch_sprite_t],
    lo: usize,
    hi: usize,
    a: &mut [spritebatch_sprite_t],
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
pub extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    if size <= 0 {
        return;
    }
    let size = size as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, size) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, size) };
    b_slice.copy_from_slice(a_slice);
    spritebatch_internal_merge_sort_recurse(b_slice, 0, size, a_slice);
}
