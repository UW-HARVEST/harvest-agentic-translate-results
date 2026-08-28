# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

* C   `.so`: `c_src/build/libharvest-work-e7KbSx.so`
* Rust `.so`: `translation/target/release/libarrayfunc_lib.so`

`c_src/src/lib.c` declares **no** `static` functions, so every function defined in
the translation unit is part of the shared library's public ABI (11 total), even
though `c_src/include/lib.h` only advertises `arrayfunc`. All 11 must be exported
by the Rust `.so` under the exact same linker name.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | C signature |
|---|--------|---------|------------|-------------|
| 1 | `add_operation`            | T | T | `int add_operation(int a, int b, int unused1, int unused2)` |
| 2 | `multiply_operation`       | T | T | `int multiply_operation(int a, int b, int unused1, int unused2)` |
| 3 | `subtract_operation`       | T | T | `int subtract_operation(int a, int b, int unused1, int unused2)` |
| 4 | `modulo_operation`         | T | T | `int modulo_operation(int a, int b, int unused1, int unused2)` |
| 5 | `safe_double_to_int`       | T | T | `int safe_double_to_int(double d)` |
| 6 | `compute_scaled_value`     | T | T | `int compute_scaled_value(int base, double scale_factor)` |
| 7 | `compare_results_in_array` | T | T | `int compare_results_in_array(ResultArray *arr, int idx1, int idx2)` |
| 8 | `init_result_array`        | T | T | `void init_result_array(ResultArray *arr, int values[], int count)` |
| 9 | `process_with_foreach`     | T | T | `int process_with_foreach(ResultArray *arr, operation_func op)` |
| 10 | `compute_weighted_sum`    | T | T | `int compute_weighted_sum(ResultArray *arr)` |
| 11 | `arrayfunc`               | T | T | `int arrayfunc(int param1, int param2, int param3, int param4)` |

## Symbol diff

```
$ comm -23 c_syms.txt rust_syms.txt      # in C but not in Rust
<empty>
```

**0 missing symbols.** No C module/file was skipped by the translation: `src/lib.c`
is the only C source file listed in `CMakeLists.txt`, and all 11 of its functions
have real (non-stub) Rust implementations in `translation/src/lib.rs`.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc runtime
imports — there are **0 missing/undefined non-libc symbols**:

* glibc: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `getenv`, `getcwd`, `readlink`,
  `realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`,
  `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`, `gettid`, `dl_iterate_phdr`,
  `__errno_location`, `__tls_get_addr`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `pthread_key_create`, `pthread_key_delete`,
  `pthread_setspecific`
* libgcc unwinder: `_Unwind_*`
* weak toolchain hooks: `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__gmon_start__`

`ldd` resolves everything against `libgcc_s.so.1` and `libc.so.6` only.

## Types crossing the ABI boundary

Verified layout-identical (`#[repr(C)]`, x86-64 SysV):

| type | C layout | Rust layout |
|------|----------|-------------|
| `Result` | size 24, align 8 — `value`@0, pad@4, `scaled`@8, `rank`@16, pad@20 | identical |
| `ResultArray` | size 248, align 8 — `data`@0 (10×24=240), `count`@240, pad@244 | identical |
| `operation_func` | `int (*)(int,int,int,int)` — 8-byte pointer | `Option<unsafe extern "C" fn(...)>` (null-pointer-optimized) |

## Automated enforcement

Symbol parity is not a one-off manual check; it is re-asserted by the test suite
and by the driver scripts:

| check | where |
|-------|-------|
| every C symbol is exported by the Rust `.so` (debug) | `phase_d_symbols::d1_every_c_symbol_is_exported_by_rust` |
| every C symbol is exported by the Rust `.so` (release) | `phase_d_symbols::d1b_release_so_also_has_full_symbol_parity` |
| the C `.so`'s export set still matches this document | `phase_d_symbols::d2_expected_symbol_list_matches_the_c_library` |
| all 11 symbols resolve via `dlsym`, at 11 *distinct* addresses (no aliasing / stubbing one impl onto another) | `phase_d_symbols::d3_every_symbol_is_dlsym_resolvable_in_both` |
| no undefined non-libc symbols | `phase_d_symbols::d4_rust_so_has_no_undefined_non_libc_symbols` |
| `Result` / `ResultArray` size, alignment and field offsets agree across the ABI | `phase_d_symbols::d5_struct_layout_matches_across_the_abi` |
| `comm -23` symbol diff is empty in both profiles | `run_all.sh` step 4 |
| symbol diff is empty for every feature combination | `check_features.sh` |

Every differential test calls **both** libraries exclusively through
`dlopen`/`dlsym`, so the `#[no_mangle]` export wrappers are themselves under test
— no Rust function is ever called directly.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the only
configuration is the default (empty) feature set. There is likewise no `#ifdef`
in the C source. Phases B–D therefore have exactly one feature combination to
cover.

This is enforced rather than assumed:

* `check_features.sh` reads the feature list from `cargo metadata`, generates the
  power set, and runs the full suite per combination in both profiles. Today that
  is `default`, `--no-default-features` and `--all-features` — 6 configurations
  including profiles, all passing 84 tests.
* `phase_d_symbols::d6_feature_space_is_still_the_single_default_combination`
  fails the moment a `[features]` table or a `cfg(feature = …)` appears, so a new
  configuration cannot silently escape the matrix.
* `phase_d_symbols::d7_c_source_has_no_conditional_compilation` asserts the C
  side has no `#if`/`#ifdef`/`#ifndef`, i.e. no build-time C configuration axis
  that the Rust would have to mirror.
