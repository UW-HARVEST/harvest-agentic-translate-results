# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-XMGpqW.so`
- Rust `.so`: `translation/target/release/liboverunder_lib.so`

## C exported symbols (text/data, libc-independent)

```
$ nm -D --defined-only c_src/build/libharvest-work-XMGpqW.so | grep -v ' [aAwWuUvV] '
T copy_data_block
T handle_pointer_operations
T overunder
T process_with_fallthrough
T safe_double_to_int
```

Note: only `overunder` is declared in `include/lib.h`; the other four have
external linkage in `src/lib.c` (no `static`), so they are part of the ABI
surface and are covered here and in the differential tests.

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `safe_double_to_int`        | T | T | [x] present |
| 2 | `process_with_fallthrough`  | T | T | [x] present |
| 3 | `copy_data_block`           | T | T | [x] present |
| 4 | `handle_pointer_operations` | T | T | [x] present |
| 5 | `overunder`                 | T | T | [x] present |

Missing from Rust: **none**. No module of `c_src` is untranslated
(`src/lib.c` is the only C source file listed in `CMakeLists.txt`).

## Undefined (imported) symbols in the Rust `.so`

All non-libc undefined symbols must be empty. Rust imports only libc/libm:
`printf`, `sqrt`, `memcpy`, plus the usual glibc/`unwind` boilerplate.
The C `.so` imports `printf`, `sqrt`, `strncpy`, `memcpy`. No non-libc
undefined symbols on either side.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
build configuration is the default one. `cargo test --no-default-features`
is therefore equivalent to `cargo test`; both are exercised in Phase D.

## Binary executable

`CMakeLists.txt` builds only `add_library(... SHARED ...)` — there is no
driver executable, and `Cargo.toml` declares only `crate-type = ["cdylib"]`
with no `[[bin]]`. The "compare binary stdout" gate is therefore N/A;
stdout is instead compared by capturing the libraries' `printf` output
through a redirected `fd 1` in the differential tests.

## Verification (run `./verify.sh` from the crate root)

```
=== 1. symbol parity (nm -D --defined-only) ===
C exports:    5
Rust exports: 5
PASS  0 C symbols missing from the Rust .so

=== 2. undefined non-libc symbols in the Rust .so ===
PASS  no unexpected undefined symbols (only libc/libm/CRT)
```

The Rust `.so` additionally imports `putchar` — LLVM lowers the
`printf("\n")` call site to `putchar('\n')`. This is an optimization of an
identical write; the byte-exact stdout comparison in Phase B confirms the
output is unchanged.

No symbol was ever stubbed: all five implementations are real translations of
`c_src/src/lib.c`, which is the only C source file in `CMakeLists.txt`.
