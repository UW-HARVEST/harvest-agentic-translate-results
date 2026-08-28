//! Level 4: the only function in `include/lib.h` -- `arr_ins(int)` -- plus a
//! direct replay of the `stbds_arr*` macro chain it is built from.
//!
//! `arr_ins` returns nothing and its asserts abort the process, so the
//! behavioural contract is "completes normally for every input". Each library
//! is additionally driven through the same macro expansion by hand so the
//! intermediate array state can be compared.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

const E: usize = std::mem::size_of::<c_int>();

#[test]
fn arr_ins_completes_for_both() {
    let (c, r) = both();
    for num in [
        0, 1, -1, 4, 5, 42, -42, 1000, i32::MAX, i32::MIN, 0x7f, -0x80, 12345678, -87654321,
    ] {
        unsafe { (c.arr_ins)(num) };
        unsafe { (r.arr_ins)(num) };
    }
    for num in -200..200 {
        unsafe { (c.arr_ins)(num) };
        unsafe { (r.arr_ins)(num) };
    }
}

/// `arr_ins` is a fixed sequence of `arrpush` / `arrins` / `arrfree` on an
/// `int *`. Replaying the macro expansion lets us compare the array contents
/// and header at every step, which `arr_ins` itself hides.
unsafe fn replay_arr_ins(api: &Api, num: c_int) -> Vec<(HeaderSnap, Vec<c_int>)> {
    unsafe {
        let mut out = Vec::new();
        let mut arr: *mut c_void = std::ptr::null_mut();

        // stbds_arrmaybegrow(a, n)
        let maybegrow = |a: &mut *mut c_void, n: usize| {
            if a.is_null()
                || (*(*a as *mut ArrayHeader).sub(1)).length + n
                    > (*(*a as *mut ArrayHeader).sub(1)).capacity
            {
                *a = (api.arrgrowf)(*a, E, n, 0);
            }
        };

        for i in 0..5usize {
            // arrpush(arr, 1..4)
            for v in 1..=4 as c_int {
                maybegrow(&mut arr, 1);
                let h = (arr as *mut ArrayHeader).sub(1);
                *(arr as *mut c_int).add((*h).length) = v;
                (*h).length += 1;
            }
            // stbds_arrins(arr, i, num) == arrinsn(arr,i,1) then arr[i] = num
            //   arrinsn -> arraddn(a,1); memmove(&a[i+1], &a[i], 4*(len-1-i))
            maybegrow(&mut arr, 1);
            let h = (arr as *mut ArrayHeader).sub(1);
            (*h).length += 1;
            let count = (*h).length - 1 - i;
            std::ptr::copy(
                (arr as *const c_int).add(i),
                (arr as *mut c_int).add(i + 1),
                count,
            );
            *(arr as *mut c_int).add(i) = num;

            let h = (arr as *mut ArrayHeader).sub(1);
            let len = (*h).length;
            out.push((
                header_snap(arr),
                std::slice::from_raw_parts(arr as *const c_int, len).to_vec(),
            ));

            // arrfree(arr)
            (api.arrfreef)(arr);
            arr = std::ptr::null_mut();
        }
        out
    }
}

#[test]
fn arr_ins_macro_chain_matches() {
    let (c, r) = both();
    for num in [0, 1, -1, 4, 99, i32::MAX, i32::MIN, -7] {
        let a = unsafe { replay_arr_ins(&c, num) };
        let b = unsafe { replay_arr_ins(&r, num) };
        assert_eq!(a, b, "arr_ins macro chain for num={num}");
        // the assertions inside the real arr_ins must hold on both sides
        for (i, (_, v)) in a.iter().enumerate() {
            assert_eq!(v[i], num, "arr[{i}] == num");
            if i < 4 {
                assert_eq!(v[4], 4, "arr[4] == 4");
            }
        }
    }
}
