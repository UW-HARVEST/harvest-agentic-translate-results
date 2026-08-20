# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libtranslated_rust.so        (cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON)
Rust: target/debug/libupdate_frame_header_lib.so  (crate-type = ["cdylib"])
```

## Build-time configuration surface

* `c_src/CMakeLists.txt`: a single `add_library(... SHARED src/lib.c)`. No
  `option()`, no `add_definitions`, no `target_compile_definitions`, no
  `#ifdef`/`#if` anywhere in `src/lib.c` or `include/lib.h`
  (`grep -c '#if\|#ifdef\|#ifndef' c_src/src/lib.c c_src/include/lib.h` → 0).
  => exactly ONE C build configuration.
* `Cargo.toml`: **no `[features]` section at all**, no `default` feature, no
  optional dependencies.
  => the complete set of valid feature combinations is the single empty set:

  | # | feature combination | cargo invocation |
  |---|---------------------|------------------|
  | 1 | *(none — the only one)* | `cargo check/test --no-default-features` |

  `--no-default-features` and the plain default build are therefore the same
  configuration; both are exercised (see `FEATURES.md` results section below).

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `update_frame_header` | `T` (0x10f9) | `T` (0x12430) | **present in both** |

`diff` of the two `--defined-only` name lists is **empty**:

```console
$ diff <(nm -D --defined-only c_src/build/libtranslated_rust.so   | awk '{print $3}' | sort) \
       <(nm -D --defined-only target/debug/libupdate_frame_header_lib.so | awk '{print $3}' | sort)
$ echo $?
0
```

The C `.so` exports exactly one function; `include/lib.h` declares exactly one
function (`void update_frame_header(tflac *t);`). The C translation unit
contains no other function definitions, no global variables, and no
macro-generated symbols, so nothing is absent from the Rust side and no
`#[no_mangle]` wrapper needed to be added and no C source needed to be
translated.

## Weak / undefined symbols

C `.so` weak-undefined set: `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`
— all toolchain/glibc boilerplate.

The Rust `.so` additionally references only libc / libgcc-unwind imports pulled
in by `std` (`malloc`, `free`, `memcpy`, `memset`, `abort`, `_Unwind_*`,
`dl_iterate_phdr`, `pthread_key_*`, …). Every one of these resolves against
`libc.so.6`/`libgcc_s.so.1`, which is confirmed by the fact that
`libloading::Library::new` on the Rust `.so` succeeds in the tests (a missing
non-libc symbol would make `dlopen` fail with `undefined symbol`).

**0 missing symbols. 0 undefined non-libc symbols.**

## Results

| configuration | `cargo check` | `cargo build` | `nm -D` diff vs C | undefined non-libc |
|---------------|---------------|---------------|-------------------|--------------------|
| features `<none>`, profile `debug`   | PASS | PASS | **empty** | **none** |
| features `<none>`, profile `release` | PASS | PASS | **empty** | **none** |

Reproduce with `./verify_all.sh` (enumerates the feature power set out of
`Cargo.toml`, builds the C `.so`, diffs `nm -D` and runs the whole test suite
for every combination x profile).

```
=== Feature enumeration (from Cargo.toml) ===
  declared non-default features: 0 (none)
  feature combinations to verify: 1
    - '<none>'
...
=== RESULT ===
  ALL CONFIGURATIONS VERIFIED
```
