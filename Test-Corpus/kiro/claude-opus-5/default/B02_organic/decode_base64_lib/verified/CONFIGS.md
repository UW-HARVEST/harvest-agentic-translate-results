# CONFIGS.md — Phase B configuration-surface table

## Axes, derived mechanically from `c_src/src/lib.c`

There are **no runtime options, modes or flags**: the public header exposes a
single function taking one argument, and the source contains no `#if`/`#ifdef`,
no `getenv`, no `switch`, no global/static mutable state (`grep -n
'#if\|#ifdef\|#ifndef\|getenv\|extern\|switch' src/lib.c include/lib.h` → no
matches). The entire configuration surface is therefore the **input shape**
cross the branch classes of the two `static` helpers.

### Axis 1 — public entry points (the full set, incl. lowest level)

| entry point | linkage | reachability |
|-------------|---------|--------------|
| `decode_base64(const char *src)` | global, in `include/lib.h` | called directly via `.so` export |
| `decode(char c)` | `static` | not exported; driven indirectly, one row per branch class below |
| `is_base64(char c)` | `static` | not exported; driven indirectly, one row per accept/reject class below |

Because the two low-level helpers have internal linkage in C they cannot be
called across the FFI boundary in either library; the rows below drive **each of
their branches individually** through the single public entry point, which is the
strongest available equivalent.

### Axis 2 — `decode()` branch classes (lines 10-27)

`A`..`Z` → `c-'A'` | `a`..`z` → `c-'a'+26` | `0`..`9` → `c-'0'+52` | `'+'` → 62 |
fallthrough → 63 (reached by `'/'`, `'='`, and every other byte)

### Axis 3 — `is_base64()` classes (lines 30-38)

accept: `A`-`Z`, `a`-`z`, `0`-`9`, `'+'`, `'/'`, `'='` — reject: all others
(including every byte `>= 0x80`, which is **negative** as a signed `char`).

### Axis 4 — filtered length `l` modulo 4 (loop at line 72)

`l % 4 == 0` (full groups) | `== 1` | `== 2` | `== 3` — controls whether
`c2`/`c3`/`c4` keep their `'A'` defaults.

### Axis 5 — padding-suppression branches (lines 99, 103)

`c3 != '='` gate on output byte 2, `c4 != '='` gate on output byte 3:
neither `=` | `c4=='='` only | `c3=='='` and `c4=='='` | `c3=='='` but `c4!='='`
| `'='` at the `c1`/`c2` slots | `'='` interior to the string.

### Axis 6 — filtered-vs-raw length (filter loop, line 67)

no ignored bytes | some ignored bytes interspersed | all bytes ignored (`l==0`).

### Axis 7 — size

0 (error, see ERRORS.md) | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | many | 1 MiB.

### Axis 8 — byte range

ASCII base64 alphabet only | printable ASCII | full `0x01..0x7F` (incl.
controls) | full `0x01..0xFF` (incl. high-bit / negative `char`).

---

## Table — one row per combination the C actually distinguishes

Every row runs **both** `.so` exports and compares the *entire* `calloc`'d
region (`strlen(src) + 14` bytes, whose contents are fully determined because
`calloc` zeroes it) byte-for-byte — this catches over-writes and under-writes,
not just the NUL-terminated prefix. Rows marked *randomized* use
`≥ 256` seeded inputs (SplitMix64, fixed seed) rather than one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `decode_base64` | axis2=upper only; `l % 4 == 0`; no ignored bytes; sizes 4,8,12; *randomized* | `cfg_01_upper_only` | [x] |
| 2 | `decode_base64` | axis2=lower only; `l % 4 == 0`; *randomized* | `cfg_02_lower_only` | [x] |
| 3 | `decode_base64` | axis2=digits only; `l % 4 == 0`; *randomized* | `cfg_03_digits_only` | [x] |
| 4 | `decode_base64` | axis2=`'+'` only (value 62 path); lengths 1..8 | `cfg_04_plus_only` | [x] |
| 5 | `decode_base64` | axis2=`'/'` only (fallthrough → 63); lengths 1..8 | `cfg_05_slash_only` | [x] |
| 6 | `decode_base64` | full 64-char alphabet mixed, `l % 4 == 0`; *randomized* | `cfg_06_full_alphabet_mod0` | [x] |
| 7 | `decode_base64` | full alphabet, `l % 4 == 1` (dangling 1); *randomized* | `cfg_07_mod1` | [x] |
| 8 | `decode_base64` | full alphabet, `l % 4 == 2`; *randomized* | `cfg_08_mod2` | [x] |
| 9 | `decode_base64` | full alphabet, `l % 4 == 3`; *randomized* | `cfg_09_mod3` | [x] |
| 10 | `decode_base64` | canonical padding `xxx=` (`c4 == '='`, `c3 != '='`); *randomized* | `cfg_10_pad_one` | [x] |
| 11 | `decode_base64` | canonical padding `xx==` (`c3 == '='` **and** `c4 == '='`); *randomized* | `cfg_11_pad_two` | [x] |
| 12 | `decode_base64` | pathological `x=x=` / `x==x` (`'='` at `c2`; `c3=='='` with `c4!='='`); *randomized* | `cfg_12_pad_pathological` | [x] |
| 13 | `decode_base64` | `'='` at the `c1` slot (`"=xxx"`, `"===x"`); *randomized* | `cfg_13_pad_leading` | [x] |
| 14 | `decode_base64` | `'='` interior, followed by more data (multi-group, decoding continues) ; *randomized* | `cfg_14_pad_interior_multigroup` | [x] |
| 15 | `decode_base64` | axis6=ignored bytes interspersed among valid ones (printable non-alphabet); *randomized* | `cfg_15_ignored_interspersed` | [x] |
| 16 | `decode_base64` | axis6=**all** bytes ignored → `l == 0`, loop never entered; *randomized* | `cfg_16_all_ignored` | [x] |
| 17 | `decode_base64` | axis8=full `0x01..0xFF` random bytes (incl. negative `char`, controls); *randomized*, 2000 cases | `cfg_17_arbitrary_bytes` | [x] |
| 18 | `decode_base64` | axis7=exhaustive short lengths 1..=8 over the alphabet; *randomized* per length | `cfg_18_all_short_lengths` | [x] |
| 19 | `decode_base64` | axis7=exhaustive **all 255** single-byte inputs `0x01..0xFF` | `cfg_19_single_byte_exhaustive` | [x] |
| 20 | `decode_base64` | axis7=exhaustive **all 65 025** two-byte inputs over `0x01..0xFF` | `cfg_20_two_byte_exhaustive` | [x] |
| 21 | `decode_base64` | axis7=large input, 1 MiB of mixed alphabet + ignored bytes | `cfg_21_large_input` | [x] |
| 22 | `decode_base64` | boundary chars only: `@ A Z [ ` a z { / 0 9 : + * , =` (one either side of every range check) | `cfg_22_range_boundaries` | [x] |
| 23 | `decode_base64` | repeated calls / no cross-call state (same input 3×, interleaved with others) | `cfg_23_no_hidden_state` | [x] |
| 24 | `decode_base64` | real-world base64 round trips (known vectors incl. RFC 4648) | `cfg_24_known_vectors` | [x] |
