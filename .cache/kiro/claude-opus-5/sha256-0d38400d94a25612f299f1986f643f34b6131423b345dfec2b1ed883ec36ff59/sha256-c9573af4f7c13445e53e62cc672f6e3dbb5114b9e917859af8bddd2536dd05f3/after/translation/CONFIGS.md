# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/include/slicing.h` + `c_src/src/slicing.c`.

## Axes the C code actually branches on

| axis | values the C distinguishes | source evidence |
|------|----------------------------|-----------------|
| entry point | `slice` — the **only** public symbol, and it is itself the lowest-level entry point. There are no convenience wrappers and no internal helpers to call separately. | `include/slicing.h` declares one prototype; `nm -D` shows one `T` symbol |
| `start_ptr` mode | `NULL` (⇒ `start = 0`) / non-`NULL` (⇒ `start = *start_ptr`) | `if (start_ptr) { ... } else { start = 0; }` |
| `stop_ptr` mode | `NULL` (⇒ `stop = len`, via `size_t`→`int` truncation) / non-`NULL` | `} else stop = len;` |
| `len` shape | `0` / `1` / small / large; `len` drives both range checks and the default `stop` | `size_t len = strlen(mystr);` |
| `start` value | `0` / interior / `len-1` / `len` (accepted boundary, strict `>` check) | `if (start > len)` |
| `stop` value | `start+1` (narrowest slice) / interior / `len` (accepted boundary) | `if (stop > len)`, `if (stop <= start)` |
| byte content | arbitrary bytes `0x01..0xFF`; the slice is emitted with `%.*s`, so the content is copied verbatim including non-UTF-8 bytes, control bytes, `%` characters, and embedded newlines | `printf("%.*s\n", stop - start, mystr + start)` |
| output width | `stop - start == 0` (only reachable with `start == len` and `stop_ptr == NULL`) / `> 0` | precision argument of `%.*s` |
| compile-time config | none — no `#ifdef`, no build options in `CMakeLists.txt`, no Cargo features | `grep -c '#if' c_src/src/slicing.c` → 1 (the header include guard only) |

There is no runtime option/flag/mode API beyond the two pointer parameters, so
the configuration space is the cross-product of the pointer modes, the string
shape, and the index values — pruned below to the combinations the code treats
differently.

Every row is exercised with **many randomized inputs** (seeded, deterministic
`SplitMix64`, `SEED = 0x5DEE_CE66_D15E_A5E5`) — at least 64 iterations per row,
256 for the index-sweeping rows — and both `.so`s are compared on
**(return code, exact stdout bytes)**.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, `len == 0` (empty string) | [x] |
| 2 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, `len == 1` | [x] |
| 3 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, random printable ASCII, `len` 2..64 | [x] |
| 4 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, long string, `len` 256..4096 | [x] |
| 5 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, random **non-UTF-8** bytes `0x80..0xFF` | [x] |
| 6 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, content containing `%`, `%s`, `%n`, `%%` (format-specifier bytes as *data*) | [x] |
| 7 | `slice` | `start_ptr=NULL`, `stop_ptr=NULL`, content containing embedded `\n` and control bytes `0x01..0x1F` | [x] |
| 8 | `slice` | `start_ptr=&0`, `stop_ptr=NULL`, random `len` 1..64 (explicit start 0 vs. implicit) | [x] |
| 9 | `slice` | `start_ptr=&s`, `stop_ptr=NULL`, random interior `0 < s < len` | [x] |
| 10 | `slice` | `start_ptr=&(len-1)`, `stop_ptr=NULL` (last character only) | [x] |
| 11 | `slice` | `start_ptr=&len`, `stop_ptr=NULL` — **accepted boundary**, zero-width output (`stop - start == 0`) | [x] |
| 12 | `slice` | `start_ptr=NULL`, `stop_ptr=&e`, random `1 <= e <= len` | [x] |
| 13 | `slice` | `start_ptr=NULL`, `stop_ptr=&len` — accepted `stop` boundary (whole string) | [x] |
| 14 | `slice` | `start_ptr=NULL`, `stop_ptr=&1` — narrowest slice from the front | [x] |
| 15 | `slice` | both non-`NULL`, random `0 <= s < e <= len`, `len` 1..64 printable ASCII | [x] |
| 16 | `slice` | both non-`NULL`, `e == s + 1` (minimum valid width), random `s` | [x] |
| 17 | `slice` | both non-`NULL`, `s == 0`, `e == len` (whole string, both boundaries explicit) | [x] |
| 18 | `slice` | both non-`NULL`, `s == len-1`, `e == len` (last character, both boundaries explicit) | [x] |
| 19 | `slice` | both non-`NULL`, `len == 1`, `s == 0`, `e == 1` (degenerate single-char string) | [x] |
| 20 | `slice` | both non-`NULL`, random **non-UTF-8** / control-byte content, random valid `s < e` | [x] |
| 21 | `slice` | both non-`NULL`, long string `len` 256..4096, random valid `s < e` | [x] |
| 22 | `slice` | `start_ptr=&0`, `stop_ptr=NULL`, `len == 0` (empty string, explicit start at the boundary `0 == len`) | [x] |
| 23 | `slice` | full 2×2 pointer-mode matrix swept exhaustively over **every** valid `(s, e)` pair for `len` 0..8 (exhaustive small-case oracle, valid pairs only) | [x] |
| 24 | `slice` | **sequence** of many `slice` calls without an intervening flush — verifies stdout buffering/ordering and that no state leaks between calls | [x] |
| 25 | `slice` | any valid configuration — assert `*start_ptr` / `*stop_ptr` are **unmodified** after the call (no out-params) | [x] |
| 26 | `slice` | any valid configuration — assert the `mystr` buffer (incl. its NUL terminator) is **unmodified** after the call (read-only input) | [x] |

## Known-untestable configuration

`len > INT_MAX`, which would make the `else stop = len;` `size_t`→`int`
truncation observable, requires a >2 GiB string. Both implementations perform
the identical truncation (`len as c_int` vs. C's implicit conversion), so the
behaviour is matched by construction, but it is not covered by an executed test.

## Negative control

`./mutation_check.sh` also covers the valid-path rows: an off-by-one in the
printed width, an off-by-one in the slice offset, `stop = len-1` instead of
`len` when `stop_ptr` is NULL, `start = 1` instead of `0` when `start_ptr` is
NULL, dropping the trailing newline from the `%.*s` format, and writing back
through `start_ptr`. All are caught.
