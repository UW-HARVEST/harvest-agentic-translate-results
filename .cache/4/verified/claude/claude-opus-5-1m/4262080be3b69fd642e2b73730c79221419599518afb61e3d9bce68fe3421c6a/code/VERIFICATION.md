# VERIFICATION.md — completion gate

Reference: `SYMBOLS.md` (symbol surface), `ERRORS.md` (error surface),
`CONFIGS.md` (configuration surface).

Reproduce everything with:

```sh
./run_all.sh          # C build + every feature combo: check, build, nm diff, differential tests
```

## Completion checklist

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | C `.so` exports `driver`, `run`; Rust `.so` exports both. `comm -23` of the defined-symbol sets is empty; enforced per configuration by `run_all.sh` and by the `sym_nm_dynamic_symbol_sets_are_equal` case. Undefined symbols in the Rust `.so` are libc + `_Unwind_*` only. |
| Phase B: every row of `CONFIGS.md` (C0–C20) passes across randomized inputs | **PASS** | 21 cases, fixed seed `0x5EED_1234_ABCD_0001`, ≈8 000 differential calls per run (single calls from fresh instances, 300–2 000-step accumulating sequences, mixed interleavings, raw bit-pattern matrix, property sweep, process-exit flush parity). |
| Phase C: every row of `ERRORS.md` has a passing error-path differential test | **PASS** | Reachable rows E1–E8 + E14 have cases; E9–E13 are documented *not reachable* with the source evidence (no pointer/length/enum/return-value parameters, `floors` overflow needs 2^31 calls). |
| All of the above under every feature combination | **PASS** | `Cargo.toml` declares no `[features]`, so the power set is one combination. `run_all.sh` verifies it as `--no-default-features` **and** as default, plus both in the `--release` profile (a genuinely different build: optimized, `panic = "abort"`). 4 configurations × 32 cases, all green. |

## Test-suite integrity (why the green is meaningful)

* **Mutation-validated.** Baseline green; 8 independent mutations of
  `src/driver.rs` each detected (see `CONFIGS.md` §4). No mutation slipped
  through, and the C source was never modified.
* **Stale-artifact trap found and closed.** `cargo test` does not rebuild
  `cdylib` artifacts — the first mutation round "passed" 31/31 against a stale
  `.so`. `assert_not_stale` in `tests/differential.rs` now fails the run when the
  loaded `.so` is older than its sources (verified: it fires), and `run_all.sh`
  always builds before testing.
* **Deterministic.** The differential target sets `harness = false` and runs its
  cases sequentially, because capturing libc output requires redirecting the
  process-wide fds 1/2. Repeated runs, `-- --test-threads=8` and name filters all
  give identical results.
* **Rust is only ever exercised through its `.so`.** Every call goes through
  `libloading` + `dlsym` on the `cdylib`, so the `#[no_mangle] extern "C"`
  wrappers and the exported ABI are what is tested — never a direct Rust call.

## Behavioural notes confirmed against the C

* `run` is exported by the C `.so` although it is absent from `driver.h`; it is
  part of the ABI and is exercised directly as the lowest-level entry point.
* The library's state is the file-scope `static house_t the_house`, so output is
  history-dependent; the Rust singleton reproduces that, including independence
  between separately loaded instances.
* `bedrooms += extra_bedrooms` overflows for extreme arguments. The C (`-O0` and
  `-O2`) wraps two's-complement; the Rust `wrapping_add` matches byte-for-byte.
  Nothing is "fixed": no clamping, no panic.
* Output is produced by libc `printf` in both implementations, so `%d`/`%.1f`
  formatting and flush-at-exit semantics match (row C20 tests the latter with no
  explicit `fflush`).

## Files added by this verification

| file | purpose |
|------|---------|
| `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md` | Phase A surface maps |
| `VERIFICATION.md` | this completion gate |
| `tests/differential.rs` | 32 differential cases (C `.so` vs Rust `.so` via `libloading`) |
| `run_all.sh` | builds the C reference, enumerates feature combinations, runs check/build/`nm` diff/tests for each |
| `Cargo.toml` | added `[dev-dependencies] libloading` and the `harness = false` test target |

`c_src/` was not modified; `c_src/build/` contains only cmake output.
