# CONFIGS.md — Configuration-surface table (valid inputs)

Mirror of `ERRORS.md` for inputs the C **accepts**. Derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

### Axis O — runtime options / modes / flags: **NONE**

`c_src/include/lib.h` declares exactly one function and no types, no enums, no
handle/context struct, no setters. `grep`ping `c_src/` for `option`, `flag`,
`mode`, `#ifdef`, `switch` yields nothing. There is no persistent state, no byte
order selection, no format selection. **The entire configuration surface is the
shape of the three arguments.**

### Axis P — public entry points: **exactly one**

`UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize)`.
There is no convenience/one-shot wrapper layered over a lower-level API — this
*is* the lowest-level entry point, and every row below drives it directly through
the `.so` export.

### Axis N — `numLines` (relative to the line count actually present)

`n0` = 0 · `n1` = 1 · `n2` = 2 · `nM` = many (3…) · `n=` = exactly the number of
lines present · `n<` = fewer than present (early success exit with `pos < bufferSize`)
(`n>` = more than present → rejection, see `ERRORS.md` rows 5–7)

### Axis S — `bufferSize` (relative to the bytes the requested lines consume)

`s0` = 0 · `s1` = 1 · `s=` = exactly the bytes consumed · `s+` = larger, slack
bytes left unscanned · `sT` = ends mid-line so the last line is truncated

### Axis C — buffer content shape (what line 17 sees)

`c1` every line NUL-terminated, last NUL is the final byte ·
`c2` last line unterminated ·
`c3` zero-length lines from consecutive NULs (leading / interior / trailing) ·
`c4` all-NUL buffer (`bufferSize` empty lines) ·
`c5` no NUL anywhere ·
`c6` high-bit bytes `0x80..0xFF` present (C compares a **signed** `char` against
`'\0'`; Rust compares `c_char` = `i8`) ·
`c7` one very long line (> 4 KiB) ·
`c8` random bytes with random NUL density

### Axis B — line 23 `if (pos < bufferSize) pos++` for the **last** line processed

`B+` = taken (a NUL was found and is consumed) · `B-` = not taken (`pos` reached
`bufferSize`, no terminator to skip)

### Axis E — inner-loop (line 17) exit reason

`Enul` = `buffer[pos+len] == '\0'` · `Eend` = `pos + len == bufferSize`

### Axis A — allocation size `numLines * sizeof(const char**)` (line 8)

`A0` = `malloc(0)` (`numLines == 0`) · `An` = ordinary size · (wrapping sizes →
`ERRORS.md` rows 8–11)

## Configuration table

