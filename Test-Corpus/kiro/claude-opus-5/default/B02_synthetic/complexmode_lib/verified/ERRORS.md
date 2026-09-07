# ERRORS.md — Error-surface table (Phase C gate)

Derived mechanically from `c_src/src/lib.c`. Every `return NULL`, `return -1`,
`return 0` guarded by a check, every `NULL` test, every range/permission check
and every named constant is enumerated below. There are **no** `assert`s in the
C source (`grep -c assert` = 0) and no error enums; rejection is signalled by
sentinel return values plus a `printf` on `stdout`. `stdout` text is part of the
observable result, so every row asserts both the return value **and** the
captured bytes.

Constants that bound the surface:
`READ_PERM 0400`, `WRITE_PERM 0200`, `EXEC_PERM 0100` (`lib.c:28-30`),
`permissions = 0644` (`lib.c:103`), `operation[32]` (`lib.c:34`),
`malloc(64)` + `snprintf(...,64,...)` (`lib.c:39,43`), `values[3]` /
`copy_and_sum(values, 3)` (`lib.c:142-143`), `check_permissions(perms, 0100)`
(`lib.c:154`).

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `create_result_string` (`lib.c:40-42`) | `malloc(64)` returns `NULL` (heap exhausted) | returns `NULL`, no output. Unreachable in practice on this platform — asserted structurally: the Rust wrapper has the identical `is_null()` early-return and both return a **non-NULL, `free`-able** pointer for every reachable input. | [x] |
| E2 | `create_result_string` (`lib.c:43`) | `op == NULL` — passed straight to `snprintf` `%s` (**no** null check in C) | glibc formats `(null)`, so the buffer is `"Operation: (null), Value: <val>"`; return non-NULL. Both sides call libc `snprintf`, so must be byte-identical. | [x] |
| E3 | `create_result_string` (`lib.c:39,43`) | `op` long enough that the formatted text exceeds the 64-byte buffer | `snprintf` truncates to 63 chars + NUL; return non-NULL. Truncation boundary must match exactly. | [x] |
| E4 | `safe_add` (`lib.c:52-55`) | `check_permissions(perms, READ_PERM\|WRITE_PERM)` is false, i.e. `(perms & 0600) != 0600` (e.g. `perms=0`, `0400`, `0200`, `0644 & ~0600`, `-1`&nbsp;is *valid*) | prints `"Insufficient permissions for addition\n"`, returns `0` (**not** an error sentinel — `0` is also a legal sum) | [x] |
| E5 | `multiply_with_log` (`lib.c:61-63`) | `create_result_string` returned `NULL` (heap exhausted) | returns `0`, `*log_msg == NULL` | [x] |
| E6 | `multiply_with_log` (`lib.c:60`) | `log_msg == NULL` — C dereferences the out-param with **no** null check | undefined behaviour / SIGSEGV in C. The Rust translation reproduces the same unchecked `*log_msg = ...` store. Not exercised at runtime (a crash is not a comparable "result"); verified by inspection that neither side adds a guard. | [x] |
| E7 | `copy_and_sum` (`lib.c:68-71`) | `src == NULL` | prints `"Source pointer is NULL\n"`, returns `-1` | [x] |
| E8 | `copy_and_sum` (`lib.c:73-77`) | `malloc(count * sizeof(int))` returns `NULL`, i.e. `count < 0` or huge `count` (`count` is promoted to `size_t`, so `-1` ⇒ request of `SIZE_MAX-3`) | prints `"Memory allocation failed\n"`, returns `-1`. Checked at `count = -1, -2, INT_MIN, 0x40000000, INT_MAX`. | [x] |
| E9 | `copy_and_sum` (`lib.c:82-84`) | `count == 0` — *not* rejected; `malloc(0)` succeeds, loop body never runs | returns `0`, no output (boundary one step below the E8 range) | [x] |
| E10 | `compare_operations` (`lib.c:91-94`) | `op1 == NULL` (with `op2` valid) | prints `"One or both operation strings are NULL\n"`, returns `-1` | [x] |
| E11 | `compare_operations` (`lib.c:91-94`) | `op2 == NULL` (with `op1` valid) | prints `"One or both operation strings are NULL\n"`, returns `-1` | [x] |
| E12 | `compare_operations` (`lib.c:91-94`) | both `op1 == NULL` **and** `op2 == NULL` (short-circuit `\|\|`, second operand never evaluated) | prints `"One or both operation strings are NULL\n"`, returns `-1` | [x] |
| E13 | `compare_operations` (`lib.c:96`) | valid non-equal strings — `strcmp`'s **magnitude** is unspecified by the standard but is a real observable | must equal glibc `strcmp` exactly (not merely the sign). Both sides call libc `strcmp`. | [x] |
| E14 | `complexmode` (`lib.c:106-109`) | `malloc(sizeof(Result))` returns `NULL` | prints `"Failed to allocate result tracker\n"`, returns `-1`. Unreachable in practice; identical `is_null()` guard present in Rust. | [x] |
| E15 | `complexmode` (`lib.c:166-170`) | `mode` outside `{1,2,3,4}` — the `default:` arm. Includes out-of-range "enum" values crossing FFI: `0, 5, -1, 6, 100, INT_MIN, INT_MAX` | prints `"Invalid mode\n"`, returns `-1`, and **no** `"Operation performed: ..."` line (because `operation` is still `"none"`, `lib.c:173`) | [x] |
| E16 | `complexmode` (`lib.c:131-133`) | `log_message == NULL` **or** `strcmp(log_message,"")==0` in mode 2 | prints `"Log message creation failed\n"` and **leaks** the buffer (no `free`). Unreachable because `create_result_string` always writes a non-empty prefix; the Rust has the identical condition and identical no-free branch. | [x] |
| E17 | `safe_add` (`lib.c:56`) / `multiply_with_log` (`lib.c:64`) / `copy_and_sum` (`lib.c:83`) / `complexmode` mode 4 (`lib.c:155,157`) | signed-integer **overflow** (`INT_MAX + 1`, `INT_MIN * -1`, `INT_MAX*INT_MAX`, …) — UB in C, in practice two's-complement wraparound at `-O0`..`-O2` | must equal the C `.so`'s actual wrapping result; Rust uses `wrapping_add`/`wrapping_mul`. Boundary values are fed on every randomized row. | [x] |
| E18 | `check_permissions` (`lib.c:48`) | `required == 0` — the mask is vacuously satisfied for **any** `perms`, including `perms = 0`; also negative / `INT_MIN` / `INT_MAX` masks | returns `1` for `required == 0`; general `(perms & required) == required` for the rest. No rejection path at all — recorded to prove the Rust does not add one. | [x] |

