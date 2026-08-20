# Verification report — `bin2hex` C → Rust translation

Everything below is reproducible with the two scripts in the crate root:

```sh
./verify_all.sh      # phases A–D: feature combos × profiles, symbol parity, all tests
./mutation_check.sh  # proves the differential suite is neither vacuous nor over-strict
```

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` on both `.so`s. C defines exactly one symbol, `bin2hex`; the Rust `.so` exports it under the same name. **0 missing symbols**, nothing stubbed, no untranslated C file (the library is a single TU, fully translated). |
| `ERRORS.md` | 8 rejection rows (E1–E8) + 7 generic FFI-boundary rows (G1–G7). The only rejection construct in the C is the single `abort()` on `lib.c:13`, reached from a two-term short-circuit `||`; there is no error return code, no `NULL` return, no `errno`, no `assert`. |
| `CONFIGS.md` | 23 valid-configuration rows (C1–C23) over the axes the C actually branches on: loop trip count, `hex_maxlen` slack, the two nibble-correction branches, byte-value boundaries, source/destination pointer offsets, buffer pre-state, aliasing, statelessness and concurrency. There is one public entry point (`bin2hex`) and it *is* the lowest-level one — no wrapper layer exists. |

## Build configurations enumerated

`Cargo.toml` declares **no `[features]`**, and `c_src/CMakeLists.txt` has no
options/`#ifdef`s, so the complete set of feature combinations is the single
empty one. `verify_all.sh` derives this mechanically (powerset of `[features]`)
so it stays correct if features are ever added, and runs the whole suite for:

| configuration | `cargo check` | symbol parity | Phase B | Phase C |
|---------------|---------------|---------------|---------|---------|
| `--no-default-features`, dev profile | ok | ok | 23/23 | 16/16 |
| `--no-default-features`, release profile | ok | ok | 23/23 | 16/16 |
| `--no-default-features`, harness-built `rustc` cdylib | ok | ok | 23/23 | 16/16 |

Additionally the Rust `.so` was diffed against the C library compiled at
`-O0`, `-O1`, `-O2`, `-O3` and `-Os` (`C_CDYLIB=… cargo test`): identical results
in all five, including every abort/segfault path.

## Divergence found and fixed

**Null-pointer dereference terminated with the wrong signal (`SIGABRT` instead of
`SIGSEGV`).** `ERRORS.md` rows G1/G3 (`hex == NULL`, or `bin == NULL` with
`bin_len > 0`) are UB in C and fault with `SIGSEGV`. The Rust `.so`, when built
with the default *dev* profile, terminated with `SIGABRT`: `debug-assertions`
enables Rust's UB checks, which turn a raw `*ptr` on a null address into
`panic_nounwind` → `abort()`. Fix (`Cargo.toml`): the C library carries no debug
instrumentation, so neither profile of the Rust crate may either —

```toml
[profile.release]
overflow-checks = false
debug-assertions = false

[profile.dev]
overflow-checks = false
debug-assertions = false
```

The same flags are passed by the test harness' `rustc` fallback. After the fix
both implementations die with `SIGSEGV` on G1/G3 and with `SIGABRT` on E1–E7, in
both profiles. No change to `src/lib.rs` logic was needed: the translation of the
arithmetic, the truncating casts, the short-circuit validation order and the
wrapping `size_t` multiply were already byte-exact.

## Harness self-check (mutation testing)

`mutation_check.sh` rebuilds `src/lib.rs` with 14 observable mutations
(swapped nibble order, `<=` → `<` in the capacity check, `>=` → `>` and a wrong
divisor in the length limit, missing/misplaced NUL, wrong correction mask, wrong
`87` base, wrong `-10` bias, `0xf` → `0x7` mask, signed `>> 4`, `<` → `<=` loop
bound) and injects each into the suite via `RUST_CDYLIB`. **All 14 are caught.**

Four further mutations are proven *unobservable* through the C API — the shift
amount in `(x - 10U) >> 8` (any shift in `1..=24` leaves the low byte at `0xFF`)
and both `(unsigned char)` truncations (only bits `0..15` of `x` are ever
stored, and the untruncated values are zero in bits `8..15`). These are asserted
to **survive**, which shows the tests do not over-specify behaviour the C does
not define. Both claims were verified exhaustively over all 256 input bytes.

## Test inventory

| file | rows covered | what it does |
|------|--------------|--------------|
| `tests/common/mod.rs` | — | loads *both* `.so`s with `libloading` (the Rust side always through its `#[no_mangle]` export, never a direct call), fixed-seed SplitMix64 PRNG, whole-buffer differential driver, `fork()`+`waitpid` outcome comparator with core dumps disabled |
| `tests/differential_valid.rs` | C1–C23 | 23 tests; ~76 000 differential calls incl. exhaustive 1-byte (256) and 2-byte (65 536) sweeps |
| `tests/differential_errors.rs` | E1–E8, G1–G6 | 16 tests; every call made in a forked child, comparing the exact `WTERMSIG`/`WEXITSTATUS` of C vs Rust, plus a full sweep of the `hex_maxlen` decision boundary for `bin_len ∈ 0..=8` and randomized `size_t` fuzzing |
| `tests/symbol_parity.rs` | Phase D | recomputes `nm -D --defined-only` for both `.so`s and asserts `C \ Rust == ∅`; checks the Rust `.so` borrows nothing from the C one |

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc symbols in the Rust `.so`.
- [x] Phase B: every row of `CONFIGS.md` (C1–C23) passes across randomized inputs.
- [x] Phase C: every row of `ERRORS.md` (E1–E8, G1–G6; G7 = N/A, the API has no enum) has a passing error-path differential test.
- [x] All of the above hold for every feature combination (the single empty one) and in both the dev and release profiles, plus the harness-built cdylib.
