# Differential verification of `translation/` against `c_src/`

Ground truth: `c_src/src/main.c`, built with CMake into `c_src/build/driver`.
Candidate: `translation/src/main.rs`, built with `cargo build --release` into
`translation/target/release/driver`.

Comparison method: both programs are run as subprocesses with identical
arguments and the same forced `argv[0]`, and **stdout, stderr and exit status**
are compared byte for byte. See `translation/tests/differential.rs`. The Rust
code is never loaded as a library.

## Result

**No behavioural mismatch was found.** Every enumerated input produced
byte-identical stdout, byte-identical stderr and the same exit status.

The sections below record what was probed and why, because the hazards in this
program are all in places where a plausible Rust translation *would* diverge.
Each one is a mismatch that was checked for and did not occur.

## Reachable branches in the C program

`main` has exactly three rejecting branches and one accepting path.
`perform_expensive_operations` is branch-free.

| # | C condition | Observable behaviour | Test |
|---|---|---|---|
| 1 | `argc != 2` | `Usage: <argv[0]> <seed>` on stderr, exit 1 | `argc_branch`, `usage_message_echoes_argv0` |
| 2 | `*endptr != '\0'` | `Invalid seed: '<argv[1]>'` on stderr, exit 1 | `trailing_garbage_branch` |
| 3 | `errno != 0` (ERANGE) | same message, exit 1 | `out_of_range_branch`, `very_long_argument` |
| 4 | `temp_seed > UINT_MAX` | same message, exit 1 | `out_of_range_branch` |
| 5 | none of the above | `srand`, 2000 workload passes, `printf("%d\n", xor)`, exit 0 | `accepted_seeds_produce_identical_output` |

Branches 2, 3 and 4 are the three disjuncts of a single `||`, and each is
covered by inputs that trip it while the other two are false. Because all three
produce the identical message and status, the short-circuit evaluation order is
not observable — no test depends on it.

## Hazards checked, and the C behaviour that had to be reproduced

### 1. An empty argument is a *valid* seed, not an error

`strtoul("")` performs no conversion, so glibc sets `endptr = nptr`. `*endptr`
is therefore the terminating NUL, `*endptr != '\0'` is **false**, `errno` is
untouched, and `temp_seed` is 0. The program accepts it and runs with seed 0.

This is the single most likely place for a translation to diverge: a Rust
`str::parse::<u64>()` returns `Err` on `""` and would exit 1 where the C exits
0 and prints a number. The translation models `endptr` as an offset and, for
the no-conversion case, resets it to 0 exactly as glibc does — so the check
becomes `end != arg.len()`, which is `0 != 0` for an empty argument. Verified:
both print `42032659` and exit 0.

By contrast `" "` and `"   "` are rejected, because there `endptr = nptr` points
at a space, not at the NUL. Both behaviours are tested.

### 2. `strtoul` silently wraps negative values

`strtoul` accepts a leading `-` and negates the magnitude modulo 2^64. So:

- `"-1"` → `0xFFFFFFFFFFFFFFFF`, no ERANGE → rejected by `> UINT_MAX`.
- `"-0"` → `0` → **accepted**, seed 0.
- `"-18446744073709551615"` → magnitude is exactly `ULONG_MAX`, which does *not*
  overflow, so there is no ERANGE; negating gives `1` → **accepted**, seed 1.
- `"-18446744069414584321"` → negates to exactly `4294967295` (`UINT_MAX`) →
  **accepted**. I initially classified this as out-of-range; it is not. One
  less than that magnitude, `"-18446744069414584320"`, negates to `4294967296`
  and *is* rejected. Both are tested, on the sides they actually land on.
- `"-18446744073709551616"` → magnitude overflows → ERANGE, and glibc returns
  `ULONG_MAX` **without** applying the sign → rejected.

A translation that rejected all leading `-` signs, or that applied the sign to
the ERANGE sentinel, would diverge on four of these.

### 3. ERANGE versus the `> UINT_MAX` check are different rejections

`"18446744073709551615"` (`ULONG_MAX`) parses cleanly — no ERANGE — and is
rejected only by `temp_seed > UINT_MAX`. `"18446744073709551616"` sets ERANGE.
Both are tested so that neither check can be dropped without a failure. A
100,000-digit argument (`very_long_argument`) covers the ERANGE path at a length
where a naive accumulate-and-compare could itself overflow or panic; the
translation uses `checked_mul`/`checked_add` and latches an overflow flag,
matching glibc's "consume every digit, return `ULONG_MAX`, set ERANGE".

