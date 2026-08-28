# ERRORS.md — Phase C error-surface table

Mechanically grepped from `c_src/src/lib.c`. The file contains **no** `assert`,
**no** error enum, **no** `return NULL`, and **no** min/max constant. Every
rejection path is a `ret = -1` assignment or a `break` out of the scan loop:

```sh
$ grep -n 'ret = -1\|break\|continue\|return' c_src/src/lib.c
28:            break;              # non-hex char, not ignorable -> stop scanning
32:            ret = -1;           # ERROR SITE 1
33:            break;
45:        ret = -1;               # ERROR SITE 2
53:        ret = -1;               # ERROR SITE 3
56:        return ret;             # -1
58:    return (int)bin_pos;        # success: number of bytes written
```

The only two possible return values are therefore `-1` (failure) and
`(int)bin_pos >= 0` (success). There is no distinct error code per site, so each
row below pins down *both* the return value *and* the observable side effects
(`bin` contents, `*hex_end_p`), which is what actually distinguishes the sites.

## Error rows

| #  | function  | trigger (the exact invalid input/condition) | expected C result |
|----|-----------|----------------------------------------------|-------------------|
| E1 | `hex2bin` | **Site 1**, output overflow: a hex digit is ready to be consumed while `bin_pos >= bin_maxlen`. Reached with `bin_maxlen == 0` and ≥1 hex digit, e.g. `bin_maxlen=0, hex="00"`. | `ret=-1`, loop `break` at that digit, `bin_pos` reset to 0, no byte written; returns `-1`; `*hex_end_p = &hex[hex_pos]` = the offending digit (`hex+0`) |
| E2 | `hex2bin` | **Site 1** mid-buffer: `bin_maxlen` smaller than the number of byte pairs available, e.g. `bin_maxlen=1, hex="0011"`. Note the check precedes the `state` branch, so it can only fire with `state == 0`. | returns `-1`; the bytes already stored in `bin[0..bin_maxlen]` **remain written** (C never erases them; only the returned count is zeroed); `*hex_end_p = hex+2` |
| E3 | `hex2bin` | **Site 2**, odd digit count: scan ends with `state != 0` (an unpaired trailing nibble), e.g. `hex="000"`, `bin_maxlen=2`. | `hex_pos--`, `ret=-1`; returns `-1`; `*hex_end_p = &hex[hex_pos-1]`, i.e. it points **at** the unpaired digit, not past it |
| E4 | `hex2bin` | **Site 2** via a non-hex char at an odd nibble boundary — `ignore != NULL` but `state != 0`, so the ignore set is *not* consulted and the loop `break`s, leaving `state != 0`, e.g. `hex="0 0"`, `ignore=" "`. | returns `-1`; `*hex_end_p = hex+0` (`break` at index 1, then `hex_pos--`) |
| E5 | `hex2bin` | **Site 3**, unconsumed input with no `hex_end_p`: `hex_end_p == NULL` and `hex_pos != hex_len` because a non-hex char stopped the scan, e.g. `hex="00zz"`, `ignore=NULL`, `hex_end_p=NULL`. | returns `-1` (the same input with a non-NULL `hex_end_p` returns `1` instead) |
| E6 | `hex2bin` | **Site 3** with `ignore == NULL`: any non-hex byte at all stops the scan, so *every* trailing separator becomes an error when `hex_end_p == NULL`, e.g. `hex="00:11"`, `ignore=NULL`, `hex_end_p=NULL`. | returns `-1` |
| E7 | `hex2bin` | **Site 3** where the char is non-hex **and** absent from a non-NULL `ignore` set at an even boundary, e.g. `hex="00!11"`, `ignore=" "`, `hex_end_p=NULL`. | returns `-1` |
| E8 | `hex2bin` | **Sites 1+3 combined**: overflow *and* `hex_end_p == NULL`. `ret` is already `-1` so the `else if` is skipped entirely. | returns `-1` |
| E9 | `hex2bin` | **Sites 2+3 combined**: odd digit count *and* `hex_end_p == NULL`. `ret` already `-1`. | returns `-1` |
| E10 | `hex2bin` | Leading non-hex byte, nothing consumed: `hex="zz"`, `hex_end_p=NULL`. `hex_pos=0 != hex_len=2`. | returns `-1` |
| E11 | `hex2bin` | Single hex digit only: `hex_len=1`, `hex="0"` — the minimal odd-count case, `hex_pos` goes 1 -> 0. | returns `-1`; `*hex_end_p = hex+0` |

