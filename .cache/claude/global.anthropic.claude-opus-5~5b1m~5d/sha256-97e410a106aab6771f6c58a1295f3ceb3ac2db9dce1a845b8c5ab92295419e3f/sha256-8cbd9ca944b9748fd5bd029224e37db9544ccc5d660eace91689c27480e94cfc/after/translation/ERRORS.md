# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection site in `c_src/src/lib.c`. The file
contains **no** `assert`, **no** `RETURN_ERROR` macro, **no** `return NULL`,
and **no** null-pointer checks on `bin` / `hex`. Every rejection is a
`ret = -1` assignment; the function's only error sentinel is the returned
`int` value `-1` (success returns `bin_pos`, i.e. `>= 0`).

Grep of every rejection site:

```
c_src/src/lib.c:32:            ret = -1;      # bin_pos >= bin_maxlen
c_src/src/lib.c:45:            ret = -1;      # state != 0  (odd digit count)
c_src/src/lib.c:53:        ret = -1;          # hex_end_p == NULL && hex_pos != hex_len
```

Plus the two `break` statements (line 28 non-hex char, line 33 out of room)
which are not themselves errors but are the *cause* of row 3 firing, and the
side effect at line 48 (`bin_pos = 0` when `ret != 0`).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `hex2bin` | `bin_maxlen == 0` and `hex_len > 0` with `hex[0]` a valid hex digit → `bin_pos(0) >= bin_maxlen(0)` at line 31 | `ret = -1`; returns `-1`. `*hex_end_p == &hex[0]` when `hex_end_p != NULL`. |
| 2 | `hex2bin` | `bin_maxlen` smaller than `hex_len/2`, all-valid hex (e.g. `bin_maxlen=1`, `hex="aabb"`) → line 31 fires after `bin_maxlen` bytes written | returns `-1`; `*hex_end_p == &hex[2*bin_maxlen]`; `bin` holds the first `bin_maxlen` decoded bytes (written before the error). |
| 3 | `hex2bin` | odd number of consumed hex digits, so `state != 0` at line 43 (e.g. `hex="abc"`, `hex_len=3`, `bin_maxlen` large, `hex_end_p != NULL`) | line 44 `hex_pos--`, line 45 `ret = -1`; returns `-1`; `*hex_end_p == &hex[hex_pos-1]` (points *back at* the unpaired digit). |
| 4 | `hex2bin` | `hex_end_p == NULL` and the scan stopped early (`hex_pos != hex_len`) because of a non-hex, non-ignored character (line 22/28 `break`) e.g. `hex="ab!!"`, `ignore=NULL`, `hex_end_p=NULL` | line 53 `ret = -1`; returns `-1`. |
| 5 | `hex2bin` | `hex_end_p == NULL` and the scan stopped early because of `bin_pos >= bin_maxlen` (line 33 `break`), e.g. `bin_maxlen=1`, `hex="aabb"`, `hex_end_p=NULL` | line 32 sets `ret=-1` and line 53 would also set it; returns `-1`. |
| 6 | `hex2bin` | odd digit count **and** `hex_end_p == NULL` (row 3 + row 4 combined), e.g. `hex="abc"`, `hex_end_p=NULL` | `hex_pos--` then `ret=-1` at 45; line 52 also sees `hex_pos != hex_len`; returns `-1`. |
| 7 | `hex2bin` | trailing ignore character while `state != 0` — an ignore char is **only** skipped when `state == 0` (line 23 `state == 0U`), so e.g. `hex="ab c"` with `ignore=" "` consumes `a`,`b`, then at `' '` `state==0` → skipped, then `c` → `state!=0` at end | returns `-1` (row 3 path); `*hex_end_p == &hex[3]`. |
| 8 | `hex2bin` | ignore character *between* the two nibbles of a byte, e.g. `hex="a b"` with `ignore=" "`: at `' '` `state != 0` so the ignore branch is skipped and line 28 `break`s | `state != 0` → `hex_pos--`, `ret=-1`; returns `-1`. |
| 9 | `hex2bin` | high-byte / non-ASCII invalid char (`0x80`–`0xFF`) as the first char with `hex_end_p == NULL`, `hex_len > 0` | `(c_num0 \| c_alpha0) == 0` → break at 28; `hex_pos(0) != hex_len` → `ret=-1`; returns `-1`. |
| 10 | `hex2bin` | boundary invalid chars one step outside each valid range: `'/'` (0x2F, just below `'0'`), `':'` (0x3A, just above `'9'`), `'@'` (0x40, just below `'A'`), `'G'` (0x47, just above `'F'`), `` '`' `` (0x60, just below `'a'`), `'g'` (0x67, just above `'f'`) — with `hex_end_p == NULL` | each breaks at 28 → `ret=-1`; returns `-1`. With `hex_end_p != NULL` and an even digit count these are **not** errors: returns `bin_pos`. |
| 11 | `hex2bin` | embedded NUL byte (`'\0'`) in `hex` while `ignore != NULL`: C `strchr(ignore, 0)` matches the ignore set's own terminator, so NUL is **always** treated as ignorable when `ignore != NULL` | NUL is skipped (not an error); with `ignore == NULL` the NUL breaks at 28 and, if `hex_end_p == NULL`, yields `-1`. |
| 12 | `hex2bin` | `ignore` set to the empty string `""` and `hex` contains an invalid char: `strchr("", c)` matches only `c == 0` | non-NUL invalid char → break → `-1` if `hex_end_p == NULL`. |
| 13 | `hex2bin` | `bin == NULL` combined with `bin_maxlen == 0` and a valid leading hex digit — the C never dereferences `bin` because line 31 rejects first | returns `-1`, no dereference (same as row 1). |
| 14 | `hex2bin` | `hex == NULL` with `hex_len == 0` — the `while` never runs, so `hex` is never dereferenced | returns `0` (`bin_pos == 0`); `*hex_end_p == NULL + 0 == NULL` when `hex_end_p != NULL`. Not an error. |
| 15 | `hex2bin` | oversized `hex_len` interacting with `bin_maxlen == SIZE_MAX` — no overflow check exists in C | never an error by itself; `bin_pos >= bin_maxlen` simply never fires. |
| 16 | `hex2bin` | out-of-range "enum"-like value across the FFI boundary — the C API takes **no enum parameter**; the only int-typed value crossing the boundary is the `int` *return*, whose full range must be compared. `bin_pos` is truncated to `int` by the `(int)bin_pos` cast at line 58 with no range check | returns `(int)bin_pos`, i.e. wraps for `bin_pos > INT_MAX` — untestable in practice (would need >2 GiB of hex input) but the truncation must be present in the Rust (`bin_pos as c_int`). |

