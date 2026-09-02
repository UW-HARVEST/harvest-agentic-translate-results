# ERRORS.md — differential verification log

Reference: `c_src/src/main.c`, built via `c_src/CMakeLists.txt` with no
`CMAKE_BUILD_TYPE` (so no `-O` flag) using the system GCC.

Both programs are compared by execution only:

- C: `c_src/build/driver`, built with
  `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
- Rust: `translation/target/release/driver`, built with
  `cd translation && cargo build --release`

`translation/tests/differential.rs` spawns both binaries with identical stdin
and asserts stdout, stderr and exit status all match byte for byte.

## Outcome

No mismatch was found. Every input class enumerated below produced identical
stdout, identical stderr (always empty) and identical exit status (always 0)
between the two binaries. The sections below record what was checked, the
reasoning behind each risky construct, and the evidence, so a later reader can
re-derive the result rather than take it on faith.

Total inputs exercised: ~1,200 across 17 test functions (every single byte
0x00–0xff on its own, −40..40, powers of two 2^0..2^65 ±1 in both signs, 400
pseudo-random byte strings, plus the hand-enumerated classes).

## Branch inventory of the C program

Every conditional and early exit in the C source, and the test that reaches it:

| Location | Branch | Reached by |
| --- | --- | --- |
| `printLine` | `line != NULL` — print | `nonzero_takes_the_good_branch` |
| `printLine` | `line == NULL` — print nothing, not even `\n` | `zero_takes_the_bad_branch` |
| `main` | `if (x)` true → `good()` | `nonzero_takes_the_good_branch` |
| `main` | `else` → `bad()` | `zero_takes_the_bad_branch` |
| `main` | `scanf` succeeds, assigns `x` | all good/bad-branch tests |
| `main` | `scanf` input failure (EOF before any non-whitespace), `x` untouched | `empty_input_leaves_x_at_zero`, `whitespace_only_input_is_an_input_failure` |
| `main` | `scanf` matching failure (no digits), `x` untouched | `non_numeric_input_is_a_matching_failure`, `sign_without_digits_is_a_matching_failure` |

`bad()`, `good()`, `helperBad()` and `helperGood1()` are straight-line and
unconditional; they are covered by the two `main` branches.

There is no `argv` handling, no `fgets`, no `stderr` write, and no `return`
other than `main`'s `return 0`, so exit status is 0 on every path.

## Verified behaviors that a naive translation would have gotten wrong

### 1. `helperBad()` returns NULL, not a dangling pointer — so `bad()` prints nothing

`helperBad` declares `char charString[]` with automatic storage and returns its
address. This is the CWE-562 defect the test case exists to demonstrate, and
its result is not "print garbage": GCC diagnoses it and compiles the return
value as a literal zero. Confirmed from the reference binary:

```
$ objdump -d --no-show-raw-insn c_src/build/driver | sed -n '/<helperBad>:/,/ret/p'
0000000000401158 <helperBad>:
  ...
  mov    %rax,-0x20(%rbp)      # buffer is still written to the stack
  mov    %rdx,-0x18(%rbp)
  movb   $0x0,-0x10(%rbp)
  mov    $0x0,%eax             # ...and then discarded; returns NULL
  pop    %rbp
  ret
```

Build warning, for the record:

```
c_src/src/main.c:37:12: warning: function returns address of local variable
   [-Wreturn-local-addr]
