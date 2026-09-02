# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

Both programs are compared by execution only: the built binaries are spawned as
subprocesses, fed identical bytes on stdin, and stdout / stderr / exit status are
compared byte for byte (`translation/tests/differential.rs`).

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## Result

**No output mismatches were found.** Every enumerated input class, plus a
2,200-case randomized/boundary sweep, produced identical stdout, stderr and exit
status from both binaries.

The sections below record the branch inventory that was enumerated and, for each
non-obvious behavior, the divergence a straightforward translation *would* have
introduced and how the Rust code avoids it. These are the places to re-check if
the Rust program is ever modified.

## Branch inventory of the C program

`fma_array(out, mul1, mul2, add, len)`
- Single loop: `out[i] = mul1[i] * mul2[i] + add[i]`, signed `int` arithmetic.

`call_fma(data, len)`
- `if (len == 0) return 0;` — early return, the only guarded path.
- Otherwise allocates three VLAs of length `len`, fills `ones`/`zeros`, calls
  `fma_array` with `mul1 = ones`, `mul2 = data`, `add = zeros`, and returns
  `out[len-1]`. Net effect: the function returns `data[len-1]`.

`main`
- `for (i = 0; i < 100; i++)` — bounded at 100 items.
- `if (scanf("%d", &data[i]) != 1) break;` — breaks on both an input failure
  (EOF, `scanf` returns `EOF`) and a matching failure (`scanf` returns `0`).
- `printf("%d\n", result)` — exactly one line, then `return 0`.

So the observable contract is: *print the last value successfully parsed by
`scanf("%d")` from at most the first 100 items, or `0` if none was parsed.*
Exit status is always 0 and stderr is always empty.

## Behaviors that had to be preserved (verified, not "fixed")

### 1. `scanf` reads across newlines

`scanf("%d")` skips any run of C whitespace (` `, `\t`, `\n`, `\v`, `\f`, `\r`)
before the number, so line structure is irrelevant. A translation built on
`read_line`/`BufRead::lines` and per-line parsing would diverge on
`"1\n2\n3"`, `"1\r\n2\r\n3"`, `"  \n\t 1 \x0b 2 \x0c 3 "`, and on inputs where a
single line holds several numbers.

The Rust side implements a byte-level `scanf("%d")` (`Stream` + `scanf_d`) with a
one-byte pushback, mirroring `getc`/`ungetc`, rather than any line-based reader.

### 2. A matching failure stops the loop; it is not an error and not skipped

`"1 2 x 3"` prints `2`, not `3` and not an error. `scanf` returns 0, `break`
fires, and the trailing `3` is never read. Two plausible wrong behaviors:
skipping the bad token and continuing (would print `3`), or treating it as an
error (would write to stderr and/or exit non-zero). Neither happens: the C
program always exits 0 with empty stderr.

Sub-cases verified: junk as the very first token (`"abc"` → `0`), junk glued to
digits (`"1 2 3x4"` → `3`), a float (`"1.5"` → `1`, the `.5` stops the scan),
exponent notation (`"1e9"` → `1`), `0x` prefixes (`"0x10"` → `0`, since `%d` is
base 10 and stops at `x`), and comma/semicolon separators.

### 3. Sign handling, including the malformed cases

`%d` accepts one optional `+`/`-` immediately before the digits. A sign not
followed by a digit is a matching failure with the offending character pushed
back — so `"-"`, `"+"`, `"-x"`, `"--5"`, `"- 5"` all yield `0`. `"5-"` yields
`5` (the first conversion succeeds, the second fails).

### 4. Out-of-range values: `long` saturation, then truncation to `int`

This is the subtlest part. glibc converts `%d` through a `long`-width
`strtol`-style accumulation, saturates at `LONG_MAX`/`LONG_MIN` on overflow, and
then truncates the result to `int` on assignment. Observed and matched:

| input | output | why |
|---|---|---|
| `2147483648` (`INT_MAX+1`) | `-2147483648` | fits in `long`, truncated to `int` |
| `4294967296` (`2^32`) | `0` | low 32 bits are zero |
| `4294967295` (`UINT_MAX`) | `-1` | |
| `-2147483649` | `2147483647` | |
| `9223372036854775807` (`LONG_MAX`) | `-1` | low 32 bits all ones |
| `9223372036854775808` (`LONG_MAX+1`) | `-1` | saturates to `LONG_MAX`, then truncates |
| `-9223372036854775809` | `0` | saturates to `LONG_MIN`, low 32 bits zero |
| `"9"×40` | `-1` | saturates to `LONG_MAX` |
| `"-9"×40` | `0` | saturates to `LONG_MIN` |
| `"0"×1000` | `0` | zeros never overflow, regardless of length |

