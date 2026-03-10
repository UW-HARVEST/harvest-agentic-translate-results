use std::os::raw::c_int;
use std::ptr;

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

fn merge_iteration(a: &mut [spritebatch_sprite_t], lo: usize, split: usize, hi: usize, b: &mut [spritebatch_sprite_t]) {
    let mut i = lo;
    let mut j = split;
    for k in lo..hi {
        if i < split && (j >= hi || sprite_less_than_or_equal(&a[i], &a[j])) {
            b[k] = spritebatch_sprite_t { texture_id: a[i].texture_id, sort_bits: a[i].sort_bits };
            i += 1;
        } else {
            b[k] = spritebatch_sprite_t { texture_id: a[j].texture_id, sort_bits: a[j].sort_bits };
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

#[unsafe(no_mangle)]
pub extern "C" fn merge_sort(a: *mut spritebatch_sprite_t, b: *mut spritebatch_sprite_t, size: c_int) {
    if size <= 0 {
        return;
    }
    let n = size as usize;
    unsafe {
        ptr::copy_nonoverlapping(a, b, n);
        let a_slice = std::slice::from_raw_parts_mut(a, n);
        let b_slice = std::slice::from_raw_parts_mut(b, n);
        merge_sort_recurse(b_slice, 0, n, a_slice);
    }
}
