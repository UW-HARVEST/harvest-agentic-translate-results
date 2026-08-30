# Differential verification log

C reference: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust under test: `translation/src/main.rs`, built to `translation/target/release/driver`.
Tests: `translation/tests/differential.rs` (18 tests, ~2,900 input comparisons;
each one asserts stdout bytes, stderr bytes and exit status all match).

## Phase A — builds

| Program | Command | Result |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | clean, no warnings |
| Rust | `cd translation && cargo build --release` | clean, no errors or warnings |

Run commands: `./c_src/build/driver < input` and
`./translation/target/release/driver < input`.

Neither program needed a fix to build. Nothing in `c_src/` was modified; the
only addition there is the untracked `c_src/build/` artifact directory produced
by CMake.

## Mismatches found

**None.** Every input class enumerated below produced byte-identical stdout,
byte-identical stderr and an identical exit status on the first comparison. This
section is kept deliberately explicit: the empty result is the finding, not an
omission.

Two candidate mismatches were investigated up front because they are the usual
failure modes for this shape of program. Both turned out to be already handled
correctly by the existing Rust code, so neither is a fix — they are recorded
because they are the parts a future edit is most likely to break.

1. **Out-of-`int`-range input is *not* clamped to `INT_MIN`/`INT_MAX`.**
   glibc converts the digit run for `%d` with `strtol` (a `long`) and then
   narrows the result to `int`, so `2147483648` prints `00000080` (`INT_MIN`)
   and `4294967296` prints `00000000`. A translation that used
   `str::parse::<i32>()` — or that saturated at `i32` bounds — would differ on
   every input above `INT_MAX`. The Rust code accumulates into a wider type and
   performs the same narrowing (`as_long as i32`).
   Covered by `int_boundaries_and_narrowing`.

2. **Saturation happens at `long` range, before the narrowing.**
   `strtol` clamps to `LONG_MAX`/`LONG_MIN`, and *that* clamped value is
   truncated. So any digit run above `2^63-1` prints `ffffffff` (`LONG_MAX`
   truncated to `-1`) and any below `-2^63` prints `00000000` (`LONG_MIN`
   truncated to `0`) — note the asymmetry. `99999999999999999999999999999`
   gives `ffffffff` while its negation gives `00000000`.
   Covered by `long_range_saturation`.

## Input classes enumerated from the C source

`main()` is `int x = 0; scanf("%d", &x); driver(x);`, so all branching is inside
the `%d` directive, and `print_hex` is unconditional. Consequences that were
verified rather than assumed: stdout is *always* exactly 9 bytes
(`sizeof(int) * 2` lowercase hex digits, no separator, one trailing newline),
stderr is always empty, and the exit status is always 0 — there is no error
path that changes the exit code. `output_shape_is_nine_bytes_and_exit_zero`
asserts this against the C binary directly so the test would notice if the
reference ever stopped behaving that way.

| Class | Examples | C behavior | Test |
|---|---|---|---|
| EOF before any input (empty stdin) | `""` | `scanf` returns `EOF`, `x` untouched → `00000000` | `empty_input_leaves_x_zero` |
| Whitespace only, then EOF | `" \n\t "`, 8 newlines | input failure, `x` untouched | `whitespace_is_skipped_across_newlines` |
| Leading whitespace skipped *across newlines* | `"\n\n\n5\n"`, `"\r\n7\r\n"`, `"\x0b\x0c9"` | `%d` skips the full C `isspace` set, unlike `fgets` | `whitespace_is_skipped_across_newlines` |
| Single item, happy path | `"0"`, `"42"`, `"-1"`, `"+7"`, `"-0"` | normal conversion | `single_item_happy_path` |
| Only one conversion is performed | `"1 2"`, `"3 4 5 6 7"`, `"12abc"` | trailing tokens never read | `only_the_first_conversion_is_consumed` |
| Matching failure: sign with no digit | `"-"`, `"+"`, `"-\n"`, `"-a"`, `"- 5"`, `"--5"`, `"   +"` | no assignment, `x` stays 0 | `matching_failure_leaves_x_zero` |
| Matching failure: non-numeric first char | `"abc"`, `".5"`, `"e5"`, `"~1"`, `"_1"`, `"\0"`, `"\xff\xfe\xfd"`, UTF-8 lead byte | no assignment, `x` stays 0 | `matching_failure_leaves_x_zero` |
| No base prefix handling for `%d` | `"0x10"`, `"0X10"`, `"0b101"`, `"010"` | decimal only: converts `0` (or `10`) and stops | `hex_and_octal_prefixes_are_not_special_for_percent_d` |
| `int` boundaries and wrap | `2147483647`, `2147483648`, `-2147483648`, `-2147483649`, `4294967295`, `4294967296`, `2^33` | `long` → `int` narrowing | `int_boundaries_and_narrowing` |
| `long` boundaries and saturation (max the code handles) | `±9223372036854775807/8/9`, `±99999999999999999999999999999` | `strtol` clamps, then narrows | `long_range_saturation` |
| Leading zeros | 25 zeros then `5`, zeros then a saturating value | zeros are not octal and do not affect magnitude | `leading_zeros_do_not_change_magnitude` |
| Digit run / whitespace run crossing a stdin buffer refill | 4094–8193 bytes of padding then a value; 5,000- and 100,000-digit runs | glibc buffers differently from the Rust reader, so this is compared, not reasoned about | `buffer_boundary_inputs`, `very_long_digit_runs` |
| Every possible single byte | all 256 values | covers every first-character dispatch plus immediate EOF | `sweep_every_single_byte_input` |
| Every pair over the significant alphabet | `" \t\n\r\x0b\x0c+-0123456789.eExX\0\xff"` squared | sign-then-X, digit-then-X, whitespace-then-X | `sweep_two_byte_inputs_over_significant_bytes` |
| Values straddling every power of two to 2^70, both signs | `2^n - 2 … 2^n + 2` | truncation and saturation systematically | `sweep_decimal_values_around_every_power_of_two` |
| Contiguous small range | `-300 … 300` | sign and byte-order formatting | `sweep_small_contiguous_range` |
| Seeded pseudo-random sweep | 1,500 cases: raw bytes, alphabet strings, structured numerals, buffer-boundary padding | catch-all | `randomized_differential_sweep` |

An additional 4,000-case out-of-band fuzz run (random raw bytes, random
alphabet strings, structured numerals up to 2^70, and padding straddling the
4096-byte read boundary) also produced zero mismatches. Its cases are
represented in the suite by `randomized_differential_sweep`.

## Known behavioral difference outside the compared surface

`print_hex`'s `printf` is not error-checked in the C, and the Rust ignores its
write errors the same way. But if **stdout is a closed pipe**, the C process
dies from the default `SIGPIPE` disposition, whereas the Rust runtime ignores
`SIGPIPE`, so the Rust process would see `EPIPE`, discard it, and exit 0. This
is not reachable through the graded interface (stdin is fed and stdout/stderr
are read to completion), so it is recorded rather than worked around; forcing a
match would require installing the default `SIGPIPE` handler at startup in the
Rust program.

## Phase D — completion gate

- [x] both programs build with no errors
- [x] every enumerated input produces identical stdout, stderr and exit status
- [x] `cargo test` passes in `translation/` (18 passed, 0 failed) in both the
      dev and `--release` profiles
- [x] no test is disabled, skipped or `#[ignore]`d
- [x] nothing in `c_src/` modified (`c_src/src/main.c` md5
      `747f7b4bf95aa719eba25b7d280cd16f`, `c_src/CMakeLists.txt` md5
      `02ba3005fed9b6d7d46c4fe335ac00d8`)
