# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Commands:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## Defined (exported) symbols

| symbol | in C `.so` | in Rust `.so` | notes |
|--------|-----------|---------------|-------|
| `driver` | T (yes) | T (yes) | `void driver(int x)` — the only public API (`c_src/include/driver.h`) |

`print_hex` is `static` in `c_src/src/driver.c`, therefore not part of the ABI and
not expected in `nm -D` of either library. It is verified indirectly through
`driver`, which is its only caller.

## Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

Result: **empty** — 0 symbols missing from the Rust `.so`.

The Rust `.so` exports no *extra* non-runtime symbols either (only `driver`).

## Undefined symbols (imports)

C: `printf`, `putchar` (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

Rust: the same `printf` / `putchar` plus the usual Rust std + unwinder /
allocator libc imports (`_Unwind_*`, `malloc`, `memcpy`, `abort`, …). All are
libc / compiler-runtime symbols; there are **0 missing/undefined non-libc
symbols**.

Note the Rust translation deliberately calls the C runtime's `printf` (declared
`extern "C"`) rather than Rust's `println!`, so stdout buffering and byte output
match the C library exactly.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default (empty) feature set:

```
cargo check                              # default
cargo check --no-default-features        # identical (no features exist)
```

Both are exercised; symbol parity holds in both.

## Binary targets

Neither `c_src/CMakeLists.txt` (a single `add_library(driver SHARED ...)`) nor
`translation/Cargo.toml` (`crate-type = ["cdylib"]`, no `[[bin]]`) builds an
executable driver, so the "compare binary stdout" gate is not applicable. The
equivalent coverage is obtained by capturing the library's stdout through the
FFI boundary (see `tests/differential.rs`).
