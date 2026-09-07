# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`. There are no
`#ifdef`s, no build-time options and no global/runtime state — every axis is a
**parameter** of the single entry point.

## Axes the C actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A. `ignore` | `NULL` vs non-`NULL` | L23 `ignore != ((void *)0)` |
| A′. `ignore` contents | empty `""`, whitespace set, set containing a *hex* digit char, set containing high bytes | L24 `strchr(ignore, c)` |
| B. `hex_end_p` | `NULL` vs non-`NULL` — changes whether "scan stopped early" is an error | L50 / L52 |
| C. `state` at the byte in question | `0x00` (high nibble) vs `0xFF` (low nibble) — gates both the ignore-skip (L23) and which of `c_acc=`/`bin[bin_pos++]=` runs (L35) | L23, L35, L43 |
| D. `bin_maxlen` vs needed | `0`, exactly-enough, more than enough (`usize::MAX`), one byte short, far short | L31 |
| E. `hex_len` | `0`, `1`, `2`, odd, even, "many" (≥ 512) | L16 |
| F. byte class of `hex[i]` | `'0'..'9'` (`c_num0` path), `'A'..'F'`, `'a'..'f'` (`c_alpha0` path, mixed case via `c & ~32U`), non-hex-but-ignorable, non-hex-not-ignorable, `0x00`, high-bit `0x80..0xFF` | L18–L28 |
| G. position of the non-hex byte | leading, interleaved at even nibble, interleaved at odd nibble, trailing, absent | interaction of C × F |

There is exactly **one public entry point** (`hex2bin`) and it *is* the
lowest-level one — `include/lib.h` declares nothing else, and `nm -D` on the C
`.so` exports nothing else. There are no convenience wrappers to skip past.

The project builds **no binary executable** (`CMakeLists.txt` only has
`add_library(... SHARED ...)`; `Cargo.toml` only has `[lib] crate-type =
["cdylib"]`), so the "compare binaries' stdout" clause does not apply.

