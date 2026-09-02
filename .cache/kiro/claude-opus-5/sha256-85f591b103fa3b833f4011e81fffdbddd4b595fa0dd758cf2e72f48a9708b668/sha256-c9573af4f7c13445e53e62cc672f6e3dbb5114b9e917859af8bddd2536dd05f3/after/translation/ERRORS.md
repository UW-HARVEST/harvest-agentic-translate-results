# ERRORS.md — Error-surface table (Phase A)

Mechanically derived from `c_src/src/driver.c`. The C library has **no** error
enum, **no** `RETURN_ERROR` macro, **no** `assert`, **no** `return -1` and
**no** `return NULL`. Exhaustive grep evidence:

```sh
grep -nE 'assert|RETURN_ERROR|return *-1|return *NULL|errno|E[A-Z]+|INT_MIN|INT_MAX|return (true|false)' c_src/src/driver.c
```

```
28:#include <errno.h>
29:#include <limits.h>
60:static bool parse_val(const char *str, int *val) {
61:    errno = 0;
64:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
66:        return true;
68:        return false;
73:void driver(const char *in) {
75:    if (parse_val(in, &x)) {
79:    } else {
80:        printf("An error occurred\n");
81:    }
```

So the *entire* rejection surface is the single 4-conjunct guard on line 64 of
`parse_val`, plus the implicit pointer-dereference contracts. A failure of
**any** conjunct makes `parse_val` return `false`, which makes `driver` emit
exactly `An error occurred\n` and nothing else (in particular: **no**
`The house has …` lines at all, and the `house_t` is never even constructed).

`run` has no validation whatsoever — it is unconditionally 4 × `print_house`
with mutations interleaved — so it contributes only pointer-contract and
integer-overflow rows.

## Table

One row per distinct rejection / boundary condition the C code actually checks.
`[x]` = a differential test exists, calls BOTH `.so`s, and passes (Phase C).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `driver` → `parse_val` | conjunct 1 fails, `endp == str`: **empty string** `""` (strtol performs no conversion, leaves `endptr == nptr`) | prints exactly `An error occurred\n`, returns void | `e1_empty_string` | [x] |
| E2 | `driver` → `parse_val` | conjunct 1 fails: **whitespace-only** input (`" "`, `"\t"`, `"\n"`, `"\v"`, `"\f"`, `"\r"`, mixtures) | `An error occurred\n` | `e2_whitespace_only` | [x] |
| E3 | `driver` → `parse_val` | conjunct 1 fails: **sign only** (`"+"`, `"-"`, `"  +"`, `"--1"`, `"+-1"`) | `An error occurred\n` | `e3_sign_only` | [x] |
| E4 | `driver` → `parse_val` | conjunct 1 fails: **first non-space char is not a digit/sign** (`"abc"`, `"x12"`, `"."`, `".5"`, `"e5"`, `"#"`, `"/"` (0x2F), `":"` (0x3A), high-bit byte `"\xff"`) — plus an exhaustive sweep of all 255 single bytes and of `{'+','-',' ','\t'} × 255` | `An error occurred\n` | `e4_non_numeric_lead` | [x] |
| E5 | `driver` → `parse_val` | base-10 **hex/octal prefix** handling — `"0x"`, `"0x10"`, `"0X1F"`, `"017"` all begin with a digit so strtol converts and STOPS at the letter → *accepted*; `"x0"`/`"X"` is a no-conversion → rejected | `"0x10"`→accepted (0); `"017"`→accepted (17, not octal); `"x0"`→`An error occurred\n` | `e5_prefixes` | [x] |
| E6 | `driver` → `parse_val` | conjunct 2 fails, `errno == ERANGE`: magnitude **above `LONG_MAX`** (`"9223372036854775808"`, 20 nines, 400 nines, randomized) | `An error occurred\n` | `e6_erange_above_long_max` | [x] |
| E7 | `driver` → `parse_val` | conjunct 2 fails, `errno == ERANGE`: magnitude **below `LONG_MIN`** (`"-9223372036854775809"`, `-20 nines`, `-400 nines`, randomized) | `An error occurred\n` | `e7_erange_below_long_min` | [x] |
| E8 | `driver` → `parse_val` | conjunct 3 fails, `tmp < INT_MIN`: representable in `long` but **below `INT_MIN`** (`-2147483649`, `-3000000000`, `LONG_MIN` exactly — no errno; 300 randomized) | `An error occurred\n` | `e8_below_int_min` | [x] |
| E9 | `driver` → `parse_val` | conjunct 4 fails, `tmp > INT_MAX`: representable in `long` but **above `INT_MAX`** (`2147483648`, `3000000000`, `LONG_MAX` exactly — no errno; 300 randomized) | `An error occurred\n` | `e9_above_int_max` | [x] |
| E10 | `driver` → `parse_val` | **boundary, must be ACCEPTED**: `-2147483648` (`INT_MIN`) and `2147483647` (`INT_MAX`) — proves the range check is `>=`/`<=`, not `>`/`<`; paired with the one-step-past rejections | 8 `The house has …` lines (2 × `run`), no error message | `e10_inclusive_boundaries` | [x] |
| E11 | `driver` → `parse_val` | **trailing garbage is NOT rejected** (the C never checks `*endp == '\0'`): `"12abc"`, `"7 8"`, `"5-"`, `"1.9"`, `"1e5"`, `"3\xff"` | accepted with the leading integer | `e11_trailing_garbage_accepted` | [x] |
| E12 | `driver` → `parse_val` | **leading whitespace + sign + leading zeros are NOT rejected**: `"  \t\n+0000042xyz"` | accepted, value `42` | `e12_permissive_prefix_accepted` | [x] |
| E13 | `driver` → `parse_val` | pre-existing `errno` is **cleared** (line 61 `errno = 0;`) — caller sets `ERANGE`/`EINVAL`/`ENOMEM`/`1`/`4095` first, then a valid input | accepted (stale errno must not cause rejection) | `e13_stale_errno_cleared` | [x] |
| E14 | `driver` → `parse_val` | `errno` **left set** after a rejecting call: an `ERANGE` input followed by a valid input, including C→Rust and Rust→C cross-library sequences | 2nd call accepted | `e14_no_errno_leak_between_calls` | [x] |
| E15 | `driver` | `in == NULL` — **unchecked null**; passed straight to `strtol`, which dereferences it | UB; glibc faults → child killed by `SIGSEGV` (11). Compared via `fork`+`waitpid` status, so "both crashed the same way" is asserted, not merely "both failed" | `e15_driver_null_pointer` | [x] |
| E16 | `run` | `the_house == NULL` (for 5 different `extra_bedrooms`) and 4 other bogus pointers (`1`, `8`, `0xdeadbeef`, `usize::MAX & !7`) — **unchecked null**, `print_house` dereferences immediately | UB; faults → `SIGSEGV` (11), identical wait-status in both | `e16_run_null_pointer` | [x] |
| E17 | `run` | signed **overflow of `floors`**: `floors == INT_MAX`, `add_floor` does `house->floors++` with no guard (+200 randomized 6-call sequences near the boundary) | gcc `-O0` wraps: prints `2147483647` then `-2147483648` | `e17_floors_overflow` | [x] |
| E18 | `run` | signed **overflow of `bedrooms`**: `INT_MAX + positive` and `INT_MIN + negative`, incl. `INT_MAX+INT_MAX` and `INT_MIN+INT_MIN` (+300 randomized) | wraps two's-complement | `e18_bedrooms_overflow` | [x] |
| E19 | `run` | non-finite / extreme `bathrooms` (`NaN` both signs, NaN with payload, signalling NaN, `±inf`, `-0.0`, `1e308`, `f64::MAX/MIN`, `MIN_POSITIVE`, `5e-324`) × 5 `extra_bedrooms`, single and 4-call sequences | `printf` renders `nan`/`-nan`/`inf`/`-inf`/`-0.0`/309-digit expansion; `inf+1==inf`, `nan+1==nan`, `f64::MAX+1==f64::MAX` | `e19_extreme_bathrooms` | [x] |
| E20 | `run` | `extra_bedrooms ∈ {INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1}` × 60 random structs each | wraps as per E18 | `e20_extreme_extra_bedrooms` | [x] |
| E21 | `driver` | oversized input: 1 MiB of `1`s, 1 MiB of `-9`s, 1 MiB of spaces (all rejected), and 1 MiB of leading zeros + `42` (accepted — there is no length check anywhere) | as noted | `e21_oversized_input` | [x] |
| E22 | `driver` | **embedded NUL** truncates (no explicit length is ever taken): leading NUL, NUL after digits, NUL after a whitespace run | leading/ws-then-NUL reject; `"42\0xxxx"` parses `42` | `e22_embedded_nul` | [x] |

