# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/driver.c`. Every rejection path in the
library flows through the single compound condition in `parse_val`
(`driver.c:64`); a `false` return there makes `driver` take its `else` branch
(`driver.c:78-80`) and print `An error occurred\n`.

Grep inventory of the whole C source (there is nothing else to find):

```
$ grep -n 'return\|assert\|errno\|INT_MIN\|INT_MAX\|NULL\|else' c_src/src/driver.c
61:    errno = 0;
64:    if (endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX) {
66:        return true;
67:    } else {
68:        return false;
69:    }
78:    } else {
79:        printf("An error occurred\n");
```

* `return` statements: `parse_val` → `true` (line 66) / `false` (line 68). No
  `return -1`, no `return NULL`, no error enum, no `RETURN_ERROR` macro.
* `assert`: **none** in the source.
* Explicit range checks: `tmp >= INT_MIN`, `tmp <= INT_MAX` (two distinct
  conjuncts → two distinct rows).
* Null checks: **none**. `driver(NULL)` reaches `strtol(NULL, ...)` → UB.
* min/max constants: `INT_MIN`, `INT_MAX` from `<limits.h>`.
* `errno` check: `errno == 0` after `strtol` (glibc sets `ERANGE` on overflow).
* `run` performs **no** validation of any kind — it dereferences
  `the_house` unconditionally and accepts any `int extra_bedrooms`.

## Error-surface table

Sentinel/observable for every row: the four `print_house` lines are **not**
emitted; instead exactly the 18 bytes `An error occurred\n` are written to
stdout, and `driver` returns `void`.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `driver` → `parse_val` | **`endp == str`**: empty string `""` — `strtol` consumes nothing, leaves `endp == str` | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E2 | `driver` → `parse_val` | **`endp == str`**: no leading digits at all, e.g. `"abc"`, `"!"`, `"++1"`, `"--1"`, `"."`, `"+"`, `"-"`, `"0x"`→(consumes `0`, see C7) | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E3 | `driver` → `parse_val` | **`endp == str`**: whitespace-only input, e.g. `" "`, `"\t\n\v\f\r "` — leading whitespace is skipped but no digit follows | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E4 | `driver` → `parse_val` | **`endp == str`**: sign followed by non-digit / whitespace between sign and digits, e.g. `"+ 1"`, `"- 1"`, `"+abc"` | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E5 | `driver` → `parse_val` | **`errno != 0`**: positive overflow of `long` — `strtol` sets `ERANGE` and returns `LONG_MAX`, e.g. `"9223372036854775808"`, `"99999999999999999999999999"` | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E6 | `driver` → `parse_val` | **`errno != 0`**: negative overflow of `long` — `strtol` sets `ERANGE` and returns `LONG_MIN`, e.g. `"-9223372036854775809"`, `"-99999999999999999999999999"` | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E7 | `driver` → `parse_val` | **`tmp < INT_MIN`**: parses fine as `long` (`errno == 0`) but below `INT_MIN`, e.g. `"-2147483649"` (= `INT_MIN - 1`), `"-3000000000"`, `"-9223372036854775808"` (`LONG_MIN` exactly, no `ERANGE`) | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E8 | `driver` → `parse_val` | **`tmp > INT_MAX`**: parses fine as `long` (`errno == 0`) but above `INT_MAX`, e.g. `"2147483648"` (= `INT_MAX + 1`), `"3000000000"`, `"9223372036854775807"` (`LONG_MAX` exactly, no `ERANGE`) | `parse_val` → `false`; stdout == `"An error occurred\n"` |
| E9 | `driver` → `parse_val` | Boundary **one step past** the valid range in both directions, checked as a pair against the last valid values `INT_MIN` / `INT_MAX` (`"-2147483648"` / `"2147483647"` succeed; `"-2147483649"` / `"2147483648"` are rejected) | valid pair: 8 `print_house` lines; invalid pair: `"An error occurred\n"` |
| E10 | `driver` | **NULL pointer** `driver(NULL)` → `strtol(NULL, &endp, 10)`. No null check exists in the C. | Faults with **SIGSEGV**. Tested differentially by running the call in a `fork()`ed child and comparing the wait status of the two implementations. |
| E11 | `run` | **NULL pointer** `run(NULL, extra)` for `extra ∈ {0, 1, -1, INT_MAX, INT_MIN}` — no null check; unconditional `the_house->floors` read. | Faults with **SIGSEGV**. Same `fork()`-based differential test. |
| E12 | `run` | **Signed-integer overflow** `floors == INT_MAX` then `house->floors++`; `bedrooms + extra_bedrooms` overflowing `int`. C signed overflow is UB; the un-optimised (`-O0`, no `CMAKE_BUILD_TYPE`) build emits a plain `add`, i.e. two's-complement wraparound. | Wraps (e.g. `INT_MAX` → `INT_MIN`). Rust must match, so it uses `wrapping_add`. Covered by differential tests C15–C17. |

