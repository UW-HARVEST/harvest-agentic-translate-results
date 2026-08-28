# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. Every `if`, every `goto`, every
`return`, every null check and every size constant in the C file is accounted
for below — there are no `assert`s, no error enums, no `RETURN_ERROR`-style
macros and no `return -1` / `return NULL` statements anywhere in the library
(verified with `grep -nE 'assert|RETURN_ERROR|return *-|return *NULL|errno' c_src/src/lib.c`
→ no matches).

Full inventory of the branch points in the C source:

```
lib.c:42  if (strncmp(input_str, expected_str, strlen(expected_str)) != 0)  -> row 1
lib.c:44  goto cleanup;                                                     -> row 1
lib.c:48  switch (numbers[i]) { case 10/20/30/40/default }                  -> rows 5,6,7,8
lib.c:65  malloc(50 * sizeof(char))                                         -> rows 2,9
lib.c:66  if (!dynamic_str)                                                 -> row 2
lib.c:68  goto cleanup;                                                     -> row 2
lib.c:71  snprintf(dynamic_str, 50, ...)                                    -> row 9
lib.c:80  printf("%s: %d\n", label, result)  (no validation of `label`)     -> rows 10,11,12,13,14
lib.c:84  if (dynamic_str)                                                  -> rows 3,4
lib.c:47  for (int i = 0; i < 4; i++)  (fixed count, no caller influence)   -> row 15
```

`cleanup` has no failure sentinel: **both** failure paths `goto cleanup` and
return the current value of `result`, so the return value alone cannot
distinguish success from failure — the discriminator is the message printed on
stdout. Every row below therefore asserts **both** the returned `int` and the
exact stdout bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|----------------------------------------------|-------------------|------|----|
| 1 | `cleanup` | input-string validation fails: `strncmp("VALID","VALID",5) != 0`. Unreachable through the public arguments (both operands are the same literal), so it is forced by interposing `strncmp` via `LD_PRELOAD` — the same libc call both `.so`s make. | stdout `"Input string validation failed.\n"`, **no** `"Processed numbers: ..."` line, `malloc` never called, returns `0` (the pre-switch value of `result`), `cleanup_resources(NULL)` runs (no free) | `phase_c_errors::row01_strncmp_validation_failure` (parent) + `child_row01_strncmp_fail` | [x] |
| 2 | `cleanup` | `malloc(50)` returns `NULL` (heap exhaustion). Forced by interposing `malloc` so that **only** `n == 50` fails, for both `.so`s identically. | stdout `"Memory allocation failed.\n"`, **no** `"Processed numbers: ..."` line, returns the **already accumulated** `result` (the switch loop runs *before* the allocation), `cleanup_resources(NULL)` runs (no free) | `phase_c_errors::row02_malloc_failure` (parent) + `child_row02_malloc_fail` | [x] |
| 3 | `cleanup_resources` | `dynamic_str == NULL` (the explicit null check at lib.c:84) | no `free`, no output, returns void; must not crash | `phase_c_errors::row03_cleanup_resources_null` + `child_row0304_free_accounting` (a) | [x] |
| 4 | `cleanup_resources` | `dynamic_str != NULL` — a live `malloc`ed block, incl. size 0 (`malloc(0)` returns a non-null unique pointer on glibc) and size 50 | `free(ptr)` — **exactly once, on exactly that pointer**; the parameter's `= NULL` store is dead (local copy) so the caller's pointer is unchanged; no output | `phase_c_errors::row04_cleanup_resources_frees` + `child_row0304_free_accounting` (b)–(f) | [x] |
| 5 | `cleanup` | an argument matches **no** `case` label — the "out-of-range variant" class for this API's implicit `{10,20,30,40}` enum: `0`, `±1`, `9`, `11`, `19`, `21`, `29`, `31`, `39`, `41`, `-10`, `-20`, `-30`, `-40`, `INT_MIN`, `INT_MAX` | falls to `default:` → `result += numbers[i]` (note `-10` etc. do **not** match `case 10`) | `phase_c_errors::row05_default_arm_non_case_values` | [x] |
| 6 | `cleanup` | argument `== 10`: `case 10:` has **no `break`** and falls through into `case 20:` | `result += 10` **then** `result += 20` → net **+30** (not +10) | `phase_c_errors::row06_fallthrough_case_10` | [x] |
| 7 | `cleanup` | argument `== 30`: `case 30:` has **no `break`** and falls through into `case 40:` | `result += 30` **then** `result += 40` → net **+70** (not +30) | `phase_c_errors::row07_fallthrough_case_30` | [x] |
| 8 | `cleanup` | signed-integer overflow of `result` (e.g. `INT_MAX, INT_MAX, INT_MAX, INT_MAX`, or `INT_MAX, 10, …`). UB in ISO C; the shipped `.so` is built with no `-O` flag (`CMAKE_BUILD_TYPE=""`, `CMAKE_C_FLAGS=""`) so gcc emits a plain `add` that wraps mod 2^32. | wraps two's-complement; e.g. `(INT_MAX, INT_MAX, 0, 0)` → `-2` | `phase_c_errors::row08_result_overflow_wraps` | [x] |
| 9 | `cleanup` | `snprintf` bound `50` vs. the produced string `"Processed numbers: numbers"` (26 bytes + NUL). `TO_STRING(numbers)` stringises the *macro argument*, so the printed text is the literal `numbers`, **not** the array contents. Boundary: nothing is truncated, and the bound is never exceeded for any input because the text is input-independent. | stdout exactly `"Processed numbers: numbers\n"` for every input | `phase_c_errors::row09_snprintf_bound` | [x] |
| 10 | `print_result` | `label == NULL` — the C performs **no** null check before `printf("%s", label)` | glibc `printf` prints the literal `(null)` → `"(null): <n>\n"`; must not crash | `phase_c_errors::row10_print_result_null_label` | [x] |
| 11 | `print_result` | `result` at the `int` boundaries `INT_MIN` / `INT_MAX` / `-1` / `0` | `"%d"` → `-2147483648` / `2147483647` / `-1` / `0` | `phase_c_errors::row11_print_result_int_bounds` | [x] |
| 12 | `print_result` | `label` contains `printf` conversion specifiers (`"%s %d %n %%"`) — it is an *argument*, not the format string, so no format-string interpretation may happen | the specifiers are printed literally | `phase_c_errors::row12_print_result_percent_in_label` | [x] |
| 13 | `print_result` | `label` is the empty string `""` (zero length) | `": <n>\n"` | `phase_c_errors::row13_print_result_empty_label` | [x] |
| 14 | `print_result` | oversized `label`: 64 KiB and 1 MiB NUL-terminated buffers, and a label containing embedded newlines / non-UTF-8 bytes (`0x80..0xFF`) — `printf` has no length limit here | whole label copied through verbatim, then `": <n>\n"` | `phase_c_errors::row14_print_result_oversized_and_non_utf8` | [x] |
| 15 | `cleanup` | loop count is the compile-time constant `4` over `int numbers[4]`; no caller-supplied length exists, so no out-of-bounds index is reachable. Boundary asserted indirectly: exactly the 4 arguments contribute, each exactly once, in order `a,b,c,d`. | `result` == sum of the four per-argument contributions, order-independent | `phase_c_errors::row15_exactly_four_args_contribute` | [x] |

