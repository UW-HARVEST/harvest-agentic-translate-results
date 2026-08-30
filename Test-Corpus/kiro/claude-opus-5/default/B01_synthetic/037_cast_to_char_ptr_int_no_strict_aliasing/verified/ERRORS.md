# ERRORS.md — differential findings

Program under test: `c_src/src/main.c` vs `translation/src/main.rs`.

```c
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

`driver` memcpy's `x`'s four object-representation bytes into `char raw[4]` and
`print_hex` prints them as lower-case `%02x`, followed by one `'\n'`.
Exit status is always 0; stderr is always empty.

## Enumerated input classes (all branch points in the C)

The only conditional behavior lives inside the single `%d` conversion; the
`print_hex` loop always runs exactly `sizeof(int)` = 4 times.

| Class | Example input | C behavior |
|---|---|---|
| Input failure (EOF before any non-whitespace) | `""`, `"\n"`, `"   "`, `/dev/null`, closed stdin | `scanf` returns `EOF`, `x` keeps its initialiser `0` → `00000000` |
| Matching failure | `"abc"`, `"-"`, `"+"`, `"--5"`, `"- 5"`, `".5"`, `"\0\0 5"`, `"\xff\xfe 9"`, `"\xc2\xa0 5"` | `scanf` returns `0`, nothing stored, `x` stays `0` → `00000000` |
| Success, in-range | `"0"`, `"1"`, `"-1"`, `"42"`, `"+5"`, `"007"` | value stored, little-endian bytes printed |
| Leading whitespace skipped across newlines | `"\n\n9"`, `" \t\n\r\n 7"`, 8 KiB of spaces then digits | whitespace consumed, then converted |
| Only the first token read | `"1 2"`, `"12abc"`, `"1.5"`, `"1,234"`, `"0x10"`, `"1e9"` | conversion stops at first non-digit; remainder of stdin never read |
| `int` boundaries | `2147483647`, `-2147483648` | exact |
| Past `int`, inside `long` | `2147483648`, `4294967295`, `2^32`, `2^33` | `strtol` yields the full `long`; the store through `int *` truncates to the low 32 bits |
| Past `long` (ERANGE) | `9223372036854775808`, `2^64`, 5000 nines | `strtol` saturates to `LONG_MAX`/`LONG_MIN` *first*, then truncates → `ffffffff` / `00000000` |
| Leading zeros | `"0000…005"`, 10 000 zeros then `5` | digits, not an octal prefix; do not count toward overflow |
| Arguments | `driver 99 --help` | `main()` takes none; ignored |

## Mismatches found

**None.** Every enumerated input above, plus a randomized sweep of 4 000 inputs
drawn from the alphabet `0-9 + - . , e x _ a b c ' \t \n \r \v \f \0 \xff` and a
boundary sweep around 0, 2^8, 2^16, 2^24, 2^31, 2^32, 2^63, 2^64, 10^19 and
10^20 (each ±2, with and without a sign), produced byte-identical stdout,
byte-identical stderr and identical exit status.

The pre-existing Rust translation already reproduced the two behaviors most
likely to diverge:

1. **`scanf` crosses newlines while skipping leading whitespace** (unlike
   `fgets`). `Input::peek`/`bump` skip all six C-locale whitespace bytes,
   including `\n`, `\v` and `\f`, before looking for a sign.
2. **Two-stage overflow: saturate at `long`, then truncate to `int`.** glibc's
   `%d` runs `strtol`, which clamps to `LONG_MAX`/`LONG_MIN` on ERANGE; the
   assignment through `int *` then keeps only the low 32 bits. Accumulating in
   `i128` and clamping to `i64` *before* the `as i32` cast is what makes
   `9223372036854775808` print `ffffffff` rather than `00000000`. A naive
   `i32`-parse-and-clamp translation would differ here on many inputs.

## Latent divergence corrected (not observed on this target)

`driver` originally used `x.to_le_bytes()`, which hard-codes little-endian.
The C uses `memcpy(raw, &x, sizeof(x))`, i.e. the *native* object
representation. Changed to `x.to_ne_bytes()`. On the x86-64 test target the two
are identical, so this fixed no observed mismatch, but it is the faithful
translation of `memcpy` and keeps the programs in agreement on a big-endian
build.

## Not divergent, checked explicitly

* stdout redirected to a closed descriptor or to `/dev/full`: both exit 0
  (the C ignores `printf`'s return value; the Rust ignores the `io::Result`).
* stdin closed (`0<&-`): both treat it as input failure → `00000000`, exit 0.
* Non-UTF-8 and embedded-NUL stdin: handled byte-wise on both sides, no panic.
