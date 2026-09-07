# SYMBOLS.md — public symbol parity

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-PvTb82.so

cd translation && cargo build --release
# -> translation/target/release/libdataentry_lib.so
```

## C `.so` exported (defined) dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-PvTb82.so
00000000000013fb T dataentry
```

Total: **1** symbol.

## Rust `.so` exported (defined) dynamic symbols

```
$ nm -D --defined-only translation/target/release/libdataentry_lib.so
0000000000012360 T dataentry
```

Total: **1** symbol.

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|-----------|---------------|--------|
| 1 | `dataentry` | yes (`T`) | yes (`T`) | MATCH |

**Missing from Rust: none. Symbol diff is EMPTY.**

## Why the surface is exactly one symbol

`c_src/src/lib.c` is the only translation unit (`CMakeLists.txt` lists
`src/lib.c` alone). Every other function in it has internal linkage:

| C function | linkage | translated in Rust as |
|---|---|---|
| `dataentry` | extern (public, declared in `include/lib.h`) | `#[unsafe(no_mangle)] pub extern "C" fn dataentry` |
| `find_entry` | `static` | private `fn find_entry` |
| `process_name` | `static` | private `fn process_name` |
| `calculate_lookup` | `static` | private `fn calculate_lookup` |
| `create_entries` | `static` | private `fn create_entries` |
| `modify_entries` | `static` | private `fn modify_entries` |
| `lookup_table` | `static` data | private `static LOOKUP_TABLE` |

`include/lib.h` declares exactly `int dataentry(int a, int b, int c, int d);`,
confirming the intended public surface. No C module was left untranslated; no
symbol is stubbed.

## Undefined (imported) symbols

Rust `.so` undefined non-libc symbols: none (checked with
`nm -D --undefined-only`; all entries resolve to `libc`/`libgcc`
`GLIBC_*`/`GCC_*` versioned imports).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `--no-default-features` and the default build
are the same compilation, and both produce the symbol set above. (Verified by
`cargo build --release --no-default-features`.)
