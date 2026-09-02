# VERIFICATION.md — completion gate

Differential verification of the Rust translation in `translation/` against the
C ground truth in `c_src/`. Reproduce everything with:

```sh
./scripts/verify_all.sh        # builds both .so's, all feature combos x both profiles
./scripts/mutation_check.sh    # proves the suite actually detects divergence
```

## Method

Every call crosses the FFI boundary. Both shared objects are loaded with
`libloading` (`dlopen` + `RTLD_NOW`) and driven only through their exported
symbols — the Rust crate is never called as a Rust library, so the
`#[no_mangle]` / `extern "C"` wrappers are themselves under test.

All five exported functions return `void` and communicate solely by writing to
the C `stdout`, so `tests/harness/mod.rs` captures output at the **file
descriptor** level (`dup`/`dup2` onto a temp fd, `fflush(NULL)`, restore) and
compares the two byte streams. Captures are serialised through a mutex because
fd 1 is process-global.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** C symbols missing from the Rust
      `.so` (5 of 5: `printLine`, `printIntLine`, `bad`, `good`, `driver`) and
      **0** missing/undefined non-libc symbols in the Rust `.so`. No C module
      was left untranslated (`c_src/` has exactly one translation unit).
      Enforced as tests `sym_01`–`sym_03`, and re-checked by
      `verify_all.sh` step 5 for both profiles.
- [x] **Phase B** — all **26** rows of `CONFIGS.md` pass, with seeded
      randomized inputs per row (512 random `i32` for `printIntLine`, 512 random
      flags for `driver`, a 1024-op mixed pipeline across all five entry points,
      length sweeps to 256 KiB, all 255 non-NUL byte values). Low-level entry
      points (`printIntLine`, `printLine`) and mid-level ones (`bad`, `good`) are
      driven directly, not only through the `driver` wrapper; row 25 additionally
      asserts the wrapper composes them exactly as the C does.
- [x] **Phase C** — all **7** rows of `ERRORS.md` have a passing error-path
      differential test, plus the generic boundaries: NULL pointer, zero length,
      oversized length, one-step-past-range `int` values, and out-of-range
      enum-like values crossing the boundary (`driver`'s `int` flag accepts all
      2^32 values; C truthiness means only `0` selects `bad()`).
- [x] **Every feature combination** — `Cargo.toml` declares no `[features]`
      section, so the complete set is `default` and `--no-default-features`.
      `verify_all.sh` derives the list from `Cargo.toml` mechanically and runs
      the full suite for each, against **both** the dev-profile and the
      release-profile cdylib (different codegen: inlining, `panic = "abort"`).
      Result: **43/43 cases in all 4 configurations.**

## Divergences found and fixed

None in the Rust implementation — it already matched the C on every input
tested. The defects found were in the **verification** itself, by mutation
testing:

| # | problem | fix |
|---|---------|-----|
| 1 | `struct_03` (an early "printLine has a null guard" disassembly check) passed or failed depending on which profile's `.so` happened to be newest, and produced a **false kill** in the mutation campaign. In a dev build Rust lowers the null check to an out-of-line `core::ptr::const_ptr::is_null` call, so no `cmp $0x0` appears in `printLine`'s body. | Test deleted — the guard is fully observable (`err_01`), so the structural check added brittleness and no coverage. |
| 2 | `rust_so()` picked "whichever profile's `.so` is newest", making every structural result nondeterministic. | Pinned to the dev-profile cdylib, overridable via `DRIVER_RUST_SO`; `verify_all.sh` runs both profiles explicitly. |
| 3 | `verify_all.sh` counted test cases with `grep -c '^test .* ok$'` under `cargo test -q`, which prints no per-test lines — a run that executed **zero** tests reported "ok". | Counts are summed from `test result: ok. N passed` and asserted against the number of `#[test]` attributes; too few cases now FAILS. |
| 4 | Phases B and C were **blind** to `driver` branch inversion and to `bad` forwarding to `good`, because C's `bad()` and `good()` print identical bytes. Mutants `driver_inverted` and `fold_bad_into_good` left the suite green. | Added `struct_03`/`struct_04`: branch **direction** and call graph resolved from `objdump -d` plus the `objdump -R` relocation table (handles direct PLT calls in C and GOT-indirect calls/tail-calls in Rust, dev and release). Both mutants are now killed. |

## Mutation campaign result

`scripts/mutation_check.sh`: **10 killed, 0 unexpected survivors.**

Killed: `drop_null_guard`, `driver_eq_one`, `driver_inverted`,
`int_format_space`, `int_format_unsigned`, `int_format_hex`, `line_no_newline`,
`line_payload_as_format`, `source_nonzero`, `fold_bad_into_good`.

Two mutants survive **by design** and are declared as expected:
`bad_prints_index_1` and `loop_off_by_one`. `source[10] = {0}` is all zeros and
only `data[0]` is ever printed, so every index and every loop bound >= 1 prints
the same `0`. No consumer of the *original C library* can distinguish these
either, so there is nothing for a differential test to observe. Recorded in
`CONFIGS.md`; verified instead by reading the Rust against the C line for line.

## Notes on preserved C behaviour

- **The `bad()` defect is preserved, not repaired.** C `bad()` calls
  `alloca(10)` — 10 *bytes* — then writes ten `int`s (40 bytes) into it. The
  Rust keeps the undersized request as a `black_box`'d value while backing the
  region with enough stack for the ten writes, so the overrun cannot corrupt
  unrelated runtime state. `err_10` hammers `bad()` 512× interleaved with the
  other four entry points and confirms the overrun stays unobservable on both
  sides.
- **`printf` vs `puts`.** GCC rewrites `printf("%s\n", line)` into `puts(line)`,
  so the C `.so` imports `puts`; the Rust calls `printf` directly. Byte-identical
  output, and covered by test rather than assumed.
- **`driver`'s flag is `int`, not a bool.** `if (useGood)` is C truthiness. Every
  non-zero value — including `INT_MIN`, `-1` and `0x100` — selects `good()`;
  only `0` selects `bad()`. Nothing is validated and nothing is rejected.

## Files

| file | contents |
|------|----------|
| `SYMBOLS.md` | exported-symbol parity, derived from `nm -D` |
| `ERRORS.md` | error-surface table, 7 rows, all checked |
| `CONFIGS.md` | configuration-surface table, 26 rows, all checked |
| `tests/harness/mod.rs` | `libloading` loader for both `.so`s, fd-level stdout capture, seeded PRNG |
| `tests/valid_paths.rs` | Phase B — 26 tests, one per `CONFIGS.md` row |
| `tests/error_paths.rs` | Phase C — 10 tests covering the 7 `ERRORS.md` rows + generic boundaries |
| `tests/symbol_parity.rs` | Phase D — 3 tests, `nm -D` diff must be empty |
| `tests/structural.rs` | 4 tests pinning the stdout-invisible branch direction and call graph |
| `../scripts/verify_all.sh` | feature-combination + both-profile verification driver |
| `../scripts/mutation_check.sh` | mutation campaign proving suite sensitivity |
