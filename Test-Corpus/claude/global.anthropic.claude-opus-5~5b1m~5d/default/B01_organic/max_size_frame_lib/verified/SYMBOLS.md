# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects. Nothing here is
assumed; every row is a line of `nm -D` output.

## Build commands used

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-jQRRdi.so
#    (CMakeLists.txt derives the project/library name from the PARENT directory
#     name via cmake_path(GET ... FILENAME), so the C .so filename is
#     environment-dependent; the tests glob for `lib*.so` instead of hardcoding.)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libmax_size_frame_lib.so   ([lib] name = "max_size_frame_lib")
```

## Header surface (`c_src/include/lib.h`, 5 lines, complete)

```c
#include <stdint.h>
typedef uint32_t tflac_u32;
tflac_u32 max_size_frame(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth);
```

There are **no** function-renaming / namespacing macros, no `#define`d symbol
prefixes, and no macro-generated function families in the header or the
`.c` file (verified by `grep -nE '#(if|ifdef|ifndef|else|elif|define)'` →
no matches). Therefore the exported linker symbol is exactly `max_size_frame`
and the total public symbol count is 1.

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `max_size_frame` | `T` (0x10f9) | `T` (0x11c40) | translated in `src/lib.rs`, exported via `#[unsafe(no_mangle)] pub extern "C"` |

**Missing from Rust `.so`: NONE.** No stubs, no `unimplemented!()`, no
whole-module gaps: `c_src/src/lib.c` is 10 lines and contains exactly one
function definition, which is fully translated.

Rust exports no *extra* public symbols (the only `T` in `nm -D --defined-only`
is `max_size_frame`), so the ABI surface matches in both directions.

## Undefined (imported) symbols

C `.so` imports only weak toolchain hooks:

```
w _ITM_deregisterTMCloneTable   w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5    w __gmon_start__
```

Rust `.so` imports the same weak hooks plus libc / libgcc-unwind runtime
symbols pulled in by `std` (`malloc`, `memcpy`, `free`, `abort`, `_Unwind_*`,
`dl_iterate_phdr`, `pthread_key_create`, …). Every one of these is a
**libc/compiler-runtime** symbol resolved by the dynamic loader.

> Completion criterion: **0 missing/undefined non-libc symbols in Rust.** ✅
> (checked automatically by `tests/symbols.rs::phase_d_symbol_parity`)

## Verification result

```
$ nm -D --defined-only <C .so>   -> max_size_frame
$ nm -D --defined-only <Rust .so> -> max_size_frame
symbol diff (C \ Rust): EMPTY   (0 missing)
symbol diff (Rust \ C): EMPTY   (0 extra)
Rust undefined non-libc symbols: none
```

Enforced automatically by `tests/symbols.rs` (3 tests) and by `verify.sh`.

## Feature combinations

`cargo metadata` reports `features: {}` — the crate declares **no** Cargo
features, so `default`, `--no-default-features`, and `--all-features` are the
same single configuration. There are likewise no `#[cfg]` attributes in
`src/lib.rs` and no `#ifdef`s in the C. Phases B–C are nevertheless re-run
under all three flag settings by `run_all_features.sh`.
