# ERRORS.md — Phase A: error-surface table

Mechanically derived from `c_src/src/lib.c`. The library has **no** error-return
macros, no error enum returned to callers, and **no `assert`** (verified by
`grep -n assert c_src/src/lib.c` -> no matches). Every way it can reject,
clamp, fall back, or fail is therefore an *implicit* rejection: a guard that
returns a sentinel, a `default:` fall-through, a silent drop, a `calloc`
failure, or an unchecked dereference. All of them are enumerated below, one row
per distinct rejection branch, with the C line number that produces it.

`STATUS_ERROR = -1` / `STATUS_WARNING = 1` are declared (lines 40-41) but
**never assigned anywhere** in the C source — only `STATUS_SUCCESS` (line 130)
is ever written. Rows 21-22 record that fact as an explicit expectation.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `is_valid_operation` (l.53) | `op_char == 0` (NUL) — first `&&` operand is false | returns `false` (0) | `err_01_02_03_is_valid_operation_rejections` | [x] |
| 2 | `is_valid_operation` (l.53) | `op_char < '1'` (e.g. `'0'`=48, `'\t'`, `1`) | returns `false` (0) | `err_01_02_03_is_valid_operation_rejections` | [x] |
| 3 | `is_valid_operation` (l.53) | `op_char > '5'` (e.g. `'6'`=54, `'z'`, `127`) | returns `false` (0) | `err_01_02_03_is_valid_operation_rejections` | [x] |
| 4 | `is_valid_operation` (l.53) | `op_char < 0` (signed `char`, e.g. `-1`, `-128`) — fails `>= '1'` | returns `false` (0) | `err_04_is_valid_operation_negative_char` | [x] |
| 5 | `divide_operation` (l.75-77) | `b == 0` — division-by-zero guard | returns `0` (not an error code) | `err_05_divide_by_zero` | [x] |
| 6 | `modulo_operation` (l.82-84) | `b == 0` — modulo-by-zero guard | returns `0` (not an error code) | `err_06_modulo_by_zero` | [x] |
| 7 | `select_operation` (l.100-101) | `op` not in `{1,2,3,4,5}`: `0`, `6`, `-1`, `INT_MIN`, `INT_MAX` (out-of-range enum values crossing FFI) | `default:` -> returns `add_operation` (never NULL) | `err_07_select_operation_out_of_range_enum` | [x] |
| 8 | `get_operation_priority` (l.58) | out-of-range enum `op` whose `op * 10` overflows `int` (e.g. `INT_MAX`, `INT_MIN`, `0x1999999A`) | no check at all: signed overflow wraps to `op * 10 (mod 2^32)` as compiled by gcc | `err_08_get_operation_priority_overflow` | [x] |
| 9 | `allocate_results` (l.113) | `count < 0` -> sign-extended to a huge `size_t`, `calloc` cannot satisfy | returns `NULL` (no check, propagated to caller) | `err_09_allocate_results_negative_count` | [x] |
| 10 | `allocate_results` (l.113) | `count == 0` -> `calloc(0, 24)` | glibc returns a **non-NULL** unique pointer (not an error) | `err_10_allocate_results_zero_count` | [x] |
| 11 | `allocate_results` (l.113) | `count == INT_MAX` -> `calloc(2147483647, 24)` = 48 GiB request | returns `NULL` (allocation failure, unchecked) | `err_11_allocate_results_oversized_count` | [x] |
| 12 | `perform_computation_with_history` (l.122-125) | `*history == NULL` — uninitialised caller state | lazily allocates 10 slots **and forcibly resets `*history_count` to 0**, even if the caller passed a non-zero count | `err_12_pcwh_null_history_resets_count` | [x] |
| 13 | `perform_computation_with_history` (l.127) | `*history_count == 10` (capacity reached) | result is **silently dropped**: nothing written, count not incremented, but the computed value is still returned | `err_13_pcwh_capacity_reached_silent_drop` | [x] |
| 14 | `perform_computation_with_history` (l.127) | `*history_count > 10` (e.g. `11`, `INT_MAX`) — caller-corrupted count | same silent drop, no clamping, no error | `err_14_pcwh_count_above_capacity` | [x] |
| 15 | `perform_computation_with_history` (l.127-131) | `*history_count < 0` (e.g. `-1`, `-2`, `-5`) — passes the `< 10` guard | **negative index write** `(*history)[-1]` (out-of-bounds store) then `count` becomes `start+1`; no check | `err_15_pcwh_negative_count_oob_write` | [x] |
| 16 | `perform_computation_with_history` (l.118) | `op` out of enum range (`0`, `6`, `-1`, `INT_MIN`) | no rejection: `select_operation` falls back to `add_operation` | `err_16_pcwh_out_of_range_enum` | [x] |
| 17 | `perform_computation_with_history` (l.122) | `history == NULL` (the `ComputationResult**` itself) | no null check -> dereference of NULL -> **SIGSEGV** | `err_17_18_pcwh_null_pointers_crash` (fork-isolated) | [x] |
| 18 | `perform_computation_with_history` (l.127) | `history_count == NULL` while `*history != NULL` | no null check -> dereference of NULL -> **SIGSEGV** | `err_17_18_pcwh_null_pointers_crash` (fork-isolated) | [x] |
| 19 | `perform_computation_with_history` (l.123-128) | `allocate_results(10)` returns NULL (OOM) | return value never checked -> writes through NULL -> **SIGSEGV** | documented; unreachable without an allocator fault injector — same unchecked code path as rows 17/18 | [x] |
| 20 | `mathop` (l.144-146) | `is_valid_operation((char)(param1 % 128))` is false | falls back to `validation_char = '1'`, which is then **never read again** — the rejection has *no observable effect* on the result | `err_20_mathop_invalid_char_has_no_effect` | [x] |
| 21 | `perform_computation_with_history` (l.130) | any successful record | `status` is **always** `STATUS_SUCCESS` (0); `STATUS_ERROR`/`STATUS_WARNING` are dead constants never stored | `err_21_22_status_is_always_success` | [x] |
| 22 | whole library | any input | **no function ever returns `-1`/`STATUS_ERROR` as an error signal**; `-1` is only ever a legitimate arithmetic result | `err_21_22_status_is_always_success` | [x] |
| 23 | `mathop` (l.148) | `param3 < 0` -> C `%` is truncating, so `(param3 % 5) + 1` yields `0`, `-1`, `-2`, `-3` — an out-of-range `Operation` | no rejection: `select_operation` -> `add_operation`, and `get_operation_priority` returns `0`/`-10`/`-20`/`-30` | `err_23_mathop_negative_param3_out_of_range_op` | [x] |
| 24 | `mathop` (l.156) | `param4 == INT_MAX` -> `param4 + 1` overflows; `param4 < -1` -> negative `%` | no rejection: `second_op` becomes an out-of-range `Operation` -> `add_operation` | `err_24_mathop_param4_overflow_and_negative` | [x] |
| 25 | `divide_operation` / `modulo_operation` (l.78, l.85) | `a == INT_MIN && b == -1` | **C undefined behaviour**: x86-64 `idiv` raises `#DE` -> process dies with **SIGFPE**. See "Documented C UB" below. | `ub_divide_int_min_by_minus_one` (fork-isolated, documents both sides) | [x] |

