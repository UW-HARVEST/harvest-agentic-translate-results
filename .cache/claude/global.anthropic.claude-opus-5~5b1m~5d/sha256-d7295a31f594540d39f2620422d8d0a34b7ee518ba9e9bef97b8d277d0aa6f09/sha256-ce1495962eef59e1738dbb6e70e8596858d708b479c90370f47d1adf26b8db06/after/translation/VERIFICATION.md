# Verification report — C `driver` vs Rust `driver`

## How to reproduce

```sh
cd translation
./run_all.sh          # builds the C .so, then every feature combo x profile
./mutation_check.sh   # proves the suite is non-vacuous
```

`run_all.sh` passes `--offline` because this sandbox has no crates.io access;
`libloading 0.8.9` and `libc 0.2.x` are already in the local cargo cache.

## Test architecture

Both libraries are loaded with `libloading::Library::new` and invoked **only**
through the exported `driver` symbol — the Rust `#[unsafe(no_mangle)] extern
"C"` wrapper is therefore exercised exactly as an external consumer would. No
Rust function is ever called directly.

`driver` returns `void`; its entire observable contract is the byte stream it
writes to `stdout`. The harness (`tests/common/mod.rs`) captures that by
`dup2`-ing a temp file over fd 1 around each call and `fflush(NULL)`-ing
afterwards. The C `.so`, the Rust `.so` and the test binary share one
dynamically-linked libc, hence one `stdout` FILE, so this captures both.

### Two harness bugs found and fixed during verification

1. **Parallel-test contamination.** With libtest, cases run on multiple threads
   and libtest itself prints progress to fd 1. Both leaked into the captures
   (e.g. `"test c6_only_z_invalid ... Error: x == 1 but y != 2\n..."`), producing
   6 bogus failures. Fixed by switching to a custom `harness = false` runner
   (`tests/differential.rs`) that runs cases strictly sequentially and writes
   all of its own diagnostics to **stderr**.
2. **Stale-artifact false PASS (the dangerous one).** `cargo test` does *not*
   rebuild or re-uplift a `crate-type = ["cdylib"]` library, because no test
   target links it. The tests were loading an old `target/release/libdriver.so`
   and **a deliberately broken translation still passed all 30 cases.** Fixed by
   `assert_not_stale()`, which fails loudly if the `.so` is older than
   `src/lib.rs`/`Cargo.toml` (and likewise for the C `.so` vs `driver.c`), plus
   `run_all.sh` always building before testing.

## Phase results

| phase | artifact | result |
|-------|----------|--------|
| A | `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md` | written from the C source |
| B | 15 rows in `CONFIGS.md` | all pass (randomized, fixed seed) |
| C | 12 rows in `ERRORS.md` | all pass |
| D | symbol parity + all feature combos | symbol diff empty; all combos pass |

30 cases x 2 feature combos x 2 profiles = **120 case runs, 0 failures**,
covering ~50 000 differential `driver` invocations.

## Completion gate

- [x] **`SYMBOLS.md`: 0 missing / undefined non-libc symbols in Rust.**
      C exports exactly one symbol (`driver`); Rust exports `driver`. `comm -23`
      of the two `nm -D` lists is empty. Every Rust undefined symbol is a
      libc/libgcc runtime import (asserted by `d3_...`). The C `static`
      internals `multi_stage` and `y` are exported by neither (asserted by
      `d2_...`), and `driver` resolves through `dlsym` in both (`d4_...`).
      No C module was missing, so nothing needed translating; nothing is stubbed.
- [x] **Phase B: every `CONFIGS.md` row passes across randomized inputs.**
      Includes the exhaustive `[-4, 8]³` cube (2197 triples), the 12³ extreme
      cross-product (1728 triples), and the full `i32` range via a seeded
      xorshift64* PRNG.
- [x] **Phase C: every `ERRORS.md` row has a passing differential test.**
      All three `goto fail` branches, the shared `fail:` epilogue, both
      check-ordering/short-circuit properties, `INT_MIN`/`INT_MAX`, `0`/`-1`,
      one-step-past-valid for each parameter, and arbitrary
      "out-of-range enum" ints (the C signature is `int`, so every 32-bit value
      is a legal input). There are no pointer parameters, so no null-pointer
      row exists; `B5` records that explicitly.
- [x] **All of the above under every feature combination.**
      `Cargo.toml` declares no `[features]`, so the complete set is
      *default* and `--no-default-features`; `run_all.sh` runs both in both
      release and debug. Verified by `cargo check --all-targets` + `build` +
      `test` per combination.

## Non-vacuity (mutation testing)

`./mutation_check.sh` injects 18 deliberate bugs into `src/lib.rs`, rebuilds and
re-runs the suite. **17 are caught**; the 18th is a provably **equivalent
mutant**:

> Changing `static Y: AtomicI32 = AtomicI32::new(123)` to `new(2)` is
> undetectable, because `driver` unconditionally executes
> `Y.store(local_y, …)` before `multi_stage` ever reads `Y`, and `Y` is not
> exported. The C original has exactly the same dead initialiser
> (`static int y = 123;`), so no input to the public API can distinguish it.

Mutations caught include every comparison constant, every status code, every
message string (including letter case and the trailing newline of
`Result: %d\n`), dropping the `fail:` epilogue, dropping the `y` write, and
swapping `multi_stage`'s argument order.

## Notes on translation fidelity

* **`puts` vs `printf`.** The C compiler rewrites the constant-format
  `printf("...\n")` calls into `puts("...")`; the Rust build routes them
  through `printf("%s", …)`. This is an internal codegen detail — the bytes
  written to `stdout`, their order, and the stdio buffering are identical,
  which is what every test asserts. No message contains a `%`, so the `%s`
  form is safe.
* **`static int y` -> `AtomicI32`.** The C global is a plain `int`; the Rust
  translation uses a relaxed atomic to express the same mutable-global
  semantics without `static mut`. For single-threaded use — the only use the C
  version defines, since concurrent `driver` calls are a data race and thus UB
  in C — behaviour is identical, and `C12`/`C13`/`B6` verify state persistence
  across long call sequences. Concurrent callers are therefore out of scope for
  differential comparison (the C behaviour is undefined); the Rust version is
  strictly better-defined there.
* The `y = 123` initialiser is dead in both implementations; `C11` runs in a
  fresh library-load position to confirm the very first `driver` call behaves
  identically.

**Conclusion: the Rust translation is byte-identical to the C ground truth
across every configuration and error path enumerated from the C source. No
divergence was found, so no changes to `src/lib.rs` were required.**
