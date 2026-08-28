# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-oyPNjC.so` (built via
  `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
- Rust `.so`: `translation/target/{debug,release}/libfallcalc_lib.so`
  (`crate-type = ["cdylib"]`, lib name `fallcalc_lib`)

## Public header surface

`c_src/include/lib.h` declares exactly one entry point:

```c
int fallcalc(int a, int b, int c, int d);
```

However `c_src/src/lib.c` defines **six** non-`static` functions, so all six get
external linkage and land in the dynamic symbol table. All six are therefore part
of the ABI surface that must be reproduced, not just the one declared in the
header. (This is the "lowest-level entry points, not just the convenience
wrapper" case from Phase A: `fallcalc` is the one-shot wrapper; the other five
are the low-level entry points and are tested directly.)

## Symbol table

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `safe_double_to_int`           | T | T | `int (double)`             | NaN/Inf/range-clamping double->int |
| 2 | `process_array_reverse`        | T | T | `int (int *, int)`        | walks *backwards* from `end` |
| 3 | `switch_fallthrough_calculator`| T | T | `int (int, int)`          | deliberate `switch` fallthrough |
| 4 | `allocate_and_compute`         | T | T | `int (int, double)`       | `malloc`, may return `-1` |
| 5 | `foreach_sum`                  | T | T | `int (int *, int)`        | `FOREACH` macro expansion |
| 6 | `fallcalc`                     | T | T | `int (int, int, int, int)`| the header entry point |

## Verification

`nm -D --defined-only` output, sorted by name:

```
C:    allocate_and_compute fallcalc foreach_sum process_array_reverse
      safe_double_to_int switch_fallthrough_calculator
Rust: allocate_and_compute fallcalc foreach_sum process_array_reverse
      safe_double_to_int switch_fallthrough_calculator
```

**Symbol diff (C minus Rust): EMPTY.** 0 missing symbols, 0 stubs.
No `unimplemented!()`/`todo!()`/fake symbol appears in `translation/src/lib.rs`.

No whole C module was skipped: `c_src` contains exactly `src/lib.c` +
`include/lib.h`, and every function in `src/lib.c` has a real translation in
`translation/src/lib.rs`.

### Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc_s / loader symbols. It deliberately
imports `malloc`/`free` from libc (rather than using Rust's allocator) so
allocation-failure behaviour — including `malloc(0)` and the huge `size_t`
requests produced by negative `int` sizes — matches the C bit-for-bit.

`src/lib.rs` also contains one *private* helper, `c_malloc`, which is **not**
exported (it does not appear in `nm -D`, and the test asserts the Rust `.so`
exports nothing beyond the six C symbols). It exists solely to stop LLVM from
optimizing the constant-size `malloc` in `fallcalc` into a stack allocation,
which would delete the `return -1` allocation-failure branch; see the
"Divergence found and fixed" section of `ERRORS.md`.

```
Rust non-libc undefined symbols: NONE
```

Automated by `tests/symbols.rs::c_and_rust_export_identical_symbol_sets`, which
re-runs `nm -D` on both objects at test time and asserts set equality.
