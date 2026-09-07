# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the axes `c_src/src/lib.c` actually branches on.

## Public entry points

`c_src/include/lib.h` declares exactly one function, which is therefore both
the lowest-level and the only entry point:

```c
int hex2bin(uint8_t *bin, size_t bin_maxlen, const char *hex,
            size_t hex_len, const char *ignore, const char **hex_end_p);
```

There are no convenience wrappers to hide behind — every test drives this
function directly through the `.so` export.

## Axes the C code branches on

| axis | source of the branch | values enumerated |
|------|----------------------|-------------------|
| `ignore` pointer | `ignore != ((void*)0)` (line 23) and `strchr(ignore, c)` (line 24) | `NULL`, `""`, `" "`, `":"`, `" :\n\t-"` |
| `hex_end_p` pointer | `hex_end_p != ((void*)0)` (line 50) vs the `else if (hex_pos != hex_len)` fallback (line 52) | `NULL`, non-`NULL` |
| `state` parity | `state == 0U` (lines 23, 35), `state != 0U` (line 43), `state = ~state` (line 40) | even digit count (ends `state==0`), odd digit count (ends `state!=0`) |
| `bin_maxlen` vs digits | `bin_pos >= bin_maxlen` (line 31) | `0`, exactly `hex_len/2`, `< hex_len/2`, `> hex_len/2`, `SIZE_MAX` |
| `hex_len` | `hex_pos < hex_len` (line 16) | `0`, `1`, `2`, `3`, small odd, small even, large (≥1024) |
| char class of `hex[i]` | branch-free classification lines 18–22: `c_num0` digit mask, `c_alpha0` alpha mask, `(c_num0\|c_alpha0)==0` reject | `'0'..'9'`, `'a'..'f'`, `'A'..'F'`, mixed case, `'/'`,`':'`,`'@'`,`'G'`,`` '`' ``,`'g'` boundary rejects, `0x00`, `0x80..0xFF` high bytes, full `0..255` sweep |
| ignore-char position | interaction of line 23 `state == 0U` with the ignore set | leading, trailing, between bytes (`state==0`, skipped), **between the two nibbles of one byte** (`state!=0`, NOT skipped → break) |
| `bin` pointer | dereferenced only at line 38, and only after the line-31 guard | valid buffer, `NULL` when `bin_maxlen==0` |
| `hex` pointer | dereferenced only inside the loop | valid buffer, `NULL` when `hex_len==0` |

`c_src/src/lib.c` contains **no** `#ifdef`, **no** `switch`, and **no**
runtime option/mode/flag other than the two pointer-nullability options above,
so the configuration cross-product is exactly the table below.

## Table

