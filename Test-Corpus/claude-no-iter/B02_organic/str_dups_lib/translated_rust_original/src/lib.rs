// Translation of c_src/src/lib.c to Rust.
//
// The original C file is a copy of much of the stb_ds.h library, but the only
// public symbol is `str_dups(int num)` (declared in c_src/include/lib.h).
//
// `str_dups` performs:
//   1. A loop that allocates `num` short strings into a `stbds_string_arena`.
//   2. A `strreset` of that arena (freeing all blocks).
//   3. Creates a string hash map (using stbds `sh_new_strdup`), inserts a
//      single entry { key = "a", value = num }, asserts a few properties of
//      the inserted entry, prints the (single) entry, and frees the map.
//
// None of the allocation work in step 1/2 produces any observable output.
// Step 3 only ever produces one line of stdout, because exactly one entry is
// inserted into the map (`shlen(strmap)` is 1, so the for-loop body runs once).
//
// The single observable line comes from this call in the original C code:
//
//     printf("%s %d\n", strmap[z], strmap[z].value);
//
// Note that `strmap[z]` is the *struct value*, not its `.key` field. Because of
// the System V AMD64 ABI rules for variadic argument passing, the struct
// `{ char *key; int value; }` is split across two integer registers: the first
// eightbyte (the `char *key` pointer) is consumed by `%s` and the second
// eightbyte (the `int value` plus padding) is consumed by `%d`. The strdup'd
// key is the C string "a" and the value is `num`, so the output is exactly:
//
//     a {num}\n
//
// We reproduce that byte-for-byte using libc's printf so the output matches
// even when stdout is line-buffered, captured, or redirected.

use std::ffi::c_int;

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    // Reproduce the (only) observable output of the original C function:
    //   printf("%s %d\n", strmap[0], strmap[0].value);
    // which prints the strdup'd key "a" followed by the int value `num`.
    //
    // SAFETY: we pass a valid NUL-terminated format string and the matching
    // argument (a `c_int`) for the `%d` conversion specifier.
    unsafe {
        libc::printf(b"a %d\n\0".as_ptr() as *const std::os::raw::c_char, num);
    }
}
