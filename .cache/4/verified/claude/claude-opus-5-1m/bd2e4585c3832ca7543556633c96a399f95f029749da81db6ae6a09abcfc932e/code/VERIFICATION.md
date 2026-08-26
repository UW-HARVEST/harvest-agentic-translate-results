# VERIFICATION.md — how to reproduce, and what was found

The C in `c_src/` is the ground truth. Everything below compares the Rust
translation against it and fixes only the Rust.

## Reproduce

```bash
bash scripts/run_all.sh              # everything: stages 0-5, ~13 min
```

Individual stages / shards (each finishes well under 10 minutes):

```bash
bash scripts/run_all.sh 0 1 2        # cargo check matrix, build artifacts, nm -D parity
bash scripts/run_all.sh 3:add        # differential tests, the 8 OP=add configurations
bash scripts/run_all.sh 3:sub
bash scripts/run_all.sh 3:mul
bash scripts/run_all.sh 4:rest       # `OP` unset / `default` spellings
bash scripts/run_all.sh 4:add 4:sub  # alias + `REPEAT` unset spellings
bash scripts/run_all.sh 4:mul
bash scripts/run_all.sh 5            # 648 executable comparisons
bash scripts/run_all.sh 6            # same, for the optimised --release profile
bash scripts/mutation_check.sh       # anti-vacuity check
```

Only one of these may run at a time — they share `target/` and `artifacts/`.

## What each script does

| script | what it does |
|--------|--------------|
| `scripts/combos.sh`          | prints the 24 canonical `(OP, REPEAT)` feature combinations |
| `scripts/check_all.sh`       | `cargo check` for **60** feature configurations (24 canonical + 24 `repeat_N` aliases + 11 "cache variable unset" + `default`) |
| `scripts/build_artifacts.sh` | builds, per configuration, the C `.so` (gcc), the C executable (CMake), the Rust `cdylib` and the Rust executable into `artifacts/<op>_<repeat>/` |
| `scripts/symdiff.sh`         | `nm -D` parity between the C `.so` and the Rust `.so`, all 24 configurations |
| `scripts/diff_bins.sh`       | 27 argument sets × 24 configurations = 648 C-executable vs Rust-executable comparisons |
| `scripts/mutation_check.sh`  | injects 18 behavioural mutations into the Rust and requires each to be caught (anti-vacuity check) |
| `scripts/release_check.sh`    | rebuilds all 24 configurations with `--release` (`panic = "abort"`, optimised) and re-checks symbols and executable output against the C |
| `scripts/run_all.sh`         | all of the above plus the `cargo test` matrix |

A bare `cargo test --no-default-features --features <op>,<n>` also works on a
clean tree: `tests/differential.rs::ensure_artifacts` builds the four artifacts
for the configuration it was compiled with if they are missing.

Tests must run with `--test-threads=1`: comparing printed output requires
temporarily redirecting file descriptor 1, which is process-global.

## Phase A artifacts

* `SYMBOLS.md` — 8 exported symbols, derived from `nm -D`; parity table.
* `ERRORS.md` — 22-row error-surface table, derived by grepping the C for every
  rejection / silent-ignore / build-time refusal.
* `CONFIGS.md` — 33-row configuration-surface table over the axes the C branches
  on (`OP` × `REPEAT` × entry point × input shape).

## Test design

`tests/differential.rs` (41 tests) loads **both** `.so`s with `libloading` and
never calls a Rust function directly, so the `#[no_mangle]` export wrappers are
themselves under test. For each call it compares

1. the returned `int`, and
2. the exact bytes printed to file descriptor 1.

Printed output is captured by `dup2`-ing a scratch file over fd 1 — this catches
the C library's `printf` (which uses this process's libc `stdout`, flushed with
`fflush(NULL)`) and the Rust `cdylib`'s own `std` `Stdout` alike.

The lowest-level entry points (`op_add`/`op_sub`/`op_mul`, and the raw `G_OP` /
`G_OP_NAME` `.data` objects, read *and written*) are driven directly, not only
through `helper_call` / `helper_ptr`. `b16` additionally replays the whole
`mdmain.c` pipeline out of the `.so` exports and checks the reconstruction
against the real C executable's stdout.

Randomization uses a fixed-seed xorshift64\* PRNG (one seed per test), so every
run is reproducible; roughly 60 000 differential calls are made per
configuration.

## Divergences found and fixed

