# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libtranslated_rust.so   (cmake, gcc 11.5, x86_64)
Rust: target/debug/libmathop_lib.so       (crate-type = ["cdylib"])
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

```cmake
add_library(${project_name} SHARED src/lib.c)
```

`c_src/` contains only `src/lib.c` (174 lines) and `include/lib.h`
(1 line: `int mathop(int a, int b, int c, int d);`). Every function
definition in `lib.c` has a corresponding `#[no_mangle] extern "C"`
definition in `src/lib.rs`, so **no C module was skipped** by the
translation and no stubbing was required.

There are no `#ifdef`/`#if` configuration branches, no `option()` /
`target_compile_definitions` in CMake, and no `[features]` in
`Cargo.toml` — therefore exactly **one** build configuration exists
(the default / empty feature set).

## Defined dynamic symbols (`nm -D --defined-only`)

| # | symbol | in C `.so` | in Rust `.so` | C signature |
|---|--------|-----------|---------------|-------------|
| 1 | `is_valid_operation` | T | T | `bool(char)` |
| 2 | `get_operation_priority` | T | T | `int(Operation)` |
| 3 | `add_operation` | T | T | `int(int,int,int)` |
| 4 | `multiply_operation` | T | T | `int(int,int,int)` |
| 5 | `subtract_operation` | T | T | `int(int,int,int)` |
| 6 | `divide_operation` | T | T | `int(int,int,int)` |
| 7 | `modulo_operation` | T | T | `int(int,int,int)` |
| 8 | `select_operation` | T | T | `MathOperation(Operation)` |
| 9 | `get_computation_timestamp` | T | T | `time_t(void)` |
| 10 | `allocate_results` | T | T | `ComputationResult*(int)` |
| 11 | `perform_computation_with_history` | T | T | `int(int,int,Operation,ComputationResult**,int*)` |
| 12 | `mathop` | T | T | `int(int,int,int,int)` |

**Symbol diff (C-defined minus Rust-defined): EMPTY (0 missing).**

Verified by `tests/phase_d_symbols.rs`, which shells out to `nm -D` on
both objects at test time and asserts the diff is empty (so the check
cannot silently rot).

No macro-generated symbols exist in the C source (there are no
function-defining macros in `lib.c`).

## Undefined symbols in the Rust `.so`

All undefined (`U`/`w`) symbols in `libmathop_lib.so` resolve to libc
(`libc.so.6`) or the unwinder (`libgcc_s.so.1`); `ldd` shows no unresolved
dependency. The three libc functions the C code itself imports —
`calloc@GLIBC_2.2.5`, `printf@GLIBC_2.2.5`, `time@GLIBC_2.2.5` — are
imported by the Rust object too (it calls the real libc entry points
rather than re-implementing them, so heap ownership and stdio buffering
are shared with the C library).

**0 missing / undefined non-libc symbols.**

## Non-`extern` items in the Rust translation

These are type/constant translations that are intentionally *not*
exported symbols, matching C where they are `typedef`s and enumerators
(compile-time only, no runtime symbol):

`Operation`, `StatusCode`, `ComputationResult`, `MathOperation`,
`time_t`, `OP_ADD`, `OP_MULTIPLY`, `OP_SUBTRACT`, `OP_DIVIDE`,
`OP_MODULO`, `STATUS_SUCCESS`, `STATUS_ERROR`, `STATUS_WARNING`.

The two `static` locals of `mathop` (`computation_history`,
`history_count`) are `static mut` in Rust with no `#[no_mangle]`, which
matches the C `static` storage class (file-local, not exported).

## Verification status (completion gate)

| gate | status | evidence |
|------|--------|----------|
| `nm -D` shows 0 symbols missing from the Rust `.so` | PASS | `diff` of the two sorted symbol lists is empty; re-checked at test time by `d1_every_c_symbol_is_exported_by_rust` |
| 0 undefined non-libc symbols in the Rust `.so` | PASS | `d2_rust_has_no_unresolved_non_libc_symbols`; `ldd` resolves fully |
| Phase B: every `CONFIGS.md` row (C1–C40) passes | PASS | `phase_b_pure.rs`, `phase_b_history.rs`, `phase_b_mathop.rs`, `phase_b_faketime.rs` |
| Phase C: every `ERRORS.md` row (E1–E21, G1–G7) passes | PASS | `phase_c_errors.rs`, `phase_c_mathop_errors.rs` |
| Holds under every feature combination | PASS | 1 combination exists (no `[features]`); run under `--no-default-features` in both `dev` and `release` profiles by `./run_verification.sh` |

50 differential tests, all green. Reproduce with:

```sh
./run_verification.sh    # C build + every feature combo x profile + -O0/-O2/-O3 C cross-checks
./mutation_check.sh      # proves the suite catches 27 injected bugs
```

Note that plain `cargo test` is not sufficient on its own: Cargo does not rebuild
a `cdylib` that the test crates do not link against, so `cargo build` must run
first. The harness enforces this — it refuses to run if `libmathop_lib.so` is
older than `src/lib.rs` rather than silently comparing against stale code.

