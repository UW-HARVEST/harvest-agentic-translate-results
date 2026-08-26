# ERRORS.md — Error-surface table

Mechanically derived from every rejection point in `c_src/src/lib.c`. The whole
library is one function, so every row below belongs to
`UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize)`.

## Inventory of rejection machinery in the C source

`grep -n` over `c_src/src/lib.c` finds exactly these control-flow / rejection
constructs (there are **no** `assert`s, **no** error enums, **no** `RETURN_ERROR`
macros, **no** min/max constants, and **no** enum parameters anywhere in
`c_src/`):

| line | construct | kind |
|------|-----------|------|
| 8  | `malloc(numLines * sizeof(const char**))` | allocation, size computed with **unsigned wrap-around** |
| 10 | `if (bufferPtrs == NULL) return NULL;` | **rejection #1** — allocation failure |
| 12 | `while (lineIndex < numLines && pos < bufferSize)` | range check (loop guard) |
| 17 | `while ((pos + len < bufferSize) && buffer[pos + len] != '\0')` | range check (scan guard) + terminator test |
| 23 | `if (pos < bufferSize) pos++;` | range check (conditional advance) |
| 27–30 | `if (lineIndex != numLines) { free(bufferPtrs); return NULL; }` | **rejection #2** — under-run, with `free` |

So there are exactly **two** `return NULL` rejection statements. The table below
has one row per *distinct triggering condition* that reaches one of them, plus
the generic FFI-boundary boundaries (null pointer, zero length, oversized
length, one-past-range).

