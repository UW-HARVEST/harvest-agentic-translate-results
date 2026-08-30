# CONFIGS.md — Configuration surface table (valid inputs)

Mechanically derived from the C source's branch set. The library has exactly
one runtime option and two data inputs; everything else is a data *shape*.

## Axes the C code actually branches on

| axis | where in C | values the C distinguishes |
|------|-----------|----------------------------|
| A1. `driver`'s mode flag `useGood` | `driver.c:91` `if (useGood)` | `0` (⇒ `bad`) vs. any non-zero `int` (⇒ `good`). Tested over the whole `int` domain, incl. low-byte-zero and sign-bit-set patterns. |
| A2. entry-point level | `driver.h` + `nm -D` | high-level one-shot wrapper `driver`; mid-level `good`, `bad`; **low-level leaf printers `printLine`, `printHexCharLine`** (called directly, not only through the wrappers) |
| A3. `printLine` pointer shape | `driver.c:32` `line != NULL` | NULL / empty / 1 byte / many bytes / 32 KiB; ASCII / high-bit / non-UTF-8; containing `%`, `\n`, `\t`, `\\`, `"` ; NUL-terminated mid-buffer |
| A4. `printHexCharLine` value shape | `driver.c:40` `%02x` + default arg promotion | `0`; `1..15` (needs the `0` pad); `16..127` (2 digits, no pad); `-1..-128` (sign-extended ⇒ **8** digits, pad ignored); the full 256-value domain; caller passing an out-of-`char` `int` |
| A5. `char` signedness | `limits.h` `CHAR_MAX`, `char result = data * 2` | target is `x86_64-linux` ⇒ `char` is **signed**, `CHAR_MAX == 127`, `CHAR_MAX/2 == 63`. Rust mirrors it via `std::ffi::c_char::MAX`, so it stays correct on unsigned-`char` targets too. |
| A6. `good`'s two internal sub-modes | `driver.c:86-87` | `goodG2B` (accept branch: `2 < 63` ⇒ prints) **then** `goodB2G` (reject branch: `127 < 63` false ⇒ prints message). Ordering of the two lines is part of the contract. |
| A7. call multiplicity / interleaving | (no state in C) | 1 call; N calls; C-then-Rust and Rust-then-C interleavings on the shared `stdout` |
| A8. cargo feature set | `Cargo.toml` | **no `[features]` table exists** ⇒ single configuration (default == none == all) |
| A9. cargo profile / optimisation level | `Cargo.toml` `[profile.release] panic="abort"` | `dev` (`-O0`, unwind) vs `release` (`-O3`, `panic=abort`). **Not cosmetic:** the C-vs-Rust ABI divergence in row C5 only manifests at `-O`, so the whole matrix is run in both profiles by `scripts/check_features.sh`. |

`#ifdef` axes: `grep -c '#if' c_src/src/driver.c` → 0 (only the `DRIVER_H_`
include guard in the header). No compile-time configuration exists.

## Configuration table

