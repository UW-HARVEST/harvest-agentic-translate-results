# CONFIGS.md — Configuration-surface table (valid inputs)

## Axes the C actually branches on

Derived mechanically from `c_src/src/lib.c` / `c_src/include/lib.h`.

**Public entry points (the FULL set).** The header declares exactly one:

```c
char *encode_base64(int size, const char *src);
```

There is no lower-level public entry point to reach past it: `encode()` is
`static` and therefore not part of the ABI (confirmed absent from `nm -D`). So
"the lowest-level entry point" and "the convenience wrapper" are the same
function, and every row below drives it directly through the `.so` export.

**Runtime options / modes.** There is no option struct, no flag, no mode, no
global state, and no `#ifdef`. The only mode selector is the *value* of `size`:

| axis | branch in C | states |
|------|-------------|--------|
| A. length source | `if (!size) size = strlen(src);` (line 37) | A1 `size == 0` → length taken from `strlen` (NUL-terminated mode); A2 `size != 0` → caller-supplied length (explicit-length mode, allows embedded NULs) |
| B. `size mod 3` (tail shape) | `if (i + 1 < size)` (lines 53, 69) and `if (i + 2 < size)` (lines 57, 75) — only differ on the final iteration | B0 `size % 3 == 0` → no `=` padding; B1 `size % 3 == 1` → two `=`; B2 `size % 3 == 2` → one `=` |
| C. iteration count | `for (i = 0; i < size; i += 3)` (line 48) | C0 zero iterations (`size <= 0` after step A); C1 exactly one iteration (`1 <= size <= 3`); Cn many iterations |
| D. byte value → `encode()` arm | 5 arms at lines 8/11/14/17/21 | D1 `u < 26` (`A`–`Z`); D2 `26 <= u < 52` (`a`–`z`); D3 `52 <= u < 62` (`0`–`9`); D4 `u == 62` (`+`); D5 `u == 63` (`/`) |
| E. sign of the source byte | `unsigned char b1 = src[i];` — `char` is signed on this target, so bytes `>= 0x80` are converted via `(unsigned char)` | E1 all bytes `< 0x80` (ASCII); E2 bytes `>= 0x80` present (high-bit set) |
| F. embedded NUL bytes | only observable in mode A2 (in A1 a NUL terminates the length) | F1 no NULs; F2 NULs inside the payload |
| G. sign of `size` | `size * 4 / 3 + 4` in `int` arithmetic, plus loop guard | G+ positive; G0 zero (→ axis A1); G− negative (covered in `ERRORS.md` rows 2–8) |

`encode()`'s D-arms are reached purely as a function of the payload bytes, so
they are covered by sweeping payload values inside every other row rather than
as standalone rows — plus one dedicated exhaustive row (row 12).

**Comparison method for every row.** The C returns a `calloc`'d buffer whose
length is exactly `n = size * 4 / 3 + 4` bytes. Each test compares the **full
`n` bytes** of the C and Rust buffers (not just up to the first NUL), so the
trailing zero padding that `calloc` supplies is verified too, and frees both
with `free()`.

**Randomization.** Every row below is driven with many randomized payloads from
a fixed-seed xorshift64\* PRNG (seed `0x243F6A8885A308D3`), not a single
hand-picked value.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|------------------------------------------|-----|
| 1 | `encode_base64` | A1 (`size = 0`) + empty NUL-terminated string `""` → C0 zero iterations, `n = 4` | [x] |
| 2 | `encode_base64` | A1 (`size = 0`) + NUL-terminated ASCII, `strlen % 3 == 0` (lengths 3, 6, 9, …), E1/F1 | [x] |
| 3 | `encode_base64` | A1 (`size = 0`) + NUL-terminated ASCII, `strlen % 3 == 1` (lengths 1, 4, 7, …) → B1, two `=` | [x] |
| 4 | `encode_base64` | A1 (`size = 0`) + NUL-terminated ASCII, `strlen % 3 == 2` (lengths 2, 5, 8, …) → B2, one `=` | [x] |
| 5 | `encode_base64` | A1 (`size = 0`) + NUL-terminated payload with high-bit bytes `0x80..0xFF` (E2), all `strlen % 3` classes | [x] |
| 6 | `encode_base64` | A2 (explicit `size`) + `size == 1` → C1 single iteration, B1, `n = 5` | [x] |
| 7 | `encode_base64` | A2 (explicit `size`) + `size == 2` → C1 single iteration, B2, `n = 6` | [x] |
| 8 | `encode_base64` | A2 (explicit `size`) + `size == 3` → C1 single iteration, B0 no padding, `n = 8` | [x] |
| 9 | `encode_base64` | A2 (explicit `size`) + Cn many iterations, `size % 3 == 0`, random bytes over the full `0x00..0xFF` range (E1+E2, F2) | [x] |
| 10 | `encode_base64` | A2 (explicit `size`) + Cn many iterations, `size % 3 == 1`, random bytes full range (E1+E2, F2) | [x] |
| 11 | `encode_base64` | A2 (explicit `size`) + Cn many iterations, `size % 3 == 2`, random bytes full range (E1+E2, F2) | [x] |
| 12 | `encode_base64` | A2 (explicit `size`) + payload constructed to hit **all 64** `encode()` outputs, i.e. every D-arm D1–D5 including the `u == 62` (`+`) and `u == 63` (`/`) single-value arms | [x] |
| 13 | `encode_base64` | A2 (explicit `size`) + `size` strictly **less than** the real buffer length (partial encode; verifies the C never reads past `size` and never emits the extra bytes) | [x] |
| 14 | `encode_base64` | A2 (explicit `size`) + payload that is **all** `0x00` bytes (F2 extreme: output is all `A`, tests that NULs in the payload do not terminate the explicit-length loop) | [x] |
| 15 | `encode_base64` | A2 (explicit `size`) + payload that is all `0xFF` bytes (E2 extreme: exercises D5 `/` heavily) | [x] |
| 16 | `encode_base64` | A2 (explicit `size`) + long payloads (`size` 256…4096, all three `% 3` classes) — many iterations, checks `n` sizing and the trailing zero padding of the allocation | [x] |
| 17 | `encode_base64` | A1 (`size = 0`) where the payload has a NUL **in the middle**: `strlen` stops early, so only the prefix is encoded even though more bytes exist (interaction of axes A1 × F2) | [x] |
| 18 | `encode_base64` | A2 (explicit `size`) + every `size` in `1..=64` exhaustively, random payload each — sweeps all three `% 3` classes and the C0/C1/Cn iteration-count boundary in one shot | [x] |

| 19 | `encode_base64` | A2 + **exhaustive** sweep of all 2^24 three-byte payloads — covers every possible `(b4,b5,b6,b7)` 6-bit tuple and every bit position of the b1/b2/b3 → b4..b7 repacking | [x] |

All 19 rows pass across the randomized inputs (see
`tests/differential.rs::phase_b_*`). Row 19 is `#[ignore]`d by default because
it makes ~1.7 × 10^7 FFI round trips per implementation; it was run and passed:

```
$ cargo test --release -- --ignored
test phase_b_exhaustive_all_three_byte_payloads ... ok
```

Exhaustive (not merely randomized) coverage also applies to rows 6 and 7: row 6
sweeps all 256 one-byte payloads and row 7 all 65 536 two-byte payloads.