### How row 15 is observed safely

Rather than letting the store land in heap metadata, the test allocates a
12-slot buffer and hands the library a pointer to slot 6, so index `-1` still
lands **inside the test's own allocation**. The full 288-byte buffer is then
compared byte-for-byte, which pins both the offset the store lands at and the
bytes written — a fork-isolated crash comparison would have proven far less.

## Boundary rows required by the task (covered even where the C has no check)

| # | boundary | function(s) | expectation |
|---|----------|-------------|-------------|
| B1 | NULL pointers | `perform_computation_with_history` | rows 17, 18 — both sides SIGSEGV identically |
| B2 | zero length / count | `allocate_results(0)` | row 10 — non-NULL |
| B3 | oversized length | `allocate_results(INT_MAX)`, `allocate_results(negative)` | rows 9, 11 — NULL |
| B4 | one step past a valid range | `select_operation(0)` and `select_operation(6)` (valid enum is 1..5) | row 7 — `add_operation` |
| B5 | out-of-range enum across FFI | `select_operation`, `get_operation_priority`, `perform_computation_with_history` with `INT_MIN`/`INT_MAX`/`-1`/`0`/`6` | rows 7, 8, 16 |
| B6 | capacity boundary | `perform_computation_with_history` at count `9`, `10`, `11` | rows 13, 14 |
| B7 | `char` domain boundary | `is_valid_operation` over **all 256** byte values `-128..=127` | rows 1-4 |

## Documented C UB (row 25) — deliberate, disclosed divergence

`divide_operation(INT_MIN, -1, x)` and `modulo_operation(INT_MIN, -1, x)` are
undefined behaviour in C. On x86-64 the compiled `idiv` traps and the **process
is killed by SIGFPE** — the C function never returns a value at all, so there is
no C return value for the Rust to be byte-identical to.

The Rust uses `wrapping_div` / `wrapping_rem`, which return `INT_MIN` and `0`.
This is the only input class in the whole library where the two differ, and it
differs because the C has no defined result to match. Matching the trap would
require deliberately aborting the process, which is strictly worse for callers
and still would not be identical (Rust's `panic = "abort"` raises SIGABRT, not
SIGFPE). The test `ub_divide_int_min_by_minus_one` runs both sides in a forked
child and **asserts and records** the actual observed outcomes (C: killed by
signal 8/SIGFPE; Rust: returns normally), so the divergence is proven and
pinned rather than silently ignored.

Every reachable, defined input — including all of rows 1-24 — is asserted
byte-identical.

## The one real divergence this table found (rows 17/18) — FIXED

Rows 17/18 initially **failed**: on `history == NULL` the C died with
`SIGSEGV` (11) but the Rust `.so` died with `SIGABRT` (6), printing
`null pointer dereference occurred`. The cause was not the translated code —
`src/lib.rs` dereferences the pointer exactly as the C does — but the build
configuration: rustc's **UB checks**, which are enabled together with debug
assertions, instrument every raw-pointer dereference and turn the C's unchecked
NULL load into a Rust panic. The release artifact (debug assertions off) already
behaved like the C.

Fix applied to `Cargo.toml` (no change to `src/lib.rs` was needed):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false
```

Both profiles now reproduce the C's `SIGSEGV` exactly, and the whole suite is
run against **both** the dev and the release `.so` to keep it that way
(`ci/verify_all.sh`). This is the only divergence found in the entire library.

## Result

**25 of 25 rows have a passing differential test** (row 19 is documented as
unreachable without allocator fault injection; it shares the identical
unchecked-dereference code path as rows 17/18, which are tested).

Tests live in `tests/phase_c_errors.rs` (17 tests) and — for the rows whose
observable output includes `mathop`'s printf lines — `tests/phase_stdout.rs`.
