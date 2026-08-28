# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c` by grepping **every** guard, `if`, range
check, null-sensitive dereference, clamp constant and sentinel return. This library has
no error enum, no `RETURN_ERROR` macro, no `return -1` and no `assert`; it rejects input
exclusively via (a) **guard branches that silently skip work**, (b) **clamping to a
min/max constant**, (c) a **sentinel return value**, and (d) **unchecked operations that
trap**. Each distinct branch gets its own row.

Octal constants in the C, spelled in decimal for clarity:
`01`=1, `02`=2, `03`=3, `04`=4, `010`=8, `0100`=64, `0123`=83, `0150`=104, `0777`=511.

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|---------------------------------------------|-------------------|---|
| 1 | `divide_multiplier` | `b == 0` — guard `if (b != 0)` at L54 is false | division **skipped**; `multiplier` unchanged; `operation_count` **still incremented**; returns current `multiplier` | [x] |
| 2 | `divide_multiplier` | `b == 0` **and** `multiplier` previously driven negative | same as #1: no division, returns unchanged negative `multiplier` | [x] |
| 3 | `divide_multiplier` | `multiplier == INT_MIN`, `b == -1` — no overflow guard exists; `idiv` traps | **SIGFPE** (process dies); Rust must die on the same signal | [x] |
| 4 | `divide_multiplier` | negative `multiplier / positive b` (e.g. `-7 / 2`) | C99 truncation **toward zero** → `-3` (not floor `-4`) | [x] |
| 5 | `validate_and_normalize` | `value == 0` — `is_nonzero == 0`, outer guard L81 false | returns `0` **unclamped** (0 is *not* raised to 64) | [x] |
| 6 | `validate_and_normalize` | `value < 0` — `value > 0` half of guard L81 false | returns `value` **unchanged**; negatives are *never* clamped, incl. `INT_MIN` | [x] |
| 7 | `validate_and_normalize` | `0 < value < 0100` (1..63) — L82 `value < lower_threshold` | returns `0100` = **64** | [x] |
| 8 | `validate_and_normalize` | `value > 0777` (512..`INT_MAX`) — L84 `value > upper_threshold` | returns `0777` = **511** | [x] |
| 9 | `validate_and_normalize` | boundary `value == 0100` (64) — neither L82 nor L84 fires | returns `64` unchanged (`<` is strict) | [x] |
| 10 | `validate_and_normalize` | boundary `value == 0777` (511) — neither L82 nor L84 fires | returns `511` unchanged (`>` is strict) | [x] |
| 11 | `validate_and_normalize` | one step past range: `value == 63` / `value == 512` | `64` / `511` respectively | [x] |
| 12 | `find_and_replace_char` | `search_char` absent from the string — guard `if (found)` at L69 false | **no write**; string left byte-identical | [x] |
| 13 | `find_and_replace_char` | empty string (`*str == 0`) → `strlen == 0`, `memchr(...,0)` returns NULL | **no write**; NUL byte not overwritten | [x] |
| 14 | `find_and_replace_char` | `search_char == 0` — searching for NUL over only `strlen` bytes | never found → **no write** (terminator is outside the searched range) | [x] |
| 15 | `find_and_replace_char` | `search_char` out of `unsigned char` range, e.g. `0x141`, `256+'A'` | `memchr` converts to `unsigned char` → **aliases** the low byte and *does* match | [x] |
| 16 | `find_and_replace_char` | negative `search_char`, e.g. `-191` (`0xFFFFFF41`) | low byte `0x41` = `'A'` → **matches `'A'`** | [x] |
| 17 | `find_and_replace_char` | multiple occurrences of `search_char` | only the **first** occurrence replaced with `'X'` | [x] |
| 18 | `find_and_replace_char` | `str == NULL` — no null check; `strlen(NULL)` dereferences | **SIGSEGV** | [x] |
| 19 | `process_octal_string` | `dest == NULL` — no null check; `strcpy(NULL, buffer)` | **SIGSEGV** | [x] |
| 20 | `process_octal_string` | `octal_val < 0` — `%o` takes `unsigned int`, no range check | octal field printed as **two's-complement `unsigned`**, decimal field printed **signed** (e.g. `-1` → `Octal: 037777777777, Decimal: -1`) | [x] |
| 21 | `process_octal_string` | `octal_val == INT_MIN` — widest output, `char buffer[50]` | `Octal: 020000000000, Decimal: -2147483648` (41 bytes + NUL, fits) | [x] |
| 22 | `findrep` | all four params `== 0` → `active_params == 0 < mode_add` | both `if` blocks at L132/L137 **skipped**; no accumulator/multiplier op runs | [x] |
| 23 | `findrep` | exactly one param non-zero → `active_params == 1`, `>= mode_add` but `< mode_multiply` | **only** `operations[0]` (add) runs; multiply block skipped | [x] |
| 24 | `findrep` | `accumulator <= 0150` (104) — guard L142 false | `operations[2]` (subtract) **not** invoked; `operation_count` not bumped by it | [x] |
| 25 | `findrep` | `accumulator == 0` (so `has_accumulator == 0`) → `both_active == 0` | L158 `result += accumulator + multiplier` **skipped** | [x] |
| 26 | `findrep` | `multiplier == 0` (so `has_multiplier == 0`) → `both_active == 0` | L158 **skipped** even though `accumulator != 0` | [x] |
| 27 | `findrep` | `multiplier <= 0100` (64) — guard L161 false | `operations[3]` (divide) **not** invoked | [x] |
| 28 | `findrep` | computed `result == 0` → `result_exists == 0` at L169 | returns the **sentinel `0777` = 511** instead of 0 | [x] |
| 29 | `findrep` | `'p'` not found in the hard-coded literal — guard L126 `if (found_char)` | unreachable branch (literal always contains `'p'` at index 9); asserts the offset contribution is exactly `+9`, never skipped | [x] |
| 30 | `findrep` | `INT_MIN` / `INT_MAX` params — no overflow guard anywhere | wrapping two's-complement arithmetic throughout (gcc `-O0`, no `-ftrapv`) | [x] |
| 31 | all four `operations[]` | signed overflow in `accumulator += a+b`, `multiplier *= a*b`, `accumulator -= a-b` — unchecked | wraps modulo 2^32; Rust must use `wrapping_*`, never panic | [x] |

