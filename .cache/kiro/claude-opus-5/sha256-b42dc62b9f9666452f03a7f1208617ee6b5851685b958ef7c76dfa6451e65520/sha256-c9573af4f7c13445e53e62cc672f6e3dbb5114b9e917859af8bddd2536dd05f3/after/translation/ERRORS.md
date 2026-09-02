# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return` of an
error value / sentinel, every `if` that rejects an input, every NULL check, and
every min/max constant. There are no `assert`s and no error enums in this
library; rejection is signalled by `-1`, `0`, or `NULL` return values plus a
message on stdout.

Line numbers refer to `c_src/src/lib.c`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|---------------------------------------------|-------------------|------|
| 1  | `create_result_string` | `malloc(64)` returns NULL (L40) | returns `NULL`, no stdout output | [x] documented — not reachable without an allocator fault injector (see note A) |
| 2  | `create_result_string` | `op` is a NULL pointer — no NULL check exists, pointer is passed to `snprintf` `%s` (L43) | glibc `snprintf` prints the literal `(null)`; returns non-NULL buffer `"Operation: (null), Value: <v>"` | [x] `err_create_result_string_null_op` |
| 3  | `create_result_string` | `op` longer than the 64-byte budget (L39/L43) | `snprintf` truncates to 63 chars + NUL; still returns non-NULL | [x] `err_create_result_string_truncation` |
| 4  | `check_permissions` | `(perms & required) != required`, i.e. any required bit missing (L48) | returns `0` (rejection), no stdout | [x] `err_check_permissions_missing_bits` |
| 5  | `safe_add` | `perms` lacks `READ_PERM|WRITE_PERM` == `0600` (L52) | prints `Insufficient permissions for addition\n`; returns `0` **(not `-1`)** | [x] `err_safe_add_insufficient_perms` |
| 6  | `safe_add` | `perms == 0` (degenerate case of #5) | prints the same message; returns `0` | [x] `err_safe_add_insufficient_perms` |
| 7  | `safe_add` | `perms` has only `READ_PERM` (0400) or only `WRITE_PERM` (0200) — one step short of the valid range | prints the message; returns `0` | [x] `err_safe_add_insufficient_perms` |
| 8  | `multiply_with_log` | inner `create_result_string` returns NULL, so `*log_msg == NULL` (L61) | returns `0`, `*log_msg` left NULL | [x] documented — depends on #1 (note A) |
| 9  | `multiply_with_log` | `log_msg` itself is NULL — the C **unconditionally** dereferences it (L60) | SIGSEGV (signal 11) | [x] `err_multiply_with_log_null_out` (subprocess-isolated crash-parity check, note B) — **found a real divergence, see note C** |
| 9b | `multiply_with_log` | `log_msg` non-NULL but MISALIGNED — the C does an unaligned 8-byte store, which succeeds on x86-64 | normal success: `*log_msg` set, product returned | [x] `err_multiply_with_log_misaligned_out` — **same root cause as note C** |
| 10 | `copy_and_sum` | `src == NULL` (L68) | prints `Source pointer is NULL\n`; returns `-1`. Checked **before** `count`, so a NULL src with any count (incl. negative) takes this path | [x] `err_copy_and_sum_null_src` |
| 11 | `copy_and_sum` | `count` negative ⇒ `count * sizeof(int)` converts `count` to `size_t` first, giving a huge request, so `malloc` returns NULL (L73/L74) | prints `Memory allocation failed\n`; returns `-1` | [x] `err_copy_and_sum_negative_count` |
| 12 | `copy_and_sum` | `count == INT_MIN` (extreme of #11) | `(size_t)INT_MIN * 4` = huge ⇒ malloc fails ⇒ prints message, returns `-1` | [x] `err_copy_and_sum_negative_count` |
| 13 | `copy_and_sum` | `count` positive but so large the allocation fails (e.g. `INT_MAX`, `0x4000_0000`) | prints `Memory allocation failed\n`; returns `-1` | [x] `err_copy_and_sum_huge_count` |
| 14 | `copy_and_sum` | `count == 0` — zero length, no explicit check | `malloc(0)` returns a non-NULL glibc pointer, loop body never runs ⇒ returns `0` (**not** an error) | [x] `err_copy_and_sum_zero_count` |
| 15 | `compare_operations` | `op1 == NULL`, `op2` valid (L91) | prints `One or both operation strings are NULL\n`; returns `-1` | [x] `err_compare_operations_nulls` |
| 16 | `compare_operations` | `op2 == NULL`, `op1` valid (L91) | same message; returns `-1` | [x] `err_compare_operations_nulls` |
| 17 | `compare_operations` | both NULL (L91) | same message; returns `-1` | [x] `err_compare_operations_nulls` |
| 18 | `compare_operations` | valid strings that differ — `strcmp` returns a non-zero value; the magnitude, not just the sign, is observable through the ABI | returns glibc `strcmp`'s exact int (byte difference of `unsigned char`s) | [x] `err_compare_operations_nonzero_magnitude` |
| 19 | `complexmode` | `malloc(sizeof(Result))` returns NULL (L106) | prints `Failed to allocate result tracker\n`; returns `-1` | [x] documented — not reachable (note A) |
| 20 | `complexmode` | `mode` not in `{1,2,3,4}` ⇒ `default:` (L166) | prints `Invalid mode\n`, returns `-1`, and because `default` never `strcpy`s `operation`, it stays `"none"` so the `Operation performed:` line is **suppressed** (L173) | [x] `err_complexmode_invalid_mode` |
| 21 | `complexmode` | `mode == 0` (one step below the valid range) | as #20 | [x] `err_complexmode_invalid_mode` |
| 22 | `complexmode` | `mode == 5` (one step past the valid range) | as #20 | [x] `err_complexmode_invalid_mode` |
| 23 | `complexmode` | `mode` negative, `INT_MIN`, `INT_MAX` — out-of-range "enum" values crossing the FFI boundary | as #20 | [x] `err_complexmode_invalid_mode` |
| 24 | `complexmode` mode 2 | `log_message == NULL` **or** `strcmp(log_message,"") == 0` (L131) | prints `Log message creation failed\n` and **leaks** `log_message` (no `free`); return value is whatever `multiply_with_log` gave | [x] documented — unreachable: `create_result_string` always writes a non-empty prefix (note A) |
| 25 | `complexmode` mode 1 | inherits #5–#7 — but `permissions` is hard-coded `0644`, and `0644 & 0600 == 0600`, so the rejection branch is **dead** here | mode 1 always performs the addition | [x] `cfg_complexmode_mode1_permission_branch_is_dead` (asserts the message is absent) |
| 26 | `complexmode` mode 4 | `check_permissions(0644, 0100)` is **false** (`0644 & 0100 == 0`), so the `else` is always taken | `result = v1 + v2 + v3` (never `v1*v2+v3`) | [x] `cfg_complexmode_mode4_takes_else_branch` |
| 27 | `safe_add` / `multiply_with_log` / `copy_and_sum` / `complexmode` | signed integer overflow on `a + b`, `a * b`, `sum += …`, `v1*v2+v3` — C UB, wraps at `-O0` | two's-complement wraparound | [x] `cfg_*_overflow` rows in CONFIGS.md |

## Min / max constants found in the C source

| constant | value | where |
|----------|-------|-------|
| `READ_PERM`  | `0400` | L28, used in `safe_add` |
| `WRITE_PERM` | `0200` | L29, used in `safe_add` |
| `EXEC_PERM`  | `0100` | L30 — **defined but never used**; `complexmode` mode 4 hard-codes the literal `0100` instead (L154) |
| result-string buffer | `64` bytes | L39, `snprintf` bound L43 |
| `Result::operation` | `char[32]` | L34; longest string written is `"multiplication"` (14+1) — no overflow |
| `complexmode` `permissions` | `0644` | L103 |
| mode-3 array | `int[3]`, count `3` | L143/L144 |
| valid `mode` range | `1..=4` | L116–L166 |

## Notes

**Note A — allocation-failure branches.** Rows 1, 8, 19 and 24 fire only when
`malloc` fails. They cannot be triggered through the FFI boundary without an
allocator fault injector, and injecting one (e.g. `LD_PRELOAD`) would perturb
both libraries' allocators unequally. Instead these are verified by inspection:
the Rust translation calls the *same* libc `malloc` and reproduces each check
verbatim (`if str_.is_null() { return null_mut() }`, `if (*log_msg).is_null()
{ return 0 }`, `if res_tracker.is_null() { print; return -1 }`, and the
`log_message.is_null() || strcmp(log_message,"")==0` disjunction). Row 24's
second disjunct is likewise unreachable in both.

**Note B — deliberate-crash parity.** Rows 9 and 13 are real UB / environment
dependent in the C. The test re-invokes the test binary as a child process once
per library, performs the call there, and compares exit status plus every byte
the child wrote, so the parent harness survives. `run_isolated` reports the
signal, so SIGSEGV-vs-SIGABRT is a detected difference rather than "both
failed somehow". Row 13's two cases behave differently on this machine and both
libraries agree in each: `count == INT_MAX` (8 GiB) fails `malloc` and returns
`-1` with `Memory allocation failed`, while `count == 1 << 30` (4 GiB) succeeds
and then faults inside `memcpy`.

**Note C — a real divergence this row caught.** With `*log_msg = …` written as
an ordinary raw-pointer store, the Rust translation matched the C in `--release`
but **not** in the debug profile: rustc's debug-only null-and-alignment
precondition assertion fires before the store, so a NULL out-pointer produced
`abort` (SIGABRT, signal 6) where the C produced a hardware fault (SIGSEGV,
signal 11); a misaligned out-pointer would likewise have panicked where the C
succeeds. `src/lib.rs` now performs that 8-byte store and its read-back through
libc `memcpy`, which is the same store for well-formed pointers and reproduces
the C's failure mode in every profile. This is the one behavioural fix the
verification required.