### Generic FFI-boundary coverage (required by Phase C beyond the table)

| # | what | test | [x] |
|---|------|------|-----|
| G1 | **Out-of-range "enum" values across the FFI boundary.** The library declares no `enum`, so the only integer crossing the boundary is `int extra_bedrooms`. It is swept over every ABI-relevant value a C enum could deliver — `INT_MIN`, `INT_MIN+1`, `±2^k`, `±(2^k−1)` for the 8/16/32-bit boundaries, `0`, `INT_MAX` — plus 100 random `i32`s, each against both a benign and an extreme `house_t`. | `generic_int_boundary_sweep` | [x] |
| G2 | Zero length, minimal lengths, lengths straddling the 10-vs-11-digit accept/reject boundary, and a 4 MiB oversized input. | `generic_length_boundaries` | [x] |
| G3 | Null pointers on both entry points, and non-null-but-invalid pointers. | `e15_*`, `e16_*` | [x] |

## Equivalent-mutant note: the `errno == 0` conjunct

A mutation that **deletes** the `errno == 0` conjunct is not detectable by any
input, and this is a property of the C, not a gap in the tests. On LP64 glibc,
`strtol` sets `errno` only to `ERANGE`, and in exactly that case it returns
`LONG_MAX` or `LONG_MIN` — both outside `[INT_MIN, INT_MAX]` — so the following
range conjuncts reject the input anyway. Verified by exhaustive search over
14 580 shaped inputs (9 prefixes × 60 digit-lengths × 9 leading digits × 3
trailing forms) looking for a string with `errno != 0` **and** a result inside
`int` range: 0 counterexamples. The Rust keeps the conjunct regardless, so the
translation matches the C source line for line.

Every other mutation of the Rust translation is killed by this suite — see
`mutation_check.sh` (19 of 20 mutants killed; the survivor is the equivalent
mutant above).

There are no other rejection sites: `add_floor`, `add_bedrooms`, `print_house`
and `run` contain zero conditionals (lines 37–58 of `driver.c` have no `if`,
`switch`, `while`, `for`, `assert`, or `return` of an error code).