## Notes on "out-of-range enum values across the FFI boundary"

The public API declares **no enum type** — `c_src/include/lib.h` exposes only
`int findrep(int,int,int,int)`. The equivalent class of "an integer with no valid
variant" is covered by:

* rows **15/16** — `search_char` outside `unsigned char`, the *actual* narrowing
  the C performs via `memchr`;
* rows **20/21/30** — `octal_val` / params spanning the whole `int` range including
  `INT_MIN`/`INT_MAX`;
* the `mode_add`..`mode_divide` constants (`01`..`04`) are compared against
  `active_params`, which is structurally bounded to `0..4`, so `mode_subtract` (`03`)
  and `mode_divide` (`04`) are **dead constants** — the C never branches on them.
  Rows 22–24 pin the reachable `active_params` values `0..4`.
* `operations[4]` is indexed only by the literals `0..3`, so no out-of-bounds index
  is reachable from any input.

---

## Divergences found by the Phase C tests and FIXED in the Rust

Both were on error paths that happy-path testing cannot reach, and in both cases the
Rust was changed to match the C (the C is ground truth).

### 1. ERRORS #3 — `INT_MIN / -1` did not trap  (`src/lib.rs`)

`divide_multiplier` used `wrapping_div`, which **returns `INT_MIN`** for
`INT_MIN / -1`. The C guards only `b != 0`, so gcc's single `idiv` instruction
executes and raises the `#DE` fault, killing the process with **SIGFPE**.

```
case `div_intmin_by_neg1`:
  C    -> signal 8 (SIGFPE),  no quotient printed
  Rust -> exit 0,             quotient=-2147483648      <-- divergence
```

State is reachable purely through the public API
(`multiply_with_multiplier(INT_MIN, 1)` sets `multiplier = INT_MIN`, then
`divide_multiplier(_, -1)`), so this was a genuine behavioural difference and not a
theoretical one. Fixed by adding `c_idiv`, which reproduces the same hardware fault
via an `idiv` in inline asm on `x86_64` (and `raise(SIGFPE)` elsewhere).

### 2. ERRORS #18 / #19 — null dereference aborted instead of segfaulting  (`Cargo.toml`)

Only in the **dev profile**. `strlen(NULL)` / `strcpy(NULL, ..)` fault in the C
(**SIGSEGV**), but Rust's default dev-profile `debug-assertions` insert a
"null pointer dereference occurred" precondition check in `c_strlen`, producing a
non-unwinding panic that **aborts (SIGABRT)**.

```
case `replace_null` (dev profile):
  C    -> signal 11 (SIGSEGV)
  Rust -> signal  6 (SIGABRT)                          <-- divergence
```

The release profile already matched. Fixed by pinning
`[profile.dev] debug-assertions = false, overflow-checks = false`, since the C is
compiled with no such preconditions and no overflow traps. All profiles now agree.

## Verification status

All 31 rows have a passing differential test. Rows 3, 18 and 19 are fatal in both
libraries, so they are verified in `tests/crash.rs` by running each case in a child
process and comparing the **exact terminating signal**, not merely "both failed".
