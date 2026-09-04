# SYMBOLS.md — Phase A / Phase D symbol surface

## Source inventory

The whole C library is ONE translation unit:

| C file | translated in Rust? | notes |
|---|---|---|
| `c_src/src/lib.c` | yes — `translation/src/lib.rs` | 20 lines: `cn_rnd_next` (static) + `next_double` |
| `c_src/include/lib.h` | yes — `cn_rnd_t` in `src/lib.rs` | 7 lines: `cn_rnd_t` typedef + `next_double` decl |

No C source file is missing from the translation. There is no second module,
no `#ifdef`-gated file, and `CMakeLists.txt` lists exactly `src/lib.c`.

## `nm -D --defined-only` on the C `.so`

Build: `c_src/build/libharvest-work-n9KYBK.so`
(the CMake target name is derived from the parent directory name, so the file
name varies with the checkout directory; the tests glob for it.)

Global (`T`) text symbols, excluding weak/absolute libc/CRT bookkeeping
(`_init`, `_fini`, `__bss_start`, `_edata`, `_end`, `_ITM_*`,
`__gmon_start__`, `__cxa_finalize`, `_Jv_RegisterClasses`):

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `next_double` | `T` (global text) | YES — `#[no_mangle] pub unsafe extern "C" fn next_double` |

`cn_rnd_next` is `static` in C, so it is deliberately NOT exported. The Rust
translation keeps it as a private `fn`, which correctly produces no dynamic
symbol. Exporting it would be an ABI *mismatch*, not a fix.

## `nm -D --defined-only` on the Rust `.so`

Build: `translation/target/release/libnext_double_lib.so`
(`crate-type = ["cdylib"]`, `name = "next_double_lib"`).

| # | symbol | type | in C `.so`? |
|---|--------|------|-------------|
| 1 | `next_double` | `T` (global text) | YES |

Rust additionally emits `rust_eh_personality` / `__rust_*` allocator shims in
some configurations; these are runtime-support symbols with no C counterpart
and are excluded from the diff the same way libc/CRT symbols are on the C side.
With `panic = "abort"` in `[profile.release]` the current build emits none.

## Symbol diff

```
C-only symbols (missing from Rust):   (none)
```

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust.

Verified mechanically by `tests/differential.rs::phase_d_symbol_parity`, which
runs `nm -D` on both `.so` files, filters the libc/CRT/Rust-runtime allowlist,
and asserts the C-only set is empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**. The only build
configuration is the default one, so "every feature combination" is a single
combination. `tools/check_features.sh` enumerates features from `Cargo.toml`
and confirms the set is empty before running the default-configuration tests.
