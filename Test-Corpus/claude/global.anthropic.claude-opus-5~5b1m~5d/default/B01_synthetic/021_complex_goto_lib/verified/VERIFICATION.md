# Verification report — `c_src/src/driver.c` → `translation/src/lib.rs`

## Scope

The C library is a single translation unit exposing a single public function:

```c
void driver(int x, int y);   /* c_src/include/driver.h */
```

It returns nothing and allocates nothing; its entire observable contract is the
byte stream it writes to stdout with `printf`. Both implementations are loaded
as shared objects with `libloading` and invoked only through their exported
`driver` symbol, so the `#[no_mangle] extern "C"` wrapper is under test too.

## How to reproduce

```bash
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation
cargo test --release          # Phases B, C, D
./run_all_combos.sh           # all profile x feature combinations + symbol diff
./mutation_check.sh           # proves the suite is actually sensitive
```

(The crate is built `--offline`; `libloading 0.8.9` comes from the local cargo
cache. `.cargo/config.toml` sets `net.offline = true` so no command needs the
network.)

## Completion gate

| gate | status | evidence |
|------|--------|----------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** | `tests/symbols.rs::phase_d_symbol_parity`; both `.so`s export exactly `driver` |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** | `tests/configs.rs::phase_b_all_config_rows`, rows C1–C16, ~4000 input pairs from a fixed-seed xorshift64\* |
| Binary stdout comparison | **N/A** | neither `CMakeLists.txt` nor `Cargo.toml` builds an executable; stdout is instead compared per-call via fd-1 redirection |
| Phase C: every `ERRORS.md` row has a passing error-path differential test | **PASS** | `tests/errors.rs::phase_c_all_error_rows`, rows E1–E12 |
| All of the above under every feature combination | **PASS** | `./run_all_combos.sh` → `ALL COMBINATIONS PASSED` (6 combinations: {release, debug} × {default, no-default-features, all-features}) |

## Outcome for the translation itself

**No divergence was found.** `src/lib.rs` was not modified to fix behaviour —
the translation of the `goto label1` / `goto label2` control flow into a nested
loop with a `skip_label1` flag is faithful to the C, including:

* the `x == 1 && y == 4` test being evaluated **once per `while` iteration**
  (the backwards `goto label1` jumps past it);
* `continue` inside the C `while` body re-testing the outer guard, not the
  inner backwards jump;
* `printf` (not Rust `println!`) so stdout buffering and the emitted bytes match.

The two changes made were to the *build and test harness*, not the library:

1. `crate-type = ["cdylib", "rlib"]` — without `rlib`, `cargo test` does not
   rebuild the cdylib and the suite silently tests a stale `.so`.
2. `libloading` added to `[dev-dependencies]`.

## Why the "PASS"es are trustworthy

Passing differential tests are only meaningful if the harness can actually see a
difference. Two harness defects were found and fixed during this work — both
initially produced *wrong* results (four bogus divergences, then a run of
spurious mutation "kills"):

* libtest's parallelism let its own progress output land inside the fd-1 capture
  window (fixed: one sequential `#[test]` per phase, plus flushing Rust's
  `Stdout` before each capture);
* `cargo test` refreshes `target/<profile>/deps/libdriver.so` but not
  `target/<profile>/libdriver.so` (fixed: load the newest candidate and assert
  it is not older than `src/lib.rs`).

`./mutation_check.sh` now pins this down empirically:

```
== negative control (semantics-preserving edit must PASS) ==
OK: null mutant survived
killed 14/14 mutants
all mutants killed
```

The 14 mutants cover every condition (`x > 0 || y > 0`, `x == 1 && y == 4`,
`x > 0`, `y == 0`, `x < 3`), both decrements, all three printed literals, and the
`skip_label1` reset. The null mutant confirms the suite does not fail
gratuitously.

Additional non-vacuity guards inside the tests themselves:

* E9 (non-terminating input `y < 0, x > 0`) includes a positive control,
  `driver(4, 4)`, which must be **observed to terminate** — so "both hung"
  cannot be satisfied by a broken detector.
* E10 requires ≥ 32 KiB of comparable driver output from each side before it
  will pass.
* C16 re-invokes the same `.so` handles repeatedly and requires byte-identical
  output each round, catching accidental state in the Rust port.
