# Verification report

Differential verification of the Rust translation (`src/lib.rs`, built as
`libcharinbuf_lib.so`) against the C ground truth (`../c_src/src/lib.c`, built as
`../c_src/build/libharvest-work-QiJ5vr.so`).

Every assertion in this suite is made **through the dynamic symbol tables of two
shared objects loaded with `libloading`** — the Rust crate is never linked into
the test binary, so the `#[no_mangle] extern "C"` wrappers, the C ABI of every
parameter (including the function-pointer slot of `apply_operation`) and the
`static` state inside each `.so` are all exercised the way an external C consumer
would exercise them.

## How to reproduce

```bash
# 1. C ground truth
cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust library + differential suite (both profiles, all feature combos)
cd ../../translation
cargo build --lib                 # NOTE: `cargo test` does NOT build a cdylib
cargo test
bash scripts/check_all_features.sh   # every feature combo x {dev, release}
python3 scripts/mutation_check.py    # sensitivity check of the suite itself
```

> **Stale-library hazard (found and fixed during this work):** `cargo test` never
> builds the `cdylib` artifact, because a cdylib-only crate is not a dependency
> of an integration test. The suite therefore silently validated an *old* `.so`.
> `tests/support/mod.rs::rust_lib_path()` now (a) rebuilds the cdylib for the
> running profile on demand and (b) asserts its mtime is not older than any
> source file, refusing to run against a stale library.

## Files added by this verification

| file | purpose |
|------|---------|
| `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md` | Phase A artifacts |
| `tests/support/mod.rs` | harness: dual `dlopen`, fd-1 stdout capture, differential helpers, fixed-seed PRNG |
| `tests/phase_b_valid.rs`, `tests/phase_b_bulk.rs` | Phase B (C1–C34, C35–C42) |
| `tests/phase_c_errors.rs`, `tests/alloc_failure.rs` | Phase C (E1–E18, G1–G10) |
| `tests/phase_d_symbols.rs` | Phase D symbol parity |
| `scripts/check_all_features.sh`, `scripts/mutation_check.py` | Phase D feature sweep, suite-sensitivity check |
| `.cargo/config.toml` | sets `RUST_TEST_THREADS=1`: capturing the libraries' `printf` output means redirecting fd 1, which is process-global, so tests must not run concurrently |
| `Cargo.toml` | `[dev-dependencies] libloading = "0.8"` (the only change; `src/lib.rs` is byte-identical to the translation under test) |

`c_src/` was not modified; the only addition there is the `build/` directory
produced by the CMake commands quoted above.

## Phase A — surface map

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | all 10 dynamic symbols of the C `.so`, each matched by the Rust `.so`; import lists of both |
| `ERRORS.md`  | 18 distinct C rejection/error branches (E1–E18) + 10 generic FFI boundary cases (G1–G10) |
| `CONFIGS.md` | 42 valid-input configurations (C1–C42) covering the runtime option axes and input shapes the C actually branches on |

## Phase B — valid-path differential tests

`tests/phase_b_valid.rs` (34 tests, rows C1–C34) and `tests/phase_b_bulk.rs`
(8 tests, rows C35–C42). All randomized rows use a fixed-seed xorshift64\* PRNG
(`SEED = 0x2026_0827_C0FF_EE01`).

For `charinbuf` — the only function that produces output — the comparison is
byte-exact on **stdout** as well as on the return value: fd 1 is redirected with
`dup2` around each call (or around a whole batch, for the bulk rows), because
both libraries print through libc `printf`.

Volume actually executed (per profile): ≈ 340 000 `charinbuf` calls per library,
≈ 1.4 M `validate_uint16_range` calls, ≈ 82 000 `find_char_in_buffer` calls,
≈ 90 000 counter-op calls.

## Phase C — error-path differential tests

`tests/phase_c_errors.rs` (23 tests: E1–E6, E8–E14, E17, E18, G1–G10) and
`tests/alloc_failure.rs` (3 tests: E7, E15, E16).

