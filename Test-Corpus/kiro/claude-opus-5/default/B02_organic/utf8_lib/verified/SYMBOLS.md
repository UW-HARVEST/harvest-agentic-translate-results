# SYMBOLS.md — Phase A/D symbol parity

Source of truth: `nm -D --defined-only` on the C `.so`
(`c_src/build/libdriver.so`) vs the Rust `.so`
(`translation/target/release/libdriver.so`).

## C public (dynamic, defined) symbols

The C translation unit `c_src/src/lib.c` defines exactly two non-static
functions. `include/lib.h` only declares `w_utf8_filter`, but `w_utf8_drop`
is also non-static and therefore exported.

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001169 T w_utf8_drop
0000000000001375 T w_utf8_filter
```

## Rust exported symbols

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011770 T w_utf8_drop
0000000000011840 T w_utf8_filter
```

## Parity table

| # | symbol        | declared in    | C `.so` | Rust `.so` | notes |
|---|---------------|----------------|---------|------------|-------|
| 1 | `w_utf8_drop`   | `src/lib.c` (non-static, no header decl) | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 2 | `w_utf8_filter` | `include/lib.h` | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C"` |

**Missing from Rust: 0.**

There are no function-like macros in the C that generate additional symbols;
`valid_1` … `valid_4` and `REPLACEMENT_INC` are preprocessor macros only and
emit no symbols. No other C source files exist under `c_src/` (verified:
`c_src/src/lib.c` is the only `.c` file, `c_src/include/lib.h` the only header),
so no module was skipped by the translation.

## Undefined-symbol check (no dangling non-libc references)

C `.so` undefined: `__assert_fail`, `malloc`, `memcpy`, `realloc`, `strdup`,
`strlen` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`) — all libc.

Rust `.so` undefined: libc (`malloc`, `realloc`, `strdup`, `strlen`, `memcpy`,
`abort`, `free`, `calloc`, `memset`, …), libgcc unwinder (`_Unwind_*`), and
weak glibc symbols. **0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default (empty) feature set. `cargo check
--no-default-features` and the default build are the same compilation.
There is no `[[bin]]` target and no `src/main.rs`, so there is no driver
binary whose stdout could be compared (`crate-type = ["cdylib"]` only, and
`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)`).
