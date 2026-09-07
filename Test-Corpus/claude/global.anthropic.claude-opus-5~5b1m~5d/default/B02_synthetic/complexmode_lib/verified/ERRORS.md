# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return` that is
reachable from a rejection test, every NULL check, every `else`/`default`
fallback, and every allocation-failure branch. There are no `assert`s, no error
enums, and no `RETURN_ERROR`-style macros in this translation unit.

Rejection sites found (`grep -n 'NULL\|return -1\|return 0\|default:' src/lib.c`):
lines 40–42, 52–55, 61–63, 68–71, 74–77, 91–94, 106–109, 166–170.

Note: "expected C result" includes the exact bytes written to `stdout`, because
several rejection paths are observable *only* through `printf`. Every row's test
captures `stdout` from both libraries and compares it byte-for-byte in addition
to comparing the return value.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `create_result_string` | `malloc(64)` returns `NULL` (line 40) | returns `NULL`, prints nothing | [x] |
| 2 | `safe_add` | `check_permissions(perms, 0400\|0200) == 0`, i.e. `perms & 0600 != 0600` (line 52) — e.g. `perms=0`, `0400`, `0200`, `0100`, `0444`, `-1`&`~0600` | prints `Insufficient permissions for addition\n`, returns `0` (NOT `a+b`) | [x] |
| 3 | `multiply_with_log` | `create_result_string` returned `NULL` so `*log_msg == NULL` (line 61) | returns `0`, `*log_msg` left `NULL` | [x] |
| 4 | `multiply_with_log` | `log_msg == NULL` — the C does **no** NULL check and dereferences it (line 60) | undefined behaviour (SIGSEGV) in both; NOT tested, documented only | [x] |
| 5 | `copy_and_sum` | `src == NULL` (line 68) | prints `Source pointer is NULL\n`, returns `-1` — checked *before* `count` | [x] |
| 6 | `copy_and_sum` | `src == NULL` **and** `count` also invalid (e.g. `count=-1`, `count=INT_MIN`) | NULL check wins: prints `Source pointer is NULL\n`, returns `-1` | [x] |
| 7 | `copy_and_sum` | `malloc(count * sizeof(int))` returns `NULL` (line 74). `count` is sign-extended to `size_t` then multiplied by 4 mod 2^64, so any `count < 0` (e.g. `-1`, `-8`, `INT_MIN`) requests a huge size and fails | prints `Memory allocation failed\n`, returns `-1` | [x] |
| 8 | `copy_and_sum` | `count == 0` (boundary, *not* an error in C): `malloc(0)` returns a non-NULL unique pointer, `memcpy(...,0)`, loop body never runs | returns `0`, prints nothing | [x] |
| 9 | `copy_and_sum` | `count` larger than the real `src` buffer (out-of-range read, e.g. `count=64` on a 3-element array) | reads past the array; both libraries must read the *same* bytes and return the same sum. Exercised only where the over-read stays inside a deliberately over-allocated buffer | [x] |
| 10 | `compare_operations` | `op1 == NULL`, `op2` valid (line 91) | prints `One or both operation strings are NULL\n`, returns `-1` | [x] |
| 11 | `compare_operations` | `op1` valid, `op2 == NULL` (line 91) | prints `One or both operation strings are NULL\n`, returns `-1` | [x] |
| 12 | `compare_operations` | both `op1 == NULL` and `op2 == NULL` (line 91) | prints `One or both operation strings are NULL\n`, returns `-1` | [x] |
| 13 | `compare_operations` | valid but *unequal* strings — return value is raw `strcmp`, not normalised to ±1 | returns glibc `strcmp`'s exact value (sign and magnitude must match) | [x] |
| 14 | `complexmode` | `malloc(sizeof(Result))` returns `NULL` (line 106) | prints `Failed to allocate result tracker\n`, returns `-1` | [x] |
| 15 | `complexmode` | `mode` matches no `case` — the `default:` arm (line 166). `mode` is a plain `int` so *any* out-of-range value is a real input: `0`, `5`, `-1`, `6`, `100`, `INT_MIN`, `INT_MAX`, and every value one step past the valid range | prints `Invalid mode\n`, returns `-1`, and prints **no** `Operation performed:` line because `operation` is still `"none"` | [x] |
| 16 | `complexmode` | `mode == 1` with the built-in `permissions = 0644` — `0644 & 0600 == 0600`, so `safe_add` never rejects; the rejection branch of row 2 is **unreachable** from `complexmode` | prints `Mode 1: Addition\nResult: <a+b>\nOperation performed: addition\n`, returns `a+b` | [x] |
| 17 | `complexmode` | `mode == 4` with `permissions = 0644` — `check_permissions(0644, 0100)` is `0`, so the `else` branch is taken (line 157) and the `EXEC_PERM` branch is **dead** | returns `value1+value2+value3` (NOT `value1*value2+value3`) | [x] |
| 18 | `complexmode` | `mode == 2` where `log_message` is non-NULL but compares equal to `""` (line 131) — unreachable because `snprintf` always writes the `"Operation: "` prefix | dead branch; `Mode 2: <msg>` is always printed instead | [x] |
| 19 | signed overflow | `safe_add`/`multiply_with_log`/`copy_and_sum`/`complexmode` mode 4 with operands whose sum/product overflows `int` (e.g. `INT_MAX + 1`, `INT_MIN * -1`) | C UB, but as compiled it wraps two's-complement; the Rust must produce the identical wrapped value | [x] |
| 20 | `check_permissions` | no rejection path exists; total function. `required == 0` always yields `1` (including `perms == 0`); negative `perms`/`required` are accepted as bit patterns | returns `1` or `0`, never an error | [x] |

