# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. There is exactly one function and
exactly one error sentinel (`-1`); the C code contains **no** `assert`, no error
enum, no `RETURN_ERROR` macro, and no `NULL`-pointer validation of `bin`/`hex`.

Grep of every control-flow site that can reject input (line numbers from
`c_src/src/lib.c`):

```
22:  if ((c_num0 | c_alpha0) == 0U)        -> non-hex character detected
23:  if (ignore != NULL && state == 0U &&
24:      strchr(ignore, c) != NULL)        -> skip, else fall through to
28:      break;                            -> stop parsing at this character
31:  if (bin_pos >= bin_maxlen)
32:      ret = -1;                         -> ERROR: output buffer exhausted
33:      break;
43:  if (state != 0U)
45:      ret = -1;                         -> ERROR: trailing odd nibble
47:  if (ret != 0) bin_pos = 0;            -> on error the length is zeroed
50:  if (hex_end_p != NULL) *hex_end_p = &hex[hex_pos];
52:  else if (hex_pos != hex_len)
53:      ret = -1;                         -> ERROR: unconsumed input & no
                                              hex_end_p to report it through
55:  if (ret != 0) return ret;             -> returns -1
58:  return (int)bin_pos;                  -> success: number of bytes written
```

Three distinct `ret = -1` assignments ⇒ three primary rejection rows; the
remaining rows are the distinct *triggers* that reach them plus the generic
FFI-boundary boundaries required by the task.

| #  | function  | trigger (exact invalid input/condition)                                                                                             | expected C result                                                                                                    | [x] |
|----|-----------|-------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|-----|
| 1  | `hex2bin` | `bin_maxlen == 0` and `hex` starts with a hex digit (`bin_pos >= bin_maxlen` on the very first digit, line 31)                        | `-1`; nothing written to `bin`; `*hex_end_p == &hex[0]`                                                                | [x] |
| 2  | `hex2bin` | `bin_maxlen < hex_len/2`: buffer fills, another hex digit follows (line 31)                                                          | `-1`; `bin` holds the first `bin_maxlen` decoded bytes (written before the error); `*hex_end_p == &hex[2*bin_maxlen]`  | [x] |
| 3  | `hex2bin` | odd number of hex digits consumed, i.e. `state != 0U` at loop exit (line 43) — e.g. `hex = "abc"`, `hex_len = 3`                       | `-1`; `hex_pos` decremented by 1 so `*hex_end_p == &hex[hex_len-1]` (points at the *last* digit, not one past it)      | [x] |
| 4  | `hex2bin` | odd digit count caused by a delimiter mid-byte: valid `ignore` char appearing while `state != 0U` (line 23 `state == 0U` fails)       | `-1` via row 3 path; `*hex_end_p` points at the digit preceding the delimiter                                          | [x] |
| 5  | `hex2bin` | `hex_end_p == NULL` and a non-hex character stops parsing early, so `hex_pos != hex_len` (line 52)                                    | `-1`; return value is *not* the partial length                                                                        | [x] |
| 6  | `hex2bin` | `hex_end_p == NULL` and buffer-full error (`hex_pos != hex_len`, line 52 reached with `ret` already `-1`)                              | `-1`                                                                                                                  | [x] |
| 7  | `hex2bin` | `hex_end_p == NULL` and odd digit count (rows 3+52 combined)                                                                          | `-1`                                                                                                                  | [x] |
| 8  | `hex2bin` | non-hex character with `ignore == NULL` (line 23 first conjunct fails) → `break` at line 28                                          | `break`; then `-1` if `hex_end_p == NULL`, else `bin_pos` (possibly `0`) with `*hex_end_p` at the offending byte        | [x] |
| 9  | `hex2bin` | non-hex character **not** contained in `ignore` (line 24 `strchr` returns `NULL`) → `break`                                          | same as row 8                                                                                                         | [x] |
| 10 | `hex2bin` | *first* character is non-hex and non-ignorable, `hex_end_p != NULL`                                                                  | returns `0` (success, zero bytes) with `*hex_end_p == &hex[0]` — **not** an error                                      | [x] |
| 11 | `hex2bin` | byte `0x00` inside `hex` with `ignore != NULL` and `state == 0U`: `strchr(ignore, 0)` matches the NUL **terminator** of `ignore`      | the NUL byte is silently *skipped* (`continue`) even for `ignore = ""` — quirk, reproduced verbatim                    | [x] |
| 12 | `hex2bin` | byte `0x00` inside `hex` with `ignore == NULL`                                                                                       | `break` at the NUL (rows 5/8/10 apply)                                                                                | [x] |
| 13 | `hex2bin` | byte `0x00` inside `hex` with `ignore != NULL` and `state != 0U`                                                                      | `break` → `-1` via row 3                                                                                              | [x] |
| 14 | `hex2bin` | high bytes `0x80..=0xFF` in `hex` (never hex digits; `c & ~32U` arithmetic wraps in `unsigned int`)                                   | treated as non-hex → rows 8/9/10; matched against `ignore` byte-wise (signed-`char` comparison in `strchr`)            | [x] |
| 15 | `hex2bin` | characters that are *adjacent* to the hex ranges and must be rejected: `'/'`(0x2F) `':'`(0x3A) `'@'`(0x40) `'G'`(0x47) `'\``(0x60) `'g'`(0x67) | all non-hex → rows 8/9/10                                                                                        | [x] |
| 16 | `hex2bin` | `hex_len == 0` (loop never entered)                                                                                                 | returns `0`; `*hex_end_p == &hex[0]`; `hex` may even be `NULL`                                                         | [x] |
| 17 | `hex2bin` | `hex == NULL` with `hex_len == 0` and `hex_end_p != NULL`                                                                            | returns `0`; `*hex_end_p == NULL` (`&hex[0]` on a null pointer)                                                        | [x] |
| 18 | `hex2bin` | `bin == NULL` with `bin_maxlen == 0` and at least one hex digit                                                                      | `-1` via row 1 — `bin` is never dereferenced                                                                          | [x] |
| 19 | `hex2bin` | `bin == NULL`, `bin_maxlen == 0`, `hex_len == 0`                                                                                     | returns `0`                                                                                                           | [x] |
| 20 | `hex2bin` | oversized `bin_maxlen` (`SIZE_MAX`) with a short even hex string                                                                     | success; returns `hex_len/2`; the `bin_pos >= bin_maxlen` check never trips                                            | [x] |
| 21 | `hex2bin` | oversized `bin_maxlen` (`SIZE_MAX`) with an odd hex string                                                                           | `-1` via row 3 (the two error paths are independent)                                                                  | [x] |
| 22 | `hex2bin` | `ignore` is the empty string `""` and `hex` contains a non-hex, non-NUL byte                                                          | `strchr("", c) == NULL` → `break` (rows 5/8/10); only `c == 0` is skipped (row 11)                                      | [x] |
| 23 | `hex2bin` | `ignore` contains characters that *are* valid hex digits (e.g. `"abc0"`)                                                             | no effect whatsoever — line 22 is only reached for non-hex bytes, so those `ignore` entries are unreachable            | [x] |
| 24 | `hex2bin` | every byte of `hex` is ignorable (e.g. `hex = "::::"`, `ignore = ":"`), `hex_end_p != NULL`                                            | returns `0`, `*hex_end_p == &hex[hex_len]` (all consumed, `hex_pos == hex_len`)                                        | [x] |
| 25 | `hex2bin` | every byte of `hex` is ignorable and `hex_end_p == NULL`                                                                             | returns `0` (**not** `-1`: `hex_pos == hex_len`, so line 52 does not fire)                                             | [x] |

