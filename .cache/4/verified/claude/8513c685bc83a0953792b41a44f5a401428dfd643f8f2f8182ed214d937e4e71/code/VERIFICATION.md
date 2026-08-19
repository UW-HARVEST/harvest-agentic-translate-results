# VERIFICATION.md — completion gate

C ground truth: `c_src/src/driver.c` (81 lines, one translation unit).
Rust translation: `src/lib.rs`.

## Result

**No behavioural divergence was found. `src/lib.rs` required zero changes.**

Everything the C library does — `printf`/`puts` byte formatting, `strtol` base-10
parsing semantics, `errno` handling, `INT_MIN`/`INT_MAX` range rejection, signed
wraparound of `bedrooms`, `double` accumulation of `bathrooms`, the persistent
`static the_house` state across calls, and even the unguarded NULL dereference —
matches byte-for-byte through the FFI boundary.

## How to reproduce

```sh
./verify_all.sh       # builds C .so, enumerates feature combos, runs all phases
./mutation_check.sh   # proves the suite catches injected bugs (restores lib.rs)
```

## Completion checklist

- [x] **`SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.**
      C exports `{driver, run}`; Rust exports `{driver, run}`. Symbol diff = ∅.
      Enforced as a test (`tests/phase_d_symbols.rs`), which also loads both
      libraries with `RTLD_NOW` — a successful bind proves there are no
      unresolvable undefined symbols. The 6 `static` C functions and the `static`
      `the_house` object are correctly *not* exported by either library. No C
      module was skipped: `c_src/src/` holds exactly one file, fully translated.
- [x] **Phase B: every row in `CONFIGS.md` passes across randomized inputs.**
      24/24 rows, `tests/phase_b_configs.rs`. Rows 1–11 drive the **low-level**
      exported entry point `run` directly (it is absent from `driver.h` but has
      external linkage); rows 12–24 drive `driver`, including sequences that
      interleave both entry points against the shared mutable state.
- [x] **Phase C: every row in `ERRORS.md` has a passing error-path differential test.**
      11 table rows + 7 generic-boundary rows, `tests/phase_c_errors.rs`. Each
      asserts the *same specific* signal (the exact bytes `"An error occurred\n"`,
      plus no `run()` side effects) rather than "both failed somehow", and
      `assert_accepted` guards the range boundaries from the other side so an
      over-eager rejection cannot pass. `driver(NULL)` is checked for identical
      termination — both die on **SIGSEGV (11)**.
- [x] **All of the above hold under EVERY feature combination.**
      `Cargo.toml` declares **no `[features]`** and `c_src/CMakeLists.txt` has no
      `option()`/`add_definitions()`/conditional branches (the only `#if` is
      `driver.h`'s include guard), so the powerset is the single empty/default
      combination. `verify_all.sh` derives this mechanically and runs
      `cargo check`, `cargo check --all-targets`, and the full suite for it —
      under **both** the dev and release profiles, since `panic = "abort"` and
      optimisation apply only to release.

## Test inventory — 49 tests, all passing (dev and release)

| file | tests | covers |
|------|-------|--------|
| `tests/smoke.rs` | 3 | harness self-test: both `.so`s load, capture is non-vacuous, fork isolation gives pristine state |
| `tests/phase_b_configs.rs` | 24 | Phase B — `CONFIGS.md` rows 1–24 |
| `tests/phase_c_errors.rs` | 19 | Phase C — `ERRORS.md` rows 1–18 + a state-contamination test |
| `tests/phase_d_symbols.rs` | 3 | Phase D — symbol parity, `RTLD_NOW` resolution, static-linkage parity |

## Harness design (`tests/common/mod.rs`)

Both public functions return `void`, so their entire observable behaviour is the
bytes on `stdout`. The harness therefore:

1. loads **both** `.so`s via `libloading` and calls **only** their exported C
   symbols — the Rust implementation is never called directly, so the
   `#[no_mangle] extern "C"` wrappers are themselves under test;
2. never invokes either library from the parent process, so each library's
   private `static the_house` stays pristine;
3. `fork()`s once per scenario, so scenarios cannot contaminate one another and
   every one starts from `{floors: 2, bedrooms: 5, bathrooms: 2.5}`;
4. in the child, redirects fd 1 to a temp file, replays the op sequence against
   one library, `fflush`es, restores fd 1, and repeats for the other (each
   library owns an independent `the_house`, so back-to-back replay keeps their
   states in lockstep);
5. compares transcripts byte-for-byte in the parent, reporting the first
   differing line and the exact op that produced it.

Randomized rows use a fixed-seed xorshift64\* generator, so failures reproduce.

## Anti-vacuity evidence

`./mutation_check.sh` injects bugs into `src/lib.rs` one at a time and requires
the suite to fail. **25/25 real bugs were caught.** Three mutations are
classified as behaviour-preserving and correctly *not* caught; each is justified
rather than papered over:

| mutation | why it is unobservable |
|----------|------------------------|
| `floors` saturating instead of wrapping | `floors` starts at 2 and only ever `+1`, so the two differ solely after 2^31 `run()` calls — infeasible to reach (and UB in the C anyway) |
| dropping the `errno == 0` conjunct | proved non-decisive by a dedicated C probe over ~15 000 inputs: glibc base-10 `strtol` raises `ERANGE` only when the result saturates to `LONG_MIN`/`LONG_MAX`, which already fail the `INT_MIN`/`INT_MAX` conjunct |
| masking the low 32 bits before the cast | that line is reached only when `INT_MIN <= tmp <= INT_MAX`, where the mask-and-reinterpret is bit-identical to a truncating cast |

The Rust keeps both redundant C checks verbatim — the C is ground truth, so
redundant logic is translated, not optimised away.

## Notes

* `c_src/` was not modified; only `c_src/build/` (cmake output) was added, as the
  task instructs.
* The C `.so` imports `puts` rather than `printf` because gcc rewrites
  `printf("An error occurred\n")` into `puts("An error occurred")`. This is a
  codegen detail: the emitted bytes are identical, and the
  `error_message_no_newline` mutation confirms the tests would notice if they
  were not.
* The crate is `cdylib`-only, so integration tests do not link it and
  `cargo test` alone will not build the `.so`. `verify_all.sh` runs
  `cargo build` first; the harness panics with that instruction if the `.so` is
  missing.
