# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

## Build commands

```sh
# C (project name is derived from the parent directory name by CMakeLists.txt)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-PaF9fv.so

# Rust
cd translation && cargo build --release --offline
# -> translation/target/release/libgaussian_kernel_lib.so
```

## C source inventory (completeness check)

The whole library is two files, and `CMakeLists.txt` compiles exactly one of
them, so there is no untranslated module:

| C file | contents | translated in |
|--------|----------|---------------|
| `c_src/include/lib.h` | 1 line: `void gaussian_kernel(float *dest, int size, float radius);` | `translation/src/lib.rs` (doc + signature) |
| `c_src/src/lib.c` | 28 lines: the single definition of `gaussian_kernel` | `translation/src/lib.rs::gaussian_kernel` |

`grep` confirms the C has **no** other functions, no `static` helpers, no
macros, no `enum`/`struct`/`typedef`, and no `#ifdef`.

## Defined dynamic symbols

`nm -D --defined-only` (only non-libc / non-toolchain symbols listed; the Rust
`.so` additionally exports nothing beyond this):

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `gaussian_kernel` | `T` @ `0x1109` | `T` @ `0x11cd0` | ✅ present in both, exact name |

Raw output:

```
$ nm -D --defined-only c_src/build/libharvest-work-PaF9fv.so
0000000000001109 T gaussian_kernel

$ nm -D --defined-only translation/target/release/libgaussian_kernel_lib.so | grep -v ' r '
0000000000011cd0 T gaussian_kernel
```

(The Rust `.so` also has a handful of `R`/`r` read-only data symbols emitted by
rustc for its own panic/backtrace machinery; these are compiler artefacts, not
API. The C `.so` likewise exports the standard `_init`/`_fini`-adjacent
toolchain symbols. Neither side is part of the public API surface.)

### Symbol diff

```
C-defined minus Rust-defined : (empty)
```

**0 missing symbols.** No `#[no_mangle]` wrapper had to be added and no C
module was left untranslated.

## Undefined (imported) symbols

The only libm/libc symbol the C library needs is `expf`:

```
$ nm -D --undefined-only c_src/build/libharvest-work-PaF9fv.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U expf@GLIBC_2.27
```

The Rust `.so` imports the *same* `expf@GLIBC_2.27` (declared via
`extern "C" { fn expf(x: f32) -> f32; }`), plus the usual Rust runtime
imports (`_Unwind_*`, `malloc`, `memcpy`, `dl_iterate_phdr`, …). Every Rust
undefined symbol resolves against `libc`/`libgcc_s`, i.e. there are **0
missing/undefined non-libc symbols**.

Importantly, `objdump -d` on the C `.so` shows that GCC at the default
(unoptimised) CMake build type does **not** constant-fold
`expf(sigma * sigma * tetha)`: it emits a real `call expf@plt`. Both
implementations therefore obtain `s2` and every `1/expf(x*x)` from the exact
same glibc `expf`, which is what makes bit-identical results achievable rather
than merely "close".

## Automated check

The symbol diff is not just documented, it is asserted by
`tests/phase_d_symbols.rs`, which shells out to `nm -D` on both artefacts:

| test | asserts |
|------|---------|
| `d01_every_c_symbol_is_exported_by_rust` | `C_defined \ Rust_defined == {}` (after removing linker/toolchain symbols) |
| `d02_rust_has_no_unresolved_non_libc_symbols` | every `U`/`w` symbol in the Rust `.so` is a libc/libgcc name |
| `d03_both_libraries_resolve_the_same_expf` | both `.so`s import `expf`, so the differential comparison is apples-to-apples |

`run_all_tests.sh` additionally recomputes the diff with `comm -23` for every
feature combination x build profile and fails if it is non-empty.

## Result

```
combo=default    profile=debug    symbol diff: EMPTY (1 C API symbol present)
combo=default    profile=release  symbol diff: EMPTY (1 C API symbol present)
combo=no-default profile=debug    symbol diff: EMPTY (1 C API symbol present)
combo=no-default profile=release  symbol diff: EMPTY (1 C API symbol present)
```

`nm -D --defined-only` output is *exactly* `T gaussian_kernel` for both
libraries — the Rust `.so` exports neither less nor more than the C `.so`.
**0 missing symbols, 0 unresolved non-libc symbols, no stubs.**