The three allocation-failure rows are genuinely reached, not merely asserted to
be unreachable: the test re-executes itself as a child process, clamps
`RLIMIT_AS` to the current `VmSize` plus a small margin and drains the malloc
arena with a pointer chain (no Rust allocation while the heap is exhausted), so
that `malloc` really fails inside the library. Observed child verdicts:

```
child(create_buffer):        C null=true  Rust null=true
child(charinbuf mode 2): arena_empty=true C rc=-1 Rust rc=-1  (identical stdout)
child(charinbuf mode 4): arena_empty=true C rc=0  Rust rc=0   (identical stdout)
```

Out-of-range enum-like values are covered: `mode` is a plain `int`, and
`e11b_charinbuf_invalid_mode_random` feeds 4096 random `i32`s plus the dense
range `-8..=12` plus `INT_MIN`, `INT_MIN+1`, `INT_MAX-1`, `INT_MAX` and
`(int)0x80000000` through both libraries.

## Phase D — symbol parity, profiles, feature combinations

* `tests/phase_d_symbols.rs` diffs `nm -D --defined-only` of both `.so`s: the
  set difference `C \ Rust` is **empty** (10/10 symbols), and the C library
  exports nothing beyond the 10 documented names. `dlopen(RTLD_NOW)` on the Rust
  `.so` succeeds, so it has no dangling imports.
* `scripts/check_all_features.sh` extracts the feature list from `Cargo.toml`
  (there are **no** declared features, so the combination set is
  `{default, --no-default-features, --all-features}`), rebuilds the cdylib for
  each combination and runs the full suite under both the `dev` and `release`
  profiles. Result: `ALL FEATURE COMBINATIONS PASSED` (6 configurations).
* Totals per configuration: **76 tests, 0 failures**
  (`alloc_failure` 3, `phase_b_bulk` 8, `phase_b_valid` 34, `phase_c_errors` 26,
  `phase_d_symbols` 5).

## Suite sensitivity (mutation testing)

Passing tests only prove something if they can fail. `scripts/mutation_check.py`
injects 25 one-line mutations into `src/lib.rs`, runs the full suite for each and
restores the file afterwards:

```
23 behaviour-changing mutants killed, 2 equivalent mutants correctly survived,
0 unexpected, 0 skipped
```

Killed mutants include: `UINT16_MAX` off-by-one; `<`→`<=` and `>`→`>=` in
`validate_uint16_range`; dropped NULL checks in `is_string_empty`,
`find_char_in_buffer` and `create_buffer`; `*str != 0` → `*str > 0` (the signed
`char` trap); `size-1` in the `memchr` call; `apply_operation(NULL)` returning 0
instead of −1; wrong counter arithmetic; a dropped `counter = 0` reset in
`charinbuf`; `+= 10` → `+= 11`; `strlen+1`; swapped `opt1`/`opt2` in mode 3;
decrement 5 → 6; `'X'` → `'Y'`; offset off-by-one; `default:` returning −2;
mode 5 folded into mode 0; and two single-character typos in `printf` literals.
The two survivors are provably equivalent (`%u` vs `%d` for a positive constant;
zero- vs sign-extending the `char` before `memchr`, which narrows to the same
`unsigned char`).

## Divergences found in the translation

**None.** Every configuration in `CONFIGS.md` and every row in `ERRORS.md`
matches the C byte-for-byte (return values *and* stdout). The only defect found
during verification was in the harness itself (the stale-`.so` hazard described
above), which is now fixed and guarded by an assertion.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols in the Rust `.so`; 0 dangling
      non-libc imports.
- [x] Phase B: all 42 `CONFIGS.md` rows pass across randomized inputs.
- [x] Phase C: all 18 `ERRORS.md` rows (+10 generic boundary rows) have passing
      error-path differential tests.
- [x] All of the above hold under every feature combination and under both the
      `dev` and `release` profiles.
