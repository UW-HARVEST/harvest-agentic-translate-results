# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c` (66 lines) and
`c_src/include/driver.h`.

## Mechanical grep result

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|ERROR|if *\(|switch|#if' src/driver.c
  -> 0 hits in function bodies
```

The entire library is **straight-line code with no rejection paths**:

* no `return` statement anywhere (both public functions are `void`);
* no `RETURN_ERROR`-style macro, no error enum, no sentinel value;
* no `assert`, no `abort`, no `exit`;
* no `if` / `switch` / ternary / loop — zero branches in the whole file;
* no range check, no min/max constant;
* no null check — and no pointer is reachable from the public API: both
  `run(int)` and `driver(int)` take a single `int` by value. The only pointers
  in the file (`&the_house` passed to `add_floor` / `add_bedrooms`) are formed
  internally from the address of a `static` object and can never be null;
* no enum type exists anywhere in the API, so there is no "out-of-range enum
  variant" to smuggle across FFI — the only argument type is `int`, for which
  *every* bit pattern is a valid value (row 3 below).

Consequently there is **no error return value to compare**. The
error-equivalent obligation for this library is: for every input that a
hostile caller could consider "invalid" or "out of range", the C does not
reject it — it proceeds and produces output — and the Rust must produce the
**byte-identical output** rather than panicking, aborting, or trapping.

That last point is the real risk here, and it is a *Rust-specific* failure
mode with no C counterpart: `house->bedrooms += extra_bedrooms` and
`house->floors++` are signed `int` arithmetic. In C these wrap in practice
(the C is compiled at `-O0`, no `-ftrapv`, no UBSan). A naive Rust
translation using `+` / `+= 1` would **panic in debug** and **wrap in
release**, i.e. it would diverge. The rows below pin this down.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `run` | `extra_bedrooms = INT_MAX` (`2147483647`) — max in-range value, one step past which `int` cannot represent | No rejection. `bedrooms` becomes `bedrooms + INT_MAX` computed as wrapping 32-bit signed add; 4 lines printed; returns `void`. Rust must NOT panic/abort. |
| 2 | `run` | `extra_bedrooms = INT_MIN` (`-2147483648`) — min in-range value, one step below which `int` cannot represent | No rejection. Wrapping 32-bit signed add; 4 lines printed; `void`. Rust must NOT panic/abort. |
| 3 | `run` | `extra_bedrooms` = an arbitrary out-of-"documented"-range int (e.g. `-1`, and bit patterns like `0x80000000`, `0x7FFFFFFF`, `0xFFFFFFFF` reinterpreted as `int`) — the header documents no valid range, so *every* `int` bit pattern is accepted | No rejection for any bit pattern. Every value is added as-is. `void`. |
| 4 | `run` | signed-overflow **on the `bedrooms` accumulator**: current `bedrooms` already large/positive and `extra_bedrooms > 0` pushes the sum past `INT_MAX` (C: UB; observed: two's-complement wrap to negative) | No rejection, no trap. `bedrooms` wraps to a negative value and `%d` prints it with a `-` sign. Rust must reproduce the wrap, not panic. |
| 5 | `run` | signed-underflow on the `bedrooms` accumulator: `extra_bedrooms < 0` pushes the sum below `INT_MIN` | No rejection, no trap. Wraps to a positive value; `%d` prints it. Rust must wrap. |
| 6 | `run` | overflow of the `floors` counter via `house->floors++` (`add_floor`) when `floors == INT_MAX` | No rejection. Wraps to `INT_MIN`. (Reachable only after `INT_MAX - 2` calls, so not directly drivable through the public API; asserted structurally — Rust uses `wrapping_add(1)`, matching.) |
| 7 | `driver` | `x = INT_MAX` / `INT_MIN` / arbitrary bit pattern — same as rows 1–3 but through the wrapper, which applies the value **twice** (`run(x); run(x);`), so the accumulator overflow of rows 4–5 is reached in a single public call | No rejection. Wrapping adds applied twice; 8 lines printed; `void`. |
| 8 | `run` / `driver` | called repeatedly so the *global* `the_house` state accumulates without bound (`bathrooms += 1.0` each call, `floors++` each call) — there is no reset/teardown entry point, so "already-used state" is an input condition the caller cannot avoid | No rejection and no re-initialisation. State persists across calls and across `run`/`driver` mixing; output reflects the accumulated values. Rust's `static mut THE_HOUSE` must accumulate identically. |
| 9 | `run` / `driver` | `bathrooms` grown large enough that `%.1f` changes width / loses the exact `.5` (after ~2^53 calls the `double` can no longer represent the increment) | No rejection; `printf("%.1f")` formats whatever the `double` holds. Rust passes the same `c_double` to the same `printf`, so identical. (Structural; not reachable in test time.) |

## Checklist (checked only when the differential test passes against BOTH .so's)

- [x] 1 — `errors::row1_run_int_max`
- [x] 2 — `errors::row2_run_int_min`
- [x] 3 — `errors::row3_arbitrary_bit_patterns`
- [x] 4 — `errors::row4_bedrooms_overflow_wraps`
- [x] 5 — `errors::row5_bedrooms_underflow_wraps`
- [x] 6 — `errors::row6_floors_counter_wraps_structural`
- [x] 7 — `errors::row7_driver_applies_value_twice`
- [x] 8 — `errors::row8_no_reset_state_accumulates`
- [x] 9 — `errors::row9_bathrooms_large_magnitude_formatting`

## Notes on how these rows are verified

All rows live in `tests/errors.rs`. Rows 1–5, 7 and 8 are true differential
tests (construct the condition, call BOTH `.so`s, compare bytes and the
resulting accumulator). Rows 6 and 9 are **structural**, and marked as such in
the test bodies, because their triggers need ~2^31 / ~2^53 public calls:

* Row 6 asserts the *mechanism* — the C uses `house->floors++` and the Rust
  must use `wrapping_add(1)`; `saturating_add` / `checked_add` are explicitly
  rejected, since those would diverge from C's wrap.
* Row 9 asserts that both `.so`s embed the **identical** format-string bytes
  and both import libc `printf`, which makes `%.1f` rendering identical by
  construction for every `double` value, including unreachable magnitudes.

Additional generic FFI-boundary tests beyond the table:

| test | covers |
|------|--------|
| `generic_no_pointer_or_enum_in_public_api` | proves mechanically (from the header + source) that the API takes no pointer and no enum, so there is no null-pointer and no out-of-range-enum path to test |
| `generic_one_step_past_int_extremes` | full neighbourhood of `INT_MAX`/`INT_MIN` through both entry points |
| `generic_zero_and_oversized_values` | zero-value idempotence and largest-magnitude values |

### Why the debug cdylib matters here

These tests run against the **debug** Rust cdylib, which has
`overflow-checks = on`. A translation using `+`/`+= ` instead of
`wrapping_add` would abort on rows 1, 2, 4, 5 and 7 and be caught. The suite is
then re-run against the **release** cdylib (`panic = "abort"`, overflow checks
off) via `RUST_DRIVER_SO`, so both arithmetic configurations are covered.

### UB sensitivity

`bedrooms += extra` and `floors++` are signed-overflow UB in C. The suite is run
against the C reference compiled at `-O0`, `-O2` and `-O3`; the Rust matches all
three, so the wrapping behaviour the translation reproduces is not an artifact
of one optimisation level.