There is **no enum in this API**, so the "out-of-range enum value across FFI"
class has no instance here; the analogous "any `int`/`size_t` bit pattern is a
legal argument" class is covered by rows 1–2 and 8–11 (values far outside any
sane range, including ones that make the `size_t` multiply wrap).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `UTIL_createLinePointers` | `numLines = SIZE_MAX` → `numLines*8` wraps to `SIZE_MAX-7` (0xFFFF_FFFF_FFFF_FFF8); `malloc` fails | `NULL` via line 10 (no `free`) |
| 2 | `UTIL_createLinePointers` | `numLines = 2^60` → `malloc(2^63)` (8 EiB) fails | `NULL` via line 10 (no `free`) |
| 3 | `UTIL_createLinePointers` | `bufferSize = 0`, `numLines = 1` → loop guard `pos < bufferSize` false immediately, `lineIndex = 0 != 1` | `NULL` via line 30 (after `free`) |
| 4 | `UTIL_createLinePointers` | `bufferSize = 0`, `numLines` large (e.g. 1000) | `NULL` via line 30 (after `free`) |
| 5 | `UTIL_createLinePointers` | buffer holds fewer NUL-terminated lines than requested, e.g. `"a\0b\0"` (`bufferSize=4`) with `numLines=3` → `lineIndex` reaches 2, `pos` reaches 4 | `NULL` via line 30 (after `free`) |
| 6 | `UTIL_createLinePointers` | buffer has **no** NUL at all, e.g. `"abcd"` (`bufferSize=4`) with `numLines=2` → 1 unterminated line consumes whole buffer | `NULL` via line 30 (after `free`) |
| 7 | `UTIL_createLinePointers` | `numLines = bufferSize + 1` on an all-NUL buffer (max possible lines is `bufferSize`) → off-by-one past the maximum achievable line count | `NULL` via line 30 (after `free`) |
| 8 | `UTIL_createLinePointers` | `numLines = 2^61` → `numLines*8` **wraps to 0**, `malloc(0)` returns non-NULL so line 10 does *not* fire; with `bufferSize = 0` the loop is skipped and `lineIndex = 0 != 2^61` | `NULL` via line 30 (after `free`) — allocation-size wrap must be reproduced or Rust would take a different branch |
| 9 | `UTIL_createLinePointers` | `numLines = 2^61 + 1` → `numLines*8` **wraps to 8**; `bufferSize = 0` | `NULL` via line 30 (after `free`) |
| 10 | `UTIL_createLinePointers` | `numLines = 2^61 + 1` (alloc = 8 bytes = exactly 1 slot), `bufferSize = 1`, `buffer = "\0"` → exactly one slot written, then `pos == bufferSize` ends loop, `lineIndex = 1 != 2^61+1` | `NULL` via line 30 (after `free`) |
| 11 | `UTIL_createLinePointers` | `numLines = 2^61 + 2` (alloc = 16 bytes = 2 slots), `bufferSize = 2`, `buffer = "\0\0"` | `NULL` via line 30 (after `free`) |
| 12 | `UTIL_createLinePointers` | `buffer = NULL`, `numLines = 1`, `bufferSize = 0` → loop never entered so `buffer` is never dereferenced; under-run rejection | `NULL` via line 30 (after `free`) |
| 13 | `UTIL_createLinePointers` | `buffer = NULL`, `numLines = 0`, `bufferSize = 0` → `malloc(0)` non-NULL, loop skipped, `0 == 0` | **non-NULL** (zero-length array) — the degenerate *success* at the boundary; must not be rejected |
| 14 | `UTIL_createLinePointers` | `buffer = NULL`, `numLines = 0`, `bufferSize = 100` → loop guard `lineIndex < numLines` false, `buffer` never dereferenced | **non-NULL** (zero-length array) |
| 15 | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize = 0`, non-NULL buffer → `malloc(0)` | **non-NULL** (zero-length array) |

### A note on rows 8–11 (the wrapping multiply)

Rows 8–11 exercise the branch that the wrap enables, and they prove C and Rust
agree. They cannot, however, *distinguish* `wrapping_mul` from `saturating_mul`
through the return value, and that is provable rather than a test gap:

* the multiply only differs when `numLines >= 2^61`;
* with `saturating_mul` the request becomes `SIZE_MAX`, `malloc` fails, line 10
  returns `NULL`;
* with `wrapping_mul` the request is some `W`, and even if `malloc(W)` succeeds
  the call can only return non-NULL if `lineIndex` reaches `numLines >= 2^61`,
  which requires `bufferSize >= 2^61` — unallocatable.

So **every** `numLines >= 2^61` returns `NULL` either way. The difference is
memory-safety-only (wrapping can heap-overflow), not observable through the ABI.
The Rust nonetheless uses `wrapping_mul`, matching the C exactly.

The *non-wrapping* part of the same arithmetic — `sizeof(const char**) == 8` — is
pinned observably by `CONFIGS.md` row 25 (`malloc_usable_size` parity), which was
confirmed to fail if the element size is changed to 16.

### Deliberately NOT tested (undefined behaviour in the C itself — identical in both)

* `buffer = NULL` with `bufferSize > 0` **and** `numLines > 0`: line 17
  dereferences `buffer[0]` → segfault in C. The Rust does the same load, so it
  segfaults too; a differential test cannot observe a return value.
* `numLines * 8` wrapping to a value **smaller** than the number of slots the
  loop actually writes (e.g. `numLines = 2^61`, `bufferSize = 4`): heap buffer
  overflow in the C. Rows 8–11 pick the wrap values where the write count
  provably fits in the (wrapped) allocation, so the branch is exercised without
  invoking the overflow.
* `bufferSize` larger than the real allocation behind `buffer`: out-of-bounds
  read in the C.

## Status

| row | test | status |
|-----|------|--------|
| 1 | `err_01_num_lines_size_max_malloc_fails` | ☑ |
| 2 | `err_02_num_lines_2pow60_malloc_fails` | ☑ |
| 3 | `err_03_zero_buffer_size_one_line` | ☑ |
| 4 | `err_04_zero_buffer_size_many_lines` | ☑ |
| 5 | `err_05_fewer_lines_than_requested` | ☑ |
| 6 | `err_06_no_nul_at_all` | ☑ |
| 7 | `err_07_num_lines_one_past_max_achievable` | ☑ |
| 8 | `err_08_alloc_size_wraps_to_zero` | ☑ |
| 9 | `err_09_alloc_size_wraps_to_eight_buffer_zero` | ☑ |
| 10 | `err_10_alloc_size_wraps_to_eight_one_slot_written` | ☑ |
| 11 | `err_11_alloc_size_wraps_to_sixteen_two_slots_written` | ☑ |
| 12 | `err_12_null_buffer_zero_size_nonzero_lines` | ☑ |
| 13 | `err_13_null_buffer_zero_lines_zero_size` | ☑ |
| 14 | `err_14_null_buffer_zero_lines_nonzero_size` | ☑ |
| 15 | `err_15_zero_lines_zero_size_real_buffer` | ☑ |
