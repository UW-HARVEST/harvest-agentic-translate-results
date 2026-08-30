# Differential verification report — `driver`

C ground truth: `c_src/src/main.c` (unmodified; only `c_src/build/` was created
by CMake).
Rust under test: `translation/src/main.rs`.

## How each program is run

```bash
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver < input

# Rust
cd translation && cargo build --release
./translation/target/release/driver < input
```

`translation/tests/differential.rs` spawns both binaries as subprocesses, pipes
the same bytes to stdin, and compares **stdout, stderr and exit status** for
every case. The Rust code is never loaded as a library.

## What the program actually does

```
main: read up to 100 ints with scanf("%d"), stop at the first non-success
call_fma: len == 0 -> 0; otherwise out[i] = ones[i]*data[i] + zeros[i]
          -> returns data[len-1]
print "%d\n"; return 0
```

So the only observable output is **the last integer successfully read** (or `0`
when none was read), followed by `\n`, on stdout, with an empty stderr and exit
status `0` for every possible input. `int data[100]` is never read beyond index
`i-1`, so its uninitialised tail is not observable.

## Mismatches found

**None.** Every input class enumerated below already produced byte-identical
stdout, byte-identical stderr and an identical exit status. No change to
`translation/src/main.rs` was required; the file is bit-for-bit the one I
started from.

Because there was nothing to fix, the rest of this document records (a) the
non-obvious C behaviours that a naive translation *would* have got wrong and
that the existing Rust code already reproduces, and (b) the mutation testing I
used to prove the test suite would actually have caught a mismatch.

## Non-obvious C behaviours that are correctly reproduced

These are the places where an ordinary Rust rewrite (`str::parse::<i32>()`,
`BufRead::lines()`, `println!`) diverges from the C. Each is pinned by a test.

| # | C behaviour | Naive Rust would do | Test |
|---|---|---|---|
| 1 | `scanf("%d")` **reads across newlines** — whitespace, tabs, VT, FF, CR and newlines are all just separators, so `"1\n2 3\n\n 4"` is four values, not one line. `fgets` would not. | Treat one line as one record | `several_items`, `indentation_and_odd_whitespace` |
| 2 | glibc converts `%d` via `strtol`: on overflow the value is **clamped to `LONG_MAX`/`LONG_MIN` and then truncated to `int`**, and `scanf` still returns 1, so the loop keeps going. `"99999999999999999999"` prints `-1`; the negative form prints `0`. | Return a parse error, or saturate to `i32::MAX` (`2147483647`) | `long_boundaries_and_overflow`, `overflow_is_not_the_last_value` |
| 3 | Values between `INT_MAX` and `LONG_MAX` are **wrapped, not clamped**: `2147483648` prints `-2147483648`, `4294967296` prints `0`, `4294967297` prints `1`. | Clamp to `2147483647` | `int_boundaries` |
| 4 | A **matching failure ends the loop but is not an error**: `"1 2 x 3"` prints `2` and exits `0`. Garbage never reaches stderr and never changes the exit status. | Print a parse error to stderr and/or exit non-zero | `garbage_stops_the_loop` |
| 5 | `%d` accepts **no `0x` prefix, no decimal point, no exponent**. `"0x10"` converts `0` then fails on `x` (prints `0`); `"1e5"` prints `1`; `"3.14"` prints `3`. | Accept/reject the whole token | `garbage_stops_the_loop` |
| 6 | A lone `-`, `+`, `--5` or `-a` yields **no conversion at all**, so `i == 0` and `call_fma`'s `len == 0` guard prints `0`. | Panic, or treat `-` as `0` | `first_token_not_a_number` |
| 7 | **Empty and whitespace-only input** hit the same `len == 0` guard: stdout is exactly `"0\n"`, stderr empty, exit `0`. | Exit non-zero, or print nothing | `empty_input`, `whitespace_only_input` |
| 8 | The loop bound is `i < 100`, so the **101st token is never consumed** — `seq 1 150` prints `100`, and trailing garbage after 100 good values is never seen. | Read all of stdin | `exactly_below_at_and_above_the_maximum`, `maximum_followed_by_garbage` |
| 9 | Output is `printf("%d\n")` — a **single trailing newline, no other text**. | `println!` with a label/prefix | every test |
| 10 | `int` arithmetic in `fma_array` is C's, i.e. **two's-complement wrap**, not a debug-build panic. The Rust uses `wrapping_mul`/`wrapping_add`, so `cargo test` passes in the debug profile too (overflow checks on). | Overflow panic in debug | whole suite run in both profiles |
| 11 | stdin is **arbitrary bytes, not UTF-8** — an embedded `\x00` or a `\xff` is just a non-digit that ends the loop. | `read_to_string` fails on invalid UTF-8 | `arbitrary_binary_bytes` |
| 12 | If stdout is closed, C's `printf` fails silently and `main` still `return 0`. The Rust ignores the write error (`let _ = write!`) instead of panicking on a broken pipe. | Panic / exit 101 | verified manually with `>&-` |

## Input classes enumerated from the C source

Every branch in the C, and the input class that reaches it:

* `call_fma`: `len == 0` early return → empty input, whitespace-only input,
  first token non-numeric.
* `call_fma`: normal path → 1 item, 2 items, 99, 100.
* `main`: loop condition `i < 100` false → exactly 100 and more than 100 tokens
  (101, 150, 500, 20 000).
* `main`: `scanf != 1` break → EOF mid-stream, matching failure at position 1,
  mid-stream, and at element 100.
* `scanf("%d")` internals: leading-whitespace skip (all six C space
  characters), optional `+`/`-`, sign-then-EOF, sign-then-non-digit, digit run,
  leading zeros, `LONG`/`INT` boundaries and both overflow directions, and the
  pushback of the offending character on matching failure.
* Reader plumbing: values straddling the Rust reader's 8 KiB refill boundary
  (offsets 8188–8193, 16383, 16384) and a 20 000-token input.
* Byte-level: all 256 byte values, high bytes, invalid UTF-8 sequences, NUL.

Plus 600 deterministic pseudo-random cases (fixed seed) in
`fuzz_over_parser_alphabet` and `fuzz_over_numeric_tokens`, and a separate
throwaway sweep of 6000 random inputs run outside the suite — all identical.

## Mutation testing (proof the suite is not vacuous)

A suite where everything passes is only meaningful if it fails when the Rust is
wrong. I injected each of these faults into `translation/src/main.rs`, ran
`cargo test`, then restored the file:

| Injected fault | Failing tests |
|---|---|
| clamp to `i32` range instead of truncating (`as_long.clamp(..) as i32`) | 4 |
| loop bound `i < 101` | 6 |
| `len == 0` guard returns `1` | 8 |
| drop the trailing `\n` from the output | 20 |
| drop VT/FF/CR from the whitespace set | 4 |
| `exit(1)` when nothing was read | 8 |
| return `out[0]` instead of `out[len-1]` | 13 |

One mutation was **not** detected, correctly so: removing the `ungetc`
pushback on a matching failure. `main` breaks out of the loop immediately after
the first non-success and never calls `scanf` again, so whether the offending
character is pushed back is unobservable from outside the process. It is kept
in the Rust because it is what C does, but no input can distinguish it.

## Status

* `c_src` builds with no errors; `c_src/src/main.c` is unmodified.
* `cargo build --release` succeeds with no errors and no warnings.
* `cargo test` passes in both the debug and release profiles: 19 tests, 0
  failed, 0 ignored.
* No test is disabled, skipped or `#[ignore]`d.
