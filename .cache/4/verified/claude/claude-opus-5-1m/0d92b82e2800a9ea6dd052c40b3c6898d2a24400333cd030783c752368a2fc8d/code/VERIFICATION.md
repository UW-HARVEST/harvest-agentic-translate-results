# VERIFICATION.md — completion gate

Library under test: `rgb_to_hsv` (single C translation unit, single exported
symbol). Reference: `c_src/` built as `libtranslated_rust.so`. Candidate:
`src/lib.rs` built as `librgb_to_hsv_lib.so`. Every comparison goes through
`dlopen` + `dlsym` on **both** shared objects — no Rust function is called
directly, so the `#[no_mangle] extern "C"` wrapper is part of what is tested.

Run everything with:

```sh
./verify.sh
```

## Gate

| gate | evidence | status |
|------|----------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc symbols in the Rust `.so` | `verify.sh` step 4, both profiles: "all 1 C symbol(s) exported by the Rust .so" | ✅ |
| Phase B: every row of `CONFIGS.md` (28 rows) passes across randomized inputs | `tests/configs.rs`, 28 tests, ~250 000 differential calls with a fixed seed | ✅ |
| Phase C: every row of `ERRORS.md` (24 rows) has a passing error-path differential test | `tests/errors.rs`, 25 tests (24 rows + the child-side probe helper) | ✅ |
| All of the above under EVERY feature combination | `Cargo.toml` declares no `[features]` → 1 combination; run as `--no-default-features`, default and `--all-features`, in dev **and** release profiles | ✅ |

## What was checked beyond the minimum

* **Bit-exact comparison** (`f32::to_bits`) rather than approximate equality, so
  `+0.0` vs `-0.0` and NaN payload propagation are part of the contract.
* **Output-buffer poisoning + canaries**: proves the implementation writes
  exactly `dest[0..3]` and reads exactly `src[0..3]`.
* **Aliasing**: `dest == src`, `dest = src+1`, `dest+1 = src`, and unaligned
  (4-byte-aligned but not 16-byte-aligned) views.
* **UB inputs** (null / unmapped pointers) compared by child-process termination
  signal — see the note at the end of `ERRORS.md` for the dev-profile
  `-C ub-checks` caveat.
* **Optimisation-level independence**: the same suite passes with the reference C
  rebuilt at `-O2` and `-O3 -march=native` (`HARVEST_C_SO=<path> cargo test`).
* **Harness self-check (mutation testing)**: five deliberate mutations of
  `src/lib.rs` were each caught by the suite before it was accepted —
  1. `min = if min < g {min} else {g}` → `min.min(g)` (NaN semantics) → 6 tests failed
  2. `if h < 0.0` → `if h <= 0.0` (signed zero) → 11 tests failed
  3. dropping the `|| max == 0.0` disjunct → 6 tests failed
  4. `+ 1e-7` in the green hue sector → 11 tests failed
  5. `max > b` → `max >= b` (tie-break direction) → caught by the signed-zero row
  The un-mutated translation passes all 53 tests.
* **Stale-artifact guard**: `cargo test` does *not* emit a `cdylib`, so the
  harness rebuilds `librgb_to_hsv_lib.so` for the current profile itself and
  refuses to run if the `.so` is older than `src/lib.rs` (this exact trap had
  initially hidden the mutations behind a stale library).

## Divergences found and fixed

None in `src/lib.rs`: the translation was already bit-exact for every
configuration and error condition enumerated in `CONFIGS.md` / `ERRORS.md`,
including NaN payloads, signed zeros, subnormals, overflow of `max - min`, the
`max == 0` early-exit that skips the division, and the `-0.0` hue that must *not*
receive the `+360` fixup.

The only changes made were to the test scaffolding: `libloading` added to
`[dev-dependencies]`, the three Phase-A artifacts, `tests/`, and `verify.sh`.
`c_src/` is untouched.