## Notes on unreachable / non-applicable rejection classes

* **Out-of-range enum values across the FFI boundary**: the API declares **no
  enum, no flag word, and no mode parameter** (`c_src/include/lib.h` has a
  single function taking two pointers, two `size_t`, one `const char *` and one
  `const char **`). There is therefore no enum-like integer whose out-of-range
  value could be mishandled. The nearest analogue — an arbitrary out-of-domain
  *byte* value in `hex` — is covered by rows 11–15, which sweep the full
  `0x00..=0xFF` domain exhaustively in `tests/differential.rs`.
* **`hex_pos--` underflow (line 44)**: `state != 0U` requires at least one hex
  digit to have been consumed *and* `hex_pos++` to have run, so `hex_pos >= 1`
  whenever line 44 executes. The Rust translation still uses
  `wrapping_sub(1)` so the ABI would agree even if it were reachable.
* **`bin`/`hex` non-null validation**: absent from the C. Passing a garbage
  non-null pointer is UB in both languages and is therefore not a testable
  rejection; only the `NULL` + zero-length combinations (rows 17–19), which the
  C provably never dereferences, are exercised.

All 25 rows are implemented and passing — see `tests/errors.rs`
(`errors_md_row_XX_*` test names map 1:1 to the `#` column).

## Suite-adequacy evidence (mutation check)

Passing tests only mean something if the tests can fail. `scripts/mutation_check.sh`
injects 15 behaviour-changing edits into `translation/src/lib.rs`, rebuilds the
cdylib and re-runs the suite for each. **All 15 are detected.** It also injects 3
edits that are *provably* unobservable and confirms those still pass, so the
suite is not over-fitted to implementation detail:

```
mutants caught: 15   missed: 0   skipped: 0
equivalent mutants confirmed: 3   unexpected: 0
MUTATION CHECK: PASS
```

The behaviour-changing mutants cover each error path in this table: dropping
`hex_pos--` (rows 3, 4, 21), dropping the `hex_pos != hex_len` branch (rows
5–7), the off-by-one on `bin_pos >= bin_maxlen` (rows 1, 2), dropping the
`strchr` NUL quirk (rows 11, 13), dropping the `state == 0U` guard (rows 4, 13),
and returning `bin_pos` instead of `ret` on error (all rows).
