# VERIFICATION.md — completion gate

Differential verification of `translation/` against `c_src/` (ground truth).
Every call in every test goes through `dlopen`/`dlsym` (`libloading`) on **both**
shared objects; the Rust crate is never called directly, so the `#[no_mangle]`
`extern "C"` export wrapper is under test too.

## Reproduce

```bash
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation
./verify_all.sh        # build both .so, symbol parity, full suite per feature combo
./mutation_check.sh    # anti-vacuity: 7 wrong Rust builds must all be detected
```

## Result

61 tests, all passing, in **both** build configurations (`default` and
`--no-default-features`); `verify_all.sh` exits 0.

| binary | tests | covers |
|--------|-------|--------|
| `tests/smoke.rs` | 2 | harness loads both `.so`; call-rate probe (105 M input pairs/s) |
| `tests/phase_b_exhaustive.rs` | 21 | `CONFIGS.md` rows 1–4, 25, 26 (the exhaustive sweeps) |
| `tests/phase_b_configs.rs` | 20 | `CONFIGS.md` rows 5–24 (per-axis, randomized) |
| `tests/phase_c_errors.rs` | 14 | `ERRORS.md` rows 1–13 + generic FFI boundaries |
| `tests/phase_d_symbols.rs` | 4 | `nm -D` parity, no untranslated imports, `hdr_valid` private on both |

## Completion checklist

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
  The C `.so` exports exactly one symbol, `hdr_compare`; the Rust `.so` exports it
  under the identical name. Symbol diff is empty. Every undefined symbol in the
  Rust `.so` is glibc or the `libgcc` unwinder (Rust `std` panic/backtrace
  machinery) — no non-libc import, i.e. no C module was left untranslated. Both
  `.so` files were checked against the complete C source: `CMakeLists.txt`
  compiles one translation unit, whose only other function (`hdr_valid`) is
  `static` and correctly private in the Rust too. Enforced automatically by
  `tests/phase_d_symbols.rs` and by `verify_all.sh`.

- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
  All 26 rows checked off. Rows 1, 2, 3, 4, 25 and 26 are *exhaustive*, not
  sampled: row 1 alone enumerates all 2^32 combinations of the four bytes that
  matter when the sync byte is valid (≈8.6 × 10^9 FFI calls, 16 parallel shards,
  ~2 s wall). Rows 5, 6 and 24 add 40 M+ randomized draws with fixed SplitMix64
  seeds. `CONFIGS.md` records why rows 1 + 2 + 24 + 25 + 26 are jointly exhaustive
  over the library's entire meaningful input space. Low-level entry points: the
  C exposes exactly one function and the tests drive it directly — there is no
  convenience wrapper to hide behind. `hdr_valid`, the file-local helper, is
  reached through every one of its five rejection branches (`ERRORS.md` rows 1–5).

- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential
  test.** All 13 rows checked off, each asserting the two `.so` files return the
  *identical* `int` — and additionally that it is the C's rejection sentinel `0`,
  so a test cannot pass by both sides being wrong in the same direction. Generic
  boundaries also covered: null `h1`, unmapped `h1`, aliased pointers, every byte
  position swept over all 256 values, one step past every field boundary, and
  all-`0x00`/all-`0xff`. The API declares no enum parameters (its only inputs are
  `const uint8_t *`), so "out-of-range enum value" has no instance here; the
  equivalent — a reserved field value with no valid meaning — is rows 3, 4 and 5,
  and the full 0..=255 sweep of every byte position covers the class exhaustively.
  `h2 == NULL` is excluded with justification (the C dereferences `h2[0]`
  unconditionally, so it is UB in the C; the Rust reproduces that rather than
  diverging by adding a silent null check).

- [x] **All of the above hold under every feature combination.** `Cargo.toml`
  declares no `[features]` table, so the single configuration is the only one that
  exists; `verify_all.sh` enumerates combinations from `Cargo.toml`
  programmatically (power set when features are present) and runs `cargo check`,
  `cargo build --release`, the `nm -D` diff and the full suite for each. Both the
  default build and `--no-default-features` were run: 61/61 passing, symbol diff
  empty, in each.

## Anti-vacuity evidence

A green differential suite is only meaningful if it can fail.
`./mutation_check.sh` builds seven deliberately-broken variants of `src/lib.rs`
and requires each to be detected:

| mutant | injected defect | killed by |
|--------|-----------------|-----------|
| m1 | `& 0xFE` → `& 0xFF` (padding bit no longer ignored) | B-exhaustive, B-configs, C |
| m2 | reserved-bitrate check `== 15` → `== 14` | B-exhaustive, B-configs, C |
| m3 | drop the `(h[1] & 0xFE) == 0xe2` alternative (MPEG-2.5 class) | B-exhaustive, B-configs, C |
| m4 | invert the free-format nibble agreement test | B-exhaustive, B-configs, C |
| m5 | samplerate mask `0x0C` → `0x04` | B-exhaustive, B-configs, C |
| m6 | reserved-layer check `== 0` → `== 1` | B-exhaustive, B-configs, C |
| m7 | read `h1[1]` before the validity check (loses C's `&&` short-circuit) | C (SIGSEGV on the null/unmapped-`h1` rows) |

All seven killed. m7 is killed *only* by Phase C, which is precisely the
blind spot that happy-path testing leaves. Row 13's guard page is likewise
self-validating: a forked child reads the guard byte and the test asserts it dies
with `SIGSEGV`/`SIGBUS`, so the over-read check cannot pass vacuously.

## Divergences found and fixed

None. `translation/src/lib.rs` matched the C on every input tested, including the
full 2^32 exhaustive sweep, and required no changes. The only fix during
verification was to a test's own bookkeeping assertion (the expected count of
accepting `h1` tails in `cfg_row03`, which depends on whether `h2[2]`'s bitrate
nibble is zero: 8 rather than 32 accepting tails in the free-format case).

## Files added by this verification

```
translation/SYMBOLS.md            Phase A symbol map + Phase D diff
translation/ERRORS.md             Phase A error-surface table (13 rows) + check-off
translation/CONFIGS.md            Phase A configuration-surface table (26 rows) + check-off
translation/VERIFICATION.md       this file
translation/verify_all.sh         per-feature-combination driver
translation/mutation_check.sh     anti-vacuity driver
translation/tests/common/mod.rs   libloading harness, SplitMix64 RNG, C predicates
translation/tests/smoke.rs
translation/tests/phase_b_exhaustive.rs
translation/tests/phase_b_configs.rs
translation/tests/phase_c_errors.rs
translation/tests/phase_d_symbols.rs
```

`translation/Cargo.toml` gained `libloading` and `libc` under
`[dev-dependencies]`, plus `[profile.test] opt-level = 3` (test binaries only —
the shipped `cdylib` is unchanged) so the 2^32 sweep finishes in seconds.
Nothing in `c_src/` was modified; `src/lib.rs` was not modified.
