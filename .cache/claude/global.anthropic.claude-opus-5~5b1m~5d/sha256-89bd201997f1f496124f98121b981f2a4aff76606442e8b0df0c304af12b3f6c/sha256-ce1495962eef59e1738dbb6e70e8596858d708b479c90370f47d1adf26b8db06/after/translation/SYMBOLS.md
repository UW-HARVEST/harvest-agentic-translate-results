# SYMBOLS.md — Phase A symbol surface

C shared library: `c_src/build/libharvest-work-1Ytkcx.so` (built from the single TU `c_src/src/lib.c`).
Rust shared library: `translation/target/release/libarrayfunc_lib.so`.

`c_src/src/lib.c` declares **no** `static` functions, so every function it defines
is part of the exported ABI. `nm -D --defined-only` on the C `.so` yields exactly
11 `T` symbols.

| # | symbol | C signature | in Rust `.so`? |
|---|--------|-------------|----------------|
| 1 | `add_operation`            | `int (int a, int b, int unused1, int unused2)`     | yes |
| 2 | `multiply_operation`       | `int (int a, int b, int unused1, int unused2)`     | yes |
| 3 | `subtract_operation`       | `int (int a, int b, int unused1, int unused2)`     | yes |
| 4 | `modulo_operation`         | `int (int a, int b, int unused1, int unused2)`     | yes |
| 5 | `safe_double_to_int`       | `int (double d)`                                   | yes |
| 6 | `compute_scaled_value`     | `int (int base, double scale_factor)`              | yes |
| 7 | `compare_results_in_array` | `int (ResultArray *arr, int idx1, int idx2)`        | yes |
| 8 | `init_result_array`        | `void (ResultArray *arr, int values[], int count)`  | yes |
| 9 | `process_with_foreach`     | `int (ResultArray *arr, operation_func op)`         | yes |
| 10 | `compute_weighted_sum`    | `int (ResultArray *arr)`                            | yes |
| 11 | `arrayfunc`               | `int (int p1, int p2, int p3, int p4)`               | yes |

Only `arrayfunc` appears in the public header `c_src/include/lib.h`; the other ten
are still linker-visible and therefore in scope for differential testing.

## Types crossing the FFI boundary

```c
typedef int (*operation_func)(int a, int b, int unused1, int unused2);

typedef struct { int value; double scaled; int rank; } Result;      /* sizeof 24, off 0/8/16 */
typedef struct { Result data[10]; int count; } ResultArray;          /* sizeof 248, off 0/240 */
```

Layout confirmed identical on both sides (see `tests/differential.rs::layout_matches`,
which asserts `sizeof`/offsets by round-tripping a byte buffer through both `.so`s).

## Symbol diff

```
$ diff <(nm -D --defined-only libharvest-work-1Ytkcx.so | awk '{print $3}' | sort) \
       <(nm -D --defined-only libarrayfunc_lib.so       | awk '{print $3}' | sort)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`.** No stubs were used; every symbol is
a real translation of the corresponding C function.

Undefined (imported) symbols in the Rust `.so` are libc/`std` only
(`memcpy`, `__cxa_thread_atexit_impl`, unwinder/pthread symbols, …) — no dangling
references to untranslated code.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent). The
Phase D "every feature combination" requirement is satisfied by the single
configuration; `scripts/check_features.sh` enumerates and verifies this
mechanically.