A translation using `i32::from_str` would error on all of these and print `0`
instead; one using `i64::from_str` would error (rather than saturate) past
`LONG_MAX`. `scanf_d` accumulates into `i64`, sets a `saturated` flag on
`checked_mul`/`checked_add` failure while still consuming the remaining digits,
clamps to `i64::MIN`/`i64::MAX`, and only then casts to `i32`.

Verified against a sweep of `10^e ± {0,1,2}` for `e` in `1..40` and of
`2^31`, `2^32`, `2^63`, `2^64` (and their `-1` neighbours) with `+`, `-`, and no
sign — all identical.

### 5. Signed `int` arithmetic in `fma_array`

`ones[i] * data[i] + zeros[i]` is `1 * x + 0`, which cannot overflow, but the
Rust code uses `wrapping_mul`/`wrapping_add` so it can never panic in debug
builds even if the multiplicands change. A plain `*`/`+` would be an
overflow panic in a debug build — a behavior C does not have.

### 6. The 100-item bound

The array holds exactly 100 `int`s and the loop condition stops there. With 101
or more numbers on stdin, only the first 100 are consumed, `data[99]` is the
answer, the remaining bytes are left unread, and the exit status is still 0.
Verified at 0, 1, 2, 98, 99, 100, 101, 150 and 200 items, and with junk placed
after the 100th value.

### 7. Uninitialized `data[100]` is never observable

`int data[100]` is uninitialized in C, but `call_fma` only reads `data[0..len]`
where `len == i` is the count actually written, and returns immediately when
`len == 0`. The Rust version zero-initializes its array; because the C program
never reads an uninitialized element, this is not observable. (Had `call_fma`
lacked the `len == 0` guard, `out[len-1]` with `len == 0` would be an
out-of-bounds read and there would be nothing well-defined to match.)

### 8. Output shape and exit status

Exactly `"%d\n"` on stdout — a single line with a trailing newline, no leading
or trailing spaces, no other output. stderr is always empty and the exit status
is always 0, on every input class including pure junk and empty input.

## Non-stdin edge cases also checked (all identical)

- stdin closed (`0<&-`), stdin pointed at a directory → both print `0`, exit 0.
- stdout closed (`>&-`), stdout to an already-closed pipe, stdout to
  `/dev/full` → both exit 0 with no stderr. (Rust's default `SIGPIPE` handling
  and its ignored write errors happen to agree with glibc here, which silently
  discards the failed flush at exit.)
- Non-UTF-8 and binary stdin: embedded `NUL`, bytes `\xff`/`\xfe`, a truncated
  UTF-8 sequence, and a stream of all 256 byte values. A translation that read
  stdin through `String`/`read_to_string` would fail on these; `Stream` works on
  raw bytes.

## Test coverage map

`translation/tests/differential.rs`, 19 tests, none `#[ignore]`d:

| test | branch / input class |
|---|---|
| `empty_input` | `call_fma` `len == 0` |
| `whitespace_only_inputs` | EOF reached while skipping whitespace |
| `non_numeric_first_token_yields_zero` | matching failure on the first `scanf` |
| `sign_only_and_malformed_signs` | sign accepted then no digit |
| `signed_values` | negative / explicit-positive conversion |
| `single_item` | exactly one item |
| `result_is_the_last_value_read` | `out[len-1]` semantics |
| `scanf_crosses_newlines_and_arbitrary_whitespace` | whitespace skipping across lines |
| `stops_at_first_non_numeric_token` | mid-stream `break` |
| `item_count_boundaries` | 1, 2, 98, 99, 100, 101, 150, 200 items |
| `one_hundred_items_space_separated_without_trailing_newline` | exactly 100, no trailing newline |
| `hundredth_value_is_the_answer_even_with_junk_after` | bound reached before junk |
| `int_extremes` | `INT_MAX` / `INT_MIN` |
| `values_beyond_int_are_truncated_as_c_does` | truncation to `int` |
| `values_beyond_long_saturate_then_truncate` | `strtol` saturation |
| `very_long_digit_runs` | 1000–5000 digit tokens |
| `binary_and_high_bytes` | non-UTF-8 stdin |
| `deterministic_random_sweep` | 300 seeded random inputs across the 100-item boundary |
| `output_is_exactly_the_number_and_one_newline` | output shape, and a sanity check that the C reference is not silently broken |
