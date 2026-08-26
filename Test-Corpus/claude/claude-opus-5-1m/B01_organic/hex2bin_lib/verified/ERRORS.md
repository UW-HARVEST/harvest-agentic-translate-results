# ERRORS.md — error-surface table (Phase A / Phase C)

Mechanically derived from `c_src/src/lib.c`. Every site in the C source that
can reject input or change the result to a failure is listed. Grep of the
whole translation unit:

```
9:    int ret = 0;                              <- only error accumulator
16:   while (hex_pos < hex_len) {               <- loop bound
22:   if ((c_num0 | c_alpha0) == 0U) {          <- "not a hex digit" classifier
23:       if (ignore != NULL && state == 0U &&
24:           strchr(ignore, c) != NULL) { hex_pos++; continue; }
28:       break;                                <- REJECT: stop at non-hex char
31:   if (bin_pos >= bin_maxlen) { ret = -1; break; }   <- REJECT: output full
43:   if (state != 0U) { hex_pos--; ret = -1; } <- REJECT: odd digit count
47:   if (ret != 0) { bin_pos = 0; }            <- error => length reported 0
50:   if (hex_end_p != NULL) *hex_end_p = &hex[hex_pos];
52:   else if (hex_pos != hex_len) ret = -1;    <- REJECT: unconsumed input
55:   if (ret != 0) return ret;                 <- returns -1
58:   return (int)bin_pos;
```

There are **no** `assert()`s, **no** NULL checks on `bin` or `hex`, **no**
`errno` use, **no** enums, and **no** min/max size constants in the C source.
The only error value the function can ever return is `-1`; success returns the
non-negative number of bytes written. `bin`/`hex` NULL-ness is *not* validated
by the C code (dereferencing them with a non-zero length is UB in both C and
Rust and is out of scope, except for the length-0 case, which is well defined
and is covered below).

## Error rows

`E` = expected C result. `hex_end` = `*hex_end_p - hex` (only when
`hex_end_p != NULL`).

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `hex2bin` | `bin_maxlen == 0`, `hex_len >= 1`, first char is a valid hex digit (line 31 fires on the very first digit) | returns `-1`; `bin` untouched; `hex_end == 0`; (state is still 0 so line 43 does not fire) | `err_01_bin_maxlen_zero` | [x] |
| 2 | `hex2bin` | `bin_maxlen < hex_len/2` — output buffer fills mid-stream (line 31 fires after `2*bin_maxlen` digits) | returns `-1`; the first `bin_maxlen` bytes ARE written before the break (the C does not roll them back); `hex_end == 2*bin_maxlen` | `err_02_bin_maxlen_truncates` | [x] |
| 3 | `hex2bin` | odd number of hex digits consumed → `state != 0` at line 43 (e.g. `hex="abc"`, `hex_len=3`) | `hex_pos--` then `ret=-1`; returns `-1`; `hex_end == hex_len - 1` (points at the last, unpaired digit) | `err_03_odd_digit_count` | [x] |
| 4 | `hex2bin` | `hex_end_p == NULL` and parsing stopped before the end (`hex_pos != hex_len`, line 52) — e.g. non-hex char in the middle | returns `-1` | `err_04_null_hex_end_unconsumed` | [x] |
| 5 | `hex2bin` | non-hex char (`(c_num0\|c_alpha0)==0`) with `ignore == NULL` (line 28 break, then line 52 not taken because `hex_end_p != NULL`) | **not** an error: returns bytes decoded so far, `hex_end` == index of the offending char | `err_05_stop_char_reported_not_error` | [x] |
| 6 | `hex2bin` | non-hex char while `ignore != NULL` but the char is **not** in the ignore set (`strchr` returns NULL → line 28) | break; `-1` if `hex_end_p == NULL`, otherwise partial success as in row 5 | `err_06_char_not_in_ignore_set` | [x] |
| 7 | `hex2bin` | char **is** in the ignore set but `state != 0` (separator inside a byte, e.g. `"a:b"`, `ignore=":"`) — the `state == 0U` guard on line 23 blocks the skip | break at the separator, then line 43 fires: `hex_pos--`, returns `-1`; `hex_end` == index of the digit before the separator | `err_07_separator_mid_byte` | [x] |
| 8 | `hex2bin` | `ignore != NULL` but empty string `""` and a non-hex char present (`strchr("",c)` is NULL for `c != 0`) | identical to `ignore == NULL` for that char: break (row 5/6) | `err_08_empty_ignore_set` | [x] |
| 9 | `hex2bin` | both failures at once: odd digit count *and* buffer full (e.g. `bin_maxlen=1`, `hex="abcde"`) | line 31 breaks with `state == 0` → `ret=-1`, `bin_pos=0`, `hex_end == 2*bin_maxlen`; line 43 not taken | `err_09_both_conditions` | [x] |
| 10 | `hex2bin` | error path result of line 47: whenever `ret != 0` the byte count is zeroed and `-1` is returned instead of a partial count | return value is exactly `-1` (never a partial count) for every row above, while the bytes already stored in `bin` are *kept* | `err_10_error_return_is_minus_one` | [x] |
| 11 | `hex2bin` | first char is a non-hex byte (loop breaks immediately, `hex_pos == 0`), `hex_end_p != NULL` | returns `0`, `hex_end == 0`; **not** `-1` | `err_11_leading_invalid_char` | [x] |
| 12 | `hex2bin` | first char is a non-hex byte, `hex_end_p == NULL`, `hex_len > 0` | returns `-1` (line 52) | `err_12_leading_invalid_char_null_end` | [x] |
| 13 | `hex2bin` | embedded NUL byte in `hex` with `ignore == NULL` | NUL is not a hex digit → break (rows 5/12 apply) | `err_13_embedded_nul_no_ignore` | [x] |
| 14 | `hex2bin` | embedded NUL byte in `hex` with **any** non-NULL `ignore` (even `""`): `strchr(ignore, 0)` matches the terminator → non-NULL | quirk: the NUL is **skipped** like an ignore char (when `state == 0`); decoding continues | `err_14_embedded_nul_with_ignore` | [x] |
| 15 | `hex2bin` | every one of the 256 possible byte values used as the stop char, each × {`ignore` NULL / `""` / that byte / other byte} × {`hex_end_p` NULL / non-NULL} | exhaustive agreement of return value, `hex_end`, and buffer contents | `err_15_all_256_stop_bytes` | [x] |
| 16 | `hex2bin` | one step past each end of every accepted character range: `'/'`(0x2F), `':'`(0x3A), `'@'`(0x40), `'G'`(0x47), `` '`' ``(0x60), `'g'`(0x67), plus `0x80..=0xFF` high-bit bytes | all rejected (loop breaks) exactly as in row 5/12 | `err_16_range_boundary_chars` | [x] |
| 17 | `hex2bin` | zero lengths: `hex_len == 0` (with `bin_maxlen` 0 or >0, `bin`/`hex`/`ignore` NULL or not, `hex_end_p` NULL or not) | returns `0`; when `hex_end_p != NULL`, `*hex_end_p == hex` (i.e. `NULL` when `hex` is `NULL`); never dereferences anything | `err_17_zero_length_and_null_ptrs` | [x] |
| 18 | `hex2bin` | oversized `bin_maxlen` (`usize::MAX`, `SIZE_MAX/2`) — no overflow check exists in C | ignored; behaves as "big enough" | `err_18_oversized_bin_maxlen` | [x] |
| 19 | `hex2bin` | oversized/`SIZE_MAX`-ish `hex_len` — no check exists in C; only reachable safely when the loop breaks before reading out of bounds (first char non-hex, or `bin_maxlen == 0`) | breaks on the first char; `hex_end == 0`; return `-1` (`bin_maxlen==0`) or `0`/`-1` per rows 1/11/12 | `err_19_oversized_hex_len` | [x] |
| 20 | `hex2bin` | `ignore` NULL vs non-NULL is the only pointer the C actually tests (`ignore != NULL`, line 23) — passing a *valid but exotic* ignore string (contains hex digits, contains 0x80..0xFF, 1 byte long) | the ignore set is consulted **only** for non-hex chars, so hex digits listed in `ignore` are still decoded | `err_20_exotic_ignore_sets` | [x] |

