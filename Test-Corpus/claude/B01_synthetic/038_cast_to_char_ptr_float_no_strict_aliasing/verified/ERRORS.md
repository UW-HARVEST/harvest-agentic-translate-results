# ERRORS.md — Phase A error-surface table

## How this table was derived

`c_src/src/main.c` is 45 lines and contains **no** `RETURN_ERROR` macro, no
`assert`, no `return -1`, no `return NULL`, no error enum and no explicit range
or null check:

```
$ grep -nE 'return|assert|NULL|if |else|while |for ' c_src/src/main.c
28:    for (int i = 0; i < len; i++) {
44:    return 0;
```

The program's entire rejection surface therefore lives in the *one* library
call that can fail and whose result the C code deliberately **ignores**:

```c
int main() {
    float x = 0.f;      /* <- the value that survives every rejection */
    scanf("%f", &x);    /* return value ignored: no assignment on failure */
    driver(x);
    return 0;
}
```

Consequently **every rejection has the same observable effect**: `x` keeps its
initialiser `0.f`, so the program prints `00000000` and exits 0.  Note this is
*positive* zero — the sign of a rejected `"-…"` input is **not** applied,
because glibc keeps the leading sign out of the buffer it hands to `strtof`
and only negates *after* a successful conversion.

The rows below enumerate every distinct condition under which glibc's `%f`
conversion rejects its input.  They were derived from the character-collection
state machine in glibc's `__vfscanf_internal` (the `case L_('e')` float arm)
and each one was then *confirmed against the compiled C program* — the
`expected C result` column is the C binary's actual output, not a guess.

`ERANGE` results (overflow/underflow) are included because `scanf` still
*assigns* in those cases; they are error-ish outcomes rather than rejections.

## Table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `main`/`scanf("%f")` | empty stdin — EOF before any character (*input failure*) | no assignment; `x == +0.0f`; prints `00000000`, exit 0 |
| 2  | `main`/`scanf("%f")` | stdin is whitespace only (`" \t\n\v\f\r"`) — EOF while skipping whitespace | `00000000`, exit 0 |
| 3  | `main`/`scanf("%f")` | EOF immediately after a leading sign (`"-"`, `"+"`) | `00000000` (**not** `00000080`), exit 0 |
| 4  | `main`/`scanf("%f")` | first non-space character cannot start a number and is not `n`/`i` (`"z"`, `"_"`, `","`, `"("`, `"e5"`, `"E"`, `"*"`) → collection buffer stays empty | `00000000`, exit 0 |
| 5  | `main`/`scanf("%f")` | a second sign directly after the first (`"--1"`, `"+-1"`, `"-+"`) | `00000000`, exit 0 |
| 6  | `main`/`scanf("%f")` | NUL byte or byte ≥ 0x80 as the first non-space character (`"\0"`, `"\x80"`, `"\xff"`, UTF-8 `"π"`) | `00000000`, exit 0 |
| 7  | `main`/`scanf("%f")` | `n`/`N` not followed by `a`/`A` (`"n"`, `"nx"`, `"n5"`, `"N"`) | `00000000`, exit 0 |
| 8  | `main`/`scanf("%f")` | `na` not followed by `n`/`N` (`"na"`, `"nax"`, `"na5"`, `"-na"`) | `00000000`, exit 0 |
| 9  | `main`/`scanf("%f")` | `i`/`I` not followed by `n`/`N` (`"i"`, `"ix"`, `"i5"`) | `00000000`, exit 0 |
| 10 | `main`/`scanf("%f")` | `in` not followed by `f`/`F` (`"in"`, `"inx"`, `"in5"`, `"-in"`) | `00000000`, exit 0 |
| 11 | `main`/`scanf("%f")` | `inf` followed by `i`/`I` but **not** the complete word `infinity` (`"infi"`, `"infin"`, `"infini"`, `"infinit"`, `"infix"`, `"infinit1"`, `"-infin"`) — glibc commits to the long spelling and then fails | `00000000`, exit 0 |
| 12 | `main`/`scanf("%f")` | `0x`/`0X` prefix followed by EOF (`"0x"`, `"0X"`, `"-0x"`) → buffer is exactly `"0x"` | `00000000` (**not** `00000080` for `"-0x"`), exit 0 |
| 13 | `main`/`scanf("%f")` | `0x`/`0X` followed by a character that is neither a hex digit nor `.` (`"0xg"`, `"0x,"`, `"0x_"`, `"0x-"`, `"0x+"`, `"0x "`) | `00000000`, exit 0 |
| 14 | `main`/`scanf("%f")` | `0x`/`0X` followed directly by the exponent char (`"0xp1"`, `"0xP"`, `"-0xp1"`) — `p` is only accepted after a digit, so the buffer is still exactly `"0x"` | `00000000`, exit 0 |
| 15 | `main`/`scanf("%f")` | collected buffer is exactly `"."` — no digit anywhere (`"."`, `".e5"`, `".x"`, `"-."`, `"-.e5"`, `".."`) → `strtof` consumes nothing | `00000000`, exit 0 |
| 16 | `main`/`scanf("%f")` | overflow: magnitude above `FLT_MAX` (`"1e39"`, `"3.4028236e38"`, `"0x1p128"`, `"1e400"`, `"1e99999999999999999999"`) → `ERANGE`, but the value **is** assigned | `±HUGE_VALF`: `0000807f` / `000080ff` |
| 17 | `main`/`scanf("%f")` | underflow: magnitude below half the smallest subnormal (`"1e-46"`, `"7e-46"`, `"0x1p-150"`, `"1e-400"`, `"1e-99999999999999999999"`) → `ERANGE`, value assigned | `±0`: `00000000` / `00000080` |
| 18 | `main`/`scanf("%f")` | exponent present but with no digits after `e`/`E`/`p`/`P` or after its sign (`"1e"`, `"1e+"`, `"1e-"`, `"0x1p"`, `"0x1p-"`) — the exponent is dropped, the mantissa still converts | mantissa only: `"1e"` → `0000803f` |
| 19 | `main`/`scanf("%f")` | trailing garbage that stops the scan (`"1.5abc"`, `"1_000"`, `"1,5"`, `"1.5.5"`, `"1e5e5"`, `"0x1p1p1"`) — not an error, the prefix converts and the rest stays unread | prefix value, e.g. `"1.5abc"` → `0000c03f` |
| 20 | `driver` (FFI) | any `float` bit pattern, including values that are "invalid" as numbers: quiet NaN, signalling NaN, negative NaN, ±inf, ±0, subnormals | prints the 4 native-order bytes verbatim, e.g. sNaN `0x7fa00000` → `0000a07f` |
| 21 | `driver` (FFI) | out-of-range "enum-like" integer reinterpreted as a float — every one of the 2^32 bit patterns is a valid argument, there is no rejection path | byte-exact echo of the pattern (no NaN canonicalisation) |
| 22 | `print_hex` (internal) | `len == 0` would print only `"\n"`; unreachable from the public API because `driver` always passes `sizeof(float) == 4` | n/a — fixed at 4, verified by the 4-byte output width |

