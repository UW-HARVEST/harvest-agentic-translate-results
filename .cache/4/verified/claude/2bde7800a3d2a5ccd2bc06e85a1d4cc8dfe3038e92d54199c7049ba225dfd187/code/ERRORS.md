# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping every `return -1`,
`return 0`-on-failure, `return NULL`, every `== NULL` / `!` guard, every
`default:` label and every constant used as a limit.  `c_src` contains no
`assert`, no error enum, no `RETURN_ERROR`-style macro and no explicit numeric
range check other than the `switch (mode)` cover.

Line numbers refer to `c_src/src/lib.c`.

| #  | function | trigger (exact invalid input / condition) | expected C result | test |
|----|----------|-------------------------------------------|-------------------|------|
| 1  | `create_result_string` (L40-42) | `malloc(64)` returns `NULL` | returns `NULL`; nothing printed | `error_paths.rs::row01_..` (fault-injected `malloc`) |
| 2  | `multiply_with_log` (L61-63) | inner `create_result_string` returns `NULL` (i.e. `malloc(64)` fails) | `*log_msg` set to `NULL`, returns `0` (not `a*b`) | `error_paths.rs::row02_..` (fault-injected `malloc`) |
| 3  | `safe_add` (L52-55) | `check_permissions(perms, READ_PERM\|WRITE_PERM)` is false, i.e. `(perms & 0600) != 0600` (missing read bit, missing write bit, or both) | prints `Insufficient permissions for addition\n`, returns `0` — **not** `a+b` | `error_paths.rs::row03_..` |
| 4  | `copy_and_sum` (L68-71) | `src == NULL` (any `count`, incl. 0 / negative) | prints `Source pointer is NULL\n`, returns `-1` | `error_paths.rs::row04_..` |
| 5  | `copy_and_sum` (L73-77) | `malloc(count * sizeof(int))` returns `NULL`. Reachable with any **negative** `count`: `count` converts to `size_t`, so `-k` becomes `2^64 - 4k` — an impossible size. Also reachable for huge positive `count`, though that half is host-dependent: `count == INT_MAX` asks for 8589934588 bytes, which fails here but could succeed under generous overcommit — after which the `memcpy` reads out of bounds in *both* implementations.  Only the negative counts are unconditionally unsatisfiable, so the tests use those. | prints `Memory allocation failed\n`, returns `-1` | `error_paths.rs::row05_..` (negative counts incl. `-1`, `INT_MIN`), `row05b_..` (fault-injected size 12) and `row05c_..` (the same failure reached *through* `complexmode` mode 3 — see the note below) |
| 6  | `compare_operations` (L91-94) | `op1 == NULL`, `op2 != NULL` | prints `One or both operation strings are NULL\n`, returns `-1` | `error_paths.rs::row06_..` |
| 7  | `compare_operations` (L91-94) | `op1 != NULL`, `op2 == NULL` | same as row 6 | `error_paths.rs::row07_..` |
| 8  | `compare_operations` (L91-94) | both `op1` and `op2` `NULL` | same as row 6 | `error_paths.rs::row08_..` |
| 9  | `complexmode` (L106-109) | `malloc(sizeof(Result))` (40 bytes) returns `NULL` | prints `Failed to allocate result tracker\n`, returns `-1`, **no** `Operation performed:` line | `error_paths.rs::row09_..` (fault-injected `malloc`) |
| 10 | `complexmode` (L166-170) `default:` | `mode` not in `{1,2,3,4}` — `0`, `5`, `-1`, `INT_MIN`, `INT_MAX`, and arbitrary out-of-range ints (a C `enum`/`int` parameter accepts *any* `int` across the FFI boundary) | prints `Invalid mode\n`, returns `-1`; `operation` stays `"none"` so the trailing `Operation performed:` line is suppressed | `error_paths.rs::row10_..` |
| 11 | `complexmode` case 2 (L131-133) | `log_message == NULL` **or** `strcmp(log_message,"") == 0` after `multiply_with_log` — reachable when the 64-byte `malloc` inside `create_result_string` fails but the 40-byte tracker `malloc` succeeded | prints `Log message creation failed\n` (instead of `Mode 2: ...`), returns `0`; still prints `Operation performed: multiplication\n` | `error_paths.rs::row11_..` (fault-injected `malloc`, size 64 only) |
| 12 | `copy_and_sum` (L79) | `count == 0` — boundary, *not* an error: `malloc(0)` returns a non-NULL minimal block on glibc, the loop body never runs | returns `0`, nothing printed | `error_paths.rs::row12_..` |
| 13 | `create_result_string` (L43) | formatted text does not fit the 64-byte buffer (`"Operation: " + op + ", Value: " + digits` ≥ 64 bytes, e.g. `op` of 60 `x`s, or a short `op` with `val = INT_MIN`) — `snprintf`'s truncation limit, and its `≥ 64` return value is discarded so the loss is silent | writes 63 bytes + NUL, no error reported, returns the (truncated) buffer | `valid_paths.rs::row23_..` (op lengths 0..80 × widest `%d` values) + `heap_poison.rs::row44_..` (compares the bytes *past* the NUL too) |
| 14 | `complexmode` (L113/117/127/141/152) | `strcpy` into the fixed `char operation[32]` — the only unchecked buffer write in the library | cannot overflow: the longest literal is `"multiplication"` (15 bytes incl. NUL), so 17 bytes always remain. Both implementations must write the *same* number of bytes, including the terminator | `heap_poison.rs::row44_..` — on a heap pre-filled with a non-zero byte a short copy or a dropped NUL makes `printf("Operation performed: %s")` run into the fill pattern (verified: a mutant that drops the NUL passes every zero-heap test and is caught here) |

