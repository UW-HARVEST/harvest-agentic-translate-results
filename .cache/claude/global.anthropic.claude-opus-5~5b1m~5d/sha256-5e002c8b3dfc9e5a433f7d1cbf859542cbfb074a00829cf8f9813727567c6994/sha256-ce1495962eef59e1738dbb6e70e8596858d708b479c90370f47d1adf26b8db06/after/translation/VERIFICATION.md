# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust) against `c_src/` (C).
Both sides are exercised **only** through `dlopen`/`dlsym` on their shared
objects, so the `#[unsafe(no_mangle)] extern "C"` export wrapper is under test
too. The Rust implementation is never called directly.

## Result

**The translation is bit-exact.** Over the entire domain for which the C program
is well defined (`x ∈ -16..=8223`, all 8240 values, exhaustively) the two
objects return identical IEEE-754 bit patterns. Over the remaining ~4.29 billion
`int` values — where the C's unchecked `g_pow43[...]` subscript is out of bounds
and therefore undefined — the two objects are shown to compute the *same
subscript, `sign`, `frac`, `poly` and `mult`*, and to fault identically where the
address cannot be dereferenced. No divergence was found and no change to the
Rust source was required.

## Gate

- [x] **`SYMBOLS.md`** — `nm -D` diff is empty. The C `.so` exports exactly one
      symbol, `pow43`; the Rust `.so` exports it under the same name.
      0 missing symbols, 0 unresolved non-libc imports, `ldd -r` clean.
      Automated by `scripts/check_symbols.sh`.
- [x] **Phase B** — every one of the 26 rows in `CONFIGS.md` passes, with
      randomized (fixed-seed) inputs per row plus exact boundary values.
      Rows 11, 20, 21 and 22 are *exhaustive*, so the union covers every
      well-defined input rather than a sample.
- [x] **Phase C** — every one of the 12 rows in `ERRORS.md` has a passing
      error-path differential test, each asserting the *same* specific
      outcome (same bits, same subscript, or same fatal signal) rather than
      "both failed somehow".
- [x] **All configurations** — `scripts/check_all_features.sh` passes for the
      whole feature powerset (`Cargo.toml` declares no `[features]`, so this is
      default / `--no-default-features` / `--all-features` /
      `--no-default-features --all-features`) **and** for a dev-profile `.so`
      with `overflow-checks` enabled.

```
$ cargo test --release
   unittests src/lib.rs   2 passed
   tests/differential.rs  26 passed   <- Phase B, one test per CONFIGS.md row
   tests/errors.rs        14 passed   <- Phase C, one test per ERRORS.md row
   tests/oob_faults.rs     2 passed   <- fault parity (forked)
   tests/oob_pages.rs      1 passed   <- deep out-of-bounds subscript parity
                          45 passed; 0 failed
```

Stable across 5 consecutive full runs (the `mmap`/`fork` tests live in their own
test binaries so they never race with anything else).

## Why the tests are not vacuous — mutation adequacy

Passing tests only mean something if they can fail. 22 plausible
mistranslations were injected into `src/lib.rs` one at a time
(`scripts/mutate.sh`) and the suite was re-run with `--no-fail-fast`, so that
*every* test binary runs and the detection can be attributed correctly rather
than credited to whichever binary happens to execute first.

`diff` = `tests/differential.rs` (Phase B), `err` = `tests/errors.rs` (Phase C),
`pages` = `tests/oob_pages.rs`, `faults` = `tests/oob_faults.rs`,
`unit` = in-crate `#[cfg(test)]` tests. **Every** detected mutant is caught by at
least one *differential* (FFI, `.so`-vs-`.so`) test; none relies on the in-crate
unit tests.

| # | mutation | detected? | failing test binaries |
|---|----------|-----------|-----------------------|
| M01 | `mult = 16` → `8` | yes (22 tests) | diff, err, faults, unit |
| M02 | `mult = 256` → `255` | yes (27) | diff, err, faults, pages, unit |
| M03 | `x < 129` → `x <= 129` | yes (17) | diff, err, faults, unit |
| M04 | `x < 129` → `x < 128` | **equivalent** | — (proven below) |
| M05 | `x < 1024` → `x < 4096` | yes (17) | diff, err, unit |
| M06 | `x <<= 3` → `x <<= 2` | yes (22) | diff, err, faults, unit |
| M07 | table offset `16 + x` → `15 + x` | yes (25) | diff, err, faults, pages, unit |
| M08 | `2*x & 64` → `2*x & 32` | yes (32) | diff, err, faults, pages, unit |
| M09 | `2*x & 64` → `x & 64` | yes (26) | diff, err, pages |
| M10 | `x & ~63` → `x & ~31` | yes (25) | diff, err, faults, pages, unit |
| M11 | `x & 63` → `x & 31` | yes (26) | diff, err, faults, pages, unit |
| M12 | `(x&63) - sign` → `+ sign` | yes (26) | diff, err, faults, pages, unit |
| M13 | `(x&~63) + sign` → `- sign` | yes (26) | diff, err, faults, pages, unit |
| M14 | `4.f/3` → `1.3333333f` literal | yes (21) | diff, err, pages |
| M15 | `2.f/9` → `0.2222222f` literal | **equivalent** | — (proven below) |
| M16 | `poly` computed in `f64` | yes (17) | diff, err |
| M17 | `T*poly*mult` → `T*(poly*mult)` re-association | yes (4) | err |
| M18 | `frac` division carried out in `f64` | yes (2) | pages |
| M19 | `poly` Horner form → expanded form | yes (28) | diff, err, faults, unit |
| M20 | `wrapping_add(sign)` → `saturating_add` | yes (2) | pages |
| M21 | arithmetic `>> 6` → logical shift | yes (2) | pages |
| M22 | `>> 6` → `/ 64` (truncate instead of floor) | yes (2) | pages |

