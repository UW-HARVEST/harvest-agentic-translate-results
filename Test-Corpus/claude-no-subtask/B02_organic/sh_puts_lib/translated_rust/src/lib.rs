// Translation of c_src/src/lib.c
//
// The only public function is `sh_puts(int num)`. Although the C source
// contains an implementation of the stb_ds dynamic-array / hashmap library,
// the public function only exercises a tiny slice of that functionality
// and its observable output is fully determined by the constants involved.
//
// Tracing `sh_puts(num)`:
//   1. Allocates strings "test_0".."test_{num-1}" into a string arena.
//      `stralloc` produces no output and the arena is then reset.
//   2. Creates a string-keyed hashmap (`sh_new_arena`) and inserts one
//      record `{ key = "a", value = num }`.
//   3. Iterates over the map (length 1) and runs:
//          printf("%s %d\n", strmap[z], strmap[z].value);
//      `strmap[z]` is the whole struct `{ char* key; int value; }` passed by
//      value as a variadic argument. On the SysV-AMD64 ABI used by Linux,
//      the first 8 bytes (the `key` pointer to "a") are loaded into the
//      register that backs `%s`, and the next 8 bytes (the `value` field)
//      back `%d`. The trailing explicit `strmap[z].value` argument is
//      consumed by no remaining conversion specifier.
//      Net output: "a {num}\n".
//
// We reproduce that exact byte stream by writing it to stdout via libc's
// printf, matching the C runtime's flushing behavior.

use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_puts(num: c_int) {
    // The C implementation pushes `num` strings into a string arena and then
    // resets it. That work has no externally observable effect (no I/O, no
    // returned state), so we simply skip it.

    // Equivalent to `printf("%s %d\n", "a", num);` which is what the variadic
    // call resolves to on x86_64 SysV.
    let fmt = b"%s %d\n\0".as_ptr();
    let key = b"a\0".as_ptr();
    unsafe {
        printf(fmt, key, num);
    }
}
