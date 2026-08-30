# Verification report — `driver` (C → Rust)

Ground truth: `c_src/src/driver.c` + `c_src/include/driver.h`, compiled by cmake
into `c_src/build/libdriver.so`. Nothing in `c_src/` was modified.

Reproduce everything with:

```sh
cd translation && bash scripts/verify.sh
```

## What is under test

The Rust crate is a `cdylib`. **No Rust function is ever called directly.**
`tests/common/mod.rs` `dlopen`s *both* `libdriver.so` files with
`RTLD_NOW | RTLD_LOCAL` and `dlsym`s all five exports, so the
`#[no_mangle] extern "C"` wrappers are themselves under test. Every function in
this library returns `void` and communicates only through `stdout`, so the
harness redirects fd 1 to a scratch file around each call, `fflush(NULL)`s the
shared libc stream, and compares the captured bytes exactly.

## Phase A — surface maps

| artifact | content |
|----------|---------|
| `SYMBOLS.md`  | all 5 `nm -D` exports; C\Rust symbol diff is **empty**; `goodG2B`/`goodB2G` correctly stay unexported (they are `static` in C) |
| `ERRORS.md`   | 15 rows, mechanically derived from every guard/branch in `driver.c` (the library has no `assert`, no error return, no error enum — every function is `void`) |
| `CONFIGS.md`  | 28 rows over 9 axes (mode flag, entry-point level, pointer shape, char-value shape, `char` signedness, `good`'s two sub-modes, call multiplicity, feature set, **cargo profile**) |

## Phase B — valid paths (`tests/valid_paths.rs`, 27 tests)

All 28 `CONFIGS.md` rows pass. Highlights: `printHexCharLine` is exhaustive over
all 256 `char` bit patterns plus 4096 randomized values; `printLine` is
exhaustive over all 255 single-byte strings plus ~900 randomized byte strings
(including invalid UTF-8, `%`-specifier payloads, embedded NUL, and 1 KiB–1 MiB
payloads that cross libc's stdio buffer); `driver` gets 6144 randomized `i32`
values; and C25/C26 compare whole 512-op transcripts that interleave all five
entry points, so the composed pipeline is covered, not just per-wrapper calls.
All randomized rows use a fixed-seed xorshift64\* PRNG.

## Phase C — error paths (`tests/error_paths.rs`, 18 tests)

All 15 `ERRORS.md` rows pass. Each test both (a) diffs C against Rust and
(b) *pins the absolute expected bytes*, so a row cannot pass by both sides
failing the same way. Plus the generic boundaries: NULL pointer, zero-length and
oversized (1 MiB) buffers, values one step past `char` range in both directions,
and the full `int` bit-pattern domain for `driver`'s flag (a C `int` parameter
has no invalid variant to reject, so all 2^32 patterns are legal input — swept
with all 32 single-bit values, their complements, and 2048 random values).

## Phase D — parity, feature combos, profiles

* `tests/symbol_parity.rs` re-runs the `nm -D` diff as an assertion and pins the
  five documented API symbols plus the two `static` non-exports.
* `scripts/check_features.sh` derives the feature matrix from `Cargo.toml`.
  There is no `[features]` table, so the matrix is `<default>` and
  `--no-default-features` (identical builds), and the script additionally runs
  the whole suite under `--release`. A test (`d5`, `c27`) fails if a
  `[features]` table is ever added without widening the matrix.

## Bug found and fixed

**`printHexCharLine` did not truncate its argument register (release only).**

GCC compiles `void printHexCharLine(char charHex)` so the *callee* truncates:
`mov %edi,%eax; mov %al,-0x4(%rbp); movsbl -0x4(%rbp),%eax`. The C library's
output therefore depends only on the low byte of the argument register.

Rust's `extern "C" fn(charHex: c_char)` tags the parameter `signext`, i.e. it
*assumes* the caller already sign-extended. At `-O` LLVM elided the truncation
entirely (`mov %edi,%esi`), so the upper 24 bits leaked into `%02x`:

| call | C | Rust `--release` (before) | Rust (after) |
|------|---|---------------------------|--------------|
| `printHexCharLine(128)`  | `ffffff80` | `80`       | `ffffff80` |
| `printHexCharLine(-129)` | `7f`       | `ffffff7f` | `7f` |

Fix (`src/lib.rs`): the exported wrapper takes `c_int` and truncates explicitly,
reproducing GCC's codegen (`movsbl %dil,%esi`) for all 2^32 register values while
remaining indistinguishable from `fn(c_char)` for any correctly-extending caller.
The faithfully-typed body lives in `print_hex_char_line_impl(charHex: c_char)`.
This was found by `CONFIGS.md` row C5 / `ERRORS.md` row E7 running under
`--release`, and is guarded against regression by the
`printhex-no-abi-truncation` mutant.

## Harness self-validation

A green differential suite proves nothing unless it can go red.
`scripts/mutation_check.sh` injects 22 known mistranslations into `src/lib.rs`
one at a time and requires the suite to fail for each. **22/22 are caught, in
both `dev` and `release`.** The script's header lists six textual mutations that
are provably *observationally equivalent* (because `bad`/`goodG2B`/`goodB2G` take
no input and use hard-coded constants, e.g. `127 < 63` vs `127 <= 63`); those are
deliberately excluded rather than papered over.

## Notable C behaviours replicated (not "fixed")

* `printHexCharLine` sign-extends before `%02x`, so negative values print **eight**
  hex digits and the `02` width is not honoured (`-2` ⇒ `fffffffe`).
* `bad()` performs the signed-overflow `CHAR_MAX * 2` and truncates back to
  `char`; Rust uses `as` casts so it never panics, matching C.
* `goodB2G()`'s dead store `data = ' '` (line 68) is immediately overwritten by
  `data = CHAR_MAX` (line 69). Honouring it would flip the branch and print
  `40` instead of the rejection message; row E10 asserts it stays unobservable.
* `goodB2G()`'s range check `127 < CHAR_MAX/2` is always false, so the
  "safe multiply" branch is dead code and the rejection message always prints.
* `driver()` tests the whole `int` for non-zero, not its low byte or low bit.
* `CHAR_MAX` is derived from `std::ffi::c_char::MAX`, so the translation stays
  correct on targets where `char` is unsigned (e.g. aarch64-linux) instead of
  hard-coding 127.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols in the Rust `.so`, and 0
      unresolvable non-libc imports (checked in both profiles by `verify.sh`).
- [x] Phase B: every `CONFIGS.md` row passes across randomized inputs.
- [x] Phase C: every `ERRORS.md` row has a passing error-path differential test.
- [x] Holds under every feature combination (`<default>`,
      `--no-default-features`) **and** both cargo profiles (`dev`, `release`).
- [x] Suite validated by mutation testing (22/22 caught).
