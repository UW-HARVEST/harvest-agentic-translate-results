# Verification report

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `c_src/src/lib.c`.

Both implementations are loaded as **shared objects** through `libloading` and
called only through their exported C symbols — the Rust crate is never linked or
called directly, so the `#[no_mangle] extern "C"` export wrappers are themselves
under test.

## How to run

```sh
cd translation
cargo test                # bootstraps both .so files if needed, then runs everything
ci/verify_all.sh          # the full Phase D gate: all profiles x all feature combos
```

A bare `cargo test` works from a clean checkout: the harness builds the C library
with the documented `cmake` invocation and the Rust cdylib (which `cargo test`
does not emit on its own) before loading them.

## Completion gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | all 12 C symbols exported by the Rust `.so`; `ldd -r` clean on both. Enforced by `tests/phase_d_symbols.rs`, not just by hand |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** | 25/25 rows, 28 tests |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | **PASS** | 25/25 rows, 20 tests |
| All of the above under every feature combination | **PASS** | no `[features]` exist, so the axis is a single point; run anyway under `dev`/`release` x default/`--no-default-features` = 4 configurations |

```
== Summary
ALL CONFIGURATIONS VERIFIED
```

## Test inventory (47 checks per configuration, x 4 configurations)

| binary | checks | what it covers |
|--------|--------|----------------|
| `tests/phase_b_configs.rs` | 17 | CONFIGS rows 1-16, 25 — the low-level entry points and the composed pipeline |
| `tests/phase_c_errors.rs` | 17 | ERRORS rows 1-19, 21-22, 25 — including fork-isolated NULL-pointer crashes |
| `tests/phase_stdout.rs` | 11 | CONFIGS rows 17-24 and ERRORS rows 20, 23, 24 — everything that drives `mathop`, with byte-exact stdout comparison |
| `tests/phase_d_symbols.rs` | 2 | symbol parity and unresolved-symbol check |

Randomization is seeded (`0x2545F4914F6CDD1D`, xorshift64\*) so every run is
reproducible. Inputs are biased towards corner values (`0`, `±1`, `INT_MIN`,
`INT_MAX`, ...) rather than drawn uniformly, and `mathop` additionally gets its
full 4-fold corner cross-product (**6561** combinations).

## What was actually wrong, and what was fixed

`src/lib.rs` needed **no logic changes** — the translation was already faithful,
including the subtleties that matter most:

- `wrapping_*` arithmetic matching gcc's signed-overflow wrapping;
- C's truncating `%` (so a negative `param3` produces the out-of-range
  `Operation` values `0, -1, -2, -3`, and `select_operation` falls back to ADD);
- the `default:` arm of `select_operation` for out-of-range enum values;
- `calloc` with a sign-extended negative count returning `NULL`;
- the lazy-allocation branch that *resets* a caller's non-zero `history_count`;
- the silent drop at capacity;
- calling the platform `printf`/`calloc`/`time` so stdio buffering and allocation
  behaviour are identical.

One real divergence was found and fixed:

**NULL-pointer termination signal (`ERRORS.md` rows 17/18).** With
`history == NULL`, the C died with `SIGSEGV` but the Rust `.so` died with
`SIGABRT`, printing `null pointer dereference occurred`. rustc's UB checks —
enabled together with debug assertions — instrument raw-pointer dereferences and
turned the C's unchecked NULL load into a panic. Fixed in `Cargo.toml` by
disabling debug assertions and overflow checks for `profile.dev`, so the dev
artifact behaves like the release artifact and like the C. The suite now runs
against **both** profiles' `.so` to keep it that way.

## Deliberate, disclosed non-match

`divide_operation(INT_MIN, -1, _)` and `modulo_operation(INT_MIN, -1, _)` are
**undefined behaviour in C**: on x86-64 the `idiv` traps and the process is
killed by `SIGFPE`, so the C function returns no value for the Rust to match. The
Rust returns `INT_MIN` / `0` via `wrapping_div` / `wrapping_rem`.

This is the only input class where the two differ, and it differs because the C
has no defined result. `ub_divide_int_min_by_minus_one` runs both sides in forked
children and **pins both observed outcomes** (C: signal 8; Rust: normal exit) so
the divergence is proven rather than ignored, and it asserts that every input
adjacent to the trap (`INT_MIN/1`, `INT_MIN/-2`, `INT_MIN+1/-1`, `INT_MAX/-1`,
`INT_MIN/INT_MIN`, `-1/-1`) still matches exactly. All inputs that reach this
trap are excluded from the randomized batches by `mathop_would_trap()`, which
replicates the C's operation selection to decide whether a given `mathop` call
would reach a trapping `idiv`.

## Notes on test-harness correctness

Two harness bugs were found and fixed while building the suite; both would have
produced false results:

1. **libtest's own stdout pollutes captured regions.** The harness writes
   `test <name> ... ok` to fd 1, which landed inside a `dup2`-captured region of a
   concurrently running test and added a phantom line. All stdout comparisons were
   moved into a `harness = false` binary that runs them sequentially, and the
   capture parser now validates that every captured line is one of the library's
   own four lines, so foreign output can never be silently absorbed again.
2. **Reading a not-yet-allocated history buffer.** `assert_buffers_match` now
   compares allocation state before dereferencing, so a NULL buffer is a reported
   mismatch rather than a segfault in the test process.
