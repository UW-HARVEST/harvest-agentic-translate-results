# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/driver.c`. There is exactly **one**
rejection site in the whole library:

```c
static bool parse_val(const char *str, int *val) {
    errno = 0;
    char *endp = (char *)str;
    long tmp = strtol(str, &endp, 10);
    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
        *val = tmp;
        return true;
    } else {
        return false;          // <-- the only error return
    }
}

void driver(const char *in) {
    int x;
    if (parse_val(in, &x)) { run(x); run(x); }
    else { printf("An error occurred\n"); }   // <-- the only error output
}
```

## Inventory of rejection mechanisms (exhaustive grep)

| mechanism | count | where |
|-----------|-------|-------|
| `return false` / error return | 1 | `parse_val` else-branch |
| error message emission | 1 | `driver` else-branch → `"An error occurred\n"` |
| `assert` | 0 | none in source |
| `return -1` / `return NULL` / error enum | 0 | both public functions return `void`; no error codes exist |
| explicit range check | 2 | `tmp >= INT_MIN`, `tmp <= INT_MAX` |
| `errno` check | 1 | `errno == 0` (ERANGE from `strtol`) |
| "no conversion" check | 1 | `endp != str` |
| null check | 0 | **none** — `in` is passed unchecked to `strtol` |
| min/max constants | 2 | `INT_MIN`, `INT_MAX` (`<limits.h>`) |

**Observable error signal:** both public functions are `void`, so the only
observable rejection is the exact byte string `"An error occurred\n"` on
`stdout` (and the *absence* of any `run()` output). Every row below asserts C
and Rust emit the **same** signal, not merely "both failed".

The guard is a 4-conjunct `&&`, so there are 4 distinct trigger classes.
`run()` performs **no** validation whatsoever and has no error path.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | ✅ |
|---|----------|----------------------------------------------|-------------------|----|
| 1 | `driver` / `parse_val` | conjunct 1 `endp != str` fails: empty string `""` — `strtol` performs no conversion | `"An error occurred\n"` | [x] |
| 2 | `driver` / `parse_val` | conjunct 1 fails: purely non-numeric, e.g. `"abc"`, `"!!"`, `"@"` | `"An error occurred\n"` | [x] |
| 3 | `driver` / `parse_val` | conjunct 1 fails: whitespace only, e.g. `" "`, `"\t\n "` (`strtol` skips it, then no digits) | `"An error occurred\n"` | [x] |
| 4 | `driver` / `parse_val` | conjunct 1 fails: sign with no digits, `"+"`, `"-"`, `"  -"`, `"+ 1"` | `"An error occurred\n"` | [x] |
| 5 | `driver` / `parse_val` | conjunct 1 fails: base-10 rejects hex/other prefixes with no leading digit, e.g. `"0x"` parses `0` (**succeeds**), but `"x10"`, `"e5"`, `"."`, `"-."` do not | `"An error occurred\n"` | [x] |
| 6 | `driver` / `parse_val` | conjunct 2 `errno == 0` fails: `ERANGE`, value **> LONG_MAX**, e.g. `"9223372036854775808"`, `"99999999999999999999"` | `"An error occurred\n"` | [x] |
| 7 | `driver` / `parse_val` | conjunct 2 fails: `ERANGE`, value **< LONG_MIN**, e.g. `"-9223372036854775809"`, `"-99999999999999999999"` | `"An error occurred\n"` | [x] |
| 8 | `driver` / `parse_val` | conjunct 3/4 `tmp <= INT_MAX` fails: parses fine, `errno == 0`, but `> INT_MAX`, e.g. `"2147483648"` (INT_MAX+1) | `"An error occurred\n"` | [x] |
| 9 | `driver` / `parse_val` | conjunct 3 `tmp >= INT_MIN` fails: parses fine, `errno == 0`, but `< INT_MIN`, e.g. `"-2147483649"` (INT_MIN−1) | `"An error occurred\n"` | [x] |
| 10 | `driver` / `parse_val` | conjunct 4 fails at the *long* extreme with `errno == 0`: `"9223372036854775807"` (LONG_MAX exactly — converts without ERANGE, then fails `<= INT_MAX`) | `"An error occurred\n"` | [x] |
| 11 | `driver` / `parse_val` | conjunct 3 fails at the *long* extreme with `errno == 0`: `"-9223372036854775808"` (LONG_MIN exactly) | `"An error occurred\n"` | [x] |

