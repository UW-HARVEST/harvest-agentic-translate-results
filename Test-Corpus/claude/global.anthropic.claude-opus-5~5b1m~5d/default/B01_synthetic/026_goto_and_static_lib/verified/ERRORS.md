# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection/error construct in
`c_src/src/driver.c`. Grep basis:

```sh
grep -n 'return\|goto\|assert\|Error\|failed\|if (' c_src/src/driver.c
```

Findings — the C code contains:

* 3 explicit range/equality rejection checks (`x != 1`, `y != 2`, `z != 3`),
  each with its own message, its own `result` code and its own `goto fail`;
* 1 shared failure epilogue (`fail:` → `"Operation failed\n"`);
* 0 `assert`s, 0 null-pointer checks, 0 allocation checks, 0 error enums,
  0 min/max constants.

`driver` returns `void`, so the *only* observable result is the exact byte
sequence written to `stdout` (the `Result: %d` line carries the internal
status code out to the caller). Every row therefore states the full expected
stdout.

`y` is a file-scope `static` that `driver` overwrites with `local_y` on every
call, so the `y != 2` branch is driven purely by the 2nd argument.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `driver` → `multi_stage` | `x != 1` (any `y`, any `z`) — first check fails, `result = 1`, `goto fail` | stdout: `Error: x != 1\nOperation failed\nResult: 1\n` |
| E2 | `driver` → `multi_stage` | `x == 1` **and** `y != 2` (any `z`) — second check fails, `result = 2`, `goto fail` | stdout: `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n` |
| E3 | `driver` → `multi_stage` | `x == 1` **and** `y == 2` **and** `z != 3` — third check fails, `result = 3`, `goto fail` | stdout: `Error: x == 1 and y == 2, but z != 3\nOperation failed\nResult: 3\n` |
| E4 | `driver` → `multi_stage` (`fail:` epilogue) | any of E1/E2/E3 — the shared `fail:` label must print `Operation failed` **in addition to** the per-check message, and must **not** be reached on success | E1/E2/E3 stdout above; success path never contains `Operation failed` |
| E5 | `driver` → `multi_stage` | check ORDER / short-circuit: `x != 1` **and** `y != 2` **and** `z != 3` all invalid simultaneously — only the *first* failing check may report | stdout: `Error: x != 1\nOperation failed\nResult: 1\n` (E2/E3 messages absent) |
| E6 | `driver` → `multi_stage` | check ORDER: `x == 1`, `y != 2`, `z != 3` — the `y` check must win over the `z` check | stdout: `Error: x == 1 but y != 2\nOperation failed\nResult: 2\n` |

## Generic FFI boundary cases (covered even though the C has no such checks)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| B1 | `driver` | extreme sentinel arguments `INT_MIN`, `INT_MAX` in each of the 3 positions | classified by E1/E2/E3; no overflow/UB, no wrap-around into the valid values (`1`,`2`,`3`) |
| B2 | `driver` | `0` and `-1` in each position (the classic "error/absent" sentinels) | classified by E1/E2/E3 |
| B3 | `driver` | one step past each valid value: `x ∈ {0,2}`, `y ∈ {1,3}`, `z ∈ {2,4}` | E1 / E2 / E3 respectively — off-by-one must not be accepted |
| B4 | `driver` | out-of-range "enum-like" ints: the C signature is `int`, so *every* 32-bit value is a legal input, including values with no meaning to the checks (e.g. `123` — the initial value of the `static y`) | classified by E1/E2/E3; in particular `driver(1, 123, 3)` must take E2, not the success path |
| B5 | `driver` | no pointer parameters exist, so there is no null-pointer row; passing values that alias the internal `static y` initialiser (`123`) is the closest analogue and is covered by B4 | — |
| B6 | `driver` | repeated / interleaved calls — the `static y` persists between calls, so a success call after a failing call (and vice versa) must not leak state | each call independently classified by E1/E2/E3; `y` is always re-assigned from `local_y` first |

## Checklist

- [x] E1 — `tests/error_paths.rs::e1_x_not_one`
- [x] E2 — `tests/error_paths.rs::e2_y_not_two`
- [x] E3 — `tests/error_paths.rs::e3_z_not_three`
- [x] E4 — `tests/error_paths.rs::e4_fail_epilogue_only_on_error`
- [x] E5 — `tests/error_paths.rs::e5_first_check_wins_all_invalid`
- [x] E6 — `tests/error_paths.rs::e6_y_check_beats_z_check`
- [x] B1 — `tests/error_paths.rs::b1_extreme_sentinels`
- [x] B2 — `tests/error_paths.rs::b2_zero_and_minus_one`
- [x] B3 — `tests/error_paths.rs::b3_one_step_past_valid`
- [x] B4 — `tests/error_paths.rs::b4_out_of_range_enum_like_ints`
- [x] B5 — n/a (no pointer parameters); analogue covered by B4
- [x] B6 — `tests/error_paths.rs::b6_repeated_and_interleaved_calls`
