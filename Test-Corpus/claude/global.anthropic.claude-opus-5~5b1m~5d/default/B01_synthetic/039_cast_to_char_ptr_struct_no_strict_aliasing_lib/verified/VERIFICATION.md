# Verification report — C `driver` vs. Rust translation

Reproduce everything with:

```
./run_all_feature_combos.sh
```

(`--offline` is used throughout because this sandbox has no crates.io egress;
`libloading 0.8.9` was already in the local cargo cache.)

## Result

| gate | status |
|---|---|
| `cargo check` compiles cleanly (incl. all test targets) | PASS — 0 errors, 0 warnings |
| `SYMBOLS.md` — `nm -D` parity, 0 missing/undefined non-libc symbols | PASS |
| Phase B — every `CONFIGS.md` row (19) passes over randomized inputs | PASS |
| Phase C — every `ERRORS.md` row (10 + generic sweep) has a passing test | PASS |
| All of the above under EVERY feature combination and both profiles | PASS — 4 configuration/profile pairs |

30 differential cases × 4 configuration/profile pairs, ~12k `driver` invocations
per pair, all byte-identical between the two `.so` files.

## Scope of the library

The whole library is one function. `c_src/CMakeLists.txt` compiles exactly one
translation unit (`src/driver.c`, 50 lines), which defines one exported function
`void driver(int)` and one `static` helper `print_hex`. No module was left
untranslated, so no Phase A "translate the missing C source" work was required —
confirmed by the empty `nm -D` diff, not merely by inspection.

`driver` builds `house_t { int floors; int bedrooms; double bathrooms; }`, sets
`floors` from the argument and hard-codes `bedrooms = 3`, `bathrooms = 2.0`,
`memcpy`s the struct into a `char[16]`, and hex-dumps those 16 bytes followed by
a newline. Output is therefore always exactly 33 bytes, of which only the first
8 hex characters depend on the input.

## How the differential test works

Both libraries are loaded with `libloading` and invoked **only** through their
exported `driver` symbol, so the Rust `#[no_mangle] extern "C"` wrapper is part
of what is under test. Nothing in the crate is called as a Rust function.

`driver` returns `void` and communicates solely through libc `printf`, so
`tests/common/mod.rs` captures output at the file-descriptor level: flush, `dup`
fd 1, `dup2` a temp file over it, call the function, `fflush(NULL)` (required —
once fd 1 is a regular file the stream is fully buffered), restore fd 1, read the
file back.

That both handles genuinely resolve to *different* implementations — the real
risk when two `.so`s export the same name — is not assumed; it was proven
empirically by the mutation battery below, where mutating the Rust source changed
the Rust output while the C output stayed fixed.

## Two harness bugs found and fixed (both would have faked a pass)

These are worth recording because in both cases the suite *reported success* while
testing nothing meaningful.

1. **libtest output leaked into the capture.** Redirecting fd 1 is
   process-global, but libtest runs cases on multiple threads and writes its own
   progress lines (`test c10_... ok`) from another thread, plus Rust's `stdout`
   `LineWriter` can hold a newline-less fragment that flushes *after* the
   redirect is installed. Seven Phase B cases "failed" with payloads that were
   actually byte-identical, just prefixed with runner noise. Fixed by running
   both suites with `harness = false` (`run_suite` in `tests/common/mod.rs`
   executes cases strictly one at a time) and by draining `std::io::stdout()`
   before each redirect.

2. **`cargo test` silently tested a STALE `.so`.** No test target links the
   `cdylib` — the tests `dlopen` it — so cargo has no reason to rebuild it.
   A deliberate `bedrooms = 4` mutant **survived the entire suite** because the
   tests kept loading the previously built library. Fixed with `assert_fresh`,
   which compares the `.so` mtime against its sources and aborts with a rebuild
   hint, plus `run_all_feature_combos.sh` doing an explicit `cargo build` per
   configuration before each `cargo test`.

## Mutation battery (sensitivity evidence)

With the staleness guard in place, deliberate bugs were injected into the Rust
and both suites re-run, to confirm the tests can actually fail:

| mutant | result |
|---|---|
| `bedrooms = 4` (wrong constant) | KILLED — 18/19 Phase B, 9/11 Phase C |
| `bathrooms = 2.5` (wrong double) | KILLED — 18/19 Phase B |
| `*p as i8 as c_int` (sign-extension in the hex printer) | KILLED — 14/19 Phase B, 9/11 Phase C |
| `%02X` instead of `%02x` (uppercase hex) | KILLED — 16/19 Phase B, 7/11 Phase C |
| `%x` instead of `%02x` (lost zero padding) | KILLED — 18/19 Phase B |
| trailing `printf("\n")` removed | KILLED — 18/19 Phase B |
| loop bound `i < len - 1` (off-by-one) | KILLED — 18/19 Phase B |
| struct field order swapped | KILLED — 18/19 Phase B |
| `#[repr(C, packed)]` | SURVIVED — **equivalent mutant**, not a gap |

The `packed` survivor was checked rather than waved away: `{i32, i32, f64}`
contains no padding, so `repr(C)` and `repr(C, packed)` have identical size (16)
and identical offsets (0/4/8); only `align_of` differs (8 vs 1), which is
unobservable through an API that only ever `memcpy`s the struct. The mutant is
behaviourally identical to the original, so surviving is the correct outcome.

The sign-extension mutant is the most informative: it is killed by the
high-byte-pattern rows (C7, C8, C10) but *passes* the small-positive rows,
which is exactly why `CONFIGS.md` enumerates byte-pattern classes instead of
calling `driver` once with one value.

## Notes on the two "empty" surfaces

- **`ERRORS.md` has no true error rows.** Mechanically grepping the C for
  `return`/`assert`/`NULL`/range checks/`MIN`/`MAX`/`malloc` yields zero hits:
  `driver` is `void`, validates nothing, and accepts all 2^32 `int` values. The
  Phase C suite therefore asserts *identical disposition* (same 33 output bytes,
  same acceptance, no abort) on the hostile inputs the table enumerates — the
  faithful analogue of "same error code" for an API with no error channel. Rows
  for "null pointer to `driver`" or "oversized length to `driver`" were
  deliberately **not** invented: its sole parameter is a by-value `int`. The
  generic null/length boundaries are discharged where the library's only pointer
  and length actually live, inside `static print_hex` (rows E8, E9), which no
  external caller can reach — asserted via `nm -D` and a failing `dlsym` on both
  objects.
- **No feature combinations exist.** `Cargo.toml` declares no `[features]` table
  and no optional dependencies, so the power set is a single element. The script
  still runs `--no-default-features` explicitly, and additionally covers the
  `release` profile, where `panic = "abort"` and optimisation differ from `dev`.

## Verdict

The Rust translation is byte-for-byte equivalent to the C for every input class
the C distinguishes, exports exactly the C's public symbol surface (and correctly
withholds the `static` helper), and holds under every available configuration.
No divergence was found, so no changes to `src/lib.rs` were needed; the pristine
translation is what is checked in.
