# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-LVd8bU.so
nm -D --defined-only translation/target/release/libfallcalc_lib.so
```

The C library is built from a single translation unit (`c_src/src/lib.c`); every
non-`static` function in that file becomes a dynamic symbol. `c_src/include/lib.h`
only declares `fallcalc`, but the other five functions are also exported because
none of them are marked `static`. All six must therefore be exported by Rust.

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | C definition site | Rust definition site |
|---|--------|---------|-----------|-------------------|----------------------|
| 1 | `safe_double_to_int`           | T | T | `c_src/src/lib.c:48`  | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` |
| 2 | `process_array_reverse`        | T | T | `c_src/src/lib.c:67`  | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 3 | `switch_fallthrough_calculator`| T | T | `c_src/src/lib.c:79`  | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` |
| 4 | `allocate_and_compute`         | T | T | `c_src/src/lib.c:102` | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` |
| 5 | `foreach_sum`                  | T | T | `c_src/src/lib.c:126` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 6 | `fallcalc`                     | T | T | `c_src/src/lib.c:137` | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C"` |

**Missing from Rust `.so`: 0.**
**Extra in Rust `.so`: 0.**

There are no macro-generated symbols. `FOREACH`, `OCTAL_MASK_1`, `OCTAL_MASK_2`,
`OCTAL_FLAG`, `OCTAL_BASE` are object-like / statement macros and produce no
symbols. `DataPoint` is a `typedef struct` with no associated symbol.

## Undefined (imported) symbols

C imports only `malloc@GLIBC_2.2.5` and `free@GLIBC_2.2.5` (plus the standard
weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__` stubs).

Rust imports the same `malloc`/`free` (bound via `unsafe extern "C"` so the two
libraries share the exact same allocator, which matters for allocation-failure
parity) plus the Rust runtime's libc/`_Unwind_*` dependencies
(`memcpy`, `abort`, `dl_iterate_phdr`, `_Unwind_Resume`, …).

**Undefined non-libc / non-runtime symbols in Rust: 0.** Every `U` entry in the
Rust `.so` resolves against `libc.so.6` / `libgcc_s.so.1`, which is confirmed by
the fact that `libloading::Library::new` succeeds (`RTLD_NOW`) in every test.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default one (`--no-default-features` and the
default build are identical). `enumerate_features.sh` re-derives this
mechanically; Phase D therefore has exactly one combination to cover.

The crate declares `crate-type = ["cdylib"]` only — **no binary target**, and
`c_src/CMakeLists.txt` declares only `add_library(... SHARED ...)` — **no C
driver executable**. The "compare binaries' stdout" clause is therefore vacuous
for this project.