## Allocator side effects are part of the contract

`free` produces no return value and no output, so rows 3 and 4 could be "passed"
by a translation that never frees at all. The same fault shim therefore keeps
allocation bookkeeping (`harvest_shim_free_hits` / `harvest_shim_free50_hits`,
tracking the identity of every 50-byte block handed out) and
`child_row0304_free_accounting` asserts, for **both** libraries:

| case | assertion |
|------|-----------|
| (a) `cleanup_resources(NULL)` | 0 calls to `free` |
| (b) `cleanup_resources(malloc(50))` | exactly 1 `free`, of exactly that pointer |
| (c) `cleanup(...)` happy path | exactly 1 `malloc(50)` **and** 1 matching `free` — no leak, no double free |
| (d) `cleanup(...)` with `malloc` forced to fail | the allocation is attempted, and `free` is called 0 times (`cleanup_resources(NULL)`) |
| (e) `cleanup(...)` with validation forced to fail | 0 allocations and 0 frees — the `goto` precedes the `malloc` |
| (f) 400 back-to-back calls | 400 allocations and 400 frees, balanced |

Verified to be a real check by negative control: replacing the `free` in the Rust
`cleanup_resources` with a no-op leaves every output comparison green but makes
`row0304_free_accounting` fail.

## Deliberately excluded (undefined behaviour in the C — no defined result to match)

* `cleanup_resources(p)` where `p` was not returned by `malloc`, or was already
  freed (double free). The C calls `free` unconditionally on any non-null
  pointer; the outcome is allocator UB (`SIGABRT`/heap corruption) and is not a
  specified behaviour, so it is not a differential row.
* `print_result(label, n)` where `label` points at a buffer with no NUL
  terminator — `printf` reads out of bounds. Row 14 covers the *defined*
  oversized case (large but properly terminated).
* There are **no C `enum` types** in this API, so "out-of-range enum value" has
  no literal instance. Its exact analogue — an `int` reaching `switch` with no
  matching `case` label — is row 5, and is tested with the full boundary set
  including `INT_MIN`/`INT_MAX` and the values one step either side of every
  `case` label (9/11, 19/21, 29/31, 39/41).