## Rows (pruned cross-product; each is a differential test over randomized inputs)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len=0` (empty input) | [x] |
| 2 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len=2` (single byte, all-lowercase digits) | [x] |
| 3 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len` even, all `'0'..'9'` (pure `c_num0` path) | [x] |
| 4 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len` even, all `'a'..'f'` (pure `c_alpha0` lowercase path) | [x] |
| 5 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len` even, all `'A'..'F'` (pure `c_alpha0` uppercase path) | [x] |
| 6 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len` even, randomized **mixed-case** hex (both paths interleaved, exercises `c & ~32U`) | [x] |
| 7 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len` **large** (512–1024 digits) mixed case | [x] |
| 8 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` **larger than needed** (slack buffer), valid even hex | [x] |
| 9 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen = usize::MAX` (oversized), valid even hex | [x] |
| 10 | `hex2bin` | `ignore=NULL`, `hex_end_p=**non-NULL**`, `bin_maxlen` exact, valid even hex — assert returned `*hex_end_p` offset == `hex_len` | [x] |
| 11 | `hex2bin` | `ignore=NULL`, `hex_end_p=non-NULL`, valid hex followed by **trailing non-hex garbage** — success + `*hex_end_p` at the first garbage byte | [x] |
| 12 | `hex2bin` | `ignore=NULL`, `hex_end_p=non-NULL`, `hex_len=1` (single digit, odd) — the `hex_pos--` path, assert the *decremented* end pointer | [x] |
| 13 | `hex2bin` | `ignore=non-NULL` (`" "`), `hex_end_p=NULL`, spaces at **even** nibble positions only (skippable), otherwise valid even hex | [x] |
| 14 | `hex2bin` | `ignore=non-NULL` (`" \t\n\r:-"`), `hex_end_p=NULL`, multi-char ignore set, ignorables at even positions, runs of 1..3 consecutive ignorables | [x] |
| 15 | `hex2bin` | `ignore=non-NULL`, `hex_end_p=NULL`, **leading** ignorables before the first digit | [x] |
| 16 | `hex2bin` | `ignore=non-NULL`, `hex_end_p=non-NULL`, **trailing** ignorables after the last digit — they are consumed, so `*hex_end_p == &hex[hex_len]` and the call succeeds | [x] |
| 17 | `hex2bin` | `ignore=non-NULL`, `hex_end_p=non-NULL`, ignorable byte at an **odd** nibble (breaks; row 6 of ERRORS.md from the success side — assert the end pointer position) | [x] |
| 18 | `hex2bin` | `ignore=""` (empty non-NULL string), `hex_end_p=non-NULL`, valid even hex — behaves like a set that matches only `0x00` | [x] |
| 19 | `hex2bin` | `ignore` **contains hex digit chars** (e.g. `"abc0"`), `hex_end_p=non-NULL` — the L22 `(c_num0\|c_alpha0)==0` test wins, so those chars are still *decoded*, never ignored | [x] |
| 20 | `hex2bin` | `ignore` contains **high-bit bytes** (`"\x80\xC3\xFF"`), `hex` contains those bytes at even nibbles — skipped (`strchr` on `unsigned char`) | [x] |
| 21 | `hex2bin` | `ignore=non-NULL`, `hex` contains an **embedded `0x00`** at an even nibble — skipped because `strchr` matches its own terminator (C quirk) | [x] |
| 22 | `hex2bin` | `ignore=NULL`, `hex` contains an embedded `0x00` — breaks; with `hex_end_p=non-NULL` the call still "succeeds" up to that point | [x] |
| 23 | `hex2bin` | `ignore=non-NULL`, `hex_end_p=non-NULL`, `bin_maxlen` **one byte short**, hex containing ignorables (buffer-full path composed with the ignore path) | [x] |
| 24 | `hex2bin` | **All 256 byte values** as a single input byte, and as the 2nd byte after a valid digit, across `ignore ∈ {NULL, "", " \t"}` × `hex_end_p ∈ {NULL, non-NULL}` (byte-class exhaustive) | [x] |
| 25 | `hex2bin` | Fully randomized fuzz: random `hex` over the full `0x00..0xFF` byte range, random `hex_len ∈ [0,64]`, random `bin_maxlen ∈ [0,40]`, random `ignore ∈ {NULL, "", " ", " \t\n:", "0aF", "\x80\xFF"}`, random `hex_end_p ∈ {NULL, non-NULL}` — 200 000 cases, fixed seed | [x] |
| 26 | `hex2bin` | Nibble-boundary sweep: for `n` valid digits, `n ∈ [0,17]`, × `bin_maxlen ∈ [0, 10]` × `hex_end_p ∈ {NULL, non-NULL}` — exhausts the odd/even × buffer-fit cross-product | [x] |
| 27 | `hex2bin` | `hex=NULL`/`bin=NULL` with `hex_len=0`, `bin_maxlen=0`, both `hex_end_p` variants — null pointers must not be dereferenced and `*hex_end_p` must come back `NULL` | [x] |
| 28 | `hex2bin` | Output-buffer **witness** check: `bin` prefilled with a poison pattern, assert the full `bin_maxlen` window is byte-identical between C and Rust *including on the error paths* (the C does not roll back already-written bytes) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. Verified mechanically by
`./check_features.sh`.

## Where each row is tested

`tests/configs.rs` — `rowNN_*` maps 1:1 onto the row numbers above. Every row
calls both `.so`s and compares the return value, the whole output window and the
`hex_end_p` pointer; all randomized rows use a fixed seed (2000 iterations per
row, 200 000 for row 25).

Additional suites that back these rows up:

| file | purpose |
|------|---------|
| `tests/exhaustive.rs` | ALL inputs of length 0–4 over 7 byte classes and 0–6 over 4 byte classes, × `bin_maxlen` × `ignore` × `hex_end_p`; plus all 256 single-byte and all 65 536 two-byte inputs. Makes the cross-product complete rather than sampled. |
| `tests/coverage_witness.rs` | Non-vacuity guard: classifies 200 000 fuzz cases by observable behaviour and fails if any of the 16 outcome classes stops being reached. |
| `negative_control.sh` | Mutation testing: injects 12 known bugs into `src/lib.rs`; 11 must be caught, and M5 is asserted to be a *provably equivalent* mutant. Proves the suite is not vacuously green. |
| `run_all.sh` | Phase D driver: builds the C `.so`, enumerates the cargo feature powerset, diffs `nm -D` per combination, runs the suite for each, repeats against the release `.so`, and re-runs everything against the C compiled at `-O0/-O1/-O2/-O3/-Os`. |

Reproduce everything: `cd translation && ./run_all.sh` then `./negative_control.sh`.