## Checklist

| # | differential test | status |
|---|-------------------|--------|
| 1  | `tests/error_paths.rs::eof_and_whitespace_only` | [x] |
| 2  | `tests/error_paths.rs::eof_and_whitespace_only` | [x] |
| 3  | `tests/error_paths.rs::eof_after_sign` | [x] |
| 4  | `tests/error_paths.rs::first_char_cannot_start_a_number` | [x] |
| 5  | `tests/error_paths.rs::double_sign` | [x] |
| 6  | `tests/error_paths.rs::non_ascii_and_nul_bytes` | [x] |
| 7  | `tests/error_paths.rs::nan_word_truncated` | [x] |
| 8  | `tests/error_paths.rs::nan_word_truncated` | [x] |
| 9  | `tests/error_paths.rs::inf_word_truncated` | [x] |
| 10 | `tests/error_paths.rs::inf_word_truncated` | [x] |
| 11 | `tests/error_paths.rs::inf_commits_to_infinity` | [x] |
| 12 | `tests/error_paths.rs::hex_prefix_only` | [x] |
| 13 | `tests/error_paths.rs::hex_prefix_then_non_hex` | [x] |
| 14 | `tests/error_paths.rs::hex_prefix_then_exponent_char` | [x] |
| 15 | `tests/error_paths.rs::lone_decimal_point` | [x] |
| 16 | `tests/error_paths.rs::overflow_to_infinity` | [x] |
| 17 | `tests/error_paths.rs::underflow_to_zero` | [x] |
| 18 | `tests/error_paths.rs::exponent_without_digits` | [x] |
| 19 | `tests/error_paths.rs::trailing_garbage_stops_scan` | [x] |
| 20 | `tests/ffi_driver_diff.rs::ffi_differential_suite` (named-specials section) | [x] |
| 21 | `tests/ffi_driver_diff.rs::ffi_differential_suite` (exhaustive + random bit patterns) | [x] |
| 22 | `tests/ffi_driver_diff.rs::ffi_differential_suite` (output-shape section) | [x] |

Additional generic-boundary tests beyond the table (all passing):

| test | covers |
|------|--------|
| `tests/error_paths.rs::closed_stdin` | fd 0 *closed* (read fails with `EBADF`, not EOF) |
| `tests/error_paths.rs::every_single_byte_boundary` | all 256 byte values alone and in 8 positions inside otherwise-valid tokens |
| `tests/error_paths.rs::one_step_past_range_ends` | one step past every documented range end (smallest subnormal, largest subnormal, smallest normal, largest finite, 24-bit integer boundary), both signs |
| `tests/error_paths.rs::hex_prefix_then_exponent_char` (complement) | `0xe`/`0xE` — `e` is a hex *digit*, not the exponent char, so these are **not** rejections (14.0f) |
| `tests/error_paths.rs::inf_commits_to_infinity` (complement) | the accepted side of row 11: `inf`, `inf`+non-`i`, and full `infinity` with trailing junk |

Note on null pointers and out-of-range enums: the C public surface is
`void driver(float)` and `int main(void)` — it has **no pointer parameter and no
enum parameter**, so there is nothing to pass a null pointer or an out-of-range
enumerator to.  The equivalent "any bit pattern crosses the FFI boundary" test
is row 21: all 2^32 `float` encodings are legal arguments (there is no rejection
path) and 145 000+ of them, including every exponent field, every NaN payload
class and both signs, are compared byte-for-byte through `dlsym`.
