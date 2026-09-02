# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

## Result

**No behavioural mismatch was found.** Across every input class enumerated
below — 3,000+ distinct stdin byte strings, including a dense `x`/`y` grid,
`scanf` failure paths, integer overflow/truncation inputs, and 980 randomised
fuzz cases — the Rust binary produced byte-identical stdout, byte-identical
stderr (always empty), and the same exit status (always 0) as the C binary.

This file therefore records what was checked, the C semantics each check pins
down, and the two input classes that cannot be compared to completion, rather
than a list of fixes. Nothing in `translation/src/` needed to change.

## How it was verified

* C reference: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver` (gcc 11.5.0, glibc).
* Rust: `cd translation && cargo build --release` → `translation/target/release/driver`.
* Both are driven **as subprocesses** by `translation/tests/`; the Rust crate is
  never loaded as a library. `tests/harness/mod.rs` builds the C reference with
  CMake on demand, writes the input to the child's stdin, and compares stdout,
  stderr and `(exit code, terminating signal)`.
* `translation/tests/differential.rs` — the enumerated input classes.
* `translation/tests/fuzz_differential.rs` — deterministic randomised inputs
  (fixed-seed LCG, no dev-dependencies).

### Coverage evidence

A `--coverage` build of an out-of-tree **copy** of `main.c` (`c_src/` itself was
never modified or rebuilt with instrumentation) driven by only the inputs the
test suite uses reports:

```
File 'main.c'
Lines executed:100.00% of 22
Branches executed:100.00% of 14
Taken at least once:100.00% of 14
Calls executed:100.00% of 5
```

Every conditional in `foo` is taken in both directions, including the
single-cell `goto label2` case, which gcov shows executing exactly once (from
the input `1 4`).

## C behaviours that had to be replicated exactly

Each of these is a place a naive translation diverges. All were checked
against the C binary and all match.

1. **`scanf` ignores its return value.** `main` never checks whether the two
   conversions succeeded, so a failed conversion leaves the local at its `0`
   initialiser rather than producing an error. `abc`, `-`, `+`, `- 5`, `.`,
   `0x10`, empty input and whitespace-only input therefore all print **nothing**
   and exit **0**. Verified: `first_field_not_a_number`, `sign_with_no_digits`,
   `whitespace_only_input`, `empty_input`.

2. **A failed first conversion suppresses the second.** `scanf` stops at the
   first failing directive, so `y` is never even attempted. Both fields stay 0.

3. **`%d` skips leading whitespace and reads across newlines.** `3\n2`,
   `5\r\n3`, `\x0b5\x0c3` and `\t\t3\t\t2\t\t` all convert two fields — the
   whitespace set is `isspace`'s, i.e. space, `\t`, `\n`, `\v`, `\f`, `\r`. The
   Rust `Scanner::is_space` matches exactly this set. Verified:
   `scanf_reads_across_newlines_and_odd_whitespace`.

4. **The literal space in `"%d %d"` may match an empty whitespace run.** `0-3`
   converts `x = 0, y = -3`; there is no requirement that a separator be
   present. Verified: `explicit_signs_and_leading_zeros`.

5. **`%d` is base 10 and stops at the first non-digit.** `0x10` converts `0`
   and then fails on `x`; `5e3 2` converts `5` and then fails on `e`, so `y`
   stays 0 (not 2). Verified: `hex_and_exponent_are_not_special_for_percent_d`.

6. **A sign with no digit behind it is a matching failure, not a zero.**
   Verified: `sign_with_no_digits`.

7. **Overflow: glibc converts at `long` width, saturates at
   `LONG_MAX`/`LONG_MIN`, then truncates to `int`.** This is the subtlest
   behaviour in the program and it is directly observable:
   * `2147483648 0` → `(int)2^31` = `-2147483648` → loop never entered, no output.
   * `4294967296 4294967296` → both truncate to `0` → no output.
   * `-4294967295 4294967296` → `x = 1`, `y = 0` → prints exactly `loop\nx\n`.
     This case distinguishes **truncation** from clamping to `INT_MAX`/`INT_MIN`;
     a clamping implementation prints gigabytes here.
   * `99999999999999999999999 0` → `LONG_MAX` → `(int)` = `-1` → no output.
     A version that saturated at `INT_MAX` would instead run ~2^31 iterations.
   * `-99999999999999999999999 7` → `LONG_MIN` → `(int)` = `0`, `y = 7`.

   `Scanner::scan_i32` reproduces this with a checked `i64` accumulator that
   saturates to `i64::MIN`/`i64::MAX` and then narrows with `as i32`. Verified:
   `values_above_int_range_are_truncated_to_int`,
   `values_beyond_long_range_saturate_then_truncate`, `fuzz_long_digit_runs`.

8. **`goto` semantics inside the `while` body.** `label1` and `label2` are both
   inside the loop body, so:
   * `goto label2` (only when `x == 1 && y == 4`) skips the `label1` block *for
     that entry only*;
   * `goto label1` re-enters the body **without re-testing `x > 0 || y > 0`**,
     so the loop condition can be false while the body keeps running;
   * `continue` *does* jump back to the condition test.

   The Rust version encodes the label region as an inner `loop` with an
   `entry_at_label1` flag, and uses `continue 'while_loop` for C's `continue`.
   Getting the `goto label1` target wrong (re-testing the condition) changes the
   output for e.g. `0 7` and `-4 3`; getting `continue` wrong changes it for
   `5 0`. Both directions are covered by `dense_grid_matches`,
   `y_only_loops_on_goto_label1`, `x_only_uses_the_continue_path`.

