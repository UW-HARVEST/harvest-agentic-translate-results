# Differential verification of the C-to-Rust translation

Ground truth: `c_src/src/main.c` + `c_src/src/lib.c`, built with CMake.
Under test: the `driver` binary produced by this crate.

Both programs are run as subprocesses with the same bytes on stdin, and
stdout, stderr and the exit status are compared byte for byte.

## Outcome

**No behavioural mismatch was found.** Across roughly 145,000 distinct inputs
the Rust binary produced byte-identical stdout, byte-identical stderr and the
same exit status as the C binary in every case.

The only failure encountered during this work was a defect in the *test*
corpus, not in the translation. It is recorded below for completeness, because
a gap in the corpus is exactly the kind of thing that hides a real mismatch.

| # | Where | What went wrong | Cause | Fix |
|---|-------|-----------------|-------|-----|
| 1 | `tests/golden_and_random.rs` | `golden_table_spans_the_expected_result_values` failed: the golden table never reached `validate_sequence`'s "few transitions" result of `20`. | The `len <= 10` band returns 20 only when `transitions < len / 3`. With runs capped at three by rule 3, and with rule 1 forcing a leading true and rule 2 a trailing false, the *only* string that satisfies this is the six-character `yyynnn`. No hand-picked case happened to be that string. | Added `3\n0\nyyynnn\n` (expected `20`) to the golden table. Both programs already agreed on it; the assertion was what was wrong. |

## What was checked

Build: `cmake -S c_src -B <build> && cmake --build <build>`, then
`cargo build --release` in `translation/`. Both are warning-free. The C was
additionally rebuilt at `-O0`, `-O2` and `-O3` and cross-compared against
itself, because `validate_sequence` casts a `char*` to `bool*` (a strict
aliasing violation, so in principle optimisation-sensitive). All three
optimisation levels agree with each other and with the Rust.

Case classes, all compared on stdout + stderr + exit status:

- **Read-failure paths.** Each of the three `fgets` calls failing in turn, on
  empty input, on `/dev/null`, and on input truncated at every line boundary
  and mid-line. Each produces its own stderr message and exit status 1.
- **Exhaustive `y`/`n` patterns.** Every string of length 1–13 for operations
  2 and 3, every string of length 1–6 for operations 0 and 1 crossed with
  parameters 0–4. (The suite itself sweeps 1–12; length 13 was swept out of
  band and adds no new branch.)
- **Operation dispatch.** Operations 0–3 plus a wide spread of unhandled
  values reaching the `default` arm's `-3`.
- **Length guards.** Empty decision string (`-1`), and one- and
  two-character strings for the operations that demand three (`-2`).
- **The 32-decision cap** in operation 2, at lengths 28–34, 40, 63–65, 100 and
  200, with all-true, all-false, single-true, single-false, alternating and
  run-structured patterns at each length.
- **`fgets` buffer boundary.** Lines of 1021–1026, 2047–2049, 3000, 4096,
  8192, 16384 and 65536 bytes in each of the three input positions, so the
  1023-byte truncation and the carry-over of the unread tail into the next
  read are both exercised.
- **`atoi` conversion.** Signs, leading whitespace, leading zeros, no digits,
  trailing junk, the `int` boundaries, values that only differ after
  truncation to `int`, the `long` boundaries, and values far beyond `long`.
- **Non-`y`/`n` bytes.** Upper case, embedded NULs at every position, bytes
  above 127, tabs, `?`, digits, `\r`.
- **Exotic invocation shapes.** stdout closed early (EPIPE), stdout on
  `/dev/full`, merged streams, extra `argv` entries, and a here-string.
- **Randomised sweeps.** Deterministically seeded structured inputs and
  arbitrary byte streams, both inside the suite (11,000 cases) and out of
  band (~25,000 cases).

## C behaviour that is easy to get wrong, and how the Rust handles it

These are the places a translation would plausibly diverge. Each was
confirmed to match rather than assumed to.