## How rows 1, 3 and 14 (allocation failure) are tested

`malloc` cannot be made to fail deterministically for a 64-byte or 40-byte
request from inside the test process without perturbing the harness itself.
Both libraries import `malloc@GLIBC` from the *same* shared glibc, so they take
the identical branch under identical allocator state. The tests cover these rows
by:

* asserting the *structure* of the branch is identical — the code inspects
  `dest`/`str`/`res_tracker` for NULL before use in both implementations
  (verified by reading the Rust: `s.is_null()`, `(*log_msg).is_null()`,
  `res_tracker.is_null()`);
* driving the reachable sibling branch of the same allocation
  (`copy_and_sum` with a negative `count`, row 7) which *does* make `malloc`
  return NULL for real, and confirming both print `Memory allocation failed\n`
  and return `-1`.

Row 4 is genuine UB in the C (unconditional `*log_msg = ...`); the Rust
reproduces the unconditional store, so both crash identically. It is documented
rather than executed so as not to abort the test process.

## Row → test mapping

| ERRORS row | test in `tests/phase_c_errors.rs` |
|---|---|
| 1 | `err01_create_result_string_malloc_failure` (forked, drained heap) |
| 2 | `err02_safe_add_insufficient_permissions` |
| 3 | `err03_multiply_with_log_null_log` (forked, drained heap) |
| 4 | `err04_multiply_with_log_always_stores_out_param` (UB documented, not executed) |
| 5 | `err05_copy_and_sum_null_src` |
| 6 | `err06_copy_and_sum_null_src_and_bad_count` |
| 7 | `err07_copy_and_sum_alloc_failure` |
| 8 | `err08_copy_and_sum_zero_count` |
| 9 | `err09_copy_and_sum_over_read` |
| 10 | `err10_compare_operations_null_first` |
| 11 | `err11_compare_operations_null_second` |
| 12 | `err12_compare_operations_both_null` |
| 13 | `err13_compare_operations_raw_strcmp_value` |
| 14 | `err14_complexmode_tracker_alloc_failure` (forked, drained heap) |
| 15 | `err15_complexmode_invalid_mode` |
| 16 | `err16_complexmode_mode1_never_rejected` |
| 17 | `err17_complexmode_mode4_takes_else_branch` |
| 18 | `err18_complexmode_mode2_log_never_empty` |
| 19 | `err19_signed_overflow_wraps_identically` |
| 20 | `err20_check_permissions_is_total` |
| generic FFI boundaries | `err_generic_boundaries` — NULL pointers, zero/negative/oversized lengths, out-of-range "enum" values for `mode` (`INT_MIN`, `INT_MAX`, `0x1234_5678`, one step past 1..=4), permission masks one step past `0600`/`0100` |

### Anti-vacuity guarantee for the allocation-failure rows

Rows 1, 3 and 14 are no longer "structural only": they are executed for real.
`run_oom` forks a child, redirects fd 1, lowers `RLIMIT_AS` to just above the
current address-space size, and then drains every remaining block out of the
allocator, so the library's own `malloc` genuinely returns `NULL`. Each of those
tests first asserts that the **C** child actually took the failure branch —

* row 1: `create_result_string` returned `NULL` and printed nothing;
* row 3: `multiply_with_log` returned `0` with `*log_msg == NULL` **and** the
  out-parameter was written;
* row 14: `complexmode` returned `-1` and printed exactly
  `Failed to allocate result tracker\n`

— and only then compares the C and Rust children. All three assertions hold, so
the branches really were reached rather than skipped.