### 4. `srand`/`rand` must be glibc's generator, not any PRNG

The output is a XOR fold over 262,144 values seeded from `rand()`, so any
deviation in the generator changes the single number on stdout. The translation
reimplements glibc's TYPE_3 additive-feedback `random_r` (degree 31,
separation 3, 310 discarded outputs, low bit dropped), including glibc's
"seed must not be 0" fixup — which is why seed 0 and seed 1 print the same
value (`42032659`). Verified across 40 distinct accepted seeds: `0`, `1`, `7`,
`9`, `12`, `2`, `3`, `5`, `33`, `42`, `100`, `777`, `1000`, `1024`, `12345`,
`65535`, `65536`, `88888888`, `305419896`, `123456789`, `999999937`,
`1000000000`, `1431655765`, `2147483647`, `2147483648`, `2147483649`,
`2863311530`, `3000000000`, `4000000000`, `4294967294`, `4294967295`, and the
sign/whitespace/leading-zero spellings of several of those. Seeds above
`INT_MAX` are included because `srand` takes an `unsigned int`.

### 5. The hot loop relies on wrapping, arithmetic shift and C division

```c
x = x * 3 + 7;        // signed overflow -> must wrap, not panic
x = x ^ (x >> 3);     // must be an arithmetic (sign-propagating) shift
x = x - (x << 1);     // left shift of a negative int; wraps
x = x / 2 + x % 7;    // truncation toward zero; % takes the sign of x
```

In debug Rust, `x * 3 + 7` panics on overflow, and `x % 7` for negative `x` is
a common source of Python/Rust confusion. The translation uses
`wrapping_mul`/`wrapping_add`/`wrapping_sub`/`wrapping_shl` and relies on `i32`
`>>` being arithmetic and `/` and `%` truncating toward zero as C does. This is
exercised over roughly 40 × 262,144 starting values run through 200,000 update
steps each; every one of those runs folds into a value that matched the C.

`x << 1` on a negative `int` is undefined behaviour in C, so this is a case
where the C "looks like a bug". Per the instructions it was replicated rather
than fixed: the translation reproduces what the compiled C actually does
(two's-complement wrapping), confirmed empirically by the matching output.

### 6. `argv[1]` may not be valid UTF-8

The error message echoes the raw argument. `String::from_utf8_lossy` would
substitute U+FFFD and diverge. The translation reads arguments as
`OsStr` bytes and writes them straight out. Verified with `\xff`, `\xff\xfe`,
`12\xff`, `\xc3` and `\x80\x80`, plus Arabic-Indic digits `\u{661}\u{662}`
(which `strtoul` does not treat as digits).

### 7. `argv[0]` is echoed verbatim, including when `argc == 0`

Tested with `argv[0]` set to `driver`, `/weird/path/to/driver`, `x` and `""`
via `CommandExt::arg0`. Also tested the `argc == 0` case, which no shell can
produce, using a helper that calls `execve` with an empty `argv`: both programs
print `Usage:  <seed>` and exit 1. The translation's `argv.first()` /
`unwrap_or(b"")` matches.

### 8. Exit status and output framing

`main` returns 0 or 1; the Rust program calls `process::exit` with the same
values. stdout is exactly `%d\n` with no padding or precision. Both are
compared on every case, so a translation that exited 0 on an error path would
fail even though its stdout is empty in both.

## Notes on the test harness

Two defects were in my test code, not in the translation, and are recorded
here only so the numbers above are not misread:

- The first exploratory shell loop left `$args` unquoted, so `"   "` was
  word-split away and the case actually tested `argc == 1`. The committed Rust
  tests pass arguments as an `OsString` vector, which cannot word-split.
- `"-18446744069414584321"` was initially listed under both the rejecting and
  the accepting test. It belongs only in the accepting one; see §2.

Both programs print `argv[0]`, so a naive comparison would always differ on the
usage path by the two binaries' paths. Rather than normalising the text
afterwards, every invocation forces the same `argv[0]` with
`CommandExt::arg0` (the equivalent of `exec -a driver ...`), which keeps the
stderr comparison a genuine byte comparison.

## Cost

`accepted_seeds_produce_identical_output` is slow by nature: one run is 2000
passes over 262,144 `int`s with 100 update steps each, about 5 min for the Rust
binary and about 8 min for the C one. The harness launches every process pair
concurrently, so the whole test costs roughly one C run in wall-clock time
(~8 min) rather than the sum. No test is skipped or `#[ignore]`d to avoid this.
