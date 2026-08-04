



extern "C" {
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct spritebatch_sprite_t {
    pub texture_id: ::core::ffi::c_ulonglong,
    pub sort_bits: ::core::ffi::c_int,
}
fn spritebatch_internal_sprite_less_than_or_equal(
    a: *mut spritebatch_sprite_t,
    b: *mut spritebatch_sprite_t,
) -> ::core::ffi::c_int {
    let a = unsafe { &*a };
    let b = unsafe { &*b };

    if a.sort_bits < b.sort_bits
        || (a.sort_bits == b.sort_bits && a.texture_id <= b.texture_id)
    {
        1
    } else {
        0
    }
}

fn spritebatch_internal_merge_sort_iteration(
    a: &mut [spritebatch_sprite_t],
    lo: usize,
    split: usize,
    hi: usize,
    b: &mut [spritebatch_sprite_t],
) {
    let mut i = lo;
    let mut j = split;
    let mut k = lo;

    while k < hi {
        if i < split
            && (j >= hi
                || unsafe {
                    spritebatch_internal_sprite_less_than_or_equal(
                        &mut a[i] as *mut spritebatch_sprite_t,
                        &mut a[j] as *mut spritebatch_sprite_t,
                    ) != 0
                })
        {
            b[k] = a[i];
            i += 1;
        } else {
            b[k] = a[j];
            j += 1;
        }
        k += 1;
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
    spritebatch_internal_merge_sort_iteration(b, lo as _, split as _, hi as _, a);
}

#[no_mangle]
pub fn merge_sort(a: &mut [spritebatch_sprite_t], b: &mut [spritebatch_sprite_t], size: i32) {
    let len = size as usize;
    b[..len].clone_from_slice(&a[..len]);
    spritebatch_internal_merge_sort_recurse(b, 0, len, a);
}

