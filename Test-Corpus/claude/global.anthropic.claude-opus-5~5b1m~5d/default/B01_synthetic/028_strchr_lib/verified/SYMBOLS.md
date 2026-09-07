# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C `.so` exported symbols (`nm -D --defined-only c_src/build/libdriver.so`)

| symbol | type | C declaration | present in Rust `.so`? |
|--------|------|---------------|------------------------|
| `driver` | `T` (text, global) | `void driver(const char *in);` (public, `include/driver.h`) | YES |
| `foo`    | `T` (text, global) | `int foo(const char *in, char c);` (not in header, but **non-`static`**, so it is part of the ABI surface) | YES |

There are no exported data symbols, no macro-generated symbols, no
`#ifdef`-conditional symbols, and no additional translation units — `c_src`
contains exactly one `.c` file (`src/driver.c`) and one header
(`include/driver.h`), both fully translated in `translation/src/lib.rs`.

## Rust `.so` exported symbols

| symbol | Rust item |
|--------|-----------|
| `driver` | `#[no_mangle] pub unsafe extern "C" fn driver(in_: *const c_char)` |
| `foo`    | `#[no_mangle] pub unsafe extern "C" fn foo(in_: *const c_char, c: c_char) -> c_int` |

## Symbol diff

```
C - Rust = {}        (0 missing)
Rust - C = {}        (0 extra)
```

No stubs, no `unimplemented!()`, no untranslated modules. Both C symbols have a
real Rust implementation exported under the exact same name.

## Undefined (imported) symbols

The Rust `.so` imports only libc symbols, matching the C `.so`:

| symbol | used by C | used by Rust |
|--------|-----------|--------------|
| `strchr` | yes (`foo`) | yes (`foo`, called via `extern "C"`) |
| `printf` | yes (`driver`) | yes (`driver`, called via `extern "C"`) |

0 missing / undefined **non-libc** symbols in the Rust `.so`.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, so there is exactly one build configuration
(`--no-default-features` is equivalent to the default build). Phases B and C
therefore cover 100% of the feature space; this is verified by
`check_features.sh`.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)`. There is no
`add_executable`, so the project produces **no driver binary** and the
"compare C and Rust stdout of the binary" clause is not applicable. Note that
`driver()` itself writes to stdout, and that stdout **is** compared
byte-for-byte in Phase B by redirecting fd 1 to a temp file around each call.
