use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct spritebatch_sprite_t {
    pub texture_id: std::os::raw::c_ulonglong,
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
    for k in lo..hi {
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
    // After recursion, b is the source (immutable view) and a is the destination
    // We need to call iteration with b as input and a as output.
    // Borrow checker: split into separate calls.
    let b_view: &[spritebatch_sprite_t] =
        unsafe { std::slice::from_raw_parts(b.as_ptr(), b.len()) };
    spritebatch_internal_merge_sort_iteration(b_view, lo, split, hi, a);
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
    let n = size as usize;
    // memcpy(b, a, sizeof * size);
    std::ptr::copy_nonoverlapping(a, b, n);

    let a_slice = std::slice::from_raw_parts_mut(a, n);
    let b_slice = std::slice::from_raw_parts_mut(b, n);
    spritebatch_internal_merge_sort_recurse(b_slice, 0, size, a_slice);
}
