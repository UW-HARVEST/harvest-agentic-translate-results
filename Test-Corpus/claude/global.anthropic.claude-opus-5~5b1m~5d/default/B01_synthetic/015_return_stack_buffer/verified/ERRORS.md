# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

Both binaries were built and compared as subprocesses (stdout, stderr, exit
status) over every input class the C program branches on. Test suite:
`translation/tests/differential.rs`.

* C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
* Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`

## Input classes enumerated from the C source

`main()` does exactly one `scanf("%d", &x)` and then branches on `x`:

| # | Input class | C behavior | Reached by |
|---|---|---|---|
| 1 | Non-zero integer (`1`, `-5`, `+7`) | `good()` → `helperGood1 string\n`, exit 0 | `single_nonzero_takes_good_branch` |
| 2 | Zero (`0`, `-0`, `+0`, `0000`) | `bad()` → prints nothing, exit 0 | `single_zero_takes_bad_branch` |
| 3 | Empty stdin (`scanf` returns `EOF`, `x` untouched) | `bad()`, exit 0 | `empty_input_leaves_x_zero_and_takes_bad_branch` |
| 4 | Whitespace only (input failure after skipping space) | `bad()`, exit 0 | `whitespace_only_input_is_an_input_failure` |
| 5 | Matching failure (`abc`, `-`, `+`, `--7`, `.5`, `e5`) — `x` stays 0 | `bad()`, exit 0 | `non_numeric_input_is_a_matching_failure` |
| 6 | NUL / non-ASCII bytes (`\0`, `\x80`, `\xff`) | matching failure → `bad()` | `non_ascii_and_nul_bytes_are_matching_failures` |
| 7 | Leading whitespace incl. newlines before the number (`scanf` reads *across* newlines, unlike `fgets`) | value is converted | `scanf_skips_leading_whitespace_including_newlines` |
| 8 | Digits followed by trailing junk (`1abc`, `3.7`, `0x10`) — conversion stops at first non-digit, rest of stdin never read | value of the leading digits | `scanf_stops_at_first_non_digit_and_ignores_the_rest` |
| 9 | `INT_MAX`/`INT_MIN` ± 1 | truncation to `int` | `int_boundaries_round_trip` |
| 10 | Values whose low 32 bits are zero (`2^32`, `2^33`, `2^64`) | truncate to `x == 0` → `bad()` | `values_whose_low_32_bits_are_zero_take_the_bad_branch` |
| 11 | Overflowing conversions (`>LONG_MAX`, `<LONG_MIN`) | glibc clamps to `LONG_MAX`/`LONG_MIN`, then truncates: positive → `-1` (`good()`), negative → `0` (`bad()`) | `overflowing_conversions_clamp_like_glibc` |
| 12 | Maximum-ish inputs: 400-digit runs, 1 MiB of digits (crosses stdio buffer boundaries) | same as class 11 / 10 | `very_long_digit_runs`, `input_much_larger_than_a_stdio_buffer` |
| 13 | Extra `argv` entries (`main()` takes none) | ignored | `extra_command_line_arguments_are_ignored` |
| 14 | stdout redirected to a file instead of a pipe (C stdio buffering mode changes) | identical bytes | `output_is_identical_when_stdout_is_a_file_rather_than_a_pipe` |
| 15 | Full cross product of {4 whitespace prefixes} × {4 sign prefixes} × {8 bodies} × {3 tails} = 384 inputs | — | `cross_product_of_whitespace_sign_and_body` |

No input produces output on stderr, and both programs always exit 0: there is
no error path in the C that exits non-zero. `printLine`'s `line != NULL` guard
is the only "error" check, and it is reached via class 2/3/4/5/6/10 (see below).

## Mismatches found

**None.** Every input class above produced byte-identical stdout, byte-identical
stderr (always empty) and identical exit status (always 0). A randomized
differential fuzz of 436 additional inputs (random mixes of digits, signs,
whitespace, letters, `\0` and `\xff`, plus 2^n boundary values) also produced
zero mismatches.

## Behaviors that had to be replicated exactly (potential mismatch sources, all verified)

These are the places where a "reasonable" translation would have diverged.
They are already handled correctly in `translation/src/main.rs`; each was
confirmed against the compiled C rather than assumed.

1. **`helperBad()` returns the address of a function-local array (CWE-562).**
   The C is undefined behavior, so the *compiled* reference is the ground truth.
   Disassembly of the CMake-default build shows GCC folding the dangling return
   into a constant zero:

   ```
   0000000000401158 <helperBad>:
     ...
     mov    $0x0,%eax      ; returns NULL
     ret
   ```

   Therefore `printLine`'s `if (line != NULL)` is false and the `bad()` path
   prints **nothing at all** — no text, no newline. Verified deterministic over
   repeated runs (`xxd` of the output is empty every time). The Rust
   `helper_bad()` returns `None` to model this, which is why `bad()` is silent.
   A naive translation that returned `"helperBad string"` would print an extra
   line here and mismatch on every zero/EOF/invalid input.

2. **`scanf` leaves `x` untouched on matching or input failure.** `x` is
   initialized to `0` before the call and the return value is discarded, so any
   unparsable input silently takes the `bad()` branch. Rust's `scanf_d` mirrors
   this by only writing through `&mut x` on a successful conversion.

3. **glibc's overflow clamping, then truncation to `int`.** `%d` accumulates at
   `long` width and saturates at `LONG_MAX`/`LONG_MIN` before the store to
   `int`. This makes the sign of a giant number observable:
   `99999999999999999999` → `LONG_MAX` → `0xFFFFFFFF` → `-1` → `good()`, while
   `-99999999999999999999` → `LONG_MIN` → `0x00000000` → `0` → `bad()`. A
   `str::parse::<i32>()`-based translation would have failed both, and a
   wrapping-multiply translation would have gotten the wrong low 32 bits.

4. **`scanf` skips *all* whitespace, including newlines, and stops at the first
   non-digit** (contrast with `fgets`, which stops at a newline). Only the first
   token is ever consumed; the remainder of stdin is never read, so a second
   input of `0\n1\n` converts only the `0`, takes the `bad()` branch and never
   sees the `1`. Covered explicitly by `zero-then-nonzero-line`.

5. **Exit status and stream contents.** `main` always `return 0`, and nothing is
   ever written to stderr. `translation` ends with an explicit
   `stdout().flush()` before `exit(0)` so buffered output is not lost — without
   the flush, `std::process::exit` could drop the line and mismatch stdout on
   the `good()` path.

## Result

Phase D gate: both programs build cleanly, `cargo test` passes (16 tests,
~2000 individual input comparisons, none `#[ignore]`d or skipped), and
`c_src/` is unmodified.
