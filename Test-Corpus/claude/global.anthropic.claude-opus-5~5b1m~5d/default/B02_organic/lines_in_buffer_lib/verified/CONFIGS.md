# CONFIGS.md — configuration-surface table (Phase A → Phase B)

## Public entry points

`c_src/include/lib.h` declares exactly one:

```c
const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
```

There are no convenience wrappers and no lower-level helpers — this *is* the
lowest-level entry point, and the tests call it directly through `dlopen`/`dlsym`
(`libloading`) on both `.so`s.

## Axes the C actually branches on

Mechanically enumerated from every `if` / `while` in `c_src/src/lib.c`:

| axis | C site | states |
|------|--------|--------|
| A. allocation outcome | `if (bufferPtrs == NULL)` (l.10) | `malloc` succeeds / fails |
| B. any line requested | `lineIndex < numLines` (l.12) | `numLines == 0` / `numLines > 0` |
| C. buffer has room | `pos < bufferSize` (l.12) | `bufferSize == 0` / `> 0` / exhausted mid-loop |
| D. scan hits buffer end | `pos + len < bufferSize` (l.17) | inner loop exits on buffer end (**unterminated tail**) |
| E. scan hits terminator | `buffer[pos+len] != '\0'` (l.17) | NUL at `pos` (**empty segment**, `len == 0`) / NUL after `len > 0` / no NUL at all |
| F. terminator skip | `if (pos < bufferSize) pos++` (l.23) | taken (NUL strictly inside buffer) / **skipped** (segment ran to the last byte) |
| G. count reconciliation | `if (lineIndex != numLines)` (l.27) | equal ⇒ success / unequal ⇒ `NULL` |
| H. size multiplication | `numLines * sizeof(const char**)` (l.8) | no wrap / wraps modulo 2^64 |

There are **no** runtime option/mode/flag arguments, no global state, no
`#ifdef`-gated behaviour, and **no Cargo features** in `translation/Cargo.toml`
(`[features]` is absent, so the only feature combination is the default/empty
one; `cargo check --no-default-features` is identical). Byte order and element
type are fixed by the ABI (`char`, `size_t`).

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed-seed PCG32,
`SEED = 0x5DEECE66D`) except where the shape is fully determined.
"result" = the exact `NULL`-ness plus, when non-NULL, the byte-identical array
of `numLines` pointers (compared as offsets into the shared buffer *and* as raw
pointer bit patterns, since both libraries are handed the same `buffer`).

| # | entry point(s) | configuration (options set + input shape) | axes | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize = 0`, `buffer = NULL` | A ok, B=0, C=0, G eq | [x] |
| 2 | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize = 0`, `buffer` = valid non-null | A ok, B=0, C=0 | [x] |
| 3 | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize` random 1..=64, random bytes | B=0, C>0 (loop still skipped) | [x] |
| 4 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = 1`, `buffer = "\0"` | E: NUL at pos, `len=0`, F skipped (`pos`→1) | [x] |
| 5 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = 1`, `buffer = "A"` (no NUL) | D: exit on buffer end, F skipped | [x] |
| 6 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = n` (2..=64), **no NUL anywhere** | D, F skipped, G eq | [x] |
| 7 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = n`, single NUL strictly inside ⇒ trailing bytes ignored | E, F taken, G eq | [x] |
| 8 | `UTIL_createLinePointers` | `numLines = k`, buffer = exactly `k` NUL-terminated non-empty strings, `bufferSize` = exact total (last byte is NUL) | E, F taken every iteration, G eq | [x] |
| 9 | `UTIL_createLinePointers` | `numLines = k`, buffer = `k` segments where the **last is unterminated** (no trailing NUL) | mixes E (first `k-1`) with D+F-skipped (last) | [x] |
| 10 | `UTIL_createLinePointers` | `numLines = k`, `bufferSize = k`, buffer = **all NUL** ⇒ `k` empty lines | E with `len=0` every iteration; F taken for the first `k-1`, skipped for the last | [x] |
| 11 | `UTIL_createLinePointers` | `numLines = k`, buffer = **all NUL**, `bufferSize = k + extra` ⇒ early stop with leftover buffer | G eq, loop exits on `lineIndex` not `pos` | [x] |
| 12 | `UTIL_createLinePointers` | `numLines = k` **strictly less** than the number of segments present (leftover tail never scanned) | B stops loop first, G eq | [x] |
| 13 | `UTIL_createLinePointers` | `numLines = k`, mixed empty and non-empty segments (consecutive NULs interleaved), exact fit | E `len=0` and `len>0` interleaved | [x] |
| 14 | `UTIL_createLinePointers` | leading NUL (first line empty) + non-empty rest | E `len=0` on iteration 0 | [x] |
| 15 | `UTIL_createLinePointers` | trailing run of NULs longer than needed, `numLines` = segment count | F taken, G eq | [x] |
| 16 | `UTIL_createLinePointers` | bytes with the **high bit set** (`0x80..=0xFF`) as line content — must be treated as ordinary non-terminators (`char` is signed on x86-64) | E `!= '\0'` on negative `char` | [x] |
| 17 | `UTIL_createLinePointers` | content containing `'\n'`, `'\r'`, `0x7F` — **not** separators despite the "line" naming; only `'\0'` splits | E | [x] |
| 18 | `UTIL_createLinePointers` | large scale: `numLines = 1000`, ~1000 random-length segments, `bufferSize` ≈ 16 KiB | all of D/E/F, many iterations | [x] |
| 19 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize` random, buffer = random bytes (NUL may or may not be present) | D vs E chosen by data | [x] |
| 20 | `UTIL_createLinePointers` | **full fuzz**: `numLines` random 0..=24, `bufferSize` random 0..=48, random bytes with ~25% NUL density ⇒ success *and* `NULL` outcomes mixed, 20 000 cases | A,B,C,D,E,F,G jointly | [x] |
| 21 | `UTIL_createLinePointers` | **full fuzz, NUL-dense** (~75% NUL density), `numLines` 0..=24, `bufferSize` 0..=48, 20 000 cases | E `len=0` dominant | [x] |
| 22 | `UTIL_createLinePointers` | **full fuzz, NUL-free** (bytes 1..=255 only), `numLines` 0..=8, `bufferSize` 0..=48, 20 000 cases | D dominant, G usually unequal | [x] |
| 23 | `UTIL_createLinePointers` | boundary sweep: for every `bufferSize` in 0..=17 × every `numLines` in 0..=18 × every NUL mask over the buffer (exhaustive for `bufferSize ≤ 12`) | exhaustive small-input cross-product | [x] |
| 24 | `UTIL_createLinePointers` | size-multiplication wrap with defined outcome: `numLines ∈ {1<<61, (1<<61)+1, 1<<58}` with `bufferSize = 0` | H wrap + G unequal | [x] |
| 25 | `UTIL_createLinePointers` | allocation-failure configuration: `numLines ∈ {1<<60, 1<<62, SIZE_MAX, SIZE_MAX-1}`, `bufferSize` 0 and 8 | A fails ⇒ line 10 | [x] |
| 26 | `UTIL_createLinePointers` | repeated / interleaved invocations on the **same** buffer (alias the caller's memory across both libraries, alternating C→Rust→C) to prove there is no hidden global state | statelessness | [x] |

All 26 rows are checked off by `translation/tests/differential.rs` (Phase B) and
`translation/tests/errors.rs` (Phase C); rows 24–25 overlap with `ERRORS.md`
rows 1–4 by design.
