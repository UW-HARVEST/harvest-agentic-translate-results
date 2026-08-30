# Verification report — C → Rust translation of `driver`

Ground truth: `c_src/` (never modified). Subject: this crate.

Both libraries are loaded as shared objects with `libloading` and are only ever
called through their exported C symbols — the Rust side is never linked or
called directly, so the `#[no_mangle] extern "C"` wrappers are themselves under
test.

## How to reproduce

```sh
# one command does everything (C build, symbol diff, all feature combos)
cd translation && ./run_all.sh

# or manually
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release --offline && cargo test --offline
```

`--offline` is used because the sandbox has no crates.io egress; `libloading
0.8.9` resolves from the local cargo cache.

## Artifacts

| file | phase | content |
|---|---|---|
| `SYMBOLS.md` | A / D | `nm -D` surface of both `.so`s and the (empty) symbol diff |
| `ERRORS.md` | A / C | 11-row error-surface table + observed UB outcomes |
| `CONFIGS.md` | A / B | 22-row configuration-surface table |
| `tests/common/mod.rs` | — | harness: dual `dlopen`, SplitMix64 RNG, forked stdout capture, forked crash-outcome comparison |
| `tests/valid_paths.rs` | B | 23 tests, one per `CONFIGS.md` row (+ a harness self-check) |
| `tests/error_paths.rs` | C | 12 tests, one per `ERRORS.md` row (+ a no-hidden-state test) |
| `run_all.sh` | D | symbol diff + tests across every feature combination |
| `mutation_check.sh` | — | negative control: proves the suite detects wrong translations |

## Results

* **Phase A** — the C library is one translation unit exporting exactly
  `driver` and `foo`. Both are implemented and exported by the Rust `.so`; no
  module was left untranslated and nothing is stubbed.
* **Phase B** — all 22 `CONFIGS.md` rows pass. Randomized, fixed-seed inputs
  (seed `0x5DEECE66D`): ~2 000 `foo` calls per row-group including the full
  255-value needle domain crossed with random 0x01..0xFF haystacks, 64 KiB and
  256 KiB buffers, adjacent-match runs, first/last-byte matches, and
  `driver` stdout compared **byte-for-byte** (not parsed) for every shape.
* **Phase C** — all 11 `ERRORS.md` rows pass, including the four
  undefined-behaviour rows, which are compared as *identical process
  termination* (same signal / same exit code) using forked children rather
  than the weaker "both failed somehow".
* **Phase D** — symbol diff is empty; 0 undefined non-libc symbols; all tests
  green under `default`, `--no-default-features` and `--all-features` (this
  crate declares no `[features]`, so that is the complete powerset — `run_all.sh`
  enumerates `[features]` from `Cargo.toml` and would expand the powerset
  automatically if any were added).
* **Negative control** — 6 deliberate mutations of `src/lib.rs` (wrong needle
  in the wrapper, rejecting negative needles, adding a null check the C lacks,
  an off-by-one count, a changed format string, and dropping the `s++` advance)
  were each *detected* by the suite. So the green result is meaningful rather
  than vacuous.

## Notable behaviours that had to be preserved exactly

1. **`foo` is exported even though `driver.h` does not declare it.** It is
   non-`static`, so it is part of the C library's ABI and must be exported.
2. **Signed `char` needle.** `foo`'s `c` parameter is `char` (signed on
   x86-64), promoted to `int` for `strchr`. A needle of `0x80..0xFF` arrives as
   a *negative* `int`; glibc `strchr` converts back to `char`, so those bytes
   match normally. A translation that guarded on `c >= 0`, or that used
   `u8`-based searching after a UTF-8 conversion, diverges — covered by
   `ERRORS.md` E5 and `CONFIGS.md` C10.
3. **`s++` after every match** — matches may be adjacent, and a match on the
   final byte leaves `s` exactly on the terminator (`CONFIGS.md` C6/C8).
4. **No input validation at all.** Null pointers, missing terminators and a
   `0` needle all fault in C; the Rust must fault the same way instead of
   returning a "safe" value (`ERRORS.md` E1–E4).
5. **`printf` output, not Rust formatting.** The translation calls libc
   `printf` with the identical `"A: %d\n"` / `"x: %d\n"` format strings, so
   the byte stream and the stdout buffering/ordering match; the *input* is
   never used as a format string (`ERRORS.md` E10).

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols for the Rust `.so`.
- [x] Phase B: every `CONFIGS.md` row passes across randomized inputs.
- [x] Phase C: every `ERRORS.md` row has a passing error-path differential test.
- [x] All of the above hold under every feature combination.

Final state: **35 differential tests, 0 failures, 0 warnings.** No changes were
needed to `src/lib.rs` — the translation was already byte-exact on every input
exercised, and the mutation run confirms the tests would have caught it if it
were not.
