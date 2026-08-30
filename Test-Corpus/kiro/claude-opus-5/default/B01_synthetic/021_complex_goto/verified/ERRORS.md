# Differential findings: `c_src/src/main.c` vs `translation/`

Method: build both programs, feed identical bytes on stdin, compare stdout,
stderr and exit status. Tests live in `tests/differential.rs` and drive the
built binaries as subprocesses; the Rust code is never loaded as a library.

- C program: `c_src/build/driver`
  (`cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`)
- Rust program: `translation/target/release/driver` (`cargo build --release`)

## Mismatches found

### 1. `scanf` whitespace skipping did not include the vertical tab

**Input:** `"\x0b5 6"` (vertical tab, then `5 6`)

| | stdout | exit |
|---|---|---|
| C | `loop x y loop x y loop x y x y x y y` (14 lines) | 0 |
| Rust (before fix) | *(empty)* | 0 |

**Cause.** `%d` skips leading whitespace as defined by C's `isspace`, which in
the "C" locale is space, `\t`, `\n`, `\v`, `\f`, `\r`. The translation used
`u8::is_ascii_whitespace`, and that method deliberately omits the vertical tab
(0x0B). So C skipped the `\v` and read `x = 5, y = 6`, while Rust treated `\v`
as a matching failure, left both variables at their initialisers of 0, and
printed nothing.

Only stdout differed; stderr and the exit status matched, so a stdout-only
assertion would still have caught this one — but the class of bug is exactly the
one that hides behind a partial comparison.

**Fix.** Added an explicit `is_c_space` helper in `src/main.rs` covering all six
C whitespace bytes and used it for the leading-whitespace skip in
`Scanner::scan_i32`.

## Behaviours deliberately replicated, not "fixed"

None of these produced a mismatch; they are recorded because they look like bugs
and must not be cleaned up.

- **`foo` never terminates when `x > 0 && y < 0`.** `y != 0` keeps the `y`
  branch alive and `x < 3` keeps jumping back to `label1` even after `x`
  reaches 0, so the loop prints forever. Both programs hang identically. These
  inputs are compared on a bounded 64 KiB prefix of stdout
  (`non_terminating_inputs`, `integer_truncation_non_terminating`,
  `extreme_magnitudes_bounded`) rather than to completion.
- **`goto label2` skips the `x` decrement** on the single input `x == 1 &&
  y == 4`, and `label1` is otherwise reached by fall-through. Reproduced with an
  explicit state machine whose states are the jump targets.
- **Out-of-range integers are converted through `long` and then truncated to
  `int`**, matching glibc's `strtol`-based `%d`. `2147483648` becomes `INT_MIN`,
  `4294967296` becomes `0`, anything past `LONG_MAX` saturates to `LONG_MAX` and
  truncates to `-1`, anything below `LONG_MIN` truncates to `0`. The Rust
  scanner accumulates in `i64` with saturation and casts to `i32`.
- **A failed conversion leaves its variable untouched**, so `x` and `y` keep the
  `= 0` initialisers from `main`. The second `%d` is only attempted if the first
  one succeeded.
- **Trailing input is never read.** `scanf` stops after two conversions, so
  `"5 6 7 8"` behaves like `"5 6"`.
- **Neither program writes to stderr or returns anything but 0** on any input
  tested.

## Input classes covered

Empty input, whitespace-only input (each of the six whitespace bytes), a single
item, no valid conversion at all, sign-without-digits, both values zero (loop
never entered), `x == 1 && y == 4` (the `goto label2` edge), `y == 0` (the
`continue` edge), `x <= 0` with `y > 0` (the `label1` body never taken), both
sides of the `x < 3` back edge, an exhaustive sweep of every terminating
`(x, y)` pair in `-4..=14`, the non-terminating class, `INT_MAX`/`INT_MIN` and
`LONG_MAX`/`LONG_MIN` boundaries, leading zeros and signs, partial conversions
(`0x5`, `5e3`, `5.9`, `5,6`), unread trailing input, non-UTF-8 and NUL bytes,
and closed stdin.
