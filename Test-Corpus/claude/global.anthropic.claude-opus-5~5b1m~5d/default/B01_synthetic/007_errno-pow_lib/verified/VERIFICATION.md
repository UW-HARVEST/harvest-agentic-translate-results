# Verification report

Differential verification of the Rust translation of `c_src` (`my_pow`) against
the C original. Both libraries are loaded as shared objects with `libloading`
and driven only through their exported `my_pow` symbol — the Rust code is never
called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

## Result

**The translation is correct.** No divergence from the C was found. No change to
`src/lib.rs` was necessary: `md5(src/lib.rs) = 718c167d9b429b01600200a3bc988dca`
is the translation as received.

| gate | status |
|------|--------|
| `cargo check` clean | PASS (no errors, no warnings) |
| `SYMBOLS.md`: `nm -D` diff empty, 0 unresolved non-libc symbols | PASS |
| Phase B: every `CONFIGS.md` row (C1–C25) passes over randomized inputs | PASS |
| Phase C: every `ERRORS.md` row (E1–E8) + boundary rows (B1–B7) | PASS |
| Phase D: symbol parity + all feature combinations | PASS |
| Negative control: 21/21 killable mutants detected, 0 blind spots | PASS |

45 tests × 6 build configurations (debug/release × default / `--no-default-features`
/ `--all-features`). `Cargo.toml` declares no `[features]`, so those three sets
are the same build; they are run anyway so a future feature is covered
automatically.

Reproduce everything:

```sh
bash translation/run_tests.sh        # all configurations
bash translation/.verif/mutate.sh    # negative control
```

## What made this library's behaviour subtle

`my_pow` looks trivial but its observable output is *three* things, not one:

1. the returned `double` — compared **bit-for-bit** (`to_bits()`), not with `==`,
   so `NaN` payloads and the `+0.0` / `-0.0` distinction are not glossed over;
2. the `-1.0` sentinel returned on the `EDOM` / `ERANGE` branches — note the C
   returns a *valid double* as its error indicator, so `my_pow(-1.0, 1.0)`
   legitimately returns `-1.0` too and the sentinel is not self-identifying;
3. the diagnostic written to `stderr` — captured by `dup2`-ing fd 2 to a temp
   file and compared **byte-for-byte**. This matters more than it looks:
   `fprintf("%.2f", 1e300)` expands to ~309 integer digits, and `%.2f` of
   `1e-300` prints as `0.00`, so the message text is genuinely
   value-dependent.

Plus the `errno` side effect, covered by `bnd_errno_side_effect_parity`.

Behaviours faithfully preserved that a "cleanup" would break — all confirmed
against the C, never assumed:

* `pow(NaN, 0.0)` returns `1.0` with `errno == 0`, so the C does **not** reject
  it. Same for `pow(1.0, NaN)` and `pow(-1.0, ±Inf)`.
* Underflow to `0.0` sets `ERANGE`, so `my_pow(1e-300, 2.0)` is *rejected* and
  returns `-1.0` even though `0.0` is a perfectly good answer.
* `errno = 0` at entry means a stale `errno` from an earlier call must not cause
  a spurious rejection.

## Two pitfalls that would have made this verification worthless

Both were found and fixed; both are now impossible to reintroduce silently.

### 1. `cargo test` does not rebuild a `cdylib`

The integration tests `dlopen` the `.so` rather than linking it, so nothing in
the test graph depends on the library target and cargo leaves a **stale** `.so`
in `target/`. The first full run of the suite passed against a `.so` built
before the tests existed.

This was caught by mutation testing: the initial run reported *every* mutation
as "not caught". A suite that validates a stale artifact passes no matter how
broken the source is.

Fixes:

* `tests/common/mod.rs::assert_fresh` refuses to run if the `.so` is older than
  `src/*.rs` or `Cargo.toml`, and says how to rebuild;
* `run_tests.sh` always runs `cargo build` before `cargo test`.

