# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-8bakVR.so
nm -D --defined-only translation/target/release/libarrayfunc_lib.so
```

The whole C library is a single translation unit (`c_src/src/lib.c`, 185 lines).
No C source file was skipped by the translation: `translation/src/lib.rs`
contains a counterpart for every function defined in `lib.c`, including the
static-like helpers that C leaves with external linkage (no `static` keyword is
used anywhere in `lib.c`, so *every* function is a dynamic symbol).

## Exported (T) symbol parity

| # | C symbol | C signature (from `lib.c`) | in Rust `.so` | Rust item |
|---|----------|----------------------------|---------------|-----------|
| 1 | `add_operation` | `int add_operation(int,int,int,int)` | yes | `add_operation` |
| 2 | `multiply_operation` | `int multiply_operation(int,int,int,int)` | yes | `multiply_operation` |
| 3 | `subtract_operation` | `int subtract_operation(int,int,int,int)` | yes | `subtract_operation` |
| 4 | `modulo_operation` | `int modulo_operation(int,int,int,int)` | yes | `modulo_operation` |
| 5 | `safe_double_to_int` | `int safe_double_to_int(double)` | yes | `safe_double_to_int` |
| 6 | `compute_scaled_value` | `int compute_scaled_value(int,double)` | yes | `compute_scaled_value` |
| 7 | `compare_results_in_array` | `int compare_results_in_array(ResultArray*,int,int)` | yes | `compare_results_in_array` |
| 8 | `init_result_array` | `void init_result_array(ResultArray*,int[],int)` | yes | `init_result_array` |
| 9 | `process_with_foreach` | `int process_with_foreach(ResultArray*,operation_func)` | yes | `process_with_foreach` |
| 10 | `compute_weighted_sum` | `int compute_weighted_sum(ResultArray*)` | yes | `compute_weighted_sum` |
| 11 | `arrayfunc` | `int arrayfunc(int,int,int,int)` | yes | `arrayfunc` (only symbol in `include/lib.h`) |

Macro-generated symbols: none. `FOREACH` is a statement-level macro that expands
inside `process_with_foreach`; it produces no symbol of its own.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/*.so   | awk '$2=="T"{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/*.so | awk '$2=="T"{print $3}' | sort)
(empty)
```

**MISSING FROM RUST: 0.** No `#[no_mangle]` wrapper had to be added and no C
module had to be translated — the diff was already empty on first measurement
(reproduced by `translation/tests/symbol_parity.rs`, which recomputes this diff
at test time rather than trusting this file).

## Undefined (U/w) symbols in the Rust `.so`

All undefined symbols are libc / libgcc-unwind imports, i.e. 0 missing
non-libc symbols:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `_Unwind_*`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `__errno_location`,
`__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`, `readlink`,
`realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`, `write`,
`writev`.

The C `.so` imports a strict subset (`_ITM_*`, `__cxa_finalize`,
`__gmon_start__`); the extra Rust imports come from `std`'s allocator and
panic/backtrace machinery, not from untranslated code.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent to the
default). `c_src/src/lib.c` contains no `#ifdef` other than none at all — there
is no conditional compilation on either side. Phase D's "every feature
combination" therefore collapses to the single default configuration, which is
verified explicitly by `translation/run_verification.sh`, which enumerates the
`[features]` table from `Cargo.toml` and loops over the cross-product (finding
none, it runs `default` and `--no-default-features`) x {dev, release} profiles.

## ABI-relevant types (not symbols, but part of the surface)

| C type | Rust counterpart | size/align checked by |
|--------|------------------|-----------------------|
| `Result { int value; double scaled; int rank; }` | `#[repr(C)] struct Result` | `tests/phase_b_array.rs::c32_struct_layout_matches_c` (24 bytes, align 8, offsets 0/8/16) |
| `ResultArray { Result data[10]; int count; }` | `#[repr(C)] struct ResultArray` | `tests/phase_b_array.rs::c32_struct_layout_matches_c` (248 bytes, align 8, `count` at offset 240) |
| `int (*operation_func)(int,int,int,int)` | `extern "C" fn(c_int,c_int,c_int,c_int) -> c_int` | used as a C callback in `tests/*` |