Each row is compared byte-for-byte between the C `.so` and the Rust `.so`
through `dlopen`/`dlsym`, with stdout captured per call. Rows marked
*randomized* use a fixed-seed xorshift64\* PRNG (seed `0x2545F4914F6CDD1D`)
so runs are reproducible.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `printHexCharLine` | **exhaustive**: all 256 `char` bit patterns `0x00..0xFF` (i.e. `-128..=127`), one call each | [x] |
| C2 | `printHexCharLine` | zero-pad boundary shapes: `0`, `1`, `9`, `0x0F`, `0x10`, `0x7F` (2-digit / padded paths only) | [x] |
| C3 | `printHexCharLine` | negative / sign-extension shapes: `-1`, `-2`, `-16`, `-127`, `-128` (8-digit paths) | [x] |
| C4 | `printHexCharLine` | *randomized*: 4096 random `char` values | [x] |
| C5 | `printHexCharLine` | caller passes a full `int` outside `char` range (ABI truncation): `128`, `255`, `256`, `-129`, `0x1234_5678`, `INT_MAX`, `INT_MIN` + 1024 randomized `i32` — **this row found a real `--release`-only divergence, see ERRORS.md row E7** | [x] |
| C6 | `printLine` | NULL pointer (the guarded shape) | [x] |
| C7 | `printLine` | empty string `""` | [x] |
| C8 | `printLine` | single-byte strings, exhaustive over all 255 non-NUL byte values `0x01..0xFF` | [x] |
| C9 | `printLine` | *randomized*: 512 random byte strings, length 0..=64, bytes `0x01..=0xFF` (non-UTF-8 included) | [x] |
| C10 | `printLine` | *randomized*: 64 long strings, length 1 KiB..32 KiB (crosses libc's 4 KiB stdio buffer ⇒ exercises partial flushes) | [x] |
| C11 | `printLine` | format-specifier payloads: `"%s"`, `"%d %d %d"`, `"%n"`, `"%%"`, `"100%"` — must be printed literally | [x] |
| C12 | `printLine` | embedded whitespace/control: `"\n"`, `"a\nb"`, `"\t"`, `"\r\n"`, `"a\x00b"` (⇒ truncated at the NUL by C) | [x] |
| C13 | `printLine` | *randomized*: 256 printable-ASCII strings, length 1..=200 | [x] |
| C14 | `bad` | no options; the single fixed path (`data = CHAR_MAX`, guard passes, overflowing `*2`) | [x] |
| C15 | `bad` | called 100× in a row (statelessness / no accumulation) | [x] |
| C16 | `good` | no options; both sub-modes in order (`goodG2B` accept line, then `goodB2G` reject line) — 2 lines, order-sensitive | [x] |
| C17 | `good` | called 100× in a row (statelessness) | [x] |
| C18 | `driver` | `useGood = 0` ⇒ `bad()` path | [x] |
| C19 | `driver` | `useGood = 1` ⇒ `good()` path | [x] |
| C20 | `driver` | `useGood` small non-zero: `2`, `3`, `-1`, `7`, `42` ⇒ `good()` path | [x] |
| C21 | `driver` | `useGood` low-byte-zero non-zero: `256`, `512`, `0x10000`, `0x0100_0000`, `INT_MIN` ⇒ `good()` path (guards against a `!= 0` → `as u8 != 0` mistranslation) | [x] |
| C22 | `driver` | `useGood` extremes: `INT_MAX`, `INT_MIN`, `-2147483647`, `0x7FFF_FFFF`, `0x8000_0001u32 as i32` | [x] |
| C23 | `driver` | *randomized*: 4096 uniformly random `i32` values (full 2^32 bit-pattern domain, incl. the exactly-one zero) | [x] |
| C24 | `driver` | *randomized*: 2048 values drawn from a zero-biased distribution (50 % exact `0`) so both branches are hit densely | [x] |
| C25 | mixed pipeline | *randomized* program of 512 ops over ALL 5 entry points in random order (`driver(r)`, `good()`, `bad()`, `printLine(rand)`, `printHexCharLine(rand)`), whole transcript compared as one byte stream — the composed pipeline, not per-wrapper | [x] |
| C26 | mixed pipeline | interleaving order flipped (Rust program run first, then C) + repeated twice, to prove no cross-library / cross-call state | [x] |
| C27 | all 5 | all rows above under feature set **default** (== none == all; `Cargo.toml` has no `[features]`) and under `--no-default-features`, via `scripts/check_features.sh` | [x] |
| C28 | all 5 | all rows above under the **`release` profile** (`-O3`, `panic="abort"`) as well as `dev`, via `scripts/check_features.sh` | [x] |

All 28 rows pass in both profiles. See `tests/valid_paths.rs` (C1–C27),
`tests/error_paths.rs`, `tests/symbol_parity.rs`, and
`scripts/check_features.sh` for C27/C28.
