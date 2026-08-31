# Verification log — C vs. Rust differential testing

Programs compared:

- C: `c_src/build/driver` — built with `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
- Rust: `translation/target/release/driver` — built with `cd translation && cargo build --release`

Test suite: `translation/tests/differential.rs`. Every case spawns both binaries,
feeds the identical stdin bytes, and asserts stdout, stderr and exit status all
match. The Rust code is never linked as a library.

## Result

**No mismatches were found.** All 21 tests pass in both the debug and release
profiles, and an additional out-of-band fuzz sweep of 3,024 inputs (boundary
sweeps around 2^31 / 2^32 / 2^63 / 2^64 / 10^19 / 10^20, random digit strings of
lengths 1–1000 with and without signs and leading zeros, and 1,500 random
garbage strings drawn from digits, signs, whitespace, `.`, `x`, `e`, `/`, `:`,
letters, NUL and `0xff`) produced zero differences in stdout, stderr or exit
status.

`nothing in c_src/ has been modified` — only the untracked `c_src/build/`
directory was created, by the build command specified for this task.

## The branch structure of the C program

`main` is:

```c
int x = 0;
scanf("%d", &x);
driver(x);
return 0;
```

`driver` and `print_hex` are branch-free apart from `for (i = 0; i < len; i++)`,
where `len` is always `sizeof(house_t)`. So the input classes all live in
glibc's `%d` conversion, and the only "length" input class is the struct layout.
Each of the following was covered by a named test:

| Input class | C behaviour | Test |
| --- | --- | --- |
| empty stdin | `scanf` returns `EOF`, `x` keeps its initialiser `0` | `empty_input_leaves_x_zero` |
| whitespace only | leading whitespace skipped, then EOF; `x` stays `0` | `whitespace_only_input_is_eof` |
| single value | `x` assigned | `single_value` |
| signs, `-0`, `+0` | assigned | `negative_and_signed_values` |
| `int` boundaries and wraparound | `strtol` result narrowed to `int` | `int_boundaries` |
| `long` boundaries / overflow | `strtol` saturates to `LONG_MAX`/`LONG_MIN`, then narrows | `long_boundaries_and_overflow_saturation` |
| whitespace containing newlines before the value | `scanf` crosses newlines (unlike `fgets`) | `scanf_skips_whitespace_across_newlines` |
| matching failure (`abc`, `-`, `+`, `--5`, `-  5`, …) | `scanf` returns `0`, `x` stays `0` | `matching_failure_leaves_x_zero` |
| value followed by non-digits (`7abc`, `0x10`, `3.9`, `1e5`) | conversion stops at the first non-digit | `conversion_stops_at_first_non_digit` |
| extra input after the value | never read; program exits | `trailing_input_after_the_value_is_ignored` |
| NUL and bytes ≥ 0x80 in stdin | not whitespace, so matching failure | `embedded_nul_and_high_bytes` |
| exact `isspace` set (` `, `\t`, `\n`, `\v`, `\f`, `\r`) vs. `\x1c` | only the six are skipped | `c_whitespace_class_exactly` |
| 100 KB digit / space runs, 50 K newlines, 200 KB junk | unbounded scanning; no truncation, no hang | `very_long_digit_runs`, `large_unparsable_input` |
| `argv` present | `main()` takes no parameters, so argv is ignored | `arguments_are_ignored` |
| stdin = `/dev/null` | plain EOF | `closed_stdin` |
| stdout write failure (`/dev/full`) | `printf`'s return value ignored; still exits `0` | `stdout_write_failure_behaves_the_same` |
| output shape | `sizeof(house_t) == 16` → 32 hex digits + `\n` | `output_shape_is_32_hex_digits_and_newline` |

## Behaviours the translation had to get right (checked, all correct)

These are the places where a plausible translation would have diverged. Each is
noted here because a passing test is only meaningful if the reason it passes is
understood.

1. **`x` survives a failed `scanf`.** `int x = 0;` is the value used when the
   conversion fails or hits EOF; the C code never checks `scanf`'s return value.
   The Rust `main` mirrors this by only overwriting `x` on success. A translation
   that treated a parse failure as an error exit would differ in exit status on
   `""` and `"abc"` while still matching stdout on the happy path — which is why
   the suite asserts the exit status on every case.

2. **Overflow saturates in `long`, then narrows to `int`.** glibc's `%d` goes
   through `strtol`, so `2147483648` becomes `(int)2147483648L` = `-2147483648`
   (`00000080…`), whereas `9223372036854775808` saturates to `LONG_MAX` and
   narrows to `-1` (`ffffffff…`). Saturating at `INT_MAX`, or wrapping instead of
   saturating, would both pass the small-value tests and fail here.

3. **`scanf` crosses newlines.** `%d` skips any run of whitespace, newlines
   included, so `"\n\n  \t\n 42"` yields `42`. An `fgets`/`read_line`-based
   translation would stop at the first newline and produce `0`.

4. **Struct object representation.** `house_t` is `{int, int, double}`: 16 bytes,
   8-byte aligned, with no padding holes on the LP64 little-endian target, so
   `memcpy` out of it is `floors` LE, `bedrooms` LE (`03000000`), then `2.0` as
   little-endian IEEE-754 (`0000000000000040`). Getting the endianness or the
   field order wrong would change every byte of output. The suite pins the C
   output length at 33 bytes so a wrong `sizeof` cannot pass silently.

5. **`print_hex` formatting.** `%02x` on `unsigned char`, no separators, one
   trailing `\n`, and nothing on stderr. Compared byte for byte.

6. **Exit status is always 0** and stderr is always empty, for every input
   including binary garbage and a failing stdout.
