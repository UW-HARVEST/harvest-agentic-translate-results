# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-T1HrAZ.so   (name comes from the parent dir name,
#    see CMakeLists.txt: project(${project_name}) via cmake_path(... FILENAME))

# Rust
cd translation && cargo build --release
# -> translation/target/release/libdataentry_lib.so
```

## Defined (exported) dynamic symbols

`nm -D --defined-only <so>`:

| # | C symbol | C type | Exported by Rust `.so`? | Rust item |
|---|----------|--------|-------------------------|-----------|
| 1 | `dataentry` | `T` (global text) | YES — `T dataentry` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn dataentry` |

The C library's only public entry point is `int dataentry(int, int, int, int)`
(declared in `c_src/include/lib.h`). Every other function in `c_src/src/lib.c`
is `static` and therefore has **no** dynamic symbol:

| C function | storage | dynamic symbol? | Rust counterpart (private, not exported — correct) |
|------------|---------|-----------------|-----------------------------------------------------|
| `find_entry` | `static` | no | `unsafe fn find_entry` |
| `process_name` | `static` | no | `unsafe fn process_name` / `process_name_lit` |
| `calculate_lookup` | `static` | no | `unsafe fn calculate_lookup` |
| `create_entries` | `static` | no | `unsafe fn create_entries` |
| `modify_entries` | `static` | no | `unsafe fn modify_entries` |
| `lookup_table` | `static` data | no | `static LOOKUP_TABLE` |

`#define MAX_ENTRIES 10` and `#define NAME_LENGTH 32` are macros — no symbols.
(`MAX_ENTRIES` is dead in the C source; it is mirrored as a dead `const` in Rust.)

## Symbol diff result

```
comm -23 c_syms.txt rust_syms.txt      # in C, missing from Rust
<empty>
```

**0 symbols missing from the Rust `.so`.** No module of the C source was
skipped: `c_src/src/lib.c` is the only translation unit in `CMakeLists.txt`
and all six of its functions plus its static table are present in
`translation/src/lib.rs`. No stubs / `unimplemented!()` were introduced.

## Undefined symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports
(`malloc`, `free`, `strlen`, `memcpy`, `memset`, `mmap64`, `_Unwind_*`,
`pthread_key_*`, …), all of which are satisfied by the system runtime.
**0 missing/undefined non-libc symbols.**

The C `.so` imports `malloc`, `free`, `sprintf`, `strcpy`, `strlen`. The Rust
side imports `malloc`/`free` from libc (so heap behaviour, including
allocation-failure thresholds, is identical) and re-implements `sprintf("%d")`,
`strcpy`, `strlen` inline — byte-for-byte equivalent for the inputs the code
can produce (see `CONFIGS.md` rows 1–14 and `ERRORS.md`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `--no-default-features` is therefore
equivalent to the default build. Verified:

```sh
cargo check                        # ok
cargo check --no-default-features  # ok
```
