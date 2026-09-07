# CONFIGS.md — Phase A configuration-surface table (VALID inputs)

## Axes the C actually branches on

`c_src/include/lib.h` exposes exactly one entry point, and it is the
lowest-level one — there is no convenience wrapper layer, no init/teardown
state, no options struct, and no runtime flag. `c_src/CMakeLists.txt` defines no
`option()`, no `target_compile_definitions`, and `src/lib.c` contains no
`#ifdef`. `translation/Cargo.toml` declares no `[features]` table, so the only
feature combination is the default one.

The branches the C takes therefore depend purely on the three arguments:

- **A. `numLines`** — `0` / `1` / `2` / many; and relative to the number of
  NUL-separated segments actually reachable in the buffer: fewer / exactly equal
  / more (the "more" case is the error surface, see `ERRORS.md`).
- **B. `bufferSize`** — `0` / `1` / small / large; and relative to the real
  storage backing `buffer`: equal / smaller (deliberate truncation).
- **C. buffer byte shape** — the inner scan (`pos+len < bufferSize` and
  `buffer[pos+len] != '\0'`) plus the conditional `if (pos < bufferSize) pos++`
  make these shapes distinct:
  - no NUL anywhere (single unterminated segment consuming the whole buffer);
  - exactly one trailing NUL;
  - every segment NUL-terminated, last NUL exactly at `bufferSize-1`;
  - final segment unterminated (NUL missing at the very end);
  - consecutive NULs → zero-length lines;
  - all bytes NUL;
  - random bytes with a random NUL density (arbitrary segment lengths);
  - NULs present only beyond `bufferSize` (invisible to the scan).
- **D. `buffer` pointer** — valid; or `NULL` in the configurations where the
  loop provably never dereferences it (`numLines == 0`, or `bufferSize == 0`).

Observable output compared byte-for-byte: NULL vs non-NULL, and — when
non-NULL — all `numLines` pointer entries of the returned array. Both
implementations receive the *same* `buffer` address, so the entries are compared
as raw absolute pointers as well as offsets.

Every row is exercised with many randomized inputs (fixed seed, deterministic
xorshift PRNG in the test harness), not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize = 0`, `buffer` valid — zero-length request, no scan | [x] |
| 2 | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize` random > 0, buffer random bytes — no scan despite content | [x] |
| 3 | `UTIL_createLinePointers` | `numLines = 0`, `buffer = NULL`, `bufferSize` random > 0 — NULL never dereferenced | [x] |
| 4 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = 1`, buffer = `"\0"` — single empty line, NUL at last byte | [x] |
| 5 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = 1`, buffer = one random non-NUL byte — unterminated 1-byte line | [x] |
| 6 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = n`, buffer = random non-NUL bytes, **no NUL at all** — `pos` ends exactly at `bufferSize`, `pos++` skipped | [x] |
| 7 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = n`, buffer = random text + trailing NUL at `n-1` | [x] |
| 8 | `UTIL_createLinePointers` | `numLines = 2`, buffer = `"<a>\0<b>\0"`, `bufferSize` = full length — both segments terminated | [x] |
| 9 | `UTIL_createLinePointers` | `numLines = 2`, buffer = `"<a>\0<b>"`, `bufferSize` = full length — final segment unterminated | [x] |
| 10 | `UTIL_createLinePointers` | `numLines = 2`, buffer = `"\0\0"`, `bufferSize = 2` — two consecutive zero-length lines | [x] |
| 11 | `UTIL_createLinePointers` | `numLines = k`, buffer = all-NUL of size `k`, `bufferSize = k` — every line zero-length, offsets `0..k-1` | [x] |
| 12 | `UTIL_createLinePointers` | `numLines = k` many (`k` up to 512), buffer = `k` random-length NUL-terminated segments, `bufferSize` = full length | [x] |
| 13 | `UTIL_createLinePointers` | `numLines = k` many, buffer = `k` random-length segments where the LAST NUL is omitted | [x] |
| 14 | `UTIL_createLinePointers` | `numLines` **fewer** than the segments present (early loop exit via `lineIndex == numLines`, remaining buffer untouched) | [x] |
| 15 | `UTIL_createLinePointers` | `bufferSize` **smaller** than the real storage: NULs exist only past `bufferSize`, `numLines = 1` — scan truncates at `bufferSize` | [x] |
| 16 | `UTIL_createLinePointers` | `bufferSize` smaller than real storage, cutting mid-segment with `numLines` = number of segments fully inside the window | [x] |
| 17 | `UTIL_createLinePointers` | fully randomized fuzz: random `bufferSize` (0..4096), random byte content with random NUL density (incl. dense-NUL and NUL-free extremes), random `numLines` (0..bufferSize+4) — success and failure outcomes both accepted, must agree | [x] |
| 18 | `UTIL_createLinePointers` | large scale: `bufferSize` = 64 KiB of random NUL-separated segments, `numLines` = exact segment count | [x] |
| 19 | `UTIL_createLinePointers` | mixed empty/non-empty lines interleaved (`"\0a\0\0bb\0"` shapes), randomized, `numLines` = exact count | [x] |
| 20 | `UTIL_createLinePointers` | `bufferSize` = 1 with `numLines` = 1 across all 256 possible byte values (value-dependent branch on `buffer[pos] != '\0'`) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)` — there is
no `add_executable`, and `translation/Cargo.toml` declares `crate-type =
["cdylib"]` with no `[[bin]]`. There is no driver binary, so the stdout
comparison item of the completion gate is not applicable.

## Feature combinations

`translation/Cargo.toml` has no `[features]` section → the single combination is
`--no-default-features` ≡ default. Verified by script (`grep -n '\[features\]'`
returns nothing) and both `cargo test` and
`cargo test --no-default-features` are run.