## Notes on unreachable / UB rejections (documented, deliberately not tested)

* `multiply_with_log(a, b, NULL)` — the C dereferences `*log_msg`
  unconditionally (L60) with no NULL guard, so a `NULL` out-parameter is
  undefined behaviour (segfault), not a rejection. The Rust translation
  reproduces the unguarded dereference verbatim. Not a table row; calling it
  would crash both libraries identically.
* `copy_and_sum(src, count)` with `count` larger than the real length of `src`
  reads out of bounds in both implementations (UB); only in-bounds `count`
  values are exercised on the valid path.
* Inside `complexmode` the permission word is hard-coded to `0644`, and
  `0644 & (READ_PERM|WRITE_PERM) == 0600`, so row 3 can never fire *through*
  `complexmode`; it is only reachable by calling the exported `safe_add`
  directly (which the tests do).
* **Every error sentinel collides with a legitimate result**, so a test that
  only checks the return value cannot tell rejection from success:
  `compare_operations` returns `-1` both for the NULL rejection and for
  `strcmp("a","b")`; `copy_and_sum` returns `-1` both for its two error paths
  and for a legitimate sum of `-1`; `safe_add` returns `0` both on denial and
  for `a + b == 0`; `multiply_with_log` returns `0` both on allocation failure
  and for `a * b == 0`. Every row's test therefore asserts the exact printed
  bytes (and, for the allocation rows, the exact `malloc` request sizes) in
  addition to the return value.
* `create_result_string` never rejects a `NULL` `op`: it is forwarded to
  `snprintf("%s")`, which glibc renders as `(null)`. That is a valid-path row in
  `CONFIGS.md` (row 22), not an error row.

## Regression note — allocation elision (row 5 via `complexmode`)

Row 5 is also reachable through `complexmode`'s mode-3 arm, and that path found a
genuine divergence in the optimised Rust build: once `copy_and_sum` was inlined,
its buffer no longer escaped and LLVM removed the `malloc`/`free` pair, so the
`NULL` branch became unreachable and the release `.so` returned the sum where the
C returns `-1` and prints `Memory allocation failed`. `src/lib.rs` now passes
every `malloc()` result through `keep_allocation()` (`core::hint::black_box`) so
the allocation, and hence the failure path, survives at every optimisation
level. Covered by `error_paths.rs::row05c_..` and by CONFIGS.md row 45.

## Completion status (Phase C)

| row | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 |
|-----|---|---|---|---|---|---|---|---|---|----|----|----|----|----|
| test passes | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] | [x] |

(Row 5 is covered by three tests: negative counts, an injected 12-byte failure,
and the same failure through `complexmode` mode 3.)

All 14 rows pass against the C `.so` built at `-O0`, `Debug`, `Release`,
`RelWithDebInfo` and `MinSizeRel`, and against the Rust `.so` built in the debug
and release profiles.
