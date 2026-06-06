use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct spritebatch_sprite_t {
    pub texture_id: u64,
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

fn spritebatch_internal_merge_sort_iteration(
    a: &[spritebatch_sprite_t],
    lo: c_int,
    split: c_int,
    hi: c_int,
    b: &mut [spritebatch_sprite_t],
) {
    let mut i = lo;
    let mut j = split;
    let mut k = lo;
    while k < hi {
        if i < split
            && (j >= hi
                || spritebatch_internal_sprite_less_than_or_equal(
                    &a[i as usize],
                    &a[j as usize],
                ) != 0)
        {
            b[k as usize] = a[i as usize];
            i += 1;
        } else {
            b[k as usize] = a[j as usize];
            j += 1;
        }
        k += 1;
    }
}

fn spritebatch_internal_merge_sort_recurse(
    b: &mut [spritebatch_sprite_t],
    lo: c_int,
    hi: c_int,
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
pub unsafe extern "C" fn merge_sort(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
    size: c_int,
) {
    if size <= 0 {
        return;
    }
    let len = size as usize;
    let a_slice = unsafe { std::slice::from_raw_parts_mut(a, len) };
    let b_slice = unsafe { std::slice::from_raw_parts_mut(b, len) };
    // memcpy(b, a, sizeof(spritebatch_sprite_t) * size);
    b_slice.copy_from_slice(a_slice);
    spritebatch_internal_merge_sort_recurse(b_slice, 0, size, a_slice);
}
