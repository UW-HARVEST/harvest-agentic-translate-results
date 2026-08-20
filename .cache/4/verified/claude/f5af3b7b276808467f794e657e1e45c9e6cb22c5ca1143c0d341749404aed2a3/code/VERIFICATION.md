# VERIFICATION.md — completion gate

Reproduce everything with:

```
bash scripts/verify_all.sh     # phases A/D: features, symbols, all differential tests
bash scripts/mutation_check.sh # proves the differential harness is not vacuous
```

## Completion gate

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols in Rust.**
      `diff` of the two symbol lists is empty; all **5** C symbols
      (`safe_double_to_int`, `process_with_fallthrough`, `copy_data_block`,
      `handle_pointer_operations`, `overunder`) are exported by the Rust
      `cdylib` under the exact same names, in both the `debug` and `release`
      profiles. The Rust `.so`'s undefined list contains only libc / libgcc
      unwind symbols. No C source file or function was left untranslated —
      `c_src/src/lib.c` is the only translation unit and all 5 of its functions
      plus the `DataBlock` type are present.
- [x] **Phase B: every one of the 40 rows in `CONFIGS.md` passes across
      randomized inputs.** Fixed seed `0x5EED_1234_ABCD_F00D` (SplitMix64),
      per-row derived streams so rows are reproducible independently of
      execution order.
- [x] **Phase C: every one of the 26 rows in `ERRORS.md` has a passing
      error-path differential test** asserting the *same* sentinel / error
      value (not merely "both failed"), plus the generic FFI boundaries: NULL
      pointers, zero and extreme scalars, one step past every documented range,
      and out-of-range enum-like `int` values crossing the FFI boundary.
- [x] **All of the above hold under every build configuration.**

## Build configurations enumerated (Phase A)

`Cargo.toml` declares **no `[features]` table** and `c_src/CMakeLists.txt`
declares **no `option()` / `add_definitions()`**, and `c_src/src/lib.c` contains
**zero `#if`/`#ifdef`** — so there is exactly **one** feature combination, which
is simultaneously the default. `scripts/verify_all.sh` derives this
mechanically from `Cargo.toml` (it builds the power set of declared features, so
it stays correct if features are added later) and verifies:

| configuration | `cargo check` | `cargo build` | symbol parity | Phase B | Phase C |
|---|---|---|---|---|---|
| `--no-default-features` (empty ≡ only combo) | ok | ok | 5/5 | ok | ok |
| crate default | ok | ok | 5/5 | ok | ok |

Because the C relies on signed-overflow wraparound (which is UB and could in
principle be optimised differently), and because `[profile.release]` sets
`panic = "abort"`, the script additionally runs the **full** Phase B + Phase C
suites over a 3 × 2 robustness matrix — **all 18 runs pass**:

| C flags | Rust `debug` | Rust `release` |
|---------|--------------|----------------|
| `-O0` (the CMake default: `CMAKE_BUILD_TYPE` is empty) | ok | ok |
| `-O2` | ok | ok |
| `-O3` | ok | ok |

## Test inventory

| binary | harness | tests | covers |
|--------|---------|-------|--------|
| `tests/phase_b_configs.rs` | libtest | 16 | `CONFIGS.md` C1–C24 (the non-printing leaf entry points) |
| `tests/phase_c_errors.rs` | libtest | 18 + 1 helper | `ERRORS.md` E1–E18, E25 + generic FFI boundary rows |
| `tests/phase_overunder.rs` | custom, single-threaded | 19 | `CONFIGS.md` C25–C40 and `ERRORS.md` E19–E24, E26 |

`tests/phase_overunder.rs` uses `harness = false` because `overunder` prints via
libc `printf`: each call is wrapped in an fd‑1 redirect so the stdout bytes can
be compared exactly, and fd 1 is process-global — libtest's own parallel
progress output would otherwise interleave into the capture. (That interleaving
was the cause of the only spurious failures seen during bring-up; the C and Rust
payload bytes were identical.)

**Both libraries are always driven through `dlopen`/`dlsym` (`libloading`).**
No test references the Rust crate directly (`grep` for
`extern crate translated_rust` / `use overunder_lib` returns 0 hits), so the
`#[unsafe(no_mangle)] extern "C"` export wrappers are themselves under test.
For `overunder`, **both** the `int` return value **and** the captured stdout
bytes are compared with `assert_eq!`.

## Bug found and fixed

`copy_data_block` was translated with `std::ptr::copy_nonoverlapping`. The C is
`memcpy(dest, src, sizeof(DataBlock))` and performs **no NULL check**, so a NULL
argument faults with `SIGSEGV`. `ptr::copy_nonoverlapping` instead trips a
`core` UB precondition assertion — the process died with a non-unwinding Rust
panic ("unsafe precondition(s) violated: ptr::copy_nonoverlapping requires that
both pointer arguments are aligned and non-null…") and a **different**
termination signal than the C. Fixed by calling libc `memcpy` directly (which is
literally what the C does), for both the struct copy in `copy_data_block` and
the array copy in `overunder`. Verified: with the fix, C and Rust die with the
identical signal (`SIGSEGV`, 11) for `dest == NULL`, `src == NULL`, and both
NULL; without it, `err_e16_null_pointers_fault_identically` fails. This is
recorded as the last entry of `scripts/mutation_check.sh`.

## Harness validation (mutation testing)

`scripts/mutation_check.sh` injects 11 behaviour-changing mutations into
`src/lib.rs` and requires each to be caught. **All 11 are caught**, covering
every function and both the return-value and stdout comparison channels:
fall-through deltas, both clamp thresholds, the NaN arm, the `2.7` factor, C vs.
Euclidean modulo, loss of the `int` overflow wrap in `d*d + a*a`, the `"Source"`
label, the `+100` in `handle_pointer_operations`, the 40-byte copy length, and
the `memcpy` fault behaviour. `src/lib.rs` is restored afterwards.

One mutation is deliberately excluded as a **semantically equivalent mutant**,
not a coverage gap: `d > (double)INT_MAX` → `d >= (double)INT_MAX`. The only
input distinguishing the predicates is `d == 2147483647.0` exactly, and there
the clamp arm and the `(int)d` arm both yield `2147483647`; `(double)INT_MAX` is
exactly representable so nothing else can differ. The mirrored case
`d < INT_MIN` → `d <= INT_MIN` at `d == -2147483648.0` is equivalent for the
same reason. Both boundary inputs are asserted explicitly by
`err_e8_e9_inrange_boundaries`.

## Notes on the C's intentional quirks (replicated verbatim, not "fixed")

* the `switch` in `process_with_fallthrough` deliberately falls through
  `5 → 4 → 3 → 2 → 1` (`+150 / +100 / +60 / +30 / +10`);
* `case 0` **discards** `base_value` rather than adding to it;
* `default` returns the sentinel `-1`, discarding `base_value`;
* `int total = 0;` is dead — it is unconditionally overwritten later;
* `d * d + a * a` is evaluated in `int`, so it wraps and can feed a negative
  value to `sqrt`, yielding `NaN` → `conv4 == 0`;
* `a % 6` uses C truncated-toward-zero modulo, so negative `a` reaches `default`;
* `handle_pointer_operations` takes the address of a local for no reason;
* `source_block`'s padding bytes are uninitialised in C and are copied by
  `memcpy`; they are never observable through any printed field, which the
  byte-exact stdout comparison confirms.

Nothing in `c_src/` was modified. The only directory added there is
`c_src/build/`, which the task's own build instructions specify; the `-O2`/`-O3`
robustness builds are created out-of-tree under `$TMPDIR`.