Notes on things deliberately *not* tested (undefined behaviour in the C, so
"matching" is meaningless): `bin == NULL` with `bin_maxlen > 0` and a digit
pair available, `hex == NULL` with `hex_len > 0`, unterminated `ignore`
strings, and `hex_end_p` pointing to unwritable memory.

## Error-path test strategy

Every row's test asserts the *exact* sentinel the C returns (`-1`) **and** the
exact side effects (`*hex_end_p` offset, the bytes stored in `bin` before the
failure), not merely "both failed somehow". Rows 15/16 additionally sweep all
256 byte values / all range-boundary bytes across the whole option matrix, and
`tests/exhaustive.rs` enumerates *every* input of length ≤ 2 and *every*
three-byte input, so no rejection branch depends on an untried byte value.

There are no enum parameters in this API, so the "out-of-range enum value"
class of FFI bug is covered by its equivalents here: out-of-range *byte* values
in `hex` (rows 15, 16 — exhaustive), out-of-range lengths (rows 18, 19:
`usize::MAX`, `usize::MAX/2`, `isize::MAX`, `1<<40`), and NULL pointers
(row 17).

## Not testable / out of scope

| condition | why |
|---|---|
| `bin_pos > INT_MAX` (the `(int)bin_pos` truncation on line 58) | would need > 4 GiB of hex input; both implementations use the identical `as c_int` / `(int)` truncation |
| `bin == NULL` with a digit pair available, `hex == NULL` with `hex_len > 0`, unterminated `ignore`, unwritable `hex_end_p` | undefined behaviour in the C, so "identical behaviour" is not defined; the debug-assertions build of the Rust `.so` panics with "null pointer dereference" instead |

## Validation of the error tests themselves

`scripts/mutation_check.py` injects 29 mutations into the Rust translation, 26
of which change behaviour; **all 26 are detected** by this suite (several only by
the error-path rows, e.g. the removed strict-mode check, the missing `hex_pos--`
and the `-1` → `-2` error-value change). The remaining 3 are provably
equivalent mutants (control group) and are correctly *not* detected.
