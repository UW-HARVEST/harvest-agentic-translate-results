# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-1GRKLj.so` (built via `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Rust `.so`: `translation/target/release/libarrayfunc_lib.so` (`crate-type = ["cdylib"]`)

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-1GRKLj.so \
  | awk '$2=="T"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libarrayfunc_lib.so \
  | awk '$2=="T"{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # must be empty
```

## Symbol table

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `add_operation`            | T | T | `int (int a, int b, int unused1, int unused2)` | `operation_func` instance |
| 2 | `multiply_operation`       | T | T | `int (int a, int b, int unused1, int unused2)` | `operation_func` instance |
| 3 | `subtract_operation`       | T | T | `int (int a, int b, int unused1, int unused2)` | `operation_func` instance |
| 4 | `modulo_operation`         | T | T | `int (int a, int b, int unused1, int unused2)` | `operation_func` instance; guards `b == 0` |
| 5 | `safe_double_to_int`       | T | T | `int (double d)` | saturating/NaN-clamping conversion |
| 6 | `compute_scaled_value`     | T | T | `int (int base, double scale_factor)` | not called internally; public leaf |
| 7 | `compare_results_in_array` | T | T | `int (ResultArray *arr, int idx1, int idx2)` | pointer-address comparison |
| 8 | `init_result_array`        | T | T | `void (ResultArray *arr, int values[], int count)` | clamps `count` to 10 |
| 9 | `process_with_foreach`     | T | T | `int (ResultArray *arr, operation_func op)` | `FOREACH` macro loop; mutates array |
| 10 | `compute_weighted_sum`    | T | T | `int (ResultArray *arr)` | pointer-difference weight |
| 11 | `arrayfunc`               | T | T | `int (int, int, int, int)` | only symbol declared in `include/lib.h` |

## Result

```
=== MISSING from Rust ===
(empty)
```

**0 symbols missing.** No stubs were required; every C symbol has a real Rust
translation with a `#[no_mangle] extern "C"` export.

## Undefined-symbol audit of the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports
(`malloc`, `memcpy`, `abort`, `_Unwind_*`, `__errno_location`, `pthread_key_*`,
`dl_iterate_phdr`, …). These come from the Rust standard library runtime, not
from untranslated C. **0 missing/undefined non-libc symbols.**

## Types crossing the FFI boundary

Verified layout-compatible (asserted by `tests/differential.rs::layout_*`):

```c
typedef struct { int value; double scaled; int rank; } Result;      // size 24, align 8
typedef struct { Result data[10]; int count; } ResultArray;         // size 248, align 8
typedef int (*operation_func)(int, int, int, int);
```

Offsets: `Result::value` 0, `Result::scaled` 8, `Result::rank` 16;
`ResultArray::data` 0, `ResultArray::count` 240.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `cargo check --no-default-features` and
`cargo test --no-default-features` are therefore equivalent to the default
build; both are exercised by `run_all.sh`.
