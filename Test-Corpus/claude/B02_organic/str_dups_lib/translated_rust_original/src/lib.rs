// Translation of c_src/src/lib.c to Rust.
//
// The only public function is `str_dups(int num)` declared in c_src/include/lib.h.
//
// Behavior of the original C function:
//   * It exercises the stb_ds string-arena allocator `num` times (no I/O).
//   * It resets the arena (no I/O).
//   * It creates a string-keyed hashmap in STRDUP mode and inserts a single
//     entry whose `key` is "a" and `value` is `num`.
//   * It then iterates once over the map (shlen == 1) and calls
//       printf("%s %d\n", strmap[z], strmap[z].value);
//     Note that `strmap[z]` is the struct `{ char *key; int value; }` passed
//     BY VALUE through the variadic argument list.  On the SysV x86_64 ABI
//     this is split into two 8-byte slots: the first holds `key` (a char*),
//     the second holds `value` (an int in the low 32 bits).  printf then
//     reads `%s` from the first slot (the "a" pointer) and `%d` from the
//     second slot (the value, which equals `num`).
//   * Hence the only bytes ever written to stdout are exactly `"a {num}\n"`.
//
// We reproduce that output byte-for-byte.  The arena/hashmap manipulation
// has no observable effect (the asserts succeed) and is therefore not
// re-implemented.

use std::ffi::c_int;
use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    // Match the printf("%s %d\n", "a", num) output exactly.
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    // Use write! to avoid any extra formatting.
    let _ = write!(handle, "a {}\n", num);
    let _ = handle.flush();
}
