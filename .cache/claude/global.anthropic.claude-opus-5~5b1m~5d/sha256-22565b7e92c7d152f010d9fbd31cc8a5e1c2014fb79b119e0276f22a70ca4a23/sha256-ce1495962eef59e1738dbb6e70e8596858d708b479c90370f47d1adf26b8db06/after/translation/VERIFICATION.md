# Verification report

Differential verification of `translation/` (Rust) against `c_src/` (C, the
ground truth). Every call in every test goes through the exported symbols of a
`dlopen`ed shared object — the C `.so` **and** the Rust `.so` — so the
`#[no_mangle] extern "C"` wrappers are themselves under test. No Rust function
is ever called directly.

## How to reproduce

```sh
# 1. build the C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. run the whole suite (80 tests)
cd ../../translation && cargo build && cargo test

# 3. run the full matrix: every feature combo × dev/release × C at -O0..-Os
./tests/feature_matrix.sh
```

## Completion gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / undefined non-libc symbols in Rust | **PASS** | symbol diff is empty; asserted by `tests/symbol_parity.rs` (3 tests) rather than by hand |
| Phase B: every row of `CONFIGS.md` passes across randomized inputs | **PASS** | 44/44 rows ↔ 44 `cfg_NN` tests in `tests/differential.rs` |
| Phase C: every row of `ERRORS.md` has a passing error-path differential test | **PASS** | 27/27 rows ↔ 27 `err_NN` tests in `tests/error_paths.rs`, plus 2 generic boundary sweeps |
| All of the above under every feature combination | **PASS** | `Cargo.toml` declares **no** `[features]` (asserted mechanically by `phase_d_cargo_toml_declares_no_features`); `tests/feature_matrix.sh` still runs default / `--no-default-features` / `--all-features` × dev / release = 6 configurations, all green |

Row↔test coverage is checked mechanically, not by eye:

```
CONFIGS.md rows: 44  ->  cfg_ tests: 44   MATCH
ERRORS.md  rows: 27  ->  err_ tests: 27   MATCH
unchecked "[ ]" boxes left in the artifacts: 0
```

## Test inventory (80 tests)

| file | tests | purpose |
|------|-------|---------|
| `tests/differential.rs` | 44 | Phase B — one test per `CONFIGS.md` row, randomized (fixed seed) |
| `tests/error_paths.rs` | 29 | Phase C — one test per `ERRORS.md` row + generic FFI boundary sweeps |
| `tests/symbol_parity.rs` | 4 | Phase D — `nm -D` diff, `ldd -r`, dlsym callability, feature-table assertion |
| `tests/smoke.rs` | 3 | harness self-check + `struct ConfigFlags` layout probe |
| `tests/common/mod.rs` | — | shared harness (loading, env control, fd capture, forked crash tests, RNG) |

Each comparison checks **three** observables, not just the return value:
the `int` result, the exact bytes written to `stdout`, and the exact bytes
written to `stderr` (captured by `dup2`-ing fds 1 and 2 onto scratch files, since
the library prints through libc `printf`/`fprintf`). Tests that pass a
`struct ConfigFlags*` additionally compare all four resulting struct bytes.

## Divergence found and fixed

**Null / misaligned `struct ConfigFlags*` aborted instead of faulting.**

`ERRORS.md` rows 11–13 record that the C validates no pointer at all —
`init_config_from_env`, `perform_operation` and `apply_bit_operations` all
dereference `flags` unchecked, so a null pointer dies with `SIGSEGV`. The Rust
translation reached the bit-fields through a reference (`&mut *flags`,
`&*flags`). rustc instruments every reference deref under `-C debug-assertions`
(on by default in the dev profile) with a null check *and* an alignment check;
the resulting panic, unwinding out of an `extern "C"` function, aborts the
process. Measured through the FFI boundary:

| input | C | Rust (before) | Rust (after) |
|-------|---|---------------|--------------|
| `init_config_from_env(NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) | `SIGSEGV` (11) |
| `perform_operation(_, _, NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) | `SIGSEGV` (11) |
| `apply_bit_operations(_, NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) | `SIGSEGV` (11) |
| misaligned `ConfigFlags*` | works | `SIGABRT` (6) | works, same results |

Fix (`src/lib.rs`): the bit-fields are now reached through `bf_load` / `bf_store`,
which move byte 0 with libc `memcpy` on the raw pointer and never form a Rust
reference. Verified by disassembly that this is the only one of the three
candidate spellings that emits no instrumentation:

| access | codegen under `-C debug-assertions` |
|--------|--------------------------------------|
| `(&*p).storage[0]` | `and $0x3` (align check) + null check → panic → abort |
| `*(p as *const u8)` | null check → panic → abort |
| `memcpy(&mut b, p, 1)` | **no checks** — plain byte move, faults like C |

This also makes the dev and release profiles behave identically, rather than the
crash semantics depending on whether debug assertions are enabled.

## Harness bugs found (not translation bugs)

These produced false results before being fixed, and are worth recording because
each one would otherwise have inflated confidence:

1. **`cargo test` does not rebuild a `cdylib`.** It builds the crate as an rlib
   for the test harness only, so `target/debug/libenvy_lib.so` stayed at a
   pre-edit timestamp and the first several runs silently verified a **stale**
   Rust `.so`. `rust_so_path()` now shells out to `cargo build --lib` for the
   active profile and then asserts the `.so` is not older than `src/lib.rs`.
2. **libtest's progress output corrupted the captures.** With more than one test
   thread, libtest writes `test foo ... ok` to the real fd 1 from the harness
   thread while a test body has fd 1 redirected onto a capture file, so those
   bytes landed inside the "C stdout" buffer and 4 tests failed spuriously. The
   library reads the process environment and writes to process-global fds, so
   serial execution is inherent: `.cargo/config.toml` sets
   `RUST_TEST_THREADS = "1"` and `assert_serial()` rejects an explicit
   `--test-threads=N > 1` with a clear message instead of producing garbage.
3. `setenv("")` is `EINVAL`, so an empty variable name can never be *present*;
   the empty-name case only has an "absent" state.

## Confirmed-equivalent behaviours worth noting

The C does several things that look like bugs but are the specification; all are
replicated and pinned by tests:

* `PROG_OPTIMIZE=""` **enables** optimization — the C tests only `!= NULL`
  (`err_25`).
* `PROG_VERBOSE` / `PROG_DEBUG` use `strchr(v, '1')`, so `"0"`/`"true"`/`"yes"`
  are false while `"31337"` and `"x1"` are true (`err_26`, `err_27`).
* `parse_env_numeric` returns `default_val` for a *missing* variable but
  `atoi("") == 0` for a *present but empty* one (`err_02`).
* An unparseable value is **not** rejected — it falls through to `atoi`, so
  `PROG_BASE_OFFSET=abc` yields `0`, not the `0100` default (`err_06`).
* `atoi` is decimal, so the string `"0100"` is `100`, while the *source literal*
  `0100` used as the default is `64` (`cfg_09`).
* The comma check short-circuits the semicolon check, so a value containing both
  only ever produces the "Invalid character" warning (`err_05`).
* On `result < 0` the roll-back returns `param1` **without re-checking**, so
  `envy` can return a negative number (`err_15`).
* `param4 >> 2` is an arithmetic shift and every arithmetic expression wraps on
  overflow (`err_18`, `err_19`).
* Bytes 1..3 of the `struct ConfigFlags` allocation unit are never read and must
  be preserved by the bit-field writes (`err_23`), and all 256 byte-0 patterns
  behave identically even though `init_config_from_env` can only ever produce
  `0x38` / `0x39` / `0x3A` / … (`err_24`).

## Robustness beyond the required matrix

* The C reference was rebuilt at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` and the
  full suite re-run against each (gcc may widen a bit-field byte
  read-modify-write at higher levels, and `CMakeLists.txt` pins no
  `CMAKE_BUILD_TYPE`). All five pass — see step 4 of `tests/feature_matrix.sh`.
  These are out-of-source builds under `target/`; **nothing in `c_src/` is
  modified**.
* `envy`'s uninitialised C locals (`state`, `state_backup`, `buffer`) are zeroed
  in Rust. This is unobservable and the tests confirm it: every field and every
  buffer byte the C later reads is unconditionally written first
  (`init_config_from_env` writes all six bit-fields, the three scalar members are
  assigned, and `snprintf` NUL-terminates the buffer), so only never-read padding
  differs.