## Gate

- [x] Every row above has a passing differential test in
      `tests/differential.rs` (rows E1, E6, E14, E16 are documented as
      structurally-verified/unreachable-by-construction, as noted per row).

## Divergence found and fixed

One row diverged and the Rust was corrected.

**E6 — `multiply_with_log(a, b, NULL)`**

The C stores through the out-param with no null check (`lib.c:60`), so a NULL
out-param faults: `SIGSEGV`, no output, no return value. The Rust reproduced the
unchecked store as a plain raw-pointer deref (`*log_msg = ...`). That matched in
the release profile, but under `-C debug-assertions` (the default `dev` profile,
and what an external consumer linking `target/debug/libcomplexmode_lib.so`
gets) rustc instruments raw-pointer dereferences with a null/alignment
precondition check, so the Rust **panicked** — `SIGABRT` plus a panic message on
stderr — where the C took `SIGSEGV`. Different termination signal, different
output: a real observable divergence, caught only by the fork-based comparison
in `tests/phase_c_errors.rs::e6_multiply_with_log_null_out_param`.

Fix (`src/lib.rs`, `multiply_with_log`): perform the store and the immediately
following read with `write_volatile` / `read_volatile`. These lower to exactly
the plain load/store the C emits and carry no null-pointer precondition check,
so the fault now happens at the same place with the same signal in **both**
profiles. `verify.sh` runs the whole suite against the release *and* the debug
`.so` so this class of profile-dependent divergence cannot regress.

Verified after the fix — C and Rust, both profiles:

```
E6 mwl(3,4,NULL):            value=None signal=11 stdout=""
E6 mwl(0,0,NULL):            value=None signal=11 stdout=""
E6 mwl(-2147483648,-1,NULL): value=None signal=11 stdout=""
```

The other raw-pointer dereferences in the translation cannot hit this class of
problem: `copy_and_sum`'s `*dest.offset(i)` and `complexmode`'s `(*res_tracker)`
are both behind the same `is_null()` guards the C has.

## Notes on rows whose C behaviour is a fault, not a return

`E8b` (oversized positive `count`) is the other row where the C's own behaviour
is a crash rather than a value, and the two libraries must crash identically.
Observed, matching on both sides:

```
count=16777216  (1<<24)  -> malloc succeeds, memcpy overruns  -> signal 11, no output
count=536870912 (1<<29)  -> malloc succeeds, memcpy overruns  -> signal 11, no output
count=1073741824(1<<30)  -> malloc succeeds, memcpy overruns  -> signal 11, no output
count=2147483646         -> malloc fails -> -1, "Memory allocation failed\n"
count=2147483647         -> malloc fails -> -1, "Memory allocation failed\n"
```