| # | what | how it showed up | fix |
|---|------|------------------|-----|
| 1 | `G_OP` / `G_OP_NAME` were emitted into `.data.rel.ro`, which RELRO makes read-only, while the C (non-`const`) globals live in writable `.data`. A C consumer assigning to `G_OP` — legal, and what `mdmain.c`'s use of a mutable global implies — would have trapped. | `readelf -S` section diff; `b14_g_op_writable_then_call_through` | `#[unsafe(link_section = ".data")]` on both statics in `src/mdcore.rs` |
| 2 | `print!` panics when a write to stdout fails, aborting the process (exit 134) — C's `printf` return value is discarded, so the C exits 0. Reproduced with stdout on `/dev/full`. | `c22_unwritable_stdout` | replaced `print!` with a `c_printf` helper in `src/mdcore.rs` and `src/main.rs` that drops the `io::Result` |
| 3 | The usage message ran `argv[0]` through `to_string_lossy`, so a program path that is not valid UTF-8 came out as U+FFFD replacement characters (23 bytes instead of the C's 19). | `c21_non_utf8_argv` | `src/main.rs` now writes `argv[0]`'s raw bytes to stderr (`usage()`), assembled into one buffer and written with `write_all` |
| 4 | cosmetic: clippy lints in `src/mdmacros.rs`. | `cargo clippy --all-targets` | doc indent fixed; the seven-arm `match` kept (one arm per C `case`) with a targeted `#[allow]` |

No divergence was found in the arithmetic, the `REPEAT` unrolling, the
`DISPATCH_REP` `switch` (including the `REPEAT = 7` asymmetry), `atoi`
emulation, exit statuses, or the symbol table.

## Anti-vacuity evidence

`scripts/mutation_check.sh` injects 18 mutations into the Rust source, rebuilds
the artifacts, and requires the differential suite to fail each time. All 18 are
detected (including re-breaking each of the three fixes listed above):

```
detected: mutation 'dispatch_rep also accepts 7'  (via c04)
detected: mutation 'op_mul uses wrapping_add'  (via b0)
detected: mutation 'op_add saturates'  (via b0)
detected: mutation 'step_op add uses i+1'  (via b0)
detected: mutation 'helper_call drops a space'  (via b05)
detected: mutation 'helper_ptr label typo'  (via b07)
detected: mutation 'gen.acc label typo'  (via c04)
detected: mutation 'G_OP points at op_sub'  (via b1)
detected: mutation 'G_OP_NAME says addd'  (via b15)
detected: mutation 'argc guard off by one'  (via c0)
detected: mutation 'usage goes to stdout'  (via c01)
detected: mutation 'usage message wording'  (via c01)
detected: mutation 'c_printf reports errors'  (via c22)
detected: mutation 'argv0 lossy conversion'  (via c21)
detected: mutation 'exit code 1 instead of 2'  (via c01)
detected: mutation 'atoi ignores sign'  (via b20)
detected: mutation 'atoi no overflow saturation'  (via c14)
detected: mutation 'G_OP no longer in .data'  (via b14)
---
ALL MUTATIONS DETECTED (tests are not vacuous)
```

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols and **0** undefined
      non-libc symbols in the Rust `.so`, in all 24 configurations
      (`scripts/symdiff.sh` → `SYMBOL PARITY OK FOR ALL CONFIGS`).
- [x] Phase B: every row of `CONFIGS.md` passes across randomized inputs.
- [x] Phase C: every row of `ERRORS.md` has a passing error-path differential
      test (rows 1–17, 19–22 executed; row 18 is `NULL`-through-a-function-pointer
      UB that crashes both and is documented instead of executed).
- [x] All of the above under **every** feature configuration: 24 canonical
      `(OP, REPEAT)` combinations plus 36 alternative spellings
      (`repeat_N` aliases, `OP` unset ⇒ `add`, `REPEAT` unset ⇒ `5`, `default`),
      each compared against the C build it must equal.
- [x] `cargo check` clean for all 60 feature configurations;
      `cargo clippy --all-targets` clean.
- [x] 648 C-executable vs Rust-executable output comparisons identical
      (stdout, stderr and exit status).
- [x] The optimised `--release` profile (`panic = "abort"`) builds for all 24
      configurations, exports the identical symbol set, and produces identical
      output — the `wrapping_*` arithmetic means optimisation level cannot change
      the result (`scripts/release_check.sh`).

One deliberate, documented deviation remains: `SIGPIPE` disposition — see the
end of `ERRORS.md`.
