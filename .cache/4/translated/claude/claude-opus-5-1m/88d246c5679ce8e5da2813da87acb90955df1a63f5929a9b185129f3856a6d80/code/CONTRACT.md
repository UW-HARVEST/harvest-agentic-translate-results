# Translation contract (READ FULLY BEFORE WRITING CODE)

We are translating **libsodium 1.0.23** from `c_src/libsodium/**` into a Rust
`cdylib` living in `src/`. The Rust `.so` must export the *exact same* symbol
names as the C `.so` and behave byte-identically.

## Hard rules

1. **Never modify anything under `c_src/`.** It is the reference.
2. **One C file → one Rust file.** The Rust file name is the C basename with
   `-` replaced by `_` and extension `.rs`, placed directly in `src/`.
   e.g. `crypto_generichash/blake2b/ref/blake2b-ref.c` → `src/blake2b_ref.rs`.
   The module is already declared in `src/lib.rs`; do not touch `lib.rs`.
3. **Only translate the preprocessor path the reference build takes.**
   The CMake build defines **no** `HAVE_*` macros, no `NATIVE_LITTLE_ENDIAN`,
   no `NATIVE_BIG_ENDIAN`, no `HAVE_TI_MODE`, no `__SSE2__`/`__AVX2__`/etc.,
   no `HAVE_INLINE_ASM`, no `HAVE_LIBCTGRIND`, no `SODIUM_LIBRARY_MINIMAL`,
   no `_MSC_VER`, no `__native_client__`, no `__EMSCRIPTEN__`, no `__wasi__`.
   It **is** Linux/glibc x86-64 ELF, `__GNUC__` defined, `__ELF__` defined,
   `HAVE_MMAP`/`HAVE_MPROTECT`/`HAVE_MLOCK`/`HAVE_MADVISE`/`HAVE_POSIX_MEMALIGN`
   are **NOT** defined (configure was not run — no `config.h`), so take the
   fallback branches for those too. When in doubt, run the C preprocessor
   yourself to see what survives:
   ```
   gcc -E -Ic_src/libsodium/include -Ic_src/libsodium/include/sodium \
       -Ic_src/libsodium/<subsystem-dir> <file.c> | less
   ```
4. **Exported functions.** Every C function with *external* linkage (i.e. not
   `static`) becomes:
   ```rust
   #[unsafe(no_mangle)]
   pub unsafe extern "C" fn <FINAL_LINKER_NAME>(...) -> ... { ... }
   ```
   The `<FINAL_LINKER_NAME>` is the name **after** macro renaming performed by
   `c_src/libsodium/include/sodium/private/quirks.h`. That header renames e.g.
   `ge25519_p3_tobytes` → `_sodium_ge25519_p3_tobytes`,
   `blake2b_update` → `_sodium_blake2b_update`,
   `softaes_block_encrypt` → `_sodium_softaes_block_encrypt`, etc.
   **Always check quirks.h** for every non-`static` function you translate.
   The authoritative per-file symbol list is in `SYMBOLS.md`; your file's
   exported names MUST match it exactly (no more, no fewer).
5. **`static` C functions / variables** become plain private Rust items. Never
   put `#[no_mangle]` on them.
6. **Cross-file calls.** If your file calls a function defined in a *different*
   C file, declare it locally:
   ```rust
   extern "C" {
       fn crypto_hash_sha512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
   }
   ```
   using the FINAL linker name. Do **not** `use crate::other_module::...` for
   these — the linker resolves them inside the cdylib. This keeps files
   independent. (Exception: `crate::common` helpers, see below.)
7. **Exported global variables** (the `*_implementation` structs):
   ```rust
   #[unsafe(no_mangle)]
   pub static crypto_stream_chacha20_ref_implementation:
       crypto_stream_chacha20_implementation = ...;
   ```
   All structs shared across the FFI boundary must be `#[repr(C)]`, with
   **exact** field order/sizes/alignment from the C header.
8. **No behaviour changes.** Reproduce integer overflow/wrapping, evaluation
   order, error-check order, and even apparent bugs exactly. Use
   `wrapping_add`/`wrapping_mul`/`<<`-with-mask etc. as needed; the crate is
   built with `overflow-checks = false` but be explicit anyway.
9. `libc` is **not** a dependency. Use `std` (the crate links std). For
   syscalls not in std, declare them yourself in an `extern "C"` block
   (`extern "C" { fn getrandom(buf: *mut c_void, len: usize, flags: c_uint) -> isize; }`
   etc.).
10. Do not add `#[cfg(target_arch)]` gates. Target is x86-64 Linux.
11. `sodium_runtime_has_*()` all return 0 in the reference build (no HAVE_*),
    so any `if (sodium_runtime_has_x())` dispatch resolves to the fallback.
    Still translate the dispatch code faithfully (call the extern fn).

## Available shared helpers — `crate::common`

`src/common.rs` already provides (all `pub`):

```rust
pub fn rotl32(x: u32, b: i32) -> u32;
pub fn rotl64(x: u64, b: i32) -> u64;
pub fn rotr32(x: u32, b: i32) -> u32;
pub fn rotr64(x: u64, b: i32) -> u64;
pub unsafe fn load64_le(src: *const u8) -> u64;
pub unsafe fn store64_le(dst: *mut u8, w: u64);
pub unsafe fn load32_le(src: *const u8) -> u32;
pub unsafe fn store32_le(dst: *mut u8, w: u32);
pub unsafe fn load64_be(src: *const u8) -> u64;
pub unsafe fn store64_be(dst: *mut u8, w: u64);
pub unsafe fn load32_be(src: *const u8) -> u32;
pub unsafe fn store32_be(dst: *mut u8, w: u32);
pub unsafe fn xor_buf(out: *mut u8, in_: *const u8, n: usize);
pub const SODIUM_SIZE_MAX: u64;
pub unsafe fn memcpy(dst: *mut u8, src: *const u8, n: usize);
pub unsafe fn memmove(dst: *mut u8, src: *const u8, n: usize);
pub unsafe fn memset(dst: *mut u8, c: u8, n: usize);
```

Use `use crate::common::*;` at the top of your file.

## Style notes

* Start each file with:
  ```rust
  use crate::common::*;
  use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
  ```
  (import only what you need).
* Prefer raw-pointer arithmetic that mirrors the C, or
  `core::slice::from_raw_parts{,_mut}` where lengths are known. Correctness and
  fidelity beat elegance.
* `unsigned long long` → `c_ulonglong` (u64). `size_t` → `usize`.
  `unsigned char *` → `*mut u8`. `const unsigned char *` → `*const u8`.
  `char *` → `*mut c_char`. `int` → `c_int`.
* For `static` mutable C globals use `static mut` + `unsafe`, or
  `UnsafeCell`/atomics if that is more convenient — but the observable
  behaviour must match.
* When a C function is `static inline` in a header shared by several .c files,
  just duplicate it as a private helper in each Rust file that needs it.

## Verification

After writing, run from the crate root:
```
cargo build --release 2>&1 | tail -40
```
and fix all errors. Then check your symbols:
```
nm -D --defined-only target/release/libsodium.so | awk '{print $3}' | sort
```
