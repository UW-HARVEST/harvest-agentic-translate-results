# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-l7Jx3E.so
nm -D --defined-only translation/target/release/liboverunder_lib.so
```

The C library is built from exactly one translation unit (`c_src/src/lib.c`,
per `c_src/CMakeLists.txt`). `c_src/include/lib.h` declares only `overunder`,
but the four helper functions in `lib.c` are not `static`, so they are all
dynamic symbols and all part of the ABI surface that must be matched.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | C signature | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `safe_double_to_int`       | T | T | `int (double)`                              | non-static helper; not in `lib.h` |
| 2 | `process_with_fallthrough` | T | T | `int (int, int)`                             | non-static helper; not in `lib.h` |
| 3 | `copy_data_block`          | T | T | `void (DataBlock *, const DataBlock *)`      | non-static helper; `DataBlock` is TU-local |
| 4 | `handle_pointer_operations`| T | T | `int (int)`                                  | non-static helper; not in `lib.h` |
| 5 | `overunder`                | T | T | `int (int, int, int, int)`                    | the only symbol declared in `lib.h` |

Symbols exported by C but missing from Rust: **0**
Symbols exported by Rust but missing from C: **0** (Rust exports no extra
non-libc, non-toolchain globals)

No macro-generated symbols exist: `MAKE_VAR_NAME` and `PRINT_VAR` expand only
inside function bodies (local variable names / string literals), so they create
no linker-visible names.

## Undefined (imported) symbols

Every undefined symbol in the Rust `.so` is libc or toolchain runtime
(`_Unwind_*`, `__cxa_*`, `malloc`, `memcpy`, `printf`, `putchar`, ...).
**0 missing/undefined non-libc symbols.**

The C `.so` imports `sqrt`, `strncpy` and `memcpy`; the Rust `.so` imports
`memcpy` (it calls libc `memcpy` deliberately — see below) but not `sqrt` or
`strncpy`, because LLVM lowers `sqrt` to the `sqrtsd` instruction
(bit-identical, IEEE-754 correctly rounded) and inlines the fixed-length
`strncpy("Source", 19)` into stores. These are code-generation differences, not
ABI differences — no exported symbol is affected, and the differential suite
confirms identical results including the printed `label`.

## Fix applied during verification

`copy_data_block` originally used `std::ptr::copy_nonoverlapping`. That is
value-identical to C for valid pointers, but `copy_nonoverlapping` carries
debug-profile precondition assertions that convert a NULL argument into a Rust
panic (SIGABRT), whereas the C `memcpy` faults with SIGSEGV — a divergence in
the *fault mode* that only appears in the `dev` profile. Both copies in the
translation now call libc `memcpy` directly, matching the C source
instruction-for-instruction and matching its fault mode in **both** profiles.
Verified by `err12_copy_data_block_has_no_null_check`, which forks a child per
implementation and compares the termination signal.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. There are no additional
`--no-default-features --features <combo>` permutations to verify; the
automation script `check_features.sh` enumerates the `[features]` table
mechanically, confirms it is empty, and then runs cargo check + build + symbol
diff + the full differential suite for the single resulting configuration.

Because the C library is built at `-O0` (no `CMAKE_BUILD_TYPE`) while the Rust
`cdylib` ships at `-O3`, the suite is additionally run against the `dev`-profile
Rust `.so` (`RUST_SO=target/debug/liboverunder_lib.so cargo test --release`),
which enables Rust's arithmetic-overflow checks and disables LLVM optimization.
Both profiles pass all 53 tests.
