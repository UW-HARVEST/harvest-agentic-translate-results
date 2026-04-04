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
unsafe extern "C" fn spritebatch_internal_sprite_less_than_or_equal(
    mut a: *mut spritebatch_sprite_t,
    mut b: *mut spritebatch_sprite_t,
) -> ::core::ffi::c_int {
    if (*a).sort_bits <= (*b).sort_bits {
        return 1 as ::core::ffi::c_int;
    }
    if (*a).sort_bits == (*b).sort_bits && (*a).texture_id <= (*b).texture_id {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn spritebatch_internal_merge_sort_iteration(
    mut a: *mut spritebatch_sprite_t,
    mut lo: ::core::ffi::c_int,
    mut split: ::core::ffi::c_int,
    mut hi: ::core::ffi::c_int,
    mut b: *mut spritebatch_sprite_t,
) {
    let mut i: ::core::ffi::c_int = lo;
    let mut j: ::core::ffi::c_int = split;
    let mut k: ::core::ffi::c_int = lo;
    while k < hi {
        if i < split
            && (j >= hi
                || spritebatch_internal_sprite_less_than_or_equal(
                    a.offset(i as isize),
                    a.offset(j as isize),
                ) != 0)
        {
            *b.offset(k as isize) = *a.offset(i as isize);
            i = i + 1 as ::core::ffi::c_int;
        } else {
            *b.offset(k as isize) = *a.offset(j as isize);
            j = j + 1 as ::core::ffi::c_int;
        }
        k += 1;
    }
}
unsafe extern "C" fn spritebatch_internal_merge_sort_recurse(
    mut b: *mut spritebatch_sprite_t,
    mut lo: ::core::ffi::c_int,
    mut hi: ::core::ffi::c_int,
    mut a: *mut spritebatch_sprite_t,
) {
    if hi - lo <= 1 as ::core::ffi::c_int {
        return;
    }
    let mut split: ::core::ffi::c_int = (lo + hi) / 2 as ::core::ffi::c_int;
    spritebatch_internal_merge_sort_recurse(a, lo, split, b);
    spritebatch_internal_merge_sort_recurse(a, split, hi, b);
    spritebatch_internal_merge_sort_iteration(b, lo, split, hi, a);
}
#[no_mangle]
pub unsafe extern "C" fn merge_sort(
    mut a: *mut spritebatch_sprite_t,
    mut b: *mut spritebatch_sprite_t,
    mut size: ::core::ffi::c_int,
) {
    memcpy(
        b as *mut ::core::ffi::c_void,
        a as *const ::core::ffi::c_void,
        (::core::mem::size_of::<spritebatch_sprite_t>() as size_t).wrapping_mul(size as size_t),
    );
    spritebatch_internal_merge_sort_recurse(b, 0 as ::core::ffi::c_int, size, a);
}