Every row is run through **both** `.so` exports with **many randomized inputs**
(fixed seed, deterministic PRNG) and the returned pointer array is compared
element-by-element for byte-identical values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `UTIL_createLinePointers` | `n0`,`s0`, real non-NULL buffer, `A0` — `malloc(0)` returned, zero slots written, outer loop skipped | [x] |
| 2 | `UTIL_createLinePointers` | `n0`,`s+`, non-empty buffer, `A0` — loop skipped via `lineIndex < numLines`, buffer never read | [x] |
| 3 | `UTIL_createLinePointers` | `n1`,`s1`,`c4` (`"\0"`) — one empty line, `Enul`, `B+`, `pos` ends `== bufferSize` | [x] |
| 4 | `UTIL_createLinePointers` | `n1`,`s1`,`c5` (`"A"`) — one unterminated 1-byte line, `Eend`, `B-` | [x] |
| 5 | `UTIL_createLinePointers` | `n1`,`s1`,`c6` (`"\xFF"`) — high-bit byte must **not** count as terminator, `Eend`, `B-` | [x] |
| 6 | `UTIL_createLinePointers` | `n1`,`s=`,`c1` (`"hello\0"`) — one terminated line, exact fit, `Enul`, `B+` | [x] |
| 7 | `UTIL_createLinePointers` | `n1`,`sT`,`c5` (`"hello"`, size 5) — one line truncated by `bufferSize`, `Eend`, `B-` | [x] |
| 8 | `UTIL_createLinePointers` | `n1`,`s+`,`c1` — one line requested, buffer holds slack after it; success exits on `lineIndex == numLines` with `pos < bufferSize` | [x] |
| 9 | `UTIL_createLinePointers` | `n2`,`s=`,`c1` (`"a\0b\0"`) — two terminated lines, exact fit, both `Enul`/`B+` | [x] |
| 10 | `UTIL_createLinePointers` | `n2`,`s=`,`c2` (`"a\0bb"`) — first line `Enul`/`B+`, last line `Eend`/`B-` (mixed branches in one call) | [x] |
| 11 | `UTIL_createLinePointers` | `nM`,`s=`,`c1` — many (3…64) terminated lines, exact fit | [x] |
| 12 | `UTIL_createLinePointers` | `nM`,`s=`,`c2` — many lines, final one unterminated | [x] |
| 13 | `UTIL_createLinePointers` | `nM`,`s=`,`c3` leading empty line (`"\0abc\0"`) | [x] |
| 14 | `UTIL_createLinePointers` | `nM`,`s=`,`c3` interior consecutive NULs (`"a\0\0b\0"`) — zero-length line in the middle | [x] |
| 15 | `UTIL_createLinePointers` | `nM`,`s=`,`c3` trailing extra NUL (`"a\0\0"`) — trailing zero-length line | [x] |
| 16 | `UTIL_createLinePointers` | `n=`,`s=`,`c4` all-NUL buffer, `numLines == bufferSize` — maximum achievable line count, every line zero-length | [x] |
| 17 | `UTIL_createLinePointers` | `n<`,`s+`,`c4` all-NUL buffer, `numLines < bufferSize` — early success exit mid-buffer | [x] |
| 18 | `UTIL_createLinePointers` | `n<`,`s+`,`c1` — buffer holds more lines than requested; unscanned tail | [x] |
| 19 | `UTIL_createLinePointers` | `n1`,`s=`,`c7` single 8193-byte unterminated long line (`Eend`) and 8192+NUL terminated variant (`Enul`) | [x] |
| 20 | `UTIL_createLinePointers` | `nM`,`s=`,`c6` many lines whose payloads are random `0x01..0xFF` bytes incl. high-bit | [x] |
| 21 | `UTIL_createLinePointers` | `nM`,`s+`/`sT`,`c8` **property test**: random buffer bytes, random NUL density (dense→sparse), random `bufferSize`, random `numLines ≤ lines present` — 4 000 randomized cases | [x] |
| 22 | `UTIL_createLinePointers` | `n=`/`n<`,`s=`/`s+`,`c3`+`c2` **property test**: randomly generated line-length vectors including zero-length lines and a randomly terminated/unterminated tail — 4 000 randomized cases | [x] |
| 23 | `UTIL_createLinePointers` | `nM` large: 100 000 lines over a 200 000-byte all-NUL-separated buffer, `An` large allocation | [x] |
| 24 | `UTIL_createLinePointers` | `n1`,`s1`..`s64` sweep over every `bufferSize` 1..64 on an all-`0x41` buffer and on an all-NUL buffer, `numLines` swept 0..bufferSize (full 2-D boundary sweep of the two range checks) | [x] |
| 25 | `UTIL_createLinePointers` | allocation-size parity: `malloc_usable_size` of the returned block compared between C and Rust for `numLines` 0…79, 100, 255…257, 1000, 4096, 8192, 16000, 16381 — pins `sizeof(const char**) == 8` and the `numLines * sizeof(...)` product | [x] |

## Notes

* **Same buffer, both callees.** Every row hands the *same* `buffer` pointer to
  both `.so`s (the C function only reads `buffer`), so the returned `const char*`
  values are directly comparable as raw pointers, not merely as offsets.
* **Cross-check against an independent model.** Each row also compares the agreed
  result against `common::model()`, a line-by-line re-implementation of
  `c_src/src/lib.c`, so a row cannot pass by both sides being wrong in the same
  way, and cannot pass vacuously by both returning `NULL`
  (`assert_ok` forbids that where success is expected).
* **Row 25 range limit.** `malloc_usable_size` is only a valid differential
  observable below glibc's 128 KiB mmap threshold. Measured: it first diverges at
  `numLines == 16382` (131 056 bytes) *for an identical request*, because glibc's
  dynamic mmap threshold depends on allocator history. Row 25 therefore stops at
  16 381. Element size is independently pinned by every other row, since slot `i`
  is read back at byte offset `8*i`.
