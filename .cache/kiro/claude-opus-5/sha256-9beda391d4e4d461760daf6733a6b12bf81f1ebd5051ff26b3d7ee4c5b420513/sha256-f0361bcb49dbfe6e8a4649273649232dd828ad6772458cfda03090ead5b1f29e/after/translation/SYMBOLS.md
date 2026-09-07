# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#  -> c_src/build/libharvest-work-14fyVt.so
cd translation && cargo build --release
#  -> translation/target/release/libsiphash_lib.so
```

## C `.so` dynamic symbols (`nm -D --defined-only`)

| symbol | type | present in Rust `.so`? | notes |
|--------|------|------------------------|-------|
| `stbds_hash_bytes` | `T` (global text) | YES | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` |
| `siphash`          | `T` (global text) | YES | `void siphash(int init)` |

Total C exported symbols: **2**. Total missing from Rust: **0**.

## Symbols intentionally NOT exported

| symbol | why |
|--------|-----|
| `stbds_siphash_bytes` | `static` in `c_src/src/lib.c` (line 6) → internal linkage, not in the C `.so` dynamic table. Kept private (`unsafe fn`) in Rust so the exported surface matches exactly. |

There are no macro-generated exports in this library (no `#define`-generated
function names in `c_src/src/lib.c` or `c_src/include/lib.h`).

## Modules / translation units

`c_src/CMakeLists.txt` compiles exactly one source file (`src/lib.c`) into a
`SHARED` library and links `m`. There is **no** `add_executable`, so the project
builds **no driver binary** — the "compare C and Rust stdout for a binary"
clause of the completion gate is not applicable. (`siphash()` does write to
stdout; that is covered as a library call in Phase B by redirecting fd 1.)

No C source file is untranslated: `src/lib.c` is the whole library, and all
three of its functions (`stbds_siphash_bytes`, `stbds_hash_bytes`, `siphash`)
have Rust counterparts in `translation/src/lib.rs`.

## Undefined (imported) symbols

The Rust `.so` must not require any non-libc symbol. Checked with
`nm -D -u`: the only undefined symbols are libc/`ld.so` ones
(`printf`, `memcpy`, `__errno_location`, unwinder/`__cxa` stubs, etc.).
0 missing/undefined non-libc symbols.

## Gate status

- [x] `nm -D` shows 0 symbols exported by the C `.so` that the Rust `.so` lacks.
- [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.

## Verification methodology note (important)

`cargo test` does **not** rebuild a `cdylib`-only lib target when the
integration tests do not `use` the crate — the tests `dlopen` the `.so` instead.
That means a stale `.so` can be loaded silently and every differential test
becomes vacuous. This was caught here by a negative control: two deliberate bugs
were injected into `src/lib.rs`, `cargo test` was run, and all tests still
passed against the stale artifact.

Two mitigations are in place:

1. `tests/common/mod.rs::assert_so_fresh` fails the test if
   `target/<profile>/libsiphash_lib.so` is older than `src/lib.rs`.
2. `scripts/verify_all.sh` always runs
   `cargo build --lib --example siphash_driver` before `cargo test`, for every
   profile and feature combination.

The negative control was then repeated with the rebuild in place: 13 of 47 tests
failed, and a single-bit change (`v2 ^= 0xff` → `0xfe`) is caught by
`broad_random_fuzz` on its first short-length iteration. The suite is therefore
demonstrably able to detect divergence.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` section, so the only
combination is the default (empty) feature set. `scripts/verify_all.sh`
enumerates features mechanically from `Cargo.toml` and would expand to the full
power set if any were added. Both the `release` and `dev` profiles are verified;
`dev` matters because it turns on Rust's arithmetic overflow checks, and the C
this translates deliberately relies on wrapping/overflowing arithmetic.
