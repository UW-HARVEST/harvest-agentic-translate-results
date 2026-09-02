# VERIFICATION.md — completion gate for the C-to-Rust differential verification

Ground truth: `c_src/` (never modified — `md5sum` of all three files re-checked
after every mutation experiment below). Subject: `translation/`.

## How to reproduce

```sh
# 1. C shared library (ground truth)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + the whole suite under every feature combination
cd translation && ./tests/feature_matrix.sh
```

`feature_matrix.sh` builds the C library, extracts the feature list from
`cargo metadata`, and for every combination runs `cargo check`, rebuilds the
Rust `.so`, diffs `nm -D`, and runs all three test binaries.

## Test layout

| file | phase | tests | covers |
|---|---|---|---|
| `tests/common/mod.rs` | harness | — | loads both `.so`s via `libloading`; forked, fd-1-redirected execution; differential assertions; SplitMix64 PRNG (seed `0x5EED_D00D`) |
| `tests/phase_b_configs.rs` | B | 16 | one test per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | C | 12 | one test per `ERRORS.md` row + generic FFI boundaries |
| `tests/phase_d_symbols.rs` | D | 4 | `nm -D` parity, unresolved-import check, `dlopen`/`dlsym` of every C export, artifact presence |
| `tests/feature_matrix.sh` | D | — | the whole suite × every feature combination |

**32 tests, all passing, in all 3 build configurations.**

Both libraries are driven exclusively through `dlopen`/`dlsym` on their exported
symbols — the Rust functions are never called directly, so the
`#[no_mangle] extern "C"` wrappers are themselves under test.

### Why forked execution

The library's only observable output is bytes on `stdout` (both GCC and LLVM
lower `printf("%s\n", line)` to `puts`). Each call script runs in a forked child
whose fd 1 is a temp file. This (a) is isolated, so cargo's parallel test
threads cannot corrupt each other's capture, and (b) makes the `SIGSEGV` that
`driver` is *supposed* to raise for negative `data` an observable outcome that
can be compared between the two builds. The child sets `RLIMIT_CORE = 0` and
`PR_SET_DUMPABLE = 0` purely for speed — without them each intentional crash is
handed to `systemd-coredump` at ~0.4 s a piece (Phase C measured at 188 s;
6 s with them). Neither call changes the signal number the parent observes.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** symbols exported by the C `.so` and
      missing from the Rust `.so` (`driver`, `printLine`; `printLine` is not in
      `driver.h` but is non-`static`, so it is part of the ABI and is exported).
      **0** unresolved non-libc undefined symbols in the Rust `.so`. Enforced by
      `sym_01`/`sym_02` and re-checked per feature combination by
      `feature_matrix.sh`. No symbol needed a new wrapper and no C source was
      found untranslated — `c_src` is a single translation unit containing
      exactly two function definitions, both present in `src/lib.rs`.
- [x] **Phase B** — all **16** `CONFIGS.md` rows pass across randomized inputs
      (~5 000 randomized cases per run plus exhaustive sweeps of `data ∈ [0,99]`,
      all 255 single-byte strings, and all 128 high bytes).
- [x] **Phase C** — all **9** `ERRORS.md` rows have a passing differential test,
      each asserting the *specific* observable (exact byte stream; exact signal
      number), plus 3 extra tests for the generic FFI boundaries: NULL pointer,
      zero length, oversized length, one step past each end of the valid range,
      and a randomized sweep of the entire `i32` domain.
- [x] **All feature combinations** — `translation/Cargo.toml` declares no
      `[features]`, so there is one build configuration; `feature_matrix.sh`
      nevertheless runs the full suite under `<default>`,
      `--no-default-features` and `--all-features` and all three are green.

## Divergences found and fixed

**None.** The Rust translation matched the C byte-for-byte and
status-for-status on every case tried, including the two deliberate
memory-safety defects in `driver.c` (the unchecked negative `data` reaching
`strncpy`'s length via sign extension, and the unchecked `dest[data]` index).
`src/lib.rs` was not modified during verification; only
`translation/Cargo.toml` (dev-dependencies), `translation/tests/`, and the three
Phase A markdown artifacts were added.

## Evidence the suite is not vacuous

A test suite that passes is only meaningful if it can fail. Seven mutations were
injected into `translation/src/lib.rs`, each built and run, then reverted
(`src/lib.rs` verified byte-identical to the original afterwards):

| # | mutation | caught by | result |
|---|----------|-----------|--------|
| 1 | `data < 100` → `data <= 100` (off-by-one guard) | `err_02` | FAILED — "stdout diverged [err_02 data=100]" |
| 2 | `data as usize` → `data.unsigned_abs() as usize` ("fixes" the negative-data defect) | `err_06` | FAILED — "termination status diverged [err_06 data=-1]" |
| 3 | `printLine`'s NULL check removed | `err_01` | FAILED — "termination status diverged [err_01 single NULL]" |
| 4 | `memset(source,'A',99)` → `98` | `cfg_06` | FAILED — "stdout diverged [cfg_06 data=99]" |
| 5 | format string `"%s\n"` → `"%s"` (no trailing newline) | `cfg_08` | FAILED — "stdout diverged [cfg_08 empty]" |
| 6 | `printLine` round-tripped through `String::from_utf8_lossy` | `cfg_15` | FAILED — "stdout diverged [cfg_15 all high bytes]" — invisible to the ASCII-only rows, which is why row 15 exists |
| 7 | diverge for the single rare value `data == 57` | `cfg_01`, `cfg_05`, `cfg_07`, `cfg_16` | FAILED — proves the randomized loops really explore the input space rather than short-circuiting |

Mutation 7 was *not* caught by `err_10`, which samples uniformly from the whole
`i32` range and so has ~2000/2³² odds of drawing 57. That is correct behaviour,
not a coverage gap: the `[0,99]` window is covered exhaustively by `cfg_02` and
densely by `cfg_01`/`cfg_05`.
