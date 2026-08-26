# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

## Source inventory (completeness check)

`c_src/CMakeLists.txt` builds exactly one translation unit into `libdriver.so`:

```cmake
add_library(driver SHARED
    src/driver.c)
```

| C source file | lines | translated to | status |
|---|---|---|---|
| `c_src/src/driver.c` | 50 | `src/lib.rs` | fully translated |
| `c_src/include/driver.h` | 28 | (declarations only) | n/a |

There is **no** untranslated C module: the library consists of a single `.c`
file, and both of its functions are present in `src/lib.rs`.

## Build commands used

```
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cargo build --offline           # -> target/debug/libdriver.so
```

## `nm -D --defined-only` comparison

### C `libdriver.so` (defined, global)

```
0000000000001139 T printLine
000000000000115b T driver
```

### Rust `libdriver.so` (defined, global — project symbols)

```
0000000000012810 T printLine
0000000000012660 T driver
```

### Parity table

| # | C symbol | type | exported by Rust `.so` | Rust item |
|---|----------|------|------------------------|-----------|
| 1 | `printLine` | `T` (text, global) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn printLine` |
| 2 | `driver`    | `T` (text, global) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver` |

**Missing symbols: 0.** The symbol diff is empty.

## Undefined (imported) symbols

The Rust `.so` must not require any non-libc symbol that the C `.so` does not.

| symbol | C `.so` | Rust `.so` | note |
|---|---|---|---|
| `memset@GLIBC_2.2.5` | U | (inlined / `compiler_builtins`) | `ptr::write_bytes` |
| `strncpy@GLIBC_2.2.5` | U | (open-coded `strncpy_c`) | byte loop, same semantics |
| `puts@GLIBC_2.2.5` | U | — | gcc folds `printf("%s\n",p)` → `puts(p)` |
| `printf@GLIBC_2.2.5` | — | U | Rust keeps the literal `printf("%s\n", p)` |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_*` | w (weak) | w / — | CRT bookkeeping, not API |

`puts(p)` and `printf("%s\n", p)` emit byte-identical output on the shared
`stdout` `FILE*` (both write the bytes of `p` followed by `'\n'`); the return
values are discarded because `printLine` returns `void`, so the substitution is
not observable. This is verified by the differential tests in
`tests/differential.rs` (rows C1–C9 of `CONFIGS.md`).

`nm -D` shows **0 missing/undefined non-libc symbols** in the Rust `.so`:

```
$ nm -D --undefined-only target/debug/libdriver.so | grep -v GLIBC | grep -v ' w '
(empty)
```

## Feature combinations

`Cargo.toml` has **no `[features]` section** and `c_src/CMakeLists.txt` defines
no `-D` compile options / `#ifdef` configuration. Therefore there is exactly
**one** valid build configuration:

| # | cargo invocation | C equivalent |
|---|------------------|--------------|
| 1 | `cargo test --offline --no-default-features` (= no features) | default `cmake` build |

`--all-features` is identical to the above because the feature set is empty.
