# Differential verification notes

Comparison basis: both programs are built and run as subprocesses over identical
stdin bytes, and `stdout`, `stderr` and the exit status are compared byte for
byte. See `tests/differential.rs`.

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`

## Mismatches found

None. Every input class enumerated below produced identical stdout, identical
stderr (always empty) and identical exit status (always 0).

Coverage was established in three passes: hand-enumerated cases from reading
`c_src/src/main.c`, then a shell harness over ~100 named inputs, then randomized
sweeps (8000 byte-level cases over an alphabet including NUL, `0xff`, `0x80`,
control characters and ASCII punctuation; plus 800 structured cases over signed
digit runs of 1–30 digits with varied separators). No case diverged.

## Behaviors that had to be reproduced deliberately

These are the places where a naive translation would have diverged. They are
already handled correctly in `src/main.rs`; each has a test.

1. **`y` is a static global that `scanf` writes into.** `main` calls
   `multi_stage(x, z)` with only two arguments and `multi_stage` reads `y` from
   the global. If the second `%d` conversion never happens, `y` keeps its
   initializer `123`, so `1` alone reaches the `y != 2` branch rather than
   behaving as if `y` were `0`. Modeled as a `static AtomicI32` seeded to 123.
   Test: `single_item_only`, `stage_y_not_two`.

2. **`scanf`'s return value is discarded, and partial assignment is visible.**
   With fewer than three convertible fields, the later variables retain their
   initializers (`x = 0`, `z = 0`, `y = 123`) and the program still runs to
   completion printing a `Result:` line. There is no error path for short input.
   Tests: `no_input_at_all`, `whitespace_only_input`, `single_item_only`,
   `two_items_only`.

3. **`%d` skips whitespace including newlines.** `scanf` does not stop at a line
   boundary the way `fgets` would, so `"1\n2\n3"`, `"1\t2\t3"` and `"1 2 3"` are
   all the same input. Leading whitespace of arbitrary length is skipped.
   Test: `scanf_reads_across_newlines`.

4. **Overflow saturates before it truncates.** glibc's `%d` collects the digit
   run, converts with `strtol` semantics (saturating at `LONG_MAX` / `LONG_MIN`),
   then stores through an `int *`, truncating. The two steps are distinguishable:
   `18446744073709551617` is `2^64 + 1`, so a wrap-around implementation would
   yield `x == 1` and print `Ok!`, while the C prints `Error: x != 1`. The
   translation accumulates in `i64`, latches an overflow flag, substitutes
   `i64::MIN`/`i64::MAX`, and only then casts to `i32`.
   Test: `overflow_saturates_then_truncates`.

5. **Truncation below the saturation threshold is plain modular truncation.**
   `4294967297 4294967298 4294967299` truncates to `1 2 3` and reaches the
   `Ok!` path. Test: `truncation_past_int_range`.

6. **The digit run is unbounded in length.** A 100,000-digit field must not
   overrun any fixed scratch buffer; the translation never buffers the digits,
   it folds them as they arrive. Test: `overflow_saturates_then_truncates`.

7. **A matching failure pushes the offending byte back and stops assignment.**
   `"1abc"` assigns `x = 1` then fails, leaving `y = 123` and `z = 0`. A lone
   `-` or `+` with nothing usable after it is also a failure with no assignment.
   Tests: `matching_failure_on_*`, `digits_followed_immediately_by_letters`,
   `sign_at_end_of_input`.

8. **Error checks are ordered and every failure prints two lines.** The `goto
   fail` target means each failing branch prints its own message *and*
   `Operation failed`, while the success path prints only `Ok!` and returns
   before the label. `x` is checked before `y`, and `y` before `z`, so
   `"0 0 0"` reports only `x != 1`. Tests: `stage_*`,
   `reference_outputs_are_pinned`.

9. **Input is bytes, not UTF-8.** NUL bytes, lone `0xff`, and multibyte UTF-8
   are all just non-digit bytes that terminate a conversion. The translation
   reads `u8` and never decodes. Test: `non_utf8_and_nul_bytes`.

10. **Exit status is always 0.** `main` returns `0` unconditionally; `result` is
    only ever reported through the `Result:` line on stdout. A test asserting
    only stdout would still pass if Rust exited non-zero, so status is asserted
    on every case. Test: `stderr_always_empty_and_status_always_zero`.

## Differences checked and confirmed not observable

- **Locale.** Run under `C.UTF-8`, `en_US.UTF-8`, `de_DE.UTF-8` and
  `tr_TR.UTF-8`; `%d` accepts no digit grouping and only ASCII digits, so
  output is locale-independent in both.
- **Failing stdout.** With stdout redirected to `/dev/full`, closed (`>&-`), or
  a pipe whose reader exits, both programs exit 0 and write nothing to stderr.
  C's `printf` ignores write errors and `exit`'s flush failure does not change
  the status; the translation ignores its write and flush results to match.
- **Bytes left unread on stdin.** C's stdio reads a full block, so it consumes
  more of stdin than it parses; the translation reads byte at a time with one
  byte of pushback and consumes only what it needs. This cannot be seen through
  stdout, stderr or exit status, which is the comparison basis here, and it is
  not observable at all when stdin is a regular file or a fully-written pipe.
  Left as is rather than adding block buffering, which would change nothing
  measurable.

## One test defect fixed during this work

`reference_outputs_are_pinned` initially compared escaped output (`show(...)`,
which renders a newline as the two characters `\n`) against expected strings
containing real newlines, so it failed on correct output. Both sides now go
through the same escaping, and the raw bytes are asserted as well. This was a
bug in the test, not in the translation — no production behavior changed.
