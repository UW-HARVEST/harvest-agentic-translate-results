# VERIFICATION.md — how this translation was verified

The C library is the ground truth. Every test loads **both** shared objects with
`libloading` and calls them **only** through their exported C symbols, so the
`#[no_mangle] extern "C"` wrappers are part of what is tested. No Rust function
is ever called directly.

Because every function in this library returns `void` and communicates purely by
writing to `stdout`, "identical output" is checked by redirecting file
descriptor 1 to a temp file, invoking the symbol, `fflush(NULL)`-ing the shared
libc stream, and comparing the captured bytes **byte-for-byte**.

## Commands

```sh
# C reference library
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# Rust library + full differential suite (the harness rebuilds the cdylib itself)
cargo test --offline

# Every feature combination (enumerated from Cargo.toml, not hard-coded)
./run_all_features.sh

# Proof that the suite actually detects divergence
./mutation_check.sh
```

## Results

| suite | file | cases | result |
|---|---|---|---|
| Phase B — valid paths (`CONFIGS.md` C1–C19) | `tests/valid_paths.rs` | 19 | 19 passed |
| Phase C — error paths (`ERRORS.md` E1–E9, G1–G7) | `tests/error_paths.rs` | 12 | 12 passed |
| Phase D — symbol parity | `tests/symbol_parity.rs` | 5 | 5 passed |
| Feature sweep (`{}`, default, `--all-features`) | `run_all_features.sh` | 3 × suite | all passed |
| Release profile (`panic = "abort"`, optimized) | `cargo test --release` | 36 | all passed |
| Mutation detection | `mutation_check.sh` | 9 mutations | 9/9 detected |

Divergences found in the Rust translation: **none**. `src/lib.rs` is unchanged
(it is a faithful 1:1 translation, including the dead `static helperBad()` that
`bad()` deliberately never calls).

## Two harness bugs found and fixed (both would have made the suite lie)

1. **Stale `.so` — the suite was vacuous.** `cargo test` builds the test
   binaries but does **not** rebuild a `cdylib` lib target, so the tests were
   loading `target/debug/libdriver.so` from an earlier `cargo build`. The first
   `mutation_check.sh` run exposed this: **all 9 deliberately broken
   translations still passed.** Fixed in `tests/common/mod.rs`
   (`ensure_rust_so_built`): the harness now rebuilds the cdylib with the
   matching profile/features (`DIFFTEST_BUILD_ARGS`) and then *asserts* the
   `.so` is newer than `src/`, so a stale artifact fails loudly instead of
   passing silently. The C `.so` gets the same freshness assertion.
2. **libtest polluted the captures.** With the default harness, test threads run
   in parallel and libtest writes its own `test ... ok` lines to fd 1 — the very
   descriptor being captured — which produced a bogus mismatch (`...%%H~\nok`).
   Fixed by `harness = false` plus the sequential `common::Runner`, which reports
   only on stderr, so nothing but the library under test can write to fd 1
   during a capture.

## Row → test mapping

| artifact rows | test |
|---|---|
| C1, C2 | `c1_print_line_empty_string`, `c2_print_line_every_single_byte_value` |
| C3–C7 | `c3…c7_*` (randomized: ASCII, full 0x01–0xFF alphabet, control bytes, `printf` directives, whitespace) |
| C8, C9 | `c8_print_line_boundary_lengths` (1 B … 1 MiB), `c9_print_line_interior_unaligned_pointers` |
| C10 | `c10_print_line_many_calls_in_one_capture` |
| C11–C14 | `c11…c14_*` (`bad`/`good`, single + repeated, dead-helper check) |
| C15, C16 | `c15_driver_end_to_end`, `c16_driver_repeated_no_state_leak` |
| C17, C18 | `c17_random_interleaved_sequences`, `c18_null_interleaved_with_valid_calls` |
| C19 | `c19_flush_ordering_one_capture_vs_many` |
| C20 | `run_all_features.sh` (whole suite per feature combination) |
| E1, G1 | `e1_print_line_null_pointer_writes_nothing` |
| E2, G2 | `e2_print_line_empty_string_is_not_rejected` |
| E3 | `e3_format_directives_are_not_interpreted` |
| E4, G3 | `e4_no_length_limit_oversized_inputs` |
| E5, G4 | `e5_g4_non_ascii_and_control_bytes_pass_through` |
| E6, G7 | `e6_g7_buffered_stdout_ordering_under_rejections` |
| E7, E8, E9 | `e7…e9_*` (exact expected output of `bad`, `good`, `driver`) |
| G5 | `g5_no_enum_or_integer_parameters_exist` (asserts the no-enum/no-int-parameter premise still holds) |
| G6 | `g6_unaligned_and_interior_pointers` |

## Randomization

Fixed-seed xorshift64\* (`common::Rng`, base seed `0x2545F4914F6CDD1D`, per-row
derived seeds) — reproducible across runs while covering hundreds of inputs per
row rather than one hand-picked value.
