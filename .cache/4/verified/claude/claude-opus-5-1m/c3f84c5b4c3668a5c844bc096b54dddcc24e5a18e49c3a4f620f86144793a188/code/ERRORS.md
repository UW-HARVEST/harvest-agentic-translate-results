# ERRORS.md — Error-surface table (Phase A)

Mechanically derived from `c_src/src/lib.c`. Every way the C source rejects,
guards against, or short-circuits on input. Greps performed:

```sh
grep -n "return\|goto\|assert\|RETURN_ERROR\|NULL\|!\|if (" c_src/src/lib.c
```

Inventory of what the C actually contains:
* `goto cleanup;` × 2 (the only early-exit paths)
* `if (strncmp(...) != 0)` — string validation guard
* `if (!dynamic_str)` — malloc failure guard
* `if (dynamic_str)` — null guard inside `cleanup_resources`
* `return result;` × 1 (single return in `cleanup`)
* **no** `assert`, **no** `return -1`, **no** `return NULL`, **no** error enum,
  **no** errno use, **no** explicit range check, **no** min/max constant.

There is no error *code* surface at all: `cleanup` returns an accumulator, never
a sentinel. So "expected C result" below is the exact observable pair
(return value, stdout bytes).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `cleanup` | `strncmp(input_str, expected_str, strlen(expected_str)) != 0` — the string-validation guard fails. Both operands are the *same* literal `"VALID"`, so `strncmp` is always `0`: this branch is **statically dead / unreachable from the public API**. No argument value can reach it. | unreachable; would be `printf("Input string validation failed.\n")` then `goto cleanup` → return `0` with only that line on stdout. Asserted instead: for **all** inputs the string `"Input string validation failed."` NEVER appears on stdout, in C nor Rust. | `e1_validation_branch_is_dead_in_both` | [x] |
| E2 | `cleanup` | `malloc(50) == NULL` → `!dynamic_str` is true. Not reachable by argument choice (50-byte request); no injection hook exists in the library. | unreachable; would be `printf("Memory allocation failed.\n")` then `goto cleanup` → return the partial `result` **without** the `Processed numbers:` line. Asserted instead: for all inputs the string `"Memory allocation failed."` NEVER appears and the `Processed numbers: numbers` line ALWAYS appears, identically in C and Rust. | `e2_malloc_failure_branch_is_dead_in_both` | [x] |
| E3 | `cleanup_resources` | `dynamic_str == NULL` → the `if (dynamic_str)` guard rejects the pointer and skips `free`. This is the one guard reachable across the FFI boundary. | no-op: no crash, no output, `void` return. Must be a silent no-op in Rust too. | `e3_cleanup_resources_null_is_noop` / `e3_cleanup_resources_null_repeated` | [x] |
| E4 | `cleanup_resources` | non-NULL pointer → passes the guard and calls `free(dynamic_str)`. Passing a pointer NOT from `malloc` is UB and untestable; a genuine libc-`malloc`'d pointer is the valid non-NULL case and must be freed by both without crash/leak-abort. | silent `free`, no output, `void` return. | `e4_cleanup_resources_frees_valid_pointer` | [x] |
| E5 | `cleanup_resources` | non-NULL pointer, asking the sharper question: was `free` **actually called**? A version that skips `free` prints the same nothing and returns the same `void`, so stdout comparison alone cannot tell the two apart — it only leaks. | the pointer is really released. Observed via glibc's LIFO tcache: after a genuine `free`, the next same-size `malloc` returns the SAME address. Asserted equal between C and Rust, and asserted sound (the C side must recycle) before judging Rust. | `e5_cleanup_resources_actually_frees` | [x] |
| E5b | `cleanup_resources` | guard **polarity** inverted (`if (!dynamic_str)` instead of `if (dynamic_str)`). Reachable as a mistranslation, not as an input: because `free(NULL)` is a legal libc no-op, an inverted guard is byte-identical on stdout for every input while leaking every real pointer. | NULL → silent no-op AND non-NULL → really freed. Only both together pin the polarity. | `e5b_null_guard_polarity` | [x] |
| E6 | `cleanup` | the internal `malloc(50)` must be released through `cleanup_resources` before returning. A leak here changes neither the return value nor one byte of stdout. | heap balanced across the call: priming the 50-byte tcache bin and re-allocating after `cleanup` returns the same address. Asserted equal between C and Rust. | `e6_cleanup_frees_its_internal_buffer` | [x] |

## Generic FFI-boundary boundaries (required even though absent from the table)

