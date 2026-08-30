# Differential verification of the Rust translation

C ground truth: `c_src/src/main.c` (built with CMake, gcc, x86-64 Linux).
Rust under test: `translation/src/main.rs`.

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # ./c_src/build/driver
cd translation && cargo build --release                                 # ./translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## Mismatches found

**None.** Every input class listed below produced byte-identical stdout,
byte-identical stderr (always empty) and exit status 0 from both programs.
Roughly 13,000 differential runs were executed: the ~130 hand-enumerated cases
in `tests/differential.rs`, plus ad-hoc fuzzing over random float spellings,
hex-float spellings and float rounding boundaries. No case required a change to
`translation/src/main.rs`, and nothing in `c_src/` was touched.

Because there is nothing to record as "found and fixed", the rest of this file
documents the behaviours that *would* have produced a mismatch had the
translation got them wrong, and how each was confirmed. These are the places a
future change is most likely to break.

## Control-flow paths in the C, and the input that reaches each

`main` is a fixed sequence: `good()` (which is `goodG2B()` then `goodB2G()`),
then `bad()`. `stdin` is read exactly twice — once by `goodB2G`, once by `bad`.

| C path | Reached by |
| --- | --- |
| `printLine` with a non-NULL pointer | every run (NULL is unreachable; all call sites pass literals) |
| `goodG2B` — constant `data = 2.0F` | every run, prints `50` |
| `goodB2G` `fgets` succeeds | any non-empty stdin |
| `goodB2G` `fgets` returns NULL | empty stdin, `/dev/null`, closed fd, fd on a directory |
| `goodB2G` `fabs(data) > 0.000001` true | `2e-6`, `7`, `1.0000001e-06` |
| `goodB2G` guard false → "This would result in a divide by zero" | `0`, `-0`, `abc`, `1e-06`, `nan` |
| `bad` `fgets` succeeds | ≥ 2 lines, or a line longer than 19 bytes |
| `bad` `fgets` returns NULL → "fgets() failed." | 1 line or less on stdin |
| `bad` division by zero, no guard | `0`, `-0`, `abc`, empty stdin |
| `(int)` cast out of range | any divisor whose reciprocal ×100 exceeds 2^31 |

## Behaviours verified as byte-identical

1. **`(int)` of an out-of-range / infinite / NaN double.** In C this is
   undefined behaviour; gcc on x86-64 emits `cvttsd2si`, which returns the
   "integer indefinite" value `0x80000000`. Division by zero therefore prints
   `-2147483648`, not a trap and not `2147483647`. Rust's `as` cast *saturates*
   instead, so a naive `v as i32` would print `2147483647` for `+inf` and `0`
   for NaN. `c_double_to_int` reproduces the hardware behaviour. Confirmed at
   the boundary with divisors `4.6566127e-08` / `4.6566129e-08` (either side of
   `100.0 / 2^31`) and with `nan`, `inf`, `0`, `-0`.

2. **`fgets` does not read past its newline, and stops at 19 bytes.**
   `CHAR_ARRAY_SIZE` is 20, so at most 19 bytes are stored. A line of 19 or more
   payload bytes is *split*: `goodB2G` gets the first 19 bytes and `bad` gets
   the remainder of the same line. Using a line-oriented read, or `read_line`,
   would consume the whole line and change what `bad()` sees. Confirmed with
   18-, 19-, 20-, 23- and 25-byte lines, including one where the split lands
   inside an exponent (`1.000000000000000e-40`).

3. **`fgets` returns NULL only when nothing was stored.** A final line without a
   trailing newline is still returned successfully; NULL comes from EOF-with-no-
   bytes or from a read error. Both were exercised: EOF via empty stdin and
   `/dev/null`, and a genuine read error via an fd opened on a directory
   (`EISDIR`) — in that case the C prints `fgets() failed.` twice and the Rust
   must too.

4. **`atof` is `strtod(s, NULL)`: longest-prefix parsing, `0.0` on failure.**
   Trailing junk is ignored rather than being an error (`5.` → 5.0,
   `1,5` → 1.0, `01234567890123456789ZZZ` → truncated digits), a malformed
   exponent is not consumed (`1e` → 1.0), and `inf` / `infinity` / `nan` /
   `nan(chars)` and hex floats (`0X1.8p3`, `0x1p-149`) are all accepted
   case-insensitively. `0x` with no hex digit converts only the leading `0`.
   Rust's `str::parse::<f64>` accepts none of these forms, so a direct
   `parse().unwrap_or(0.0)` would diverge on most of them.

5. **`atof` sees a C string.** `fgets` may store NUL bytes; the conversion stops
   at the first one. Confirmed with `5\0abc`, `\0 5` and a flood of NULs from
   `/dev/zero`. Input is also not required to be UTF-8, so the buffer is handled
   as bytes throughout (`\xff\xfe`).

6. **`(float)` narrowing before the division.** `data` is a `float`, and it is
   the narrowed value that is divided and compared. This matters at the guard:
   `1e-06` narrows to `9.99999997e-07`, which is *not* `> 0.000001`, so
   `goodB2G` prints the divide-by-zero message while `bad()` prints
   `100000000` for the very same text. Values below `1e-46` narrow to `0.0f` and
   values above `3.4028235e38` narrow to `inf`, both of which change the
   printed result. Doing the arithmetic in `f64` throughout would silently pass
   the happy-path tests and fail here.

7. **`100.0 / data` is a double division.** The literal is a `double`, so
   `data` is promoted; the result is then truncated to `int`. Computing in `f32`
   would round differently for divisors such as `3` or `1e-7`.

8. **Output formatting.** `printf("%s\n")` and `printf("%d\n")` — no extra
   spacing, no padding, a trailing newline on every line, and nothing on
   stderr. The exit status is always 0, including on the `fgets() failed.`
   paths, so a test that only diffed stdout would not notice a translation that
   exited 1 on read failure.

9. **`argc`/`argv` are ignored.** Passing arguments changes nothing in either
   program.

## Note on unused files

`translation/src/cio.rs` and `translation/src/catof.rs` are not declared as
modules by `main.rs` and are therefore not compiled; the shipped binary's
behaviour comes entirely from `main.rs`. They were left untouched because
removing or wiring them in would change nothing that is observable.
