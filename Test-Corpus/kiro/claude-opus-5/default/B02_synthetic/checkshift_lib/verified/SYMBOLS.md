# SYMBOLS.md — Phase A symbol surface

Mechanically derived from:

```
nm -D --defined-only c_src/build/libharvest-work-lrkokh.so
nm -D --defined-only translation/target/release/libcheckshift_lib.so
```

C source translated: `c_src/src/lib.c` (191 lines, the only `.c` file in
`CMakeLists.txt`). Public header: `c_src/include/lib.h` (declares `checkshift`
only; the other nine symbols are non-`static` definitions in `lib.c` and are
therefore exported too).

## Exported symbol parity

| # | C symbol (`nm -D`, type `T`) | in Rust `.so`? | Rust definition |
|---|------------------------------|----------------|-----------------|
| 1 | `add_with_static`    | yes | `#[unsafe(no_mangle)] pub extern "C" fn add_with_static` |
| 2 | `apply_operation`    | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_operation` |
| 3 | `checkshift`         | yes | `#[unsafe(no_mangle)] pub extern "C" fn checkshift` |
| 4 | `compute_checksum`   | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn compute_checksum` |
| 5 | `execute_operation`  | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn execute_operation` |
| 6 | `get_operation`      | yes | `#[unsafe(no_mangle)] pub extern "C" fn get_operation` |
| 7 | `init_state`         | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_state` |
| 8 | `multiply_with_static` | yes | `#[unsafe(no_mangle)] pub extern "C" fn multiply_with_static` |
| 9 | `shift_with_static`  | yes | `#[unsafe(no_mangle)] pub extern "C" fn shift_with_static` |
| 10 | `xor_operation`     | yes | `#[unsafe(no_mangle)] pub extern "C" fn xor_operation` |

`comm -23 c_syms r_syms` → **empty**. 0 missing symbols.

## Non-exported C entities (no symbol expected)

| C entity | kind | note |
|----------|------|------|
| `static int static_multiplier = 3`     | `static` file-scope | internal (`d`/local), mirrored by `STATIC_MULTIPLIER` |
| `static int static_addend = 100`       | `static` file-scope | internal, mirrored by `STATIC_ADDEND` |
| `static int static_shift_amount = 2`   | `static` file-scope | internal, mirrored by `STATIC_SHIFT_AMOUNT` |
| `static operation_func ops[4]`         | function-`static` inside `get_operation` | lazily filled table; behaviourally equivalent to Rust's local array |
| `ComputeState`                         | typedef struct | `#[repr(C)] pub struct ComputeState` (12 bytes, align 4, no padding) |
| `operation_func`                       | function-pointer typedef | `pub type OperationFunc = Option<unsafe extern "C" fn(c_int, c_int) -> c_int>` |
| `STRINGIFY`, `LOG_VALUE`               | macros | expanded inline: `"Variable a = %d\n"`, `"Variable b = %d\n"` |
| `OP_ADD/OP_MULTIPLY/OP_XOR/OP_SHIFT`   | macros | unused by the C code; kept as `const` (dead) |
| `MAGIC_NUMBER`, `MASK_LOWER`           | macros | `const MAGIC_NUMBER`, `const MASK_LOWER` |

No macro-generated exported symbols exist in this library.

## Undefined symbols in the Rust `.so`

All `U`/`w` entries are libc / libgcc-unwind / gmon imports
(`printf`, `malloc`, `free`, `memcpy`, `_Unwind_*`, `__cxa_finalize`, …).
**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, so the only configuration is the default one. Verified with
`cargo metadata` (see `check_features.sh`).

## Binary executable

Neither `CMakeLists.txt` (only `add_library(... SHARED src/lib.c)`) nor
`Cargo.toml` (only `[lib] crate-type = ["cdylib"]`) builds an executable
driver, so the "compare binary stdout" gate is **N/A**. Instead, all
`printf` output of every entry point is captured at the fd level and compared
byte-for-byte (see `tests/common/mod.rs::capture_stdout`).

## How to reproduce

```
cd translation && ./verify.sh
```

`verify.sh` builds the C `.so`, extracts the feature list from `Cargo.toml`
mechanically, then for every (feature combination × profile) pair rebuilds the
Rust `cdylib`, diffs `nm -D`, and runs both differential test suites against
that exact `.so` (via `CHECKSHIFT_RUST_SO`).

Tests must run with `--test-threads=1`: the harness redirects file descriptor 1
process-wide to capture each library's `printf` bytes, so parallel tests would
interleave libtest's own progress output into the captured buffers. This was a
real harness defect during development (a `static Mutex` inside a generic
function is instantiated per monomorphisation and did not serialise).

## Verification results

| gate | result |
|------|--------|
| `nm -D` missing symbols (release + debug) | 0 of 10 |
| `nm -D` undefined non-libc symbols | 0 |
| Phase B — all 36 `CONFIGS.md` rows | pass |
| Phase C — all 20 `ERRORS.md` rows + 4 generic boundary sweeps | pass |
| Binary/driver stdout comparison | N/A (no executable is built); all `printf` output compared at fd level instead |
| Feature combinations | only `default` exists; verified under both `release` and `debug` (overflow checks on) |

Test sensitivity was confirmed by mutation: changing `STATIC_ADDEND` from 100 to
101 fails 16 tests. The source was restored afterwards.

## Divergence found and fixed

One: the release build optimised away `checkshift`'s `malloc` together with its
`state == NULL` failure branch. See the row-20 note in `ERRORS.md`. Fixed in
`src/lib.rs` with `core::hint::black_box`; no change was made to `c_src/`.
