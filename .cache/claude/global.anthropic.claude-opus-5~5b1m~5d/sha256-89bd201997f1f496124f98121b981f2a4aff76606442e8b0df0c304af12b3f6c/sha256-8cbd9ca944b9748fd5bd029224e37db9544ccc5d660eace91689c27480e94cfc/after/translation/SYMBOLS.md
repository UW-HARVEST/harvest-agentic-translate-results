# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C `.so`
(`c_src/build/libharvest-work-ULEAUu.so`) vs. the Rust `.so`
(`translation/target/release/libarrayfunc_lib.so`).

The C translation unit (`c_src/src/lib.c`) declares **no** `static` functions,
so every function it defines has external linkage and is part of the shared
library ABI. `include/lib.h` only advertises `arrayfunc`, but the other ten are
equally callable and are therefore all verified.

| # | C symbol | type | C signature | exported by Rust `.so`? | Rust item |
|---|----------|------|-------------|-------------------------|-----------|
| 1 | `add_operation` | T | `int (int a, int b, int, int)` | yes | `#[no_mangle] pub extern "C" fn add_operation` |
| 2 | `multiply_operation` | T | `int (int a, int b, int, int)` | yes | `#[no_mangle] pub extern "C" fn multiply_operation` |
| 3 | `subtract_operation` | T | `int (int a, int b, int, int)` | yes | `#[no_mangle] pub extern "C" fn subtract_operation` |
| 4 | `modulo_operation` | T | `int (int a, int b, int, int)` | yes | `#[no_mangle] pub extern "C" fn modulo_operation` |
| 5 | `safe_double_to_int` | T | `int (double d)` | yes | `#[no_mangle] pub extern "C" fn safe_double_to_int` |
| 6 | `compute_scaled_value` | T | `int (int base, double scale_factor)` | yes | `#[no_mangle] pub extern "C" fn compute_scaled_value` |
| 7 | `compare_results_in_array` | T | `int (ResultArray*, int idx1, int idx2)` | yes | `#[no_mangle] pub unsafe extern "C" fn compare_results_in_array` |
| 8 | `init_result_array` | T | `void (ResultArray*, int values[], int count)` | yes | `#[no_mangle] pub unsafe extern "C" fn init_result_array` |
| 9 | `process_with_foreach` | T | `int (ResultArray*, operation_func op)` | yes | `#[no_mangle] pub unsafe extern "C" fn process_with_foreach` |
| 10 | `compute_weighted_sum` | T | `int (ResultArray*)` | yes | `#[no_mangle] pub unsafe extern "C" fn compute_weighted_sum` |
| 11 | `arrayfunc` | T | `int (int, int, int, int)` | yes | `#[no_mangle] pub extern "C" fn arrayfunc` |

## Symbol diff

```
$ diff <(c symbols) <(rust symbols)
(empty)
```

**0 symbols missing from the Rust `.so`. 0 extra non-libc symbols.**
No module of the C source was skipped: `src/lib.c` is the only C source file in
`CMakeLists.txt`, and all 11 of its external functions are translated and
exported.

## ABI / layout parity (asserted by `tests/layout.rs` style checks in the diff tests)

| C type | layout | Rust type |
|--------|--------|-----------|
| `Result { int value; double scaled; int rank; }` | size 24, align 8, offsets 0 / 8 / 16 | `#[repr(C)] struct Result` — identical |
| `ResultArray { Result data[10]; int count; }` | size 248, align 8, offsets 0 / 240 | `#[repr(C)] struct ResultArray` — identical |
| `int (*operation_func)(int,int,int,int)` | pointer, size 8 | `Option<unsafe extern "C" fn(...)->c_int>` (null-pointer-optimized) |

## Undefined (imported) symbols in the Rust `.so`

All are libc / `libgcc` unwinder / glibc-pthread imports pulled in by the Rust
runtime — `memcpy`, `memset`, `memmove`, `bcmp`, `malloc`/`calloc`/`realloc`/
`free`/`posix_memalign`, `abort`, `getenv`, `getcwd`, `readlink`, `realpath`,
`open64`/`read`/`write`/`writev`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`,
`mmap64`/`munmap`, `dl_iterate_phdr`, `syscall`, `gettid`, `strlen`,
`__errno_location`, `__tls_get_addr`, `pthread_key_*`, `pthread_setspecific`,
`_Unwind_*`, and the weak `__cxa_finalize` / `__gmon_start__` /
`_ITM_*TMCloneTable` stubs.

**0 unresolved project symbols** — none of the 11 functions above appears in
`nm -D --undefined-only`, i.e. every one is genuinely defined in the Rust `.so`,
not forwarded to something else. Asserted by
`tests/phase_d_symbols.rs::phase_d_all_symbols_resolve_at_dlopen`.

Automated gate: `tests/phase_d_symbols.rs::phase_d_every_c_symbol_is_exported_by_rust`
recomputes this whole diff with `nm -D` at test time and fails on any missing
*or* extra symbol.
