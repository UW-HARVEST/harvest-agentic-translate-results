# Differential verification record

Scope: `c_src/src/main.c` (the ground truth) versus `translation/` (the Rust
port). Comparison method: build both executables, feed identical bytes on
stdin, diff stdout, stderr and exit status.

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## Mismatches found

**None.** Every input class enumerated below produced byte-identical stdout,
byte-identical stderr (always empty) and exit status 0 from both programs. In
addition to the 15 test functions in `tests/differential.rs` (roughly 1000
inputs, including a 441-case bit-field sweep and a 500-case seeded random
sweep), an out-of-tree fuzz run of 6000 further inputs — 4000 random byte
strings over `0-9 + - . , e E _ a b z X / : ` `\t \n \r \v \f` `NUL 0x80 0xff`
and 2000 randomly formatted numeric tuples — found no divergence either.

Because there were no mismatches, nothing in `translation/src/` was changed
during this verification. `c_src/` was not modified (only `c_src/build/`, a
generated cmake output directory, was added).

## What the C program branches on

`main()` itself is branch-free: four `scanf` calls, one `driver()` call, and
`return 0`. There is no `if`, no early `return`, no error path and no exit code
other than 0. All observable variation therefore comes from three places, and
these are what the tests enumerate:

1. **How many of the four `scanf` conversions succeed.** A conversion that
   fails or hits EOF leaves its variable at its initialiser of `0`. Covered:
   zero through five whitespace-separated items, plus a ten-item input whose
   tail is never read.
2. **Bit-field truncation in `foo_t`.** `x : 2` keeps the low 2 bits, `y : 3`
   the low 3, `b : 1` receives `!!b` so it is already 0 or 1. Covered by an
   exhaustive `x, y in 0..=20` sweep plus the 2^31 / 2^32 / `UINT_MAX`
   boundaries.
3. **Integer conversion width and signedness.** `%u` into `unsigned int` and
   `%d` into `int` go through `strtoul`/`strtol` at `long` (64-bit) width and
   are then truncated to 32 bits.

## Behaviours deliberately replicated, not "fixed"

These are the places a naive Rust port would diverge. Each is asserted by a
test and each was confirmed against the C binary rather than assumed.

- **`scanf` crosses newlines.** Leading whitespace — space, tab, newline,
  carriage return, vertical tab, form feed — is skipped before each conversion,
  so `1\n2\n3\n4` and `1 2 3 4` are the same input. Line-oriented (`fgets`-like)
  reading would be wrong. Tests: `whitespace_and_line_crossing`.
- **A failed conversion leaves the variable untouched, and the stream position
  behind.** For `1 abc 3 4` the second conversion stops on `a`; that `a` stays
  in the stream, so the third and fourth conversions fail on it too and the
  output is `1 0 0 0`, not `1 0 3 4`. Tests: `matching_failures`.
- **EOF is sticky.** Once a conversion hits end of input, the later ones fail
  immediately. `Scanner` keeps an `eof` flag for this; a read error is treated
  as EOF, matching a `FILE` stream's error indicator. Tests:
  `item_counts_zero_through_five`, `empty_stdin_variants`.
- **A consumed sign is not pushed back.** For `- 1 2 3` the first `%u` eats the
  `-`, fails on the space, and pushes back only the space; the next conversion
  then reads `1`. `translation/src/scanf.rs` drops the sign for exactly this
  reason. Verified observable variants: `+ 1 2 3` -> `0 1 1 3`,
  `-+1 2 3 4` -> `0 1 1 3`, `12+34 5 6` -> `0 2 1 6`. Tests:
  `sign_only_and_stray_sign_handling`.
- **`%u` accepts a minus sign and wraps.** `-1` becomes `ULONG_MAX`, truncated
  to `0xFFFFFFFF`, then masked to 2 bits -> `3`. `-3 -5 0 0` prints `1 3 0 0`.
  Tests: `unsigned_conversion_of_signed_and_overflowing_text`.
- **Overflow saturates at `long` width before truncating.** `strtoul` clamps to
  `ULONG_MAX` and `strtol` to `LONG_MAX`/`LONG_MIN`, and the sign is *not*
  reapplied after saturation, so `-99999999999999999999` for `%u` also yields
  `0xFFFFFFFF`. Truncation to 32 bits happens afterwards: `9223372036854775808`
  for `%d` saturates to `LONG_MAX` and prints `-1`. Tests:
  `z_signedness_and_truncation`, `very_long_digit_strings`.
- **Truncation to `int` happens before `!!b`.** `4294967296` is non-zero as
  text but truncates to `int` 0, so the bool field prints `0`. Tests:
  `bool_field_normalisation`.
- **No prefixes or alternate formats.** `0x10` converts as `0` and leaves `x`
  in the stream; `1.5` converts as `1` and leaves `.`; `1e5` converts as `1`.
  Tests: `matching_failures`.
- **Output format.** `printf("%u %u %d %d\n", ...)` — single spaces, no padding,
  exactly one trailing newline, nothing on stderr, exit status 0 unconditionally.

## Pathological stream states also checked

Confirmed identical by hand (not in the suite, since they need shell-level fd
manipulation): stdin closed outright (`<&-`), stdin bound to a directory,
stdout closed (`>&-`), stdout redirected to `/dev/full`, and stdout as a pipe
whose reader exits first (`| head -c 0`). All produce matching behaviour,
including exit status and absence of a terminating signal. Rust masking
`SIGPIPE` did not cause a divergence here because the C program's only write is
buffered and flushed at exit, where the failure is likewise ignored.