## Result

All 16 rows have a passing differential test in `tests/errors.rs`
(`error_row01_*` … `error_row16_*`), each asserting the **same specific**
sentinel value shared by C and Rust — `-1` for the rejection rows and the exact
non-negative `bin_pos` for the "this is deliberately NOT an error" contrast
cases — together with an identical `*hex_end_p` offset and an identical output
buffer.

Three additional generic-boundary tests cover the classes Phase C requires even
where the table has no row:

* `generic_boundaries_all_byte_values_and_extreme_lengths` — every byte value
  `0..=255` for `hex[i]` (the API has no enum parameter, so the byte domain of
  `hex` and the `size_t` domain of the two lengths are the out-of-range integer
  inputs that actually cross the FFI boundary), crossed with
  `hex_len ∈ {0,1,2}`, `bin_maxlen ∈ {0,1,SIZE_MAX}`, 3 ignore sets and both
  `hex_end_p` variants, plus `bin_maxlen ∈ {SIZE_MAX, SIZE_MAX-1, ISIZE_MAX, 2^40}`.
* `generic_boundaries_null_pointers` — every null-pointer combination the C
  provably does not dereference (all four pointers NULL; `bin=NULL` with digits
  present and `bin_maxlen=0`).
* `generic_boundaries_zero_and_oversized_lengths` — `hex_len ∈ {0,1,n-1,n}` ×
  `bin_maxlen ∈ {0,1,n/2-1,n/2,n,SIZE_MAX}` × both `hex_end_p` variants.

Deliberately **not** tested: a `hex_len` larger than the allocated `hex` buffer
or a `bin_maxlen` larger than the allocated `bin` buffer *with enough digits to
reach it*. Both make the **C** read/write out of bounds — that is a caller
contract violation with undefined behaviour, not a behaviour the Rust could
"match".