- **`fgets`, not `scanf`.** Reading stops at the newline and *keeps* it, so
  the newline is part of the buffer and `main` strips it explicitly. A
  `scanf`-style reader would skip leading whitespace and read across
  newlines, changing which line lands in which variable. `main.rs` implements
  `fgets` directly, byte at a time, keeping the newline.
- **The 1023-byte truncation.** `fgets(buf, 1024, ...)` stores at most 1023
  bytes; an over-long line is cut and the remainder stays on stdin for the
  *next* `fgets`. So a 2000-character first line makes the second `fgets`
  return the tail of it rather than the second line of the file.
- **The single reused buffer.** `main` reads all three lines into the same
  `char[1024]`. Leftover bytes from an earlier, longer line are still
  physically present but invisible, because `fgets` writes a NUL after what
  it just read and `strlen` stops there. `main.rs` reuses one array the same
  way rather than allocating a fresh string per line.
- **`atoi` is `(int) strtol(s, NULL, 10)`.** Out-of-range input saturates at
  `LONG_MAX`/`LONG_MIN` *first* and is *then* truncated to `int`. So
  `9223372036854775808` becomes `-1`, not `INT_MAX` and not `0`. And plain
  truncation means `4294967296` selects operation 0 while `4294967297`
  selects operation 1. Rust's `str::parse` would reject all of these and
  return an error instead.
- **Empty decision line exits 0, not 1.** A blank third line is a successful
  read; the newline is stripped, `len` becomes 0, and `process_decisions`
  returns `-1`, which is printed to stdout with exit status 0. This is
  distinct from the read-failure paths, which print to stderr and exit 1.
- **Signed `char`.** In C `char` is signed on x86-64, so a byte such as `0xf9`
  is negative. It still fails every `== 'y'` / `== 'n'` comparison, and
  glibc's `strtol` classifies whitespace through `unsigned char`, so treating
  the bytes as `u8` in Rust gives identical results.
- **`int` versus `size_t` comparisons.** `configure_flags` compares an `int`
  counter against `size_t` counts (`special_count == count`,
  `special_count == count - 1`), which in C converts the `int` to unsigned.
  The counter is never negative and `count` is never 0 here (a zero length
  returns `-1` earlier), so the conversion is benign; the Rust casts
  explicitly to `usize` to keep it that way. Verified under the debug profile
  too, where Rust's overflow checks are enabled and would panic on a `count - 1`
  underflow.
- **The aliasing cast in `validate_sequence`.** `bool *bools = (bool*)sequence;`
  overwrites the caller's buffer in place. Because index `i` is read before it
  is written and no earlier index is read again, this is equivalent to
  building a separate boolean array, and `main` never looks at the buffer
  afterwards. The Rust builds a separate `Vec<bool>`, which is observationally
  identical.
- **`printf("%d\n", ...)`.** A single decimal integer and one trailing
  newline, with no padding.

## C branches that no input can reach

Confirmed by exhaustive enumeration; the Rust reproduces the code shape
anyway, so behaviour cannot drift if the surrounding logic changes.

- `apply_permissions`: the `read && write` arm's fall-through. Entering that
  arm implies `!execute`, hence `permission_value == 6`, so the inner `if`
  always holds and the arm always returns 56.
- `evaluate_conditions` XOR arm, `return 90`. A true XOR of three booleans
  means an odd number are true, which is either exactly one (caught by the
  three preceding checks) or all three (caught by the fourth).
- `evaluate_conditions` NAND arm, `return 100`. A true NAND means at least one
  condition is false, so one of `200`/`150`/`151`/`152` always fires first.
- `validate_sequence` long band, `return 40`. Reaching the band needs
  `len >= 11`; surviving rule 3 caps every run at three, which forces at least
  `ceil(len / 3) - 1 >= 3` transitions, so `transitions < 3` is impossible.

## Reproducing

```sh
cd translation && cargo test              # debug profile: overflow checks on
cd translation && cargo test --release    # release profile: the graded build
```

The suite builds the C program itself, into `translation/target/c_build`, so
nothing is written inside `c_src/`. 27 tests, none ignored, skipped or
disabled.
