# VERIFICATION.md — completion record

Differential verification of `translation/` (Rust) against `c_src/` (C, the
ground truth). Both are built as shared objects; every call in every test goes
through `dlopen`/`dlsym` (`libloading`), so the `#[no_mangle] extern "C"` export
wrappers are themselves under test. No Rust function is ever called directly.

## Artifacts

| file | purpose |
|------|---------|
| `SYMBOLS.md` | Phase A/D — `nm -D` symbol surface and the (empty) diff |
| `ERRORS.md` | Phase A/C — error-surface table, 14 rows |
| `CONFIGS.md` | Phase A/B — configuration-surface table, 22 rows |
| `tests/common/mod.rs` | harness: dual `.so` loading, fd-1 stdout capture, `errno` observation, seeded SplitMix64 PRNG |
| `tests/smoke.rs` | harness self-check (proves captures are non-empty, so "both empty ⇒ equal" can never be a false pass) |
| `tests/configs.rs` | Phase B — one test per `CONFIGS.md` row |
| `tests/errors.rs` | Phase C — one test per `ERRORS.md` row |
| `tests/symbols.rs` | Phase D — symbol diff enforced as a test |
| `scripts/check_features.sh` | Phase D — derives the feature set from `Cargo.toml` and re-runs everything for each combination × `release`/`debug` |

## How the harness keeps the comparison honest

The library's only output is what it `printf`s, and it carries mutable
file-scope state (`static house_t the_house`) that every call advances. Each
`.so` has its **own** copy of that state, so a "step" delivers the same call to
C and then to Rust while holding a global lock, and compares the captured bytes.
Because every step is atomic, both libraries observe the identical global call
sequence and their states stay in lockstep for the whole run.

`errno` is treated as a second observable: `parse_val` writes it, and the value
left behind after `driver` returns is compared between C and Rust on every
error-path test.

## Results

`cargo test -- --test-threads=1`

```
tests/configs.rs   22 passed; 0 failed     (Phase B, all 22 CONFIGS.md rows)
tests/errors.rs    13 passed; 0 failed     (Phase C, all 14 ERRORS.md rows)
tests/smoke.rs      1 passed; 0 failed
tests/symbols.rs    3 passed; 0 failed     (Phase D)
```

(`ERRORS.md` has 14 rows in 13 tests: rows 6 and 14 are asserted together by
`row06_error_path_leaves_state_untouched`, because "no `run` call happened" is
exactly what proves the uninitialised `x` is never read.)

`scripts/check_features.sh`

```
features declared in Cargo.toml: 0 (none)
  <default>              release   symbol diff EMPTY   39 passed
  <default>              debug     symbol diff EMPTY   39 passed
  --no-default-features  release   symbol diff EMPTY   39 passed
  --no-default-features  debug     symbol diff EMPTY   39 passed
RESULT: all feature combinations x profiles PASSED
```

## Mutation check — the suite is not vacuous

Four deliberate regressions were injected into `translation/src/lib.rs`, each
rebuilt and run against the unmodified C, then reverted. Every one was caught:

| injected regression | detected by |
|---------------------|-------------|
| delete `*errno = 0;` in `parse_val` | 8 error-path tests, incl. the dedicated `row08_stale_errno_does_not_reject` |
| `floors` incremented by 2 instead of 1 | all 22 `configs.rs` rows |
| "fix" `parse_val` to reject trailing garbage (`*endp == 0`) | 10 rows, incl. `row13_driver_trailing_garbage` |
| print `%.2f` instead of `%.1f` | all 22 `configs.rs` rows |

`src/lib.rs` was byte-for-byte restored afterwards (`diff -q` clean).

## Notable C behaviours confirmed preserved (not "fixed")

* Trailing garbage is **accepted**: `parse_val` only tests `endp != str`, so
  `"12abc"` parses as `12`. (`CONFIGS.md` row 13.)
* `"0x1f"` under base 10 parses as `0` and is accepted. (Row 14.)
* `"3000000000"` and `"-3000000000"` are rejected by the explicit
  `INT_MIN`/`INT_MAX` range test with `errno == 0`, whereas
  `"99999999999999999999"` is rejected earlier by `errno == ERANGE`. Both print
  the same message but leave a **different** `errno`, and both are asserted.
  (`ERRORS.md` rows 2–5.)
* `int` overflow of `floors`/`bedrooms` wraps two's-complement; Rust uses
  `wrapping_add` and must not panic. (`ERRORS.md` rows 12–13.)
* `run` has external linkage despite not appearing in `driver.h`, so it is part
  of the ABI and is exercised directly, not only through `driver`.
* `driver(NULL)` is unchecked in the C and faults inside `strtol`. Verified in
  forked children: **both** libraries die with `SIGSEGV` (signal 11).
  (`ERRORS.md` row 7.)
* `the_house` is never reset; consecutive calls keep accumulating.
  (`CONFIGS.md` row 21.)

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so`; the diff
      of exported symbols is empty and 0 unresolved non-libc symbols remain
      (`ldd` reports no "not found"). The C project has exactly one source file
      (`c_src/src/driver.c`) and it is fully translated — no module was skipped
      and no symbol is stubbed.
- [x] Phase B: every one of the 22 `CONFIGS.md` rows passes across its
      randomized inputs (fixed seed, reproducible).
- [x] Phase C: every one of the 14 `ERRORS.md` rows has a passing error-path
      differential test asserting the same sentinel **and** the same `errno`,
      plus null-pointer, zero-length, oversized-length, one-past-range and
      arbitrary-`int`-across-FFI coverage.
- [x] All of the above hold under every feature combination
      (`{default}`, `{--no-default-features}` — `Cargo.toml` declares no
      features) and under both the `release` and `debug` Rust `.so`.

Nothing in `c_src/` was modified; only `c_src/build/` was created, as the build
instructions require.

**No divergence between the C and Rust implementations was found.** The one
failure encountered during this work was a bug in my own test expectation
(`row06`'s state arithmetic), which was corrected in the test, not in the
translation.
