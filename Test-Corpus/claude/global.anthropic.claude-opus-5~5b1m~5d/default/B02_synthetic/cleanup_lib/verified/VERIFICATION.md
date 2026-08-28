# VERIFICATION.md — completion gate

## How to reproduce everything

```sh
cd translation
tests/run_all.sh            # rebuilds the C .so, then every feature combo x {debug, release}
```

The script builds `c_src` with cmake, enumerates the feature powerset from
`Cargo.toml`, runs `cargo check --all-targets` / `cargo build` / `cargo test` for
each combination in both profiles, and finally diffs `nm -D` between the two
`.so`s.

Individual phases:

```sh
cargo test --test phase_b_configs   # Phase B — CONFIGS.md rows
cargo test --test phase_c_errors    # Phase C — ERRORS.md rows
cargo test --test phase_d_symbols   # Phase D — symbol parity / link completeness
```

## Test architecture

* Both implementations are loaded with `libloading` as shared objects and called
  **only** through their exported C symbols (`cleanup`, `print_result`,
  `cleanup_resources`), so the `#[unsafe(no_mangle)]` wrappers are themselves
  under test. No Rust function is ever called directly.
* Two observable channels are compared for every single call: the returned `int`
  **and** the exact bytes written to fd 1. `Capture` in `tests/common/mod.rs`
  redirects fd 1 to a scratch file and `fflush(NULL)`es around every call, so the
  comparison sees the real stdio byte stream both libraries share.
* The test targets use `harness = false` with a small sequential runner that
  reports on **stderr**; libtest's own progress output goes to fd 1 and would
  otherwise be interleaved into the captured bytes (this was observed and is why
  the harness was replaced).
* Randomised rows use SplitMix64 with the fixed seed `0x5EED_C0FF_EE00_1234`, so
  failures reproduce exactly.
* `tests/support/fault_shim.c` is compiled at test time and `LD_PRELOAD`ed into a
  re-exec of the test binary. It interposes `malloc`, `free` and `strncmp` ahead
  of libc, which reaches **both** `.so`s identically, and is what makes the two
  otherwise-unreachable `goto cleanup` branches (and the allocator side effects)
  differentially testable. Fault injection is armed only around the individual
  FFI call.

## Fix applied to the Rust during verification

`src/lib.rs` — the `strncmp("VALID", "VALID", strlen("VALID"))` operands are now
routed through `core::hint::black_box`. Previously LLVM proved both operands were
the same constant and folded the call away in the release profile, so the release
`.so` did not import `strncmp` at all while the C `.so` always calls it. The
comparison result is identical either way, but the call is observable: without it
the C takes its validation-failure branch under `strncmp` interposition and the
Rust does not (`ERRORS.md` row 1). Confirmed by negative control.

No other divergence was found: every `CONFIGS.md` and `ERRORS.md` row passed
against the C on the first run, including the switch fall-through
(`10 → +30`, `30 → +70`), the `TO_STRING(numbers)` stringisation (the literal
text `numbers`, not the array contents), two's-complement wrap of the
accumulator, and glibc's `(null)` rendering for a `NULL` `%s`.

## Negative controls (proof the suite is not vacuous)

| perturbation of the Rust | detected by |
|--------------------------|-------------|
| `case 10` fall-through removed (`+10` instead of `+30`) | 7 Phase C rows + 13 Phase B rows, in **both** profiles |
| `black_box` removed (release folds the `strncmp` call) | `phase_c_errors::row01_strncmp_validation_failure` (release) |
| `free` in `cleanup_resources` replaced by a no-op (leak) | `phase_c_errors::row0304_free_accounting` |

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` lists 3 exported symbols
      (`cleanup`, `print_result`, `cleanup_resources`); the Rust `.so` exports all
      3 under the exact same names. `comm -23 c.syms r.syms` is **empty** in both
      the debug and the release profile, asserted mechanically by
      `phase_d_symbols::symbol_parity_c_subset_of_rust`. `ldd -r` reports **0**
      unresolved imports; every remaining undefined symbol resolves out of
      libc/libgcc. Nothing was stubbed — every exported symbol is called for real
      by `every_symbol_is_dlsym_resolvable_and_live`. The C consists of a single
      translation unit, all of which is translated, so no module was skipped.
- [x] **Phase B** — all 28 `CONFIGS.md` rows pass (25 test functions plus row 27
      inside `child_row02_malloc_fail` and row 28 driven by `run_all.sh`),
      including the exhaustive 5⁴ switch-class cross product, the exhaustive
      8⁴ near-boundary sweep, and ≥ 6 000 seeded random inputs per randomised row.
- [x] **Phase C** — all 15 `ERRORS.md` rows have a passing differential test that
      asserts the *same* result, not merely that both failed: both `goto cleanup`
      branches are reached by fault injection and matched on message **and**
      return value, plus null pointers, zero/oversized lengths, one-step-past
      boundaries around every `case` label, `INT_MIN`/`INT_MAX`, and the
      out-of-range-variant class for the implicit `{10,20,30,40}` enum.
- [x] **Every configuration** — `tests/run_all.sh` reports
      `ALL CONFIGURATIONS PASSED` for 6 units: {`<default>`,
      `--no-default-features`, `--all-features`} × {`debug`, `release`}. The crate
      declares no `[features]`, so that powerset is complete; the two profiles are
      kept because they generate materially different code (`printf`→`puts`
      rewrite, constant folding).