9. **The `x < 3` boundary.** The test happens *after* `x--`, so `4 1` takes the
   fall-through and `3 1` takes the `goto`. Verified:
   `x_less_than_three_boundary`.

10. **`printf` output is exactly `"loop\n"`, `"x\n"`, `"y\n"`** with no
    additional trailing newline and no stderr output at all, and `main` always
    returns 0. Compared byte for byte on every input.

## Two input classes that cannot be run to completion

These are properties of the C program, not translation defects. They are still
tested, by comparing the observable output **prefix** (4 MiB) rather than the
final exit status.

* **`x > 0` with `y < 0`** (`1 -1`, `5 -2`, `3 -100`, `2 -2147483648`).
  Once `x` reaches 0, the body reaches `if (y == 0) continue;` with `y`
  negative, so `y--` runs forever until `y` wraps through `INT_MIN` — signed
  overflow, i.e. **undefined behaviour** in C. gcc at the default optimisation
  level wraps two's-complement, so the program emits roughly 2^32 lines of
  `y\n` (~150 MB/s of output here) before terminating. The Rust translation
  uses `wrapping_sub`, which reproduces the same wrapping sequence; the 4 MiB
  prefixes are identical. Test: `negative_y_with_positive_x_prefix_matches`.

* **`x` near `INT_MAX`** (`2147483647 0`, `2147483647 2147483647`): ~2^31
  iterations, ~15 GB of stdout. Prefixes are identical. Test:
  `int_max_x_prefix_matches`.

Note the asymmetry the C code creates and the Rust preserves: `x--` is guarded
by `if (x > 0)` so `x` can never underflow, while `y--` is guarded only by
`y != 0`, so a negative `y` is unbounded.

## Phase D checklist

| Gate | Status |
| --- | --- |
| Both programs build with no errors | yes — `cmake --build .` and `cargo build --release` |
| Identical stdout / stderr / exit status on every enumerated input | yes |
| `cargo test` passes in `translation/` | yes — 25 tests, debug and `--release` |
| No test disabled, skipped or `#[ignore]`d | yes — no `#[ignore]`, no early `return`s |
| `c_src/` unmodified | yes — only the generated `c_src/build/` directory was added |
