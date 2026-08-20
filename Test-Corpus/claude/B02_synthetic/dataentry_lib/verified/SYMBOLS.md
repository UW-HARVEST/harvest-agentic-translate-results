# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```
# C
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust
cd translated_rust && cargo build            # -> target/debug/libdataentry_lib.so
```

## C `.so` defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `dataentry` | `T` (global text) | YES — `#[unsafe(no_mangle)] pub extern "C" fn dataentry` |

That is the complete list: the C translation unit declares every other function
(`find_entry`, `process_name`, `calculate_lookup`, `create_entries`,
`modify_entries`) and both data objects (`lookup_table`) as `static`, so they
have internal linkage and are not exported. `include/lib.h` likewise declares
only `int dataentry(int a, int b, int c, int d);`.

## Rust `.so` defined dynamic symbols

| # | symbol | type |
|---|--------|------|
| 1 | `dataentry` | `T` |

`nm -D --defined-only target/debug/libdataentry_lib.so | wc -l` == 1.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
         <(nm -D --defined-only target/debug/libdataentry_lib.so | awk '{print $3}' | sort)
```
=> **empty**. 0 symbols exported by the C `.so` are missing from the Rust `.so`.

## Undefined (imported) symbols in the C `.so`

These are libc imports, not part of the exported surface; the Rust translation
reimplements their effects internally (`c_malloc_entries`/`c_free_entries` over
the system allocator, `c_strlen`, `c_strcpy`, `c_strcpy_from_buf`,
`sprintf_entry_name`):

`malloc`, `free`, `strcpy`, `strlen`, `sprintf` (+ weak
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`).

The Rust `.so` has 0 missing/undefined non-libc symbols
(`nm -D -u target/debug/libdataentry_lib.so` resolves entirely against
`libc`/`libgcc_s`/Rust std, which is statically linked into the cdylib).

## Translation completeness

`c_src` contains exactly one source file (`src/lib.c`, 199 lines) and one header
(`include/lib.h`, 1 line). Every C function in it has a corresponding Rust
implementation in `src/lib.rs`:

| C symbol (internal linkage) | Rust counterpart |
|---|---|
| `lookup_table[4][3]` | `static LOOKUP_TABLE: [[c_int; 3]; 4]` |
| `find_entry` | `unsafe fn find_entry` |
| `process_name` | `fn process_name` |
| `calculate_lookup` | `fn calculate_lookup` |
| `create_entries` | `unsafe fn create_entries` |
| `modify_entries` | `unsafe fn modify_entries` |
| `dataentry` | `pub extern "C" fn dataentry` |

No module or file from `c_src` was skipped; nothing is stubbed or
`unimplemented!()`.

## Verified

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}'
dataentry
$ nm -D --defined-only target/release/libdataentry_lib.so | awk '{print $3}'
dataentry
$ comm -23 <(...c...) <(...rust...)      # -> empty
```

`nm -D -u target/release/libdataentry_lib.so` lists only libc
(`malloc`/`free`/`memcpy`/`__errno_location`/...) and libgcc_s unwinder
(`_Unwind_*`) imports — `ldd` resolves everything to `libc.so.6` and
`libgcc_s.so.1`, i.e. **0 missing/undefined non-libc symbols**. Every test
`dlopen`s the library successfully, which additionally proves the imports
resolve at load time.

Symbol parity is re-checked automatically for every feature combination by
`run_all_features.sh`.