## Generic FFI boundary rows (required even though the C has no explicit check)

The C performs **no** null check and **no** length validation, so these rows pin
down the exact behaviour the Rust must reproduce rather than "both fail".

| #   | function  | trigger | expected C result |
|-----|-----------|---------|-------------------|
| B1 | `hex2bin` | `bin == NULL` with `bin_maxlen == 0` and ≥1 hex digit — overflow check fires *before* any dereference, so NULL is never read/written. | returns `-1` (no crash) |
| B2 | `hex2bin` | `hex == NULL` with `hex_len == 0` — loop body never runs. | returns `0`; `*hex_end_p = NULL + 0 = NULL` |
| B3 | `hex2bin` | `hex == NULL`, `hex_len == 0`, `hex_end_p == NULL`, `bin == NULL`, `bin_maxlen == 0` — everything NULL/zero. | returns `0` |
| B4 | `hex2bin` | `bin_maxlen == SIZE_MAX` (oversized length) with a short valid hex string. | returns the byte count normally; overflow check can never fire |
| B5 | `hex2bin` | `hex_len == 0` with a non-NULL, non-empty `hex` buffer — zero length wins over content. | returns `0`; `*hex_end_p = hex+0` |
| B6 | `hex2bin` | `ignore == ""` (empty ignore set). `strchr("", c)` matches only `c == 0`, so a NUL byte is skipped and every other non-hex byte breaks. | NUL ignored; other non-hex bytes -> `break` |
| B7 | `hex2bin` | **Embedded NUL quirk**: `ignore != NULL`, `state == 0`, and `hex[i] == 0`. `strchr` also matches the *terminator* of the ignore set, so a NUL inside `hex` is silently ignored. | NUL skipped, scanning continues |
| B8 | `hex2bin` | Embedded NUL with `ignore == NULL` — no ignore consultation. | `break` at the NUL |
| B9 | `hex2bin` | `ignore` set that itself contains hex digits, e.g. `"0123"`. The ignore set is only consulted for **non**-hex bytes, so those entries are dead. | digits still decoded, never ignored |
| B10 | `hex2bin` | One step past each valid class boundary, i.e. the exact chars that bracket `[0-9A-Fa-f]`: `/`(0x2F), `:`(0x3A), `@`(0x40), `G`(0x47), `` ` ``(0x60), `g`(0x67). | all six rejected as non-hex |
| B11 | `hex2bin` | Every byte value `0x00..0xFF` as the sole `hex` char, and as the 2nd char after a valid digit, under both `ignore=NULL` and `ignore` non-NULL. This is the analogue of "out-of-range enum value": the C accepts any `char` bit pattern, incl. the high half `0x80..0xFF` which `(c & ~32U) - 55U` could otherwise alias into the letter range. | exactly `[0-9A-Fa-f]` accepted, all 250 other values rejected |

There are **no enum parameters** anywhere in this API (`c_src/include/lib.h`
declares only `uint8_t*`, `size_t`, `const char*`, `const char**`), so the
out-of-range-enum requirement is discharged by row **B11**, which sweeps the full
`0..=255` domain of the only "tagged" input the API has — the `hex` byte class.

## Conditions deliberately NOT tested (untestable without UB or >2 GiB inputs)

| condition | why it is out of scope | how parity is nonetheless assured |
|-----------|------------------------|-----------------------------------|
| `bin_pos > INT_MAX` at the `return (int)bin_pos` cast | needs a >4 GiB `hex` buffer | both sides perform a plain narrowing conversion (`(int)` in C, `as c_int` in Rust), which truncates identically on this target |
| `hex_len` larger than the real `hex` allocation | reads past the buffer — undefined behaviour in the C itself, so there is no "correct" result to match | the Rust performs the same unchecked `hex.add(hex_pos)` read in the same order |
| `ignore` not NUL-terminated | `strchr` runs off the end — UB in the C | the Rust `strchr_found` scans with the same termination rule |

Every other rejection the C source contains is covered by a row above.
