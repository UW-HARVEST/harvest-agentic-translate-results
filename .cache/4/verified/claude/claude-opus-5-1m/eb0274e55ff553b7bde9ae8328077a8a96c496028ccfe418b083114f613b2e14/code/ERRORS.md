# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/main.c`. The full C source is:

```c
static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) {
        printf("%02x", p[i]);
    }
    printf("\n");
}

void driver(float x) {
    print_hex((unsigned char *)&x, sizeof(x));
}

int main() {
    float x = 0.f;
    scanf("%f", &x);
    driver(x);
    return 0;
}
```

## Mechanical grep for rejection sites

```
$ grep -nE 'return|assert|if|NULL|-1|error|ERROR|exit' c_src/src/main.c
27:    for (int i = 0; i < len; i++)      <- loop bound, only "check" in the file
41:    return 0;                          <- unconditional success
```

There is **no** `RETURN_ERROR`, no `assert`, no null check, no range check
and no error enum anywhere in the C source. The return value of `scanf` is
**ignored**. Therefore the entire error surface of this program lives in the
one libc call `scanf("%f", &x)`, plus the `strtof`-equivalent conversion it
performs internally, plus the two total-function bodies `driver`/`print_hex`
which have no failure mode at all.

The observable consequence of *every* rejection is identical in kind: `x`
retains its initialiser `0.f`, `driver(0.f)` runs anyway, and the program
prints the bytes of `+0.0f` and exits `0`. Encoded as
`00000000\n` (little-endian x86-64), exit status `0`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `main`/`scanf %f` | **input failure**: EOF reached with nothing in the stream at all (empty stdin) | `scanf` → `EOF`, `x` unchanged `+0.0f`, stdout `00000000\n`, exit 0 | [x] |
| 2 | `main`/`scanf %f` | **input failure**: stream contains only whitespace, EOF hit while skipping it (`" "`, `"\n"`, `"\t\v\f\r "`) | `EOF`, `00000000\n`, exit 0 | [x] |
| 3 | `main`/`scanf %f` | **matching failure**: first non-whitespace byte can never begin a float (`"abc"`, `"x"`, `"z"`, `"@"`, `"/"`) | `scanf` → `0`, `00000000\n`, exit 0 | [x] |
| 4 | `main`/`scanf %f` | **matching failure**: lone decimal point with no digit anywhere (`"."`, `".."`, `".e5"`, `".-5"`) | `0`, `00000000\n`, exit 0 | [x] |
| 5 | `main`/`scanf %f` | **input failure**: optional sign consumed, then EOF (`"-"`, `"+"`) | `EOF`, `00000000\n`, exit 0 | [x] |
| 6 | `main`/`scanf %f` | **matching failure**: sign followed by a byte that cannot continue a float (`"-x"`, `"+."`, `"--1"`, `"+-1"`, `"- 1"`) | `0`, `00000000\n`, exit 0 | [x] |
| 7 | `main`/`scanf %f` | **matching failure**: `n`/`N` prefix not completed to `nan` (`"na"`, `"nax"`, `"n5"`, `"-na"`) | `0`, `00000000\n`, exit 0 | [x] |
| 8 | `main`/`scanf %f` | **input failure**: `n`/`na` then EOF (`"n"`, `"na"`) | `EOF`, `00000000\n`, exit 0 | [x] |
| 9 | `main`/`scanf %f` | **matching failure**: `i`/`I` prefix not completed to `inf` (`"ix"`, `"inx"`, `"in5"`) | `0`, `00000000\n`, exit 0 | [x] |
| 10 | `main`/`scanf %f` | **input failure**: `i`/`in` then EOF (`"i"`, `"in"`) | `EOF`, `00000000\n`, exit 0 | [x] |
| 11 | `main`/`scanf %f` | **matching failure**: `inf` followed by `i` but not completed to `infinity` (`"infi"`, `"infin"`, `"infini"`, `"infinit"`, `"infix"`) — the partially consumed `inity` prefix cannot be pushed back | `0`, `00000000\n`, exit 0 | [x] |
| 12 | `main`/`scanf %f` | **matching failure**: `0x`/`0X` prefix with no hex digit following (`"0x"`, `"0X"`, `"0xg"`, `"0x.g"`, `"-0x"`, `"0xz"`) — the `x` cannot be pushed back after being consumed | `0`, `00000000\n`, exit 0 | [x] |
| 13 | `main`/`scanf %f` | **matching failure**: only a sign + `0x` prefix, i.e. token length == sign+2 | `0`, `00000000\n`, exit 0 | [x] |
| 14 | `strtof` inside `%f` | **`ERANGE` overflow**: magnitude strictly greater than the largest representable `float` (`"1e39"`, `"1e308"`, `"3.4028236e38"`, `"0x1p128"`, `"340282366920938463463374607431768211456"`) | `±HUGE_VALF`, i.e. bits `7f800000` / `ff800000`, exit 0 | [x] |
| 15 | `strtof` inside `%f` | **`ERANGE` underflow**: nonzero magnitude below half the smallest subnormal (`"1e-50"`, `"1e-46"`, `"0x1p-150"`, `"7.0064923e-46"`) | `±0.0f`, i.e. bits `00000000` / `80000000`, exit 0 | [x] |
| 16 | `strtof` inside `%f` | **exponent field with `e`/`E` but no digits** (`"1e"`, `"1e+"`, `"1e-"`, `"1ee"`) — conversion uses the mantissa only, the exponent bytes are not part of the converted subject sequence | value of the mantissa (`"1e"` → `1.0f` → `3f800000`), exit 0 | [x] |
| 17 | `strtof` inside `%f` | **binary exponent field with `p`/`P` but no digits** (`"0x1p"`, `"0x1p+"`, `"0x1p-"`) | value of the hex mantissa (`0x1` → `1.0f` → `3f800000`), exit 0 | [x] |
| 18 | `strtof` inside `%f` | **exponent digit string that overflows every integer width** (`"1e999999999999999999999"`, `"1e-999999999999999999999"`, `"1e2147483647"`, `"1e4294967296"`) | saturates: `±inf` for huge positive, `±0` for huge negative, exit 0 | [x] |
| 19 | `strtof` inside `%f` | **zero mantissa with a huge exponent** (`"0e999999999999999999999"`, `"0e-999999999999999999999"`) — must **not** overflow to inf, zero mantissa short-circuits | `+0.0f` → `00000000`, exit 0 | [x] |
| 20 | `strtof` inside `%f` | **`nan` with an n-char-sequence payload** (`"nan(1)"`, `"nan(123)"`, `"nan(0x7f)"`, `"nan("`, `"nan()"`) | quiet NaN `7fc00000` (`-nan(...)` → `ffc00000`), exit 0 | [x] |
| 21 | `driver` | `x` is a value with no "valid" interpretation — signalling NaN, negative zero, subnormal, `±inf`. **No validation exists**; the raw object representation is printed | 4 bytes of `x` in memory order, exit 0 | [x] |
| 22 | `print_hex` | `len` is `sizeof(float)` == 4; the loop `i < len` is the only bound. Not reachable with any other `len` from `driver`, and `print_hex` is `static` so it is unreachable from outside the TU | exactly 8 hex digits + `\n` | [x] |
| 23 | `main` | **`scanf` return value discarded**: the program's exit status is `return 0` unconditionally, so *no* input can make it fail | exit 0 for every possible stdin, including a closed stdin | [x] |
| 24 | `main`/`scanf %f` | **embedded NUL and non-ASCII bytes** (`"\0"`, `"\x001"`, `"1\x002"`, `"\x80\xff"`) — `scanf` is byte-oriented, NUL is just a non-matching byte | NUL-first → matching failure `00000000\n`; `"1\0"` → `1.0f`; exit 0 | [x] |
| 25 | `main`/`scanf %f` | **stdin closed / not a readable fd** (`0<&-`) — read fails immediately | input failure, `00000000\n`, exit 0 | [x] |

25 rows. Every row is covered by a differential test in
`tests/error_paths.rs` (each row is a separately named `#[test]`), plus the
`ERRORS_ROWS` corpus in `tests/exe_diff.rs`.

## Generic boundaries also covered (not distinct C rejection sites)

| condition | where tested |
|---|---|
| null pointer passed for the `float*` out-param — impossible: `&x` is always a valid stack address, and `driver` takes `float` **by value** so no pointer crosses the FFI boundary | n/a by construction |
| zero-length input | row 1 |
| oversized input (500–2000 byte literals, 1000-digit mantissas) | `tests/exe_diff.rs::test_oversized_literals` |
| one step past a valid range (`FLT_MAX` vs next representable, `0x1p-149` vs `0x1p-150`) | rows 14/15 + `tests/exe_diff.rs::test_boundary_neighbours` |
| out-of-range enum values across the FFI boundary — **the C API has no enum parameter**: `driver`'s only parameter is `float`. The equivalent "every bit pattern is a legal input" check is done exhaustively-by-sampling over all 2^32 `f32` bit patterns, including the 16.7M NaN payloads that have no "valid variant" | `tests/ffi_diff.rs::test_driver_all_bit_pattern_classes` |