```

`printLine`'s NULL guard therefore suppresses output entirely. The Rust
`helper_bad()` returns `None` to reproduce this. Two plausible translations are
wrong here and both would have been caught by `zero_takes_the_bad_branch`:

- returning the string (`Some("helperBad string")`) — the C prints **nothing**,
  so this adds a spurious 19-byte line on stdout;
- printing an empty line — the NULL branch of `printLine` skips the `\n` too,
  so stdout must be completely empty.

### 2. `scanf` failure leaves `x` at its initializer `0`, which routes to `bad()`

`scanf("%d", &x)` does not write to `x` on either input or matching failure, and
the C ignores the return value. `x` keeps the `int x = 0;` initializer and the
`else` branch runs. The Rust mirrors this by only overwriting `x` when the
conversion succeeds:

```rust
let mut x: i32 = 0;
if let Some(value) = scanner.scan_int() {
    x = value;
}
```

A translation that treated a parse failure as an error (nonzero exit, a message
on stderr, or a panic) would diverge on empty input, `"abc"`, `"-"`, `"+"`,
`".5"` and every whitespace-only input. Note that a stdout-only comparison would
*not* catch the exit-status half of that divergence, which is why every case
asserts all three streams.

### 3. `scanf` skips whitespace across newlines; it is not line-oriented

`%d` skips any run of whitespace, newlines included, before the first digit.
`"\n\n\n\n7"` yields 7 and prints, and `"  \n\t\n 42  extra"` yields 42. A
`fgets`- or `read_line`-based translation would see an empty first line and fail
the conversion, wrongly taking the bad branch. Covered by
`scanf_reads_across_newlines`, which includes all six C-locale whitespace bytes
(space, `\t`, `\n`, `\v`, `\f`, `\r`).

### 4. Overflow saturates at the `long` range, then truncates to `int`

glibc converts `%d` with `strtol` semantics — clamping to `LONG_MAX` /
`LONG_MIN` on overflow — and the store into `int x` truncates to the low 32
bits. This makes the branch taken counter-intuitive for several inputs, all
confirmed against the C binary:

| stdin | conversion result | `x` after truncation | branch |
| --- | --- | --- | --- |
| `4294967296` (2^32) | 4294967296 | 0 | **bad** |
| `12884901888` (3·2^32) | 12884901888 | 0 | **bad** |
| `-4294967296` | −4294967296 | 0 | **bad** |
| `9223372032559808512` (0x7fffffff00000000) | itself | 0 | **bad** |
| `-9223372036854775808` (LONG_MIN) | LONG_MIN | 0 | **bad** |
| `-999…9` (40 nines) | LONG_MIN (saturated) | 0 | **bad** |
| `2147483648` (2^31) | 2147483648 | −2147483648 | good |
| `4294967295` | 4294967295 | −1 | good |
| `18446744073709551616` (2^64) | LONG_MAX (saturated) | −1 | good |
| `999…9` (40 nines) | LONG_MAX (saturated) | −1 | good |

Two distinguishing observations pin down the semantics rather than assuming
them. Input `2^64` prints, which rules out modular wraparound (wrapping would
give `x == 0` and no output) and confirms saturation at `LONG_MAX`. Input
`4294967296` prints nothing, which confirms the narrowing store to `int`
actually truncates rather than saturating at `INT_MAX`. Covered by
`values_truncating_to_zero_take_the_bad_branch`,
`values_truncating_to_nonzero_take_the_good_branch` and
`powers_of_two_and_neighbours`.

### 5. Conversion stops at the first character that cannot extend the number

`"0x10"` converts as `0` and stops at `x` (bad branch, not 16); `"3.14"`
converts as `3`; `"1abc"` as `1`; `"5-3"` as `5`. Trailing input is never read.
A translation that read a whole token or a whole line and then required it to
parse in full would reject these instead of converting the prefix. Covered by
`conversion_stops_at_first_non_digit`.

### 6. A sign must be followed immediately by a digit

`"- 5"` and `"+ 9"` are matching failures, not −5 and 9: whitespace is skipped
only *before* the sign, never between the sign and the digits. Likewise `"--9"`
and `"+-3"` fail. Covered by `sign_without_digits_is_a_matching_failure`.

### 7. Digit runs far longer than any integer type

5,000-digit inputs (all zeros, all nines, negative nines, zeros followed by a
significant digit) match. `"0"×5000` is a valid conversion to 0 and takes the
bad branch; `"0"×5000 + "1"` converts to 1 and prints. Covered by
`very_long_digit_runs`.

### 8. NUL and non-ASCII bytes are ordinary non-digits

`\0`, `\xff` and UTF-8 lead bytes are neither whitespace nor digits, so they end
whitespace skipping and cause a matching failure if they appear before any
digit. `"5\0"` converts to 5 normally. The Rust scanner works on raw bytes
rather than `&str`, so no UTF-8 validation can reject input the C accepts —
`all_bytes` feeds it every byte 0x00–0xff in one input and
`every_single_byte_input` feeds each byte alone. Covered by `non_text_bytes`.

### 9. Write failures are ignored, exit status stays 0

The C ignores `printf`'s return value, so a closed or broken stdout still exits
0. The Rust discards the `write!` and `flush()` results (`let _ = ...`) for the
same reason; using `.unwrap()` or `println!` would panic or abort instead.
Checked out of band, since Cargo's test harness always supplies a live stdout:

```
$ echo 1 | c_src/build/driver 1>&-                       ; echo rc=$?   # rc=0, empty stderr
$ echo 1 | translation/target/release/driver 1>&-        ; echo rc=$?   # rc=0, empty stderr
$ echo 1 | c_src/build/driver | true                     ; echo rc=$?   # rc=0
$ echo 1 | translation/target/release/driver | true      ; echo rc=$?   # rc=0
```

## Environment dependency worth flagging

Item 1 is the one behavior that is a property of the *reference build* rather
than of the C language: `helperBad()` returning the address of a stack local is
undefined behavior, and GCC's choice to substitute NULL is what makes the bad
branch silent. A different compiler or optimization level could return the live
stack address instead, in which case the C would print `helperBad string` (or
garbage) and the Rust `None` would no longer match.

The test suite is self-correcting on this point rather than hard-coding an
expectation: it compares against whatever `c_src/build/driver` does, so a
toolchain whose codegen differs would surface as a failure in
`zero_takes_the_bad_branch` rather than as a silently wrong pass. Verified
against the GCC and default CMake configuration present in this working
directory.

## Reproduction

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test
```

`cargo test` builds the C reference itself if `c_src/build/driver` is absent, so
it can be run on its own. No test is `#[ignore]`d, skipped or conditionally
disabled, and nothing in `c_src/` was modified (`c_src/build/` holds only CMake
output).