**20 of 20 non-equivalent mutants detected.** Note that M18 and M20–M22 are
caught *only* by `oob_pages.rs`: they are invisible to any test that stays inside
the well-defined domain, which is precisely why the synthetic-mapped-page
technique was added rather than writing those inputs off as "undefined
behaviour, out of scope".

The two undetected mutants were proven to be *observationally equivalent*, not
blind spots:

* **M04** (`x < 129` → `x < 128`) and the related `x < 1024` → `x < 1023`
  change which path the boundary value takes, but the two paths are designed to
  agree there: `g_pow43[144] == 16 * g_pow43[32]` **bit-exactly** in `f32`
  (`0x44214518 == 16 * 0x42214518`), and the `frac` values coincide, so the
  result is identical. Verified by building the mutant and diffing it against
  the C over the whole domain: 0 mismatches.
* **M15** (`2.f/9` written as the literal `0.2222222f`) uses a genuinely
  *different* `f32` (`0x3e638e37` vs `0x3e638e39`), yet `poly` is unchanged for
  **every** `x`: `frac` is always small enough that the difference in the
  quadratic term falls below `f32` resolution. Verified by exhaustive search
  over all 2^31 reachable `x` (`scripts/equiv_check.rs`): 0 differing inputs.

M18 deserves a note because it is exactly the class of bug happy-path testing
misses. C's usual arithmetic conversions make `(float)num / den` a
**single-precision** division; doing it in `f64` and rounding afterwards changes
`frac` for 264,244,931 inputs but changes the observable output for only **5**
of the 2^31 reachable values of `x` — all of them deep in the out-of-bounds
region where the C would segfault. Those five witnesses
(`1163220262`, `1207959461`, `1297437987`, `1342177186`, `1431655712`) were
found by exhaustive search and are now pinned in `tests/oob_pages.rs`, which is
what makes M18 detectable at all.

## Reference stability

"Byte-identical to the C" is only meaningful if the C itself is stable. The
reference `.so` was diffed against rebuilds of the same source:

| build of `c_src/src/lib.c` | mismatches vs reference |
|----------------------------|-------------------------|
| `gcc -O0 / -O1 / -O2 / -O3 / -Os` | 0 |
| `gcc -O2 -ffp-contract=fast` / `off` | 0 |
| `gcc -O2 -fwrapv` / `-fno-wrapv` / `-std=c99` / `-std=c11` | 0 |
| `gcc -O3 -march=x86-64-v3` (FMA enabled) | **24** |
| `gcc -Ofast` (`-ffast-math`) | **2082** |

The last two rows are the C being non-portable, not the translation being wrong:
with FMA available the compiler contracts `1.f + frac * (...)` into a fused
multiply-add and changes the rounding. `c_src/CMakeLists.txt` sets no
`CMAKE_BUILD_TYPE` and no `-march`, so the reference build targets baseline
x86-64, where no contraction is possible.

The Rust side is *more* deterministic: it never auto-contracts, so it matches
the reference under every `target-cpu`.

| Rust build | mismatches vs reference |
|------------|-------------------------|
| default target | 0 |
| `-C target-cpu=native` (host has FMA) | 0 |
| `-C target-cpu=x86-64-v3` / `x86-64-v4` | 0 |

## Reproducing

```sh
# 1. build the C reference
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. everything else, from translation/
cd ../../translation
cargo test --release             # 45 differential tests
./scripts/check_symbols.sh       # Phase D symbol parity
./scripts/check_all_features.sh  # every feature combo + dev profile
./scripts/mutate.sh "label" "old code" "new code"   # harness adequacy
```

The Rust `cdylib` is built on demand into `target/ffi-so/` by the test harness
itself (`cargo test` does not emit a `cdylib` for a crate whose only
`crate-type` is `cdylib`). Override either object with `POW43_RUST_SO` /
`POW43_C_SO`.

## Files

| file | purpose |
|------|---------|
| `SYMBOLS.md` | Phase A — public symbol surface and the `nm -D` diff |
| `ERRORS.md` | Phase A — error-surface table (12 rows) + Phase C checklist |
| `CONFIGS.md` | Phase A — configuration-surface table (26 rows) + Phase B checklist |
| `tests/support/mod.rs` | harness: `dlopen` both objects, run-time table location, readability probe, forked calls, fixed-seed RNG, C algorithm oracle |
| `tests/differential.rs` | Phase B — one test per `CONFIGS.md` row |
| `tests/errors.rs` | Phase C — one test per `ERRORS.md` row |
| `tests/oob_pages.rs` | Phase C — deep out-of-bounds subscript parity (isolated binary; mutates the address space) |
| `tests/oob_faults.rs` | Phase C — fault parity via `fork` (isolated binary) |
| `scripts/check_symbols.sh` | Phase D — symbol diff, must be empty |
| `scripts/check_all_features.sh` | Phase D — feature powerset + dev-profile matrix |
| `scripts/mutate.sh` | mutation testing (harness adequacy) |
| `scripts/equiv_check.rs` | exhaustive equivalence prover for candidate float mutations |
| `scripts/cmp_so.c` | standalone two-`.so` differ, used for the reference-stability table |