| # | function | trigger | expected C result | test | ✔ |
|---|----------|---------|-------------------|------|---|
| G1 | `print_result` | `label == NULL` → passed straight to `printf("%s: %d\n", ...)`. glibc's `%s` accepts NULL and prints `(null)`; the C never null-checks. | `"(null): <result>\n"` on stdout. Rust must forward the null pointer to the same glibc `printf`, producing the identical bytes. | `g1_print_result_null_label` | [x] |
| G2 | `print_result` | zero-length label (`""`) | `": <result>\n"` | `g2_print_result_empty_label` | [x] |
| G3 | `print_result` | oversized label (4 KiB / 64 KiB, no NUL until the end) — exercises glibc's `%s` buffering past `BUFSIZ` | full label then `": <result>\n"`, byte-identical | `g3_print_result_oversized_label` | [x] |
| G4 | `print_result` | label containing conversion specifiers (`%d %s %n %%`) — must be printed literally because it is a `%s` *argument*, never a format string | literal bytes, no interpretation | `g4_print_result_label_with_format_specifiers` | [x] |
| G5 | `print_result` | label with embedded newlines/tabs/CR and non-UTF-8 bytes (`0x80..0xFF`) — a Rust `str`-based translation would corrupt these | raw bytes passed through unchanged | `g5_print_result_non_utf8_and_control_bytes` | [x] |
| G6 | `print_result` | `result` at `INT_MIN` / `INT_MAX` — one step past the signed range in each direction is not representable, so these are the extremes | `-2147483648` / `2147483647` formatted by `%d` | `g6_print_result_int_extremes` | [x] |
| G7 | `cleanup` | out-of-range "enum-like" selector values: the `switch` has cases 10/20/30/40 and `default`. Values with no matching case — including one step past each case (`9,11,19,21,29,31,39,41`), negatives, `0`, `INT_MIN`, `INT_MAX` — all fall to `default: result += numbers[i]`. C `switch` accepts any `int`, so every one of these is a real input. | `default` accumulation; no case-label match, no fallthrough | `g7_cleanup_off_by_one_around_every_case_label`, `g7_cleanup_int_extremes` | [x] |
| G8 | `cleanup` | signed-overflow accumulation: `result += numbers[i]` driven past `INT_MAX` / below `INT_MIN` (e.g. all four args `INT_MAX`, or `INT_MIN`). C is compiled at `-O0` (no `CMAKE_BUILD_TYPE`, no `-O` flag) where this wraps two's-complement; the Rust uses `wrapping_add`. | two's-complement wrapped `int` | `g8_cleanup_overflow_wraps_identically` | [x] |
| G9 | `cleanup` | fallthrough-vs-break correctness at every case label: `case 10` falls through into `case 20` (net `+30`), `case 30` falls through into `case 40` (net `+70`), while `case 20` (`+20`) and `case 40` (`+40`) break. Mis-translating fallthrough is the primary hazard in this file. | `10→+30`, `20→+20`, `30→+70`, `40→+40`, else `+v` | `g9_cleanup_fallthrough_semantics_per_case`, `b1_cleanup_exhaustive_switch_path_cross_product` | [x] |

**No `unimplemented!()`/stub was added anywhere to satisfy a row.** Rows E1 and
E2 guard genuinely dead code in the C; they are verified by asserting the
branch's observable side effect never occurs in *either* implementation, which is
the strongest available check without patching `c_src/` (forbidden).

## Why rows E5/E5b/E6 exist

They were added *because* the mutation check (`mutation_check.py`) found a real
blind spot: inverting the `if (dynamic_str)` guard in the Rust survived the whole
suite. `free(NULL)` is a no-op and a leak emits no output, so
return-value + stdout comparison is provably insufficient for this library. The
tcache address-recycling probe closes it; the mutation is now caught, and so are
two pure-leak mutations (`M15`, `M16`) that the stdout-only suite also missed.

## Probe applicability (rows E5/E5b/E6)

The `free`-observation probes rely on a glibc *heuristic*, not a guarantee: that
`free(p)` followed by `malloc(same_size)` returns the same address (tcache LIFO
reuse). `tcache_probe_usable(size)` measures that precondition directly in the
running process before each row trusts it.

* **debug profile (the profile used for verification and for `mutation_check.py`):**
  precondition holds; all three rows are conclusive and they are what catch the
  leak mutations `M9`, `M15`, `M16`.
* **`--release` profile:** the precondition does not hold in the release test
  binary, so the rows report `INCONCLUSIVE` and skip the leak half rather than
  emit a bogus pass or a bogus failure. The NULL half of E5b still runs.

An earlier version of the probe freed the pointer again when the address was not
recycled. That is unsound — "address not recycled" does not imply "not freed" —
and it aborted the process with glibc's `free(): double free detected in tcache`.
The probe now never frees that pointer and leaks a few bytes instead.

## Observability limits (stated explicitly)

* Passing a pointer that did not come from `malloc` to `cleanup_resources`, or
  double-freeing, is UB in the C. There is no defined behaviour to differentially
  compare, so it is deliberately NOT tested.
* Rows E1/E2 are unreachable in the C as written; they are pinned negatively
  (the branch's output must never appear) rather than positively triggered,
  because triggering them would require editing `c_src/`.
