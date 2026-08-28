# VERIFICATION.md — how the translation was verified, and what it found

Ground truth is `c_src/` (never modified). Everything below is produced by
scripts in `../scripts/` and tests in `tests/`; nothing is called directly in
Rust — the tests `dlopen` **both** shared objects and go through the dynamic
symbol table, so the `#[no_mangle]` export wrappers are part of what is tested.

## Artifacts

| file | phase | content |
|------|-------|---------|
| `SYMBOLS.md` | A / D | the 9 exported symbols of the C `.so`, their Rust counterparts, and the undefined-symbol audit |
| `ERRORS.md` | A / C | 24 rows: every way the C rejects/short-circuits input, plus the generic FFI boundaries |
| `CONFIGS.md` | A / B | 42 rows: the cross-product of build options, runtime global state and input shapes the C actually distinguishes |
| `VERIFICATION.md` | — | this summary |

## Build / run commands

```sh
# Phase A: every feature combination must compile (36 valid + 8 conflicting)
scripts/check_features.sh

# C shared library / genuine CMake executable for one configuration
scripts/build_c_so.sh  <op> <repeat>     # -> cbuild/so/libdriver_<op>_<repeat>.so
scripts/build_c_exe.sh <op> <repeat>     # -> cbuild/exe/<op>_<repeat>/driver

# Phases B+C+D for the 24 (OP, REPEAT) configurations
scripts/run_all.sh

# Phases B+C+D for the remaining 18 feature spellings
scripts/run_combos.sh --spellings

# Phase D symbol parity for all 24 configurations
scripts/check_symbols.sh
```

## Results

| gate | result |
|------|--------|
| `cargo check --no-default-features --features <combo>` | 44/44 combinations compile with **zero warnings** |
| `nm -D` parity | 9/9 C exports present in the Rust `.so` for all 24 configurations; symbol diff **empty**; type/bind/visibility and data-object sizes identical; 0 non-libc undefined symbols |
| Phase B (`CONFIGS.md`) | all 42 rows pass for all 42 builds |
| Phase C (`ERRORS.md`) | all 24 rows pass for all 42 builds |
| CMake end-to-end | the real `driver` executable's stdout/stderr/exit status matches the Rust `.so`'s `main` for 10 argv shapes, per configuration |
| totals | 39 tests × 42 builds = **1638 test executions**, 0 failures; each test loops over boundary values plus 64–1024 seeded random inputs |

## Bugs found and fixed in the Rust translation

1. **`mdmain.c` had never been translated** — the `main` symbol was simply
   absent from the Rust `.so` (8 of 9 exports present). Translated in full as
   `src/mdmain.rs`; it is a real translation, not a stub (argv parsing with
   `atoi`, the unrolled `REP<REPEAT>` accumulator, the three helper calls,
   dispatch through the `G_OP` global, and both `printf` lines).
2. **`helper_ptr` read the `G_OP` global.** The C initializes its local
   `int (*fp)(int,int)` from the macro `OP_FN(OP)`, so a caller that overwrites
   `G_OP` must *not* affect `helper_ptr` (only `main`'s `g.call` follows the
   global). Fixed to use the build-time op. Detected by `c16`/`e19`.
3. **`G_OP` / `G_OP_NAME` were not writable.** As plain Rust `static`s they were
   emitted into `.data.rel.ro`, which RELRO maps read-only, while the C globals
   are non-`const` and live in writable `.data` (`nm -D` shows `D` for both, so
   the symbol table hid this). Any caller storing into `G_OP` — something the C
   program fully supports and which changes `main`'s dispatch — crashed against
   the Rust `.so`. Fixed with `static mut`; verified with `readelf -S`/`-sW`.
4. **`println!` instead of `printf`.** Replaced by direct `printf`/`fprintf`
   calls into glibc (`src/cshim.rs`), which is what the C does. This matters for
   `%s` with a NULL pointer (`G_OP_NAME = NULL` must print `op=(null)`, and
   `argv[0] == NULL` must print `usage: (null) A B`), for `stdout` buffering
   semantics when a C caller interleaves its own output, and for ignoring write
   errors. `atoi` is likewise taken from libc so out-of-range and partially
   numeric operand text truncates/saturates exactly as glibc does.

## Negative controls (evidence the suite has teeth)

| injected defect | detected by |
|-----------------|-------------|
| C `.so` built with `REPEAT=6` compared against the `REPEAT=5` Rust build | 7 of 12 `valid` tests fail, e.g. `helper_call(0,0) return mismatch: C=15 Rust=10` |
| `helper_ptr` reading `G_OP` (defect #2 re-introduced) | `c16`, `c31_c32`, `c40_c41`, `c42` fail (`C=1 Rust=-1`) |
| `G_OP` as an immutable `static` (defect #3 re-introduced) | the suite dies at `c16` with `signal: 11, SIGSEGV` |

## Notes on faithfulness decisions

* `accum_<OP>` is `static` in C, so it has no exported symbol; the Rust
  counterpart is a private `fn`. `DISPATCH_REP`'s `switch` only has `case 0..6`,
  so `use_generated(n)` returns `INIT_FOR(OP)` unchanged for every other `n`
  (including `7`, which is exactly what a `REPEAT=7` build passes it) — the Rust
  reproduces that instead of "fixing" the inconsistency.
* All integer arithmetic uses `wrapping_*`, matching what the C toolchain emits
  for these expressions (checked differentially against `INT_MIN`/`INT_MAX`
  operands for every op, `helper_call`'s `r + acc`, and `main`'s six-term
  `summary`).
* `[lib] test = false` in `Cargo.toml`: because the crate exports a `#[no_mangle]
  main`, a libtest harness for the lib target would produce two `main` entry
  symbols. Verification lives in `tests/`, which loads the built `.so`, so
  nothing is lost.
