# CONFIGS.md — Configuration surface table (Phase A, gates Phase B)

Derived mechanically from `c_src/include/driver.h` + `c_src/src/driver.c`.

## Axes the C code actually branches on

**Runtime options / modes (the only one the public API can set):**

* `driver(int useGood)` — a two-mode selector, `if (useGood)`. State it toggles:
  `good()` (⇒ `goodG2B()` then `goodB2G()`) vs `bad()`. Any non-zero `int` is
  mode "good"; exactly `0` is mode "bad". No other option, flag, global, or
  `#ifdef` exists in the library (`grep -c '#if' src/driver.c` → only the
  header's `#ifndef DRIVER_H_` include guard).

**Public entry points — the FULL set, low-level first (from `nm -D`, not just
the `driver.h` convenience wrapper):**

| level | entry point | note |
|---|---|---|
| lowest | `printHexCharLine(char)` | leaf output primitive |
| lowest | `printLine(const char*)`  | leaf output primitive |
| middle | `bad(void)`               | composed of `printHexCharLine` |
| middle | `good(void)`              | composed of both leaves via the two `static` helpers |
| top    | `driver(int)`             | the only entry point declared in `driver.h` |

Note `bad`, `good`, `printLine`, `printHexCharLine` are exported but **not
declared in the header** — they are still part of the ABI surface a real
consumer can reach by `dlsym`, so they are driven directly below.

**Input shapes the code special-cases:**

* `printLine`: null vs non-null (explicit check); empty vs 1-byte vs many-byte;
  interior NUL (C string truncation); bytes ≥ 0x80; format specifiers; 64 KiB.
* `printHexCharLine`: sign of the `char` (drives `%02x` printing 2 digits vs 8
  after the int promotion); zero (zero-pad path); the full 256-value domain;
  dirty upper argument-register bits.
* `driver`: zero vs non-zero; sign; low-byte-zero-but-non-zero; `INT_MIN` /
  `INT_MAX` boundaries.

**Observable compared:** the exact stdout byte stream. Both libraries write
through the *same* process glibc `stdout`, so the harness redirects fd 1 to a
file, calls the symbol, `fflush(NULL)`s, and compares the bytes.

## Rows (cross-product, pruned to what the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `printHexCharLine` | exhaustive sweep of the entire domain: all 256 byte values `0x00..0xFF` reinterpreted as the platform `char` | [x] |
| C2 | `printHexCharLine` | randomized (seeded LCG, 4096 draws) over the full byte domain — value-dependent path coverage | [x] |
| C3 | `printHexCharLine` | boundary values only: `0`, `1`, `0x0F`, `0x10`, `0x7F` (`CHAR_MAX`), `0x80` (`CHAR_MIN`), `0xFF` — the 2-digit/8-digit and zero-pad transitions | [x] |
| C4 | `printHexCharLine` | argument passed with dirty upper register bits (int-valued arg into a `char` parameter): `0x1FF`, `0xDEADBE7F`, `-256` | [x] |
| C5 | `printLine` | `NULL` (null-check branch) | [x] |
| C6 | `printLine` | empty string `""` (zero length) | [x] |
| C7 | `printLine` | single-byte strings, exhaustive over all 255 non-NUL byte values | [x] |
| C8 | `printLine` | randomized (seeded, 512 draws) strings, length 0..64, bytes drawn from the full non-NUL range `0x01..0xFF` | [x] |
| C9 | `printLine` | strings with an interior NUL — C truncates at the first NUL | [x] |
| C10 | `printLine` | strings containing `printf` conversion specifiers (`%s`, `%d`, `%n`, `%p`, `%%`) as data | [x] |
| C11 | `printLine` | the exact literal `goodB2G` uses: `"data value is too large to perform arithmetic safely."` | [x] |
| C12 | `printLine` | oversized: 1 KiB, 4 KiB (stdio buffer boundary), 64 KiB strings | [x] |
| C13 | `bad` | no options — direct low-level call; exercises the `data = CHAR_MAX` overflow path (`127 * 2` truncated to `char`) | [x] |
| C14 | `bad` | called repeatedly (16×) — confirms no state carried between calls | [x] |
| C15 | `good` | no options — direct call; exercises the composed pipeline `goodG2B` **then** `goodB2G` (two output lines, order-sensitive) | [x] |
| C16 | `good` | called repeatedly (16×) — confirms no state carried between calls | [x] |
| C17 | `driver` | mode "bad": `useGood == 0` | [x] |
| C18 | `driver` | mode "good": `useGood == 1` | [x] |
| C19 | `driver` | mode "good" via other truthy shapes: `-1`, `2`, `42`, `0x100`, `0x7FFFFFFF` (`INT_MAX`), `0x80000000` (`INT_MIN`), `0xFFFFFF00` | [x] |
| C20 | `driver` | randomized (seeded, 2048 draws) over the full `i32` domain, mixing zero and non-zero | [x] |
| C21 | mixed pipeline | interleaved sequence across ALL five entry points in one capture (`driver(0)`, `printLine`, `printHexCharLine`, `bad`, `good`, `driver(1)`, …) driven from a seeded random program — catches divergence only visible in composition / stdio buffering | [x] |
| C22 | mixed pipeline | randomized 256-step program of random entry points with random arguments, single capture, byte-compared as a whole | [x] |

All 22 rows have a passing differential test — see `tests/differential.rs`
(`phase_b_*` tests).

## Feature combinations

`Cargo.toml` has no `[features]` table ⇒ exactly one combination (default =
empty = `--no-default-features`). Enumerated mechanically by
`check_features.sh`; the test suite is run under it.
