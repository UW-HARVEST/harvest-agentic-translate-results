# libsodium → Rust translation conventions (READ FULLY BEFORE WRITING CODE)

Workspace root: `$HARVEST_WORKDIR` (called `$W` below).

* `$W/c_src/` — the original C. **READ ONLY. NEVER MODIFY.**
* `$W/translation/` — the Rust crate (`cdylib`, no external dependencies, edition 2021).
* `$W/tools/cpp.sh <file.c>` — prints the *post-preprocessor* libsodium source for a
  `.c` file, with all `#ifdef HAVE_*` already resolved and system-header noise stripped.
  **Use this** — it removes all guesswork about which branch is compiled.

## Build configuration being reproduced

The reference CMake build compiles every `.c` under `c_src/libsodium/` with **no
`HAVE_*` macros defined at all** (equivalent to `configure --disable-asm`).
Consequences you must honour:

* `HAVE_TI_MODE` is **NOT** defined → `fe25519` is `int32_t[10]` (fe_25_5), poly1305 uses
  the 32-bit donna code, etc.
* `NATIVE_LITTLE_ENDIAN` / `NATIVE_BIG_ENDIAN` **NOT** defined → byte-wise load/store.
* No SIMD (`HAVE_*INTRIN_H`, `HAVE_ARMCRYPTO`, `HAVE_AVX*`, …) → only the soft/ref
  implementations exist; `pick_best_implementation()` always selects them.
* `HAVE_PTHREAD` / `HAVE_ATOMIC_OPS` **NOT** defined → `sodium_crit_enter/leave` are no-ops.
* Compiler-predefined macros ARE defined: `__GNUC__`, `__ELF__`, `__x86_64__`,
  `__linux__`, `__SIZEOF_INT128__`, `__LITTLE_ENDIAN__`-equivalents, etc.
* `CONFIGURED` is not defined, `DEV_MODE` is not defined (only affects `#warning`s).

If in doubt, run `$W/tools/cpp.sh` on the file. That is the ground truth.

## Crate layout

`src/lib.rs` already declares every module — **do not edit `lib.rs`**.
Shared helper modules you should use:

* `crate::common` — `load32_le/store32_le/load64_le/store64_le/load32_be/store32_be/
  load64_be/store64_be/rotl32/rotl64/rotr32/rotr64/xor_buf/SODIUM_SIZE_MAX`.
  Signatures use raw pointers, e.g. `unsafe fn load64_le(src: *const u8) -> u64`.
* `crate::csys` — hand-rolled libc bindings (`malloc`, `free`, `memcpy`, `memset`,
  `memcmp`, `strlen`, `sysconf`, `mmap`, `mprotect`, `abort`, `errno()`, `set_errno()`,
  errno/prot/open constants, …). Add more `extern "C"` declarations locally in your own
  module if you need something that is missing — do NOT edit `csys.rs`
  (duplicate `extern "C"` declarations across modules are fine).
* `crate::types` — `#[repr(C)]` types that cross module boundaries: `fe25519`,
  `ge25519_p2/p3/p1p1/precomp/cached`, `crypto_hash_sha256_state`,
  `crypto_hash_sha512_state`, `crypto_generichash_blake2b_state`, `blake2b_state`,
  `randombytes_implementation`.

## Rules for every exported function

1. Signature: `#[unsafe(no_mangle)] pub unsafe extern "C" fn NAME(...) -> ...`
   (this crate uses edition 2021; write `#[no_mangle]` — the `unsafe(...)` form is
   not needed. Use exactly `#[no_mangle]`.)
2. **The `NAME` must be the final linker symbol.** `include/sodium/private/quirks.h`
   `#define`s many internal names to a `_sodium_`-prefixed symbol, e.g.
   `blake2b_init` → `_sodium_blake2b_init`, `ge25519_p3_add` → `_sodium_ge25519_p3_add`,
   `argon2_ctx` → `_sodium_argon2_ctx`. Every non-`static` function in a file that
   includes `private/quirks.h` (directly or transitively — almost all do) must be
   checked against that list. `$W/tools/cpp.sh` output shows the renamed name directly.
3. Use `core::ffi` types (`c_int`, `c_char`, `c_void`, `c_ulonglong`, …) or the exact
   Rust equivalent (`unsigned long long` → `u64`, `size_t` → `usize`,
   `unsigned char *` → `*mut u8`, `const unsigned char *` → `*const u8`).
4. `static` C functions become private Rust `unsafe fn` (no `#[no_mangle]`).
5. Exported **data** objects (e.g. `crypto_stream_chacha20_ref_implementation`,
   `randombytes_sysrandom_implementation`, `aegis128l_soft_implementation`) must be
   `#[no_mangle] pub static NAME: Type = ...;` with a `#[repr(C)]` type. If the C
   object is non-`const` and mutated, use `pub static mut`.
6. Reproduce behaviour **exactly**, including bugs, error-check order, integer
   overflow/wrapping, and undefined-ish behaviour. Use `wrapping_*`,
   `unchecked` casts (`as`), and raw pointers freely. The crate is compiled with
   `overflow-checks = false`, but still prefer `wrapping_add`/`wrapping_mul` so the
   result is unambiguous.
7. C `int` shifts/promotions: be careful to mirror C integer promotion. When a C
   expression mixes `unsigned char` and `int`, do the arithmetic in `i32`/`u32` as C
   would, then truncate.
8. Never `panic!`. No bounds-checked indexing on data whose length you cannot prove;
   use `*ptr.add(i)` for pointer walks. Local fixed arrays may be indexed normally.
9. `sodium_misuse()` (declared in `core.h`) is `extern "C" fn sodium_misuse() -> !`.
   Declare it locally as `fn sodium_misuse();` in an `extern "C"` block if you need it.
10. `COMPILER_ASSERT(X)` is a compile-time assert; either drop it or use
    `const _: () = assert!(...);`.

## Cross-module calls

Modules are decoupled: to call a function that lives in another Rust module, declare it
in a local `extern "C" { ... }` block using the **final linker name** and the exact C
signature from the header. Do not try to `use` it from the other module. Example:

```rust
extern "C" {
    fn crypto_hash_sha512(out: *mut u8, inp: *const u8, inlen: u64) -> c_int;
    #[link_name = "_sodium_ge25519_p3_add"]
    fn ge25519_p3_add(r: *mut ge25519_p3, p: *const ge25519_p3, q: *const ge25519_p3);
}
```

(The cdylib links with undefined symbols allowed while other modules are still stubs.)

## Verifying your work

```bash
cd $W/translation && cargo build --release --target-dir /tmp-unique-dir
```
Use a **unique** `--target-dir` (e.g. `$W/_chk/<yourname>`) so parallel agents do not
fight over the lock. `cargo build` on a cdylib tolerates undefined symbols, so a
successful build means your module type-checks.

**Only edit the file(s) you were assigned.** If `cargo build` reports errors in other
modules, ignore them (they belong to other agents) — but make sure there are ZERO
errors attributed to your own file(s).

Reference symbol lists: `$W/_cbuild/csyms.txt` (all 890 exports of the C `.so`) and
`$W/_cbuild/persym.txt` (exports grouped per object file). After finishing, check that
every symbol listed for your `.c` files appears in your `.rs` file.