### Non-error notes (things that look like errors but are NOT rejected)

Recorded so they are not mistakenly tested as failures:

* **Trailing garbage is accepted.** `parse_val` only checks `endp != str`, never
  `*endp == '\0'`. `"12abc"`, `"5 "`, `"7.9"`, `"0x10"` all *succeed*
  (values 12, 5, 7, 0). See `CONFIGS.md` rows C6–C8.
* **`errno` is reset to 0 before the call**, so a pre-existing `errno` from an
  earlier operation cannot cause a spurious rejection.
* There is **no out-of-range enum** anywhere in this API: the public surface is
  `driver(const char *)` and `run(house_t *, int)`; no `enum` type exists in
  `driver.h` or `driver.c`. The nearest analogue — an arbitrary `int` with no
  "valid variant" — is `extra_bedrooms`, which is exhaustively fuzzed over the
  full `int` range including `INT_MIN`/`INT_MAX` (rows C15–C18), and an
  arbitrary bit pattern in the `double` field `bathrooms` (NaN / ±inf /
  subnormal / negative zero, rows C19–C26).
* **The `errno == 0` conjunct is redundant on this target (LP64).** Verified by
  mutation: deleting it from the Rust changes no observable behaviour on any of
  the ~14 000 differential inputs. `strtol` only sets `ERANGE`, and then returns
  `LONG_MAX`/`LONG_MIN`, both of which already fail the `INT_MIN`/`INT_MAX`
  range check; `EINVAL` cannot occur because the base is the literal `10`. The
  Rust keeps the check anyway so the translation stays structurally faithful and
  remains correct on an ILP64-style target where `long` is as wide as `int`.

## Row status

| row | test | status |
|-----|------|--------|
| E1  | `err_e1_empty_string` | [x] pass |
| E2  | `err_e2_no_leading_digits` | [x] pass |
| E3  | `err_e3_whitespace_only` | [x] pass |
| E4  | `err_e4_sign_without_digits` | [x] pass |
| E5  | `err_e5_erange_positive` | [x] pass |
| E6  | `err_e6_erange_negative` | [x] pass |
| E7  | `err_e7_below_int_min` | [x] pass |
| E8  | `err_e8_above_int_max` | [x] pass |
| E9  | `err_e9_one_step_past_range` | [x] pass |
| E10 | `err_e10_driver_null_pointer` | [x] pass |
| E11 | `err_e11_run_null_pointer` | [x] pass (**found a real bug — see below**) |
| E12 | `cfg_c13_..c16_*` (wraparound) | [x] pass |

## Divergence found and fixed (row E11)

`run(NULL, x)` diverged in any build with debug assertions enabled:

| | signal |
|---|---|
| C `libdriver.so` | `SIGSEGV` (11) |
| Rust `libdriver.so` (before fix) | `SIGABRT` (6) |

Cause: the original translation did `let house = unsafe { &mut *the_house };`
and then accessed fields through place expressions such as
`(*house).floors`. Under `-C debug-assertions` rustc emits a
null/alignment validity check for both forms, and that check calls
`abort()` — so the Rust library terminated with `SIGABRT` where the C
library simply faulted with `SIGSEGV`. The `--release` profile happened to
hide it (assertions off), which is exactly why the debug profile had to be
tested too.

Fix (`src/lib.rs`): never form a reference from the incoming pointer and
never read/write through a place expression. Field access now goes through
a raw-ref (`&raw const` / `&raw mut`, which does not dereference) followed
by `ptr::read` / `ptr::write`, which lowers to the same plain load/store the
C emits. Verified to fault identically with `SIGSEGV` in

* `--release`,
* the default `dev` profile, and
* `RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on"` on `--release`.