### 2. Process-global `stderr` races across parallel tests

`my_pow` reports errors via `stderr`, so both silencing and capturing it mean
`dup2`-ing fd 2 — a process-wide side effect, while libtest runs tests on
parallel threads. The first attempt used an `RwLock` that let many "readers"
redirect at once; captured buffers then contained other tests' diagnostics, and
a restoring guard could hand fd 2 back to the wrong target. Six error tests
failed with messages from unrelated rows.

Fix: a single `Mutex` (`STDERR_MUTEX`) *owned by the guard*, so fd 2 is only ever
redirected by one thread at a time. Guards are non-reentrant by design and no
call site nests them. Value tests also collect divergences and only panic
*after* dropping the guard — panicking inside a `/dev/null` redirect would
discard the failure message.

## Negative control (why the PASS is meaningful)

Passing tests only mean something if they can fail. `.verif/mutate.sh` injects
23 plausible translation bugs and requires the suite to catch each one:

* value semantics: swapped arguments, `base.abs()`, negated result
* sentinel: `-1.0` → `1.0` / `-0.0` / `NaN`, and only one of the two branches
* `errno`: no reset, wrong/swapped `EDOM`/`ERANGE` constants, dropped branches,
  reading `errno` too late
* diagnostics: message typos, `%.3f` / `%g` instead of `%.2f`, swapped varargs,
  wrong format string for the branch, dropped trailing newline

**21 killable mutants, 21 detected, 0 blind spots.** The script also verifies its
own restore with `cmp` before each mutation — an earlier version restored from
`$TMPDIR`, which moves between invocations, so mutations silently *stacked* and
the results were meaningless.

Two mutants survive, and both are **semantically equivalent**, not gaps:

| mutant | why no test can distinguish it |
|--------|-------------------------------|
| `base.powf(exponent)` instead of `libm_pow` | rustc lowers `f64::powf` to a call to the *same* `pow@GLIBC_2.29`. Verified: `nm -D --undefined-only` shows `U pow@GLIBC_2.29` and `objdump -d --disassemble=my_pow` shows `call <pow@GLIBC_2.29>`. There is no difference to observe. |
| `err == EDOM` → `err != 0 && err != ERANGE` | the two predicates differ only when `errno ∉ {0, EDOM, ERANGE}`. `.verif/errno_probe.c` fuzzes 8,000,000 `pow()` calls over the full 2^128 bit space plus integral exponents across the over/underflow band and observes **only** `{0, 33 EDOM, 34 ERANGE}`. Nothing runs between `errno = 0` and the read, so the differing case is unreachable. Pinned by `bnd_errno_side_effect_parity`. |

## Test inventory

| file | tests | scope |
|------|-------|-------|
| `tests/common/mod.rs` | — | harness: dual `dlopen`, staleness guard, fd-2 mutex, seeded xorshift64\* PRNG, special-value corpus |
| `tests/phase_b_configs.rs` | 25 | one test per `CONFIGS.md` row C1–C25 |
| `tests/phase_c_errors.rs` | 16 | one test per `ERRORS.md` row E1–E8, plus boundary rows B1–B7 and `errno` parity |
| `tests/phase_d_symbols.rs` | 4 | `nm -D` symbol parity, import parity, `dlsym` callability |

Randomized rows use a fixed seed (`SEED = 0x5EED_1234_ABCD_0001`, each row
deriving its own stream) so any failure is reproducible. Row C22 alone fuzzes
200,000 pairs drawn from the entire `2^128` input space, mixing NaNs,
infinities, subnormals and every magnitude — the same call is also checked with
`errno` pre-poisoned to a hostile value.

## Scope note

Both libraries call the same glibc `pow`, so the numeric results are identical
by construction on this platform; the verification's real work is the error
surface, the sentinel, the `stderr` formatting and the ABI. A build against a
*different* libm would need re-running, since `my_pow`'s output depends on that
libm's `errno` behaviour — which is inherited from the C, not a translation
choice.
