# ERRORS.md — Phase A: error-surface table

Mechanically derived from every rejection construct in `c_src/src/lib.c`.
Grep inventory of the whole C source:

```
$ grep -n 'abort\|return\|assert\|NULL\|if (' c_src/src/lib.c
12:    if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
13:        abort();
26:    return hex;
```

So the *entire* rejection surface is **one `if`, with two `||`-joined
conditions, whose consequence is `abort()`** (i.e. `SIGABRT`, not an error
return). There are:

* no error-return macros (`RETURN_ERROR`, …), no error enums, no `errno` use;
* no `assert()`;
* **no NULL checks at all** — so a NULL `hex` or NULL `bin` is *not* rejected;
  it is dereferenced, and the observable C behaviour is `SIGSEGV`. That is
  still a real input the Rust must reproduce, so it is tabulated below;
* **no enum parameters anywhere in the public API** (`lib.h` declares only
  `char*`/`size_t`/`const uint8_t*`), so the "out-of-range enum value across
  FFI" class does not exist here. The analogous "value with no valid variant"
  class for this API is an out-of-range *length*, rows 1–13.

The two conditions are `||`-short-circuited, so their **order matters** and is
tested separately (rows 5, 12): condition A is evaluated first and, when it
fires, `abort()` happens *before* any pointer is dereferenced.

`SIZE_MAX / 2` as spelled in the C source
= `18446744073709551615 / 2` = `9223372036854775807` = `0x7FFF_FFFF_FFFF_FFFF`.

Legend for "expected C result": `SIGABRT` = killed by signal 6 from `abort()`;
`SIGSEGV` = killed by signal 11 from an unchecked dereference; `ok` = returns.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `bin2hex` | cond A boundary: `bin_len == SIZE_MAX/2` (`0x7FFFFFFFFFFFFFFF`), `hex_maxlen == usize::MAX` | `abort()` → `SIGABRT` | `err01_bin_len_eq_size_max_half` | [x] |
| 2 | `bin2hex` | cond A: `bin_len == SIZE_MAX/2 + 1` (`0x8000000000000000`), `hex_maxlen == usize::MAX` | `abort()` → `SIGABRT` | `err02_bin_len_gt_size_max_half` | [x] |
| 3 | `bin2hex` | cond A: `bin_len == SIZE_MAX` (`0xFFFFFFFFFFFFFFFF`), `hex_maxlen == usize::MAX` | `abort()` → `SIGABRT` | `err03_bin_len_size_max` | [x] |
| 4 | `bin2hex` | cond A fires even though cond B would be false — proves A is not skipped: `bin_len = 0x7FFFFFFFFFFFFFFF`, `hex_maxlen = usize::MAX` (`> bin_len*2` mod 2^64) | `abort()` → `SIGABRT` | `err04_cond_a_independent_of_cond_b` | [x] |
| 5 | `bin2hex` | cond A short-circuits **before** any dereference: `bin_len = SIZE_MAX`, `hex = NULL`, `bin = NULL` | `abort()` → `SIGABRT` (never `SIGSEGV`) | `err05_cond_a_precedes_deref` | [x] |
| 6 | `bin2hex` | cond B with empty input: `bin_len == 0`, `hex_maxlen == 0` (`0 <= 0` is true) | `abort()` → `SIGABRT` | `err06_zero_len_zero_maxlen` | [x] |
| 7 | `bin2hex` | cond B exact boundary: `hex_maxlen == bin_len*2` (`bin_len = 1`, `hex_maxlen = 2`) — no room for the NUL | `abort()` → `SIGABRT` | `err07_maxlen_exactly_twice` | [x] |
| 8 | `bin2hex` | cond B, `hex_maxlen < bin_len*2` (`bin_len = 8`, `hex_maxlen = 3`) | `abort()` → `SIGABRT` | `err08_maxlen_less_than_twice` | [x] |
| 9 | `bin2hex` | cond B, `hex_maxlen == 0` with `bin_len > 0` (`bin_len = 1`) | `abort()` → `SIGABRT` | `err09_zero_maxlen_nonzero_len` | [x] |
| 10 | `bin2hex` | cond B at large scale: `bin_len = 1<<20`, `hex_maxlen = 2<<20` (exactly twice) | `abort()` → `SIGABRT` | `err10_large_maxlen_exactly_twice` | [x] |
| 11 | `bin2hex` | cond B sweep: for `bin_len` in `0..=64`, every `hex_maxlen` in `0..=bin_len*2` | `abort()` → `SIGABRT` for all | `err11_cond_b_exhaustive_sweep` | [x] |
| 12 | `bin2hex` | passes both checks, then dereferences NULL `hex`: `hex = NULL`, `hex_maxlen = 1`, `bin = NULL`, `bin_len = 0` (writes `hex[0] = 0`) | `SIGSEGV` (not `SIGABRT`) | `err12_null_hex_writes_nul` | [x] |
| 13 | `bin2hex` | passes both checks, then dereferences NULL `bin`: `bin = NULL`, `bin_len = 1`, valid 3-byte `hex` | `SIGSEGV` | `err13_null_bin_read` | [x] |
| 14 | `bin2hex` | one step *inside* the valid range (must **not** abort): `hex_maxlen == bin_len*2 + 1` | `ok`, returns `hex` | `err14_min_valid_maxlen_not_rejected` | [x] |
| 15 | `bin2hex` | largest `bin_len` that escapes cond A (`SIZE_MAX/2 - 1`) with `hex_maxlen = usize::MAX`: no abort, then runs off the end of a guard-page-terminated `hex` | `SIGSEGV`, after writing an identical page of hex output | `err15_oversized_len_runs_off_guard_page` | [x] |
| 16 | `bin2hex` | `bin` runs into a `PROT_NONE` guard page (`bin_len` larger than the mapped `bin`), `hex` large enough | `SIGSEGV`, after writing an identical prefix | `err16_bin_read_faults_at_guard_page` | [x] |

All 16 rows are exercised by `tests/differential_errors.rs`, each comparing the
**exact** termination status (signal number, or exit-code/return-value on the
non-aborting rows) of the C `.so` against the Rust `.so`, not merely "both
failed somehow". Rows that kill the process are run in a `fork()`ed child with
`RLIMIT_CORE = 0`.