Every row is driven with **many randomized inputs** (`SEED = 0x5EED_1234`,
deterministic xorshift64* PRNG in `tests/harness.rs`), not a single
hand-picked value. A row is checked only after all its randomized iterations
compare byte-identical between the C and Rust `.so` for: return value, the
whole `bin` buffer (including bytes past `bin_pos`), and the `*hex_end_p`
offset.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen = hex_len/2` exactly, `hex_len=0` (empty input) | [x] |
| 2 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, `hex_len=2` (one byte), random valid digits both cases | [x] |
| 3 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, random even `hex_len` 4..64, all-valid random digits, random case per digit | [x] |
| 4 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` exact, large even `hex_len` (1024..4096), all-valid | [x] |
| 5 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` **larger** than needed (buffer pre-filled with a random poison byte → verifies untouched tail bytes match) | [x] |
| 6 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen` **smaller** than needed (random `0..hex_len/2`), all-valid digits | [x] |
| 7 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen=SIZE_MAX`, all-valid even input | [x] |
| 8 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, odd `hex_len` (1,3, random odd), all-valid digits | [x] |
| 9 | `hex2bin` | `ignore=NULL`, `hex_end_p=**non-NULL**`, `bin_maxlen` exact, random even all-valid input → compares returned `*hex_end_p` offset | [x] |
| 10 | `hex2bin` | `ignore=NULL`, `hex_end_p=non-NULL`, odd `hex_len` → exercises the `hex_pos--` rewind of line 44 and the resulting `*hex_end_p` | [x] |
| 11 | `hex2bin` | `ignore=NULL`, `hex_end_p=non-NULL`, `bin_maxlen` smaller than needed → `*hex_end_p` at the truncation point | [x] |
| 12 | `hex2bin` | `ignore=NULL`, `hex_end_p=non-NULL`, input = valid digits followed by one **random arbitrary byte** `0..255` then more digits → early-break position | [x] |
| 13 | `hex2bin` | `ignore=NULL`, `hex_end_p=non-NULL`, `hex` = fully random bytes `0..255` (random `hex_len` 0..64), `bin_maxlen` random 0..40 — unconstrained fuzz over the whole char-class lattice | [x] |
| 14 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, `hex` = fully random bytes (same fuzz, NULL-`hex_end_p` variant → exercises line 52) | [x] |
| 15 | `hex2bin` | `ignore=""` (empty ignore set), `hex_end_p=non-NULL`, random valid digits with random invalid bytes injected | [x] |
| 16 | `hex2bin` | `ignore=" "`, `hex_end_p=non-NULL`, `bin_maxlen` exact, spaces injected at random **byte boundaries** (`state==0`) → all skipped | [x] |
| 17 | `hex2bin` | `ignore=" "`, `hex_end_p=non-NULL`, space injected at a random **nibble boundary** (`state!=0`) → ignore branch suppressed by `state==0U`, break + rewind | [x] |
| 18 | `hex2bin` | `ignore=":"`, `hex_end_p=non-NULL`, colon-separated pairs (`aa:bb:cc`) — random length, exact `bin_maxlen` | [x] |
| 19 | `hex2bin` | `ignore=" :\n\t-"` (multi-char set), `hex_end_p=non-NULL`, random separators from the set at random byte boundaries, leading and trailing | [x] |
| 20 | `hex2bin` | `ignore=" :\n\t-"`, `hex_end_p=NULL`, same shapes → trailing separators make `hex_pos != hex_len` reachable | [x] |
| 21 | `hex2bin` | `ignore` non-NULL, `hex` contains an embedded `0x00` byte → C `strchr(ignore, 0)` matches the ignore set's terminator, so NUL is skipped; both `hex_end_p` variants | [x] |
| 22 | `hex2bin` | `ignore=NULL`, `hex` contains an embedded `0x00` byte → NUL is an invalid char and breaks; both `hex_end_p` variants | [x] |
| 23 | `hex2bin` | `ignore` non-NULL, `bin_maxlen` smaller than needed **and** separators present → interaction of line 31 with the ignore skip | [x] |
| 24 | `hex2bin` | `ignore` non-NULL, odd digit count **and** separators present → interaction of line 43 rewind with skipped chars (rewind lands on a digit, not on a separator) | [x] |
| 25 | `hex2bin` | exhaustive single-char sweep: `hex_len=1`, `hex[0] = c` for **every** `c` in `0..=255`, crossed with `ignore ∈ {NULL, "", " ", " :\n\t-"}` × `hex_end_p ∈ {NULL, non-NULL}` × `bin_maxlen ∈ {0, 1}` | [x] |
| 26 | `hex2bin` | exhaustive two-char sweep: `hex_len=2`, `hex = [c0, c1]` for **every** pair in `0..=255`² (65 536 pairs), `ignore=NULL`, `hex_end_p` non-NULL, `bin_maxlen=1` — full classification lattice incl. every `c_num`/`c_alpha` mask combination | [x] |
| 27 | `hex2bin` | `bin=NULL` with `bin_maxlen=0` and a valid leading digit (C rejects at line 31 before dereferencing) | [x] |
| 28 | `hex2bin` | `hex=NULL` with `hex_len=0`, both `hex_end_p` variants (`*hex_end_p` must come back as `NULL`) | [x] |
| 29 | `hex2bin` | `bin_maxlen=0` with `hex_len=0` and `ignore` non-NULL, `hex_end_p` non-NULL — degenerate all-zero configuration | [x] |
| 30 | `hex2bin` | round-trip: random binary blob → uppercase hex, lowercase hex, and mixed-case hex; each decoded with `bin_maxlen` exact and `hex_end_p` non-NULL; asserts C and Rust both reproduce the original blob | [x] |

## Coverage

All 30 rows are implemented as `row01_*` … `row30_*` in `tests/configs.rs` and
all pass against both the debug and the release Rust `.so`.

Beyond the table, `tests/stress.rs` adds an unstructured sweep that randomizes
**every axis simultaneously** (200 000 calls), plus exhaustive 3-character and
4-character sweeps over a boundary-biased alphabet crossed with the ignore-set
/ `hex_end_p` / `bin_maxlen` options, plus 4 KiB–32 KiB inputs. These exist to
catch axis *interactions* that the one-row-at-a-time tests could miss.

## Harness-validity check (anti-vacuous-test)

A passing suite is only meaningful if it can fail. Six deliberate mutations
were injected into `src/lib.rs` one at a time and the suite re-run:

| mutation | detected? |
|----------|-----------|
| drop the `hex_pos--` rewind (line 44) | YES — 10 tests failed |
| reorder `strchr_found` so the NUL quirk is lost | YES — 3 tests failed |
| drop the `state == 0` guard on the ignore branch (line 23) | YES — 2 tests failed |
| `c_alpha0` mask off-by-one (`-16` → `-17`) | YES — 6 tests failed |
| `bin_pos >= bin_maxlen` → `bin_pos > bin_maxlen` | YES — SIGSEGV / buffer divergence |
| remove `bin_pos = 0` (line 48) | NO — **semantically dead in the C too**: when `ret != 0` the C returns `ret` at line 56, so `bin_pos` is never observable. Not a coverage gap. |

`src/lib.rs` was restored and its SHA-256 verified identical to the pre-mutation
copy afterwards.
