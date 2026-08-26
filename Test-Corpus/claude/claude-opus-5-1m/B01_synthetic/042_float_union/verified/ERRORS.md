# ERRORS.md — error-surface table (Phase C)

## How this table was derived

`c_src/src/main.c` in full:

```c
void driver(double f) {
    raw_double_t u = {.f = f};
    printf("%llx %a %.4f\n", u.x, f, f);
}

int main() {
    double f = 0.0f;
    scanf("%lf", &f);
    driver(f);
    return 0;
}
```

Mechanical grep of the C source for every rejection construct:

| construct | occurrences |
|-----------|-------------|
| `return -1` / `return NULL` / `RETURN_ERROR` / error enum | **0** |
| `assert` | **0** |
| explicit range check / `if` / comparison | **0** |
| null check | **0** |
| min/max constant | **0** |
| `goto` / early return | **0** (only the final `return 0`) |

So the program itself contains **no** error paths, no validation, and always
returns `0`. `driver()` accepts *every* one of the 2^64 bit patterns a `double`
can hold and never rejects anything.

The entire error surface is therefore the one library call that can *fail*:

```c
    scanf("%lf", &f);          /* return value ignored */
```

Because the return value is discarded and `scanf` does not store through `&f`
unless the conversion succeeds, **every** rejection — matching failure (`scanf`
returns `0`) and input failure (`scanf` returns `EOF`) alike — has exactly one
observable consequence: `f` keeps its initialiser `0.0f`, and the program prints

```
0 0x0p+0 0.0000
```

Each row below is one distinct condition under which glibc's `%lf` conversion
rejects the input, i.e. one `conv_error()` / `input_error()` / "no conversion
performed by `strtod`" branch that the Rust `scan_double()` must reproduce with a
`None`. Rows 20–24 are the generic C-API boundaries (empty/oversized input, one
step past a valid range, values with no valid variant crossing the FFI boundary).

`R` = the value glibc's `strtod` returns; `f=0.0` means the conversion was
rejected and `f` was left untouched.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `main`/`scanf` | EOF immediately — empty input | input failure, `f=0.0` → `0 0x0p+0 0.0000` |
| 2 | `main`/`scanf` | only white space, then EOF (`"   "`, `"\n\t\v\f\r "`) | input failure, `f=0.0` → `0 0x0p+0 0.0000` |
| 3 | `main`/`scanf` | EOF straight after the sign (`"-"`, `"+"`) | matching failure, `f=0.0` |
| 4 | `main`/`scanf` | sign followed by a non-numeric byte (`"-a"`, `"+z"`, `"- 1"`) | matching failure, `f=0.0` |
| 5 | `main`/`scanf` | first non-space byte cannot start a number (`"a"`, `"@"`, `"e5"`, `"p1"`, `"x1"`, `"\0"`, `"\x80"`) | matching failure, `f=0.0` |
| 6 | `main`/`scanf` | decimal point only, no digit anywhere (`"."`, `"-."`, `"+."`, `".e5"`, `".-"`) | `strtod` performs no conversion, `f=0.0` |
| 7 | `main`/`scanf` | `"n"` then EOF | matching failure, `f=0.0` |
| 8 | `main`/`scanf` | `"n"` then a byte other than `a`/`A` (`"nb"`) | matching failure, `f=0.0` |
| 9 | `main`/`scanf` | `"na"` then EOF | matching failure, `f=0.0` |
| 10 | `main`/`scanf` | `"na"` then a byte other than `n`/`N` (`"nax"`) | matching failure, `f=0.0` |
| 11 | `main`/`scanf` | `"i"` then EOF | matching failure, `f=0.0` |
| 12 | `main`/`scanf` | `"i"` then a byte other than `n`/`N` (`"ix"`) | matching failure, `f=0.0` |
| 13 | `main`/`scanf` | `"in"` then EOF | matching failure, `f=0.0` |
| 14 | `main`/`scanf` | `"in"` then a byte other than `f`/`F` (`"ing"`) | matching failure, `f=0.0` |
| 15 | `main`/`scanf` | `"infi"` then EOF — the `"inf"` already matched is **discarded** | matching failure, `f=0.0` (**not** `inf`) |
| 16 | `main`/`scanf` | `"infi"` then a byte that derails `"nity"` (`"infix"`, `"infinx"`, `"infinix"`, `"infinitx"`) | matching failure, `f=0.0` (**not** `inf`) |
| 17 | `main`/`scanf` | `"infinit"` then EOF (truncated `"infinity"`) | matching failure, `f=0.0` |
| 18 | `main`/`scanf` | hex prefix with nothing after it: the accumulated buffer is exactly `0x`/`0X` (`"0x"`, `"0X"` + EOF, `"0xg"`, `"0x "`, `"0xp1"`) | matching failure (hex prefix seen but no hex digit), `f=0.0` |
| 19 | `main`/`scanf` | the **signed** form of row 18 — buffer is exactly `-0x`/`+0X`, a three-character buffer and therefore a different length test from row 18 (`"-0x"`, `"+0X"`, `"-0xz"`) | matching failure, `f=0.0` (**not** `-0.0`, even though `strtod("-0x")` on its own would yield `-0.0`) |
| 20 | `main`/`scanf` | input consisting of a single NUL byte, or a lone high byte `\xff` | matching failure, `f=0.0` |
| 21 | `main`/`scanf` | oversized input: 100 000 leading spaces then EOF; 10 000-digit run with no exponent | row 2 behaviour / correctly-rounded finite value (no overflow, no crash) |
| 22 | `main`/`scanf` | one step past the representable range: `"1e309"`, `"-1e309"`, `"1e-324"`, `"0x1p1024"`, `"0x1p-1075"` | `ERANGE` but conversion **succeeds**: `±inf`, `±0`, `inf`, `0` |
| 23 | `driver` | every FFI-boundary "no valid variant" bit pattern: all 2048 exponent-field values x both signs x {0, 1, max} mantissa, quiet **and signalling** NaN payloads, `0x7ff0000000000001`, `0xffffffffffffffff`, `0x8000000000000000` | never rejects — prints `%llx %a %.4f` of that exact pattern |
| 24 | `driver`/`main` | no pointer argument exists anywhere in the API (`driver` takes a `double` by value, `main` takes none), so there is no null-pointer row to test | n/a — documented for completeness |

