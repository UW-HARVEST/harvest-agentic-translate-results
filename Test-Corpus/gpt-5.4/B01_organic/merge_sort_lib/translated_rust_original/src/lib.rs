use std::cmp::Ordering;
use std::ffi::c_int;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
    pub sort_bits: c_int,
}

fn sprite_less_than_or_equal(a: &spritebatch_sprite_t, b: &spritebatch_sprite_t) -> bool {
    match a.sort_bits.cmp(&b.sort_bits) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => a.texture_id <= b.texture_id,
    }
}

fn merge_sort_iteration(
    a: &[spritebatch_sprite_t],
    lo: usize,
    split: usize,
    hi: usize,
    b: &mut [spritebatch_sprite_t],
) {
    let mut i = lo;
    let mut j = split;
    for out in &mut b[lo..hi] {
        if i < split && (j >= hi || sprite_less_than_or_equal(&a[i], &a[j])) {
            *out = a[i];
            i += 1;
        } else {
            *out = a[j];
            j += 1;
        }
    }
}

fn merge_sort_recurse(
    src: &mut [spritebatch_sprite_t],
    lo: usize,
    hi: usize,
    dst: &mut [spritebatch_sprite_t],
) {
    if hi - lo <= 1 {
        return;
    }
    let split = (lo + hi) / 2;
    merge_sort_recurse(dst, lo, split, src);
    merge_sort_recurse(dst, split, hi, src);
    merge_sort_iteration(src, lo, split, hi, dst);
}

#[unsafe(no_mangle)]
pub extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    if a.is_null() || b.is_null() || size <= 0 {
        return;
    }

    let size = size as usize;

    unsafe {
        ptr::copy_nonoverlapping(a, b, size);
        let a_slice = std::slice::from_raw_parts_mut(a, size);
        let b_slice = std::slice::from_raw_parts_mut(b, size);
        merge_sort_recurse(b_slice, 0, size, a_slice);
    }
}
