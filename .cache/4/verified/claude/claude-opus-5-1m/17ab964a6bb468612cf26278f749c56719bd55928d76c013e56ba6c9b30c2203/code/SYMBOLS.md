# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/release/libdriver.so
```

## Public (dynamic, defined) symbols of the C `.so`

| # | symbol | C source | exported by Rust `.so`? | notes |
|---|--------|----------|-------------------------|-------|
| 1 | `allocate_matrix`               | `c_src/src/matrix.c` | YES (`src/matrix.rs`) | not declared in `matrix.h`, but non-`static` ⇒ exported |
| 2 | `free_matrix`                   | `c_src/src/matrix.c` | YES (`src/matrix.rs`) | |
| 3 | `initialize_matrix_from_string` | `c_src/src/matrix.c` | YES (`src/matrix.rs`) | |
| 4 | `multiply_matrices`             | `c_src/src/matrix.c` | YES (`src/matrix.rs`) | |
| 5 | `matrix_to_string`              | `c_src/src/matrix.c` | YES (`src/matrix.rs`) | |
| 6 | `write_to_file`                 | `c_src/src/write.c`  | YES (`src/write.rs`)  | |
| 7 | `driver`                        | `c_src/src/driver.c` | YES (`src/driver.rs`) | |

**Missing symbols: 0.** Every C translation unit (`matrix.c`, `write.c`,
`driver.c`) has a Rust counterpart (`src/matrix.rs`, `src/write.rs`,
`src/driver.rs`); no module was skipped, so no C source had to be translated to
close a gap and no stub was added.

`matrix.h` also declares the type `matrix_t` (no symbol); it is mirrored 1:1 as
`#[repr(C)] pub struct matrix_t { matrix: *mut *mut c_int, width: c_int, height:
c_int }` (16 bytes, LP64) in `src/matrix.rs`.

## Undefined (imported) symbols

The C `.so` imports only glibc symbols:

```
__errno_location atoi fclose fopen fprintf free fwrite malloc perror
snprintf stderr strcat strdup strerror strlen strtok_r
(weak) _ITM_deregisterTMCloneTable _ITM_registerTMCloneTable __cxa_finalize __gmon_start__
```

The Rust `.so` imports the same set (it deliberately calls the very same C
runtime entry points through `src/cffi.rs`, so that allocations remain
`free()`-able by the caller and diagnostics interleave identically on the C
`stdio` streams) plus the usual Rust runtime/libc/unwinder imports
(`memcpy`, `memset`, `calloc`, `realloc`, `posix_memalign`, `_Unwind_*`,
`dl_iterate_phdr`, `pthread_key_*`, …).

**0 missing / unresolved non-libc symbols in the Rust `.so`.** Verified with
`ldd -r target/release/libdriver.so` (no "undefined symbol" lines) — every
undefined entry resolves to `libc`/`libgcc_s`.

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** ⇒ exactly one valid feature
  combination: the empty one (`cargo check|test --no-default-features`, which is
  identical to the default build). No `#[cfg(feature = …)]` gating is required
  or possible.
* `c_src/CMakeLists.txt` defines a single target (`add_library(driver SHARED
  src/matrix.c src/write.c src/driver.c)`) with **no options, no
  `target_compile_definitions`, no `#ifdef`s** anywhere in the C sources
  (`grep -c '#if' c_src/src/*.c c_src/include/*.h` ⇒ 0) ⇒ exactly one C build
  configuration.

## Verification result (re-checked by `verify.sh`)

```
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u  >  c.syms
nm -D --defined-only target/{debug,release}/libdriver.so | awk '{print $3}' | sort -u  >  rust.syms
comm -23 c.syms rust.syms      # symbols present in C but MISSING from Rust
```

* dev profile:     `comm -23` output **empty** — all 7 symbols exported.
* release profile: `comm -23` output **empty** — all 7 symbols exported.
* `ldd -r` on both Rust `.so`s: **0** "undefined symbol" lines.
* The reverse direction is empty as well: the Rust `.so` exports exactly these 7
  `T` symbols and nothing else of its own.