## Generic C-API boundaries (required even though not in the table above)

| # | function | trigger | expected C result | ✅ |
|---|----------|---------|-------------------|----|
| 12 | `driver` | **NULL pointer** — no null check exists; `strtol(NULL, ...)` dereferences | process dies on `SIGSEGV` (11); Rust must die with the *same* signal | [x] |
| 13 | `driver` | zero-length input — `""` (see row 1); also string that is only an embedded NUL terminator | `"An error occurred\n"` | [x] |
| 14 | `driver` | **oversized** input — 4096- and 100 000-digit numeric strings (`ERANGE`), and a 65 536-byte non-numeric string | `"An error occurred\n"` | [x] |
| 15 | `driver` | one step **inside** the valid range (must SUCCEED, proving the boundary is not off-by-one): `"2147483647"` = INT_MAX, `"-2147483648"` = INT_MIN | 8 lines of `run()` output, bedrooms wrapping | [x] |
| 16 | `driver` | one step **past** the valid range (must FAIL): INT_MAX+1, INT_MIN−1, LONG_MAX+1, LONG_MIN−1 | `"An error occurred\n"` | [x] |
| 17 | `driver` | trailing garbage after a valid number — `"42abc"`, `"7 8"`, `"1,000"`: C **accepts** (only `endp != str` is required, not full consumption) | `run(42)` twice — *not* an error | [x] |
| 18 | `run` | **out-of-range "enum" values across FFI**: this API declares no `enum`, so the analogous case is the full unconstrained `int` domain reaching `run` directly — `INT_MIN`, `INT_MAX`, `-1`, `0` — every `int` bit pattern is a legal input the C accepts and must be replicated (signed overflow of `bedrooms += extra_bedrooms` wraps under gcc `-O0`) | no rejection; wrapped `bedrooms` printed | [x] |

## Non-applicable checks (documented for completeness)

* **No enum parameters** exist in the public API (`driver.h` declares only
  `void driver(const char *)`), so there is no invalid-variant case beyond the
  full `int` domain covered by row 18.
* **No allocation**, no file/socket handles, no out-params other than the
  private `int *val`, so there are no resource-exhaustion or double-free paths.
* `run()` has **zero** error paths — every `int` is valid input.

## Findings from mutation testing (`./mutation_check.sh`)

All 11 table rows + 7 generic-boundary rows pass differentially. Two rejection
mechanisms turned out to be **provably redundant** in the C source. Both are
still mirrored faithfully in the Rust (the C is ground truth; redundant code is
translated, not "optimised away"), but they are recorded here so the error
surface is honestly documented:

* **Row 6/7, the `errno == 0` conjunct is never decisive.** For it to change the
  outcome, an input would have to yield `errno != 0` *and* `endp != str` *and*
  `INT_MIN <= tmp <= INT_MAX` simultaneously. A C probe swept ~15 000 inputs
  (empty/sign/garbage forms, 1–200-digit runs of every digit in both signs, and
  ±300 around every power-of-two boundary from 2^28 to LONG_MAX) and found **no
  such input**: glibc base-10 `strtol` raises `ERANGE` only when the result
  saturates to `LONG_MIN`/`LONG_MAX`, and both of those already fail the
  `INT_MIN`/`INT_MAX` conjunct. Deleting the `errno` check from the Rust
  therefore does not change observable behaviour.
* **`floors` overflow is unreachable.** `floors` starts at 2 and only ever
  increments by 1, so `wrapping` vs `saturating` differ solely after 2^31
  `run()` calls — not reachable in any feasible test (and signed overflow is UB
  in the C regardless).

Every other injected bug — including dropping the `endp != str` check, dropping
or off-by-one'ing either `INT_MIN`/`INT_MAX` bound, not clearing `errno` first,
changing the error text, and dropping the error message's trailing newline —
**was caught** by these error-path tests.
