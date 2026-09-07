# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-tEO4wk.so` (name comes from the parent
  directory name via `cmake_path(GET parent FILENAME project_name)`).
* Rust `.so`: `translation/target/release/libnormalize_lib.so`
  (`[lib] name = "normalize_lib"`, `crate-type = ["cdylib"]`).

## C `.so` — `nm -D` verbatim

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U memset@GLIBC_2.2.5
0000000000001119 T normalize
                 U sqrtf@GLIBC_2.2.5
```

## Exported (defined, `T`/`D`/`B`) symbol parity table

Only `--defined-only` entries are real exports; `U` = undefined import,
`w` = weak toolchain/CRT hook (not part of the library's API surface).

| # | C symbol | type | present in Rust `.so`? | Rust export site |
|---|----------|------|------------------------|------------------|
| 1 | `normalize` | `T` | YES — `T normalize` | `src/lib.rs`, `#[unsafe(no_mangle)] pub unsafe extern "C" fn normalize` |

`nm -D --defined-only` counts:

| object | defined global symbols |
|--------|------------------------|
| C      | 1 (`normalize`)        |
| Rust   | 1 (`normalize`)        |

**Symbol diff (C defined − Rust defined): EMPTY.**

There are no macro-generated / namespace-renamed symbols: `include/lib.h`
contains a single line (`void normalize(float *dest, const float *src, int size);`)
with no renaming macros, so the linker name equals the source name.

No C source file was left untranslated: `CMakeLists.txt` lists exactly one
source (`src/lib.c`, 18 lines) which defines exactly one function.

## Undefined symbols in the Rust `.so`

All `U`/`w` entries in the Rust `.so` resolve to libc / libgcc-unwind /
pthread (`memset`, `memcpy`, `malloc`, `free`, `abort`, `_Unwind_*`,
`dl_iterate_phdr`, `pthread_key_*`, …). These come from the Rust standard
library's panic/backtrace machinery, not from missing translated code.

**0 missing/undefined non-libc symbols in the Rust `.so`.**

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, therefore the only
build configuration is the default (empty) feature set. `--no-default-features`
and the default build are the same compilation. Verified by
`check_features.sh`.

## No driver binary

`CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there is
no `add_executable`, and the crate declares only `crate-type = ["cdylib"]` with
no `src/main.rs` / `[[bin]]`. So the "compare C and Rust binary stdout"
requirement is **not applicable** (no binary is built by either side).

## Negative control (proof the differential harness detects divergence)

"All tests pass" is only meaningful if the harness can fail. Five mutants of
`src/lib.rs` were built into a scratch crate and the *unmodified* suite was run
against each mutant `.so` via `RUST_SO=…`:

| mutant | change | detected by |
|--------|--------|-------------|
| 1 | `sum > 0.0f32` → `sum >= 0.0f32` | `error_path`: 5 failures (rows 5, 6, 8, 9, 14) |
| 2 | accumulate in `f64`, narrow at the end | `valid_path`: 17 failures |
| 3 | accumulate in descending index order | `valid_path`: 14 failures |
| 4 | `1.0/sqrt(sum)` → `sqrt(1.0/sum)` | `valid_path`: 20 failures |
| 5 | drop the `dest != src` guard | `error_path`: SIGSEGV (row 3/15 reach the huge `memset`) |

5/5 detected. The scratch crate was deleted afterwards; the shipped
`translation/src/lib.rs` is unmodified.

## Machine-checked parity

`tests/symbols.rs` re-derives this table at test time (`nm -D` on both objects)
so the parity claim cannot silently rot:

* `phase_d_every_c_export_is_exported_by_rust` — asserts the C-minus-Rust
  defined-symbol diff is empty.
* `phase_d_no_unresolved_non_libc_symbols_in_rust` — asserts every `U` entry in
  the Rust object is libc / libgcc-unwind / pthread.
