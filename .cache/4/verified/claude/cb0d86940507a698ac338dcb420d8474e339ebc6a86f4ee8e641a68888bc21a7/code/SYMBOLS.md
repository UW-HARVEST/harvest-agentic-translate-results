# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C reference (default configuration, no CMAKE_BUILD_TYPE => -O0)
cd translated_rust/c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# => c_src/build/libtranslated_rust.so

# Rust translation (cdylib)
cd translated_rust && cargo build --release
# => target/release/libtritanopia_lib.so
```

## Build-time configuration enumeration

* `Cargo.toml` has **no `[features]` section** at all → the only valid feature
  combination is the empty/default one. `cargo check --no-default-features`
  and `cargo check` are therefore the complete set (2 invocations, 1 real
  configuration).
* `c_src/CMakeLists.txt` declares **no `option()`, no `add_definitions`, no
  `target_compile_definitions`** and the C sources contain **no
  `#if`/`#ifdef`/`#ifndef`/`defined()`** whatsoever (verified by grep over
  `c_src/src`, `c_src/include`, `c_src/CMakeLists.txt`). There is exactly one
  C build configuration.
* `src/lib.rs` contains **no `#[cfg(feature = ...)]`** — nothing is gated.

Conclusion: **one single configuration**; there is no feature cross-product to
enumerate. (Phases B/C are nevertheless re-run under `--no-default-features`
and under the default features, and against both the `-O0` and an extra `-O2`
build of the C library, to prove the result is not optimisation-dependent.)

## Defined (exported) dynamic symbols

`nm -D --defined-only --extern-only`, code/data symbols only:

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|-----------|-------|
| 1 | `tritanopia` | `T` @ `0x1670` | `T` @ `0x11d60` | the single public entry point declared in `include/lib.h` |

### Mechanical diff

```
C count: 1   RUST count: 1
=== MISSING from Rust (in C, not in Rust) ===   -> (empty)
=== EXTRA in Rust ===                           -> (empty)
```

**Symbol diff is EMPTY in both directions.** ✅

### Why there is only one symbol

`c_src/src/lib.c` is the only translation unit. Everything except
`tritanopia` is declared `static` and therefore has internal linkage, so it is
deliberately *not* exported and must *not* appear in the Rust `.so` either:

| C function | linkage | Rust counterpart | exported? |
|---|---|---|---|
| `tritanopia` | external | `pub extern "C" fn tritanopia` + `#[no_mangle]` | **yes** (both) |
| `cbRemoveGammaRGB` | `static` | `fn cbRemoveGammaRGB` (private) | no (both) |
| `cbNorm` | `static` | `fn cbNorm` (private) | no (both) |
| `cbDenorm` | `static` | `fn cbDenorm` (private) | no (both) |
| `cbApplyGammaRGB` | `static` | `fn cbApplyGammaRGB` (private) | no (both) |
| `Tritanopia` | `static` | `fn Tritanopia` (private) | no (both) |

No C source file was left untranslated: the C library is a single 60-line
`lib.c` plus a 7-line `lib.h`, and every function and both structs in them
have a counterpart in `src/lib.rs`. No stubs and no `unimplemented!()` exist
in the Rust translation (verified by grep).

## Undefined symbols (imports)

The Rust `.so` has **0 missing/undefined non-libc symbols**. Every `U`/`w`
entry resolves against `libc`, `libm`, `libgcc_s` or `ld.so`:

* Shared with the C `.so`: `pow@GLIBC_2.29`, `__cxa_finalize@GLIBC_2.2.5`,
  `_ITM_{de,}registerTMCloneTable`, `__gmon_start__`.
* Rust-runtime-only extras, all libc/libgcc: `_Unwind_*@GCC_*` (libgcc_s),
  `abort`, `bcmp`, `calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`,
  `getcwd`, `getenv`, `gettid`, `lseek64`, `malloc`, `memcpy`, `memmove`,
  `memset`, `mmap64`, `munmap`, `open64`, `posix_memalign`,
  `pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`, `read`,
  `readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
  `write`, `writev`, `__errno_location`, `__tls_get_addr`,
  `__cxa_thread_atexit_impl`. These come from the Rust `std` panic/backtrace
  machinery, not from the translated code.

Critically, **both** objects import the *same* `pow@GLIBC_2.29` from the
*same* `libm.so.6`, so `f64::powf` in Rust and `pow()` in C execute identical
machine code and are bit-for-bit equal. `ldd` on the Rust `.so` confirms
`libm.so.6` is linked.

## ABI surface of the one symbol

```c
cb_rgb_255 tritanopia(cb_rgb_255 RGB);
```

`cb_rgb_255` is `{ unsigned char R, G, B; }` → `sizeof == 3`, `alignof == 1`
(confirmed at runtime via `ctypes.sizeof` == 3). Under the x86-64 SysV ABI a
3-byte aggregate is class INTEGER, so it is **passed in the low 3 bytes of
`RDI`** and **returned in the low 3 bytes of `RAX`**; the remaining 5 bytes of
each register are unspecified. The Rust side must use `#[repr(C)]` (it does)
so the layout and the register class match. The upper-garbage-bytes behaviour
is exercised explicitly by the ABI tests (see `CONFIGS.md` rows 24–25).

## Completion checklist for this file

* [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
* [x] No extra public symbols leak from the Rust `.so`.
* [x] `nm -D` shows 0 missing/undefined non-libc symbols in the Rust `.so`.
* [x] No C source file / module was skipped by the translation.
* [x] No stub / fake / `unimplemented!()` symbol was added to fake parity.

---

## Verification results (final)

Re-checked after the `Cargo.toml` changes (`libloading` dev-dependency and the
added `rlib` crate-type). Reproduce with `./run_all.sh`:

```
=== Symbol parity (nm -D) ===
  [ OK ]   0 symbols missing from Rust .so
  [ OK ]   0 extra public symbols in Rust .so
  [ OK ]   0 undefined non-libc symbols
```

Adding `"rlib"` to `crate-type` does **not** change the cdylib's exported
surface: the diff is still empty in both directions and `tritanopia` remains
the only exported symbol.

### Why `rlib` had to be added

With `crate-type = ["cdylib"]` alone, `cargo test` does **not** rebuild or
re-uplift `target/<profile>/libtritanopia_lib.so`, because an integration test
cannot link a cdylib and therefore does not depend on the lib target. A source
edit was consequently invisible to the whole suite — every differential test
passed against a **stale** `.so`. This was caught by mutation testing (three
deliberate bugs "survived" that obviously should not have) and fixed two ways:

1. `crate-type = ["cdylib", "rlib"]`, so the lib target is a real test
   dependency and gets rebuilt; and
2. `run_all.sh` always runs `cargo build` **before** `cargo test`, and
   `tests/common/mod.rs::assert_not_stale` hard-fails if the `.so` is older
   than the newest file under `src/`.

Point 2 is the load-bearing one: `cargo build` performs the cdylib uplift that
`cargo test` skips.