## Status

All rows are exercised by `tests/errors.rs`, which for every row feeds the exact
trigger to **both** shared objects (`main` via `dlsym`, with `stdin` redirected)
**and** to both executables, and asserts the two produce byte-identical output —
i.e. the same rejection, not merely "both failed somehow". Row 23 is exercised
through `driver` directly in `tests/ffi_driver.rs` as well.

| # | test | inputs | status |
|---|------|--------|--------|
| 1 | `errors::row_01_empty_input` | 1 | [x] |
| 2 | `errors::row_02_whitespace_only` | 10 | [x] |
| 3 | `errors::row_03_sign_then_eof` | 5 | [x] |
| 4 | `errors::row_04_sign_then_non_numeric` | 15 | [x] |
| 5 | `errors::row_05_bad_first_byte` | 54 | [x] |
| 6 | `errors::row_06_dot_without_digits` | 17 | [x] |
| 7 | `errors::row_07_n_then_eof` | 5 | [x] |
| 8 | `errors::row_08_n_then_wrong_byte` | 11 | [x] |
| 9 | `errors::row_09_na_then_eof` | 6 | [x] |
| 10 | `errors::row_10_na_then_wrong_byte` | 9 | [x] |
| 11 | `errors::row_11_i_then_eof` | 5 | [x] |
| 12 | `errors::row_12_i_then_wrong_byte` | 9 | [x] |
| 13 | `errors::row_13_in_then_eof` | 6 | [x] |
| 14 | `errors::row_14_in_then_wrong_byte` | 9 | [x] |
| 15 | `errors::row_15_infi_then_eof` | 7 | [x] |
| 16 | `errors::row_16_infi_then_wrong_byte` | 13 (+8 accepted-boundary) | [x] |
| 17 | `errors::row_17_truncated_infinity` | 12 | [x] |
| 18 | `errors::row_18_bare_hex_prefix` | 18 | [x] |
| 19 | `errors::row_19_signed_bare_hex_prefix` | 13 (+6 `0x.` +6 `-0x.`) | [x] |
| 20 | `errors::row_20_nul_and_high_bytes` | 162 | [x] |
| 21 | `errors::row_21_oversized_input` | 1 + 4 + 3 (100 kB / 10 000-digit / 10 000-digit-exponent) | [x] |
| 22 | `errors::row_22_one_past_range` | 6 + 4 + 5 + 4 + 1 + 3 | [x] |
| 23 | `errors::row_23_driver_accepts_every_bit_pattern` | 11 corner patterns (+12 288 exhaustive in `ffi_driver`) | [x] |
| 24 | n/a — the API has no pointer parameter (`driver` takes a `double` by value, `main` takes none) | – | [x] |

Every row above runs its inputs through **four** paths and requires all of them to
agree: the C `.so`'s `main` via `dlsym`, the Rust `.so`'s `main` via `dlsym`, the
C executable, and the Rust executable.  Rows that expect a rejection additionally
assert the exact bytes `0 0x0p+0 0.0000`, and rows 19/22 assert the exact accepted
values of the neighbouring inputs, so "both failed somehow" cannot pass.

## Negative control

Removing the sign term from the bare-hex-prefix check in `src/imp.rs`
(`w.len() == 2 + got_sign as usize` → `w.len() == 2`, which makes `-0x` convert to
`-0.0` instead of being rejected) makes **row 19 fail** with

```
MISMATCH [row19 signed bare hex prefix] stdin="-0x" (len 3, bytes [2d, 30, 78])
```

and leaves every other row passing — i.e. the rows are individually meaningful.
Notably a flat random-byte fuzzer did *not* find this: it only showed up once the
generator was given a token grammar (`ffi_main::row_34_random_soup`), which is why
this table is derived from the C's rejection branches rather than from fuzzing.
