# Differential verification of the C → Rust translation

Ground truth: `c_src/src/main.c`, built with CMake (`c_src/build/driver`).
Under test: `translation/src/main.rs`, built with Cargo
(`translation/target/release/driver`).

Comparison method: both programs are spawned as subprocesses with identical
bytes on stdin, and **stdout, stderr and exit status** are compared byte for
byte. See `translation/tests/differential.rs`. The Rust code is never loaded as
a library.

Run commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test                                           # differential suite
```

## Mismatches found

**None.** Every input class enumerated below, plus a 4000-case randomized fuzz
sweep over a byte alphabet biased toward digits, signs, whitespace, NULs and
non-UTF-8 bytes, produced byte-identical stdout, byte-identical stderr (always
empty) and exit status 0 from both programs. No change to
`translation/src/main.rs` was required.

Because there were no mismatches to fix, the sections below record what was
*checked* — in particular the C behaviors that a translation could plausibly get
wrong, and the evidence that the Rust code reproduces each one. These are the
places a future regression would show up first.

## Branch enumeration from the C source

`main` has exactly one branch (`parse_val` succeeded or not); all the remaining
input sensitivity lives in `fgets` and in glibc `strtol`.

| # | C construct | Condition | Input class | Test |
|---|---|---|---|---|
| 1 | `fgets` returns NULL | immediate EOF, `in` stays `""` | empty stdin | `empty_input_takes_the_error_path` |
| 2 | `parse_val` → `endp != str` | false: no digits converted | `""`, `"\n"`, `"abc"`, `"+"`, `"-"`, `"- 5"`, `".5"`, `"!!!"` | `no_conversion_takes_the_error_path` |
| 3 | `parse_val` → `errno == 0` | false: `ERANGE`, outside `long` | `LONG_MAX+1`, `LONG_MIN-1`, 20+ digit numbers | `strtol_erange_takes_the_error_path` |
| 4 | `parse_val` → `tmp >= INT_MIN` | false: fits `long`, below `int` | `-2147483649`, `LONG_MIN` | `value_outside_int_range_takes_the_error_path` |
| 5 | `parse_val` → `tmp <= INT_MAX` | false: fits `long`, above `int` | `2147483648`, `10000000000`, `LONG_MAX` | `value_outside_int_range_takes_the_error_path` |
| 6 | `parse_val` returns true | success path, `run` twice | `0`, `1`, `7`, `-3`, `+42`, `007` | `single_valid_value`, `sweep_of_representative_values` |
| 7 | `printf("An error occurred\n")` | else branch | all of #1–#5 | as above |
| 8 | `strtol` partial conversion | digits then non-digits | `42abc`, `0x10`, `3.9`, `12 34`, `8,9` | `trailing_junk_is_accepted_using_the_parsed_prefix` |
| 9 | `strtol` leading `isspace` skip | space/tab/CR/VT/FF before digits | `"   42"`, `"\t8"`, `"\r12"`, `" \t\v\f\r 12"` | `leading_whitespace_is_skipped` |
| 10 | `fgets` stops at `'\n'` | more than one line available | `"7\n9\n"`, `"abc\n7\n"`, `"1\n2\n"` | `fgets_does_not_read_past_the_first_line` |
| 11 | `fgets` `sizeof(in) == 100` cap | line ≥ 99 bytes | 98/96/91/90-space padded numbers, 99 digits, 500 bytes | `fgets_truncates_at_ninety_nine_bytes` |
| 12 | `in` used as NUL-terminated string | embedded `'\0'` | `"\0 5"`, `"5\0 6"`, `"  \0 42"` | `embedded_nul_bytes_terminate_the_string` |
| 13 | signed `int` overflow in `add_bedrooms` | `5 + x` / `5 + 2x` wrap | `INT_MAX`, `INT_MIN`, `±2^30` | `int_range_extremes_overflow_identically` |
| 14 | `run(&the_house, x)` called twice | shared mutable state | `10` (full 8-line transcript pinned) | `state_carries_between_the_two_run_calls` |
| 15 | `return 0` from `main` | both paths | success, error, EOF | `exit_status_is_zero_and_stderr_empty_on_both_paths` |

## C behaviors that had to be reproduced, and were

1. **`run` is called twice on the same `house_t`.** State accumulates instead of
   resetting, so the second call starts from floors=3, bathrooms=3.5, and adds
   `extra_bedrooms` a second time. The transcript is always 8 lines ending at
   floors=4, bedrooms=`5 + 2x`, bathrooms=4.5. Pinned literally in
   `state_carries_between_the_two_run_calls`.

2. **`fgets` does not read across newlines.** Only the first line is ever seen;
   `"1\n2\n"` yields x=1, not 12 as `scanf` semantics would give. The Rust
   `fgets_stdin` reads byte-at-a-time and stops after the first `\n`, keeping it.

3. **The 99-byte cap is observable in the parsed value.** `char in[100]` with
   `fgets(in, sizeof(in), stdin)` stores at most 99 bytes plus a NUL. 91 spaces
   followed by `123456789` parses as `12345678` — the final digit is cut — and
   both programs print bedrooms=24691361 on the last line. 99 spaces followed by
   a digit loses the digit entirely and takes the error path.

4. **Partial conversion is a success, not an error.** The C code only requires
   `endp != str`, so `42abc` is accepted as 42 and `0x10` is accepted as 0 (base
   10 stops at `x`). A translation using Rust's `str::parse::<i32>()` would
   reject all of these and print `An error occurred` instead.

5. **Two distinct out-of-range rejections.** `2147483648` keeps `errno == 0`
   (it fits a 64-bit `long`) and is rejected by the `INT_MAX` comparison, while
   `9223372036854775808` is rejected by the `errno` check after `strtol`
   saturates and sets `ERANGE`. Both print the same message, but the Rust
   `strtol_base10` has to model saturation and the `ERANGE` flag separately to
   get both right, including the asymmetric negative magnitude limit
   (`|LONG_MIN|` = 2^63, one more than `LONG_MAX`). `-9223372036854775808` is
   accepted by `strtol` without `ERANGE` and then rejected as below `INT_MIN`.

6. **Signed overflow wraps.** `bedrooms += extra_bedrooms` with x = `INT_MAX`
   is UB in C but wraps two's-complement as compiled here; the C binary prints
   `-2147483644` then `3`. The Rust uses `wrapping_add`, which matches. Plain
   `+` in Rust would panic in debug builds and abort with a stderr message,
   which would fail the stderr and exit-status assertions.

7. **Input is bytes, not text.** Lone `0xff`, truncated UTF-8 sequences and
   embedded NULs all reach `strtol`. The Rust reads a `Vec<u8>` and truncates at
   the first NUL rather than going through `String`, so no lossy conversion or
   UTF-8 error can change the outcome.

8. **`%.1f` formatting.** `bathrooms` only ever takes 2.5, 3.5 and 4.5, all
   exactly representable, so Rust's `{:.1}` and C's `%.1f` cannot diverge on
   rounding for any reachable value.

9. **Exit status is always 0 and stderr is always empty**, on the success path
   and the error path alike. Asserted independently in
   `exit_status_is_zero_and_stderr_empty_on_both_paths`, because a stdout-only
   comparison would not catch a Rust program that exited non-zero.

## Status

- Both programs build with no errors.
- `cargo test` in `translation/`: 18 passed, 0 failed, 0 ignored. No test is
  disabled, skipped or `#[ignore]`d.
- Nothing in `c_src/` was modified; only the out-of-source `c_src/build/`
  directory was created in order to compile the reference binary.
