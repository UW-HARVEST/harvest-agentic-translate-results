# ERRORS.md — differential verification record

Scope: `c_src/src/main.c` (the ground truth) vs `translation/src/main.rs`.

The whole program is: read one `float` with `scanf("%f", &x)` (with `x`
pre-initialised to `0.f`), then print the four bytes of its object
representation as lowercase `%02x` followed by `\n`, then `return 0`. There is
no output on stderr and the exit status is always 0, so every input class below
was checked on all three channels: stdout bytes, stderr bytes, exit status.

## Mismatches found

**None.** Every input class enumerated below produced byte-identical stdout,
byte-identical stderr (empty in all cases) and an identical exit status (0).

That is an unusual result, so the sections that follow record what was actually
exercised, and how the test harness itself was validated — an all-green suite
is only meaningful if it can fail.

## Harness validation (mutation testing)

To prove the comparison is real rather than vacuous, `src/main.rs` was
temporarily mutated three times and the suite re-run. The original file was
restored and byte-compared (`cmp`) afterwards.

| Mutation | Result |
| --- | --- |
| `to_ne_bytes()` → `to_be_bytes()` (byte order) | suite FAILED, reported as `stdout mismatch` |
| `return 0` → `std::process::exit(1)` | suite FAILED, reported as `exit status mismatch` |
| scanf matching-failure path returns `Some(0.5)` instead of `None` | suite FAILED (5 tests) |

So a wrong value, a wrong exit status, and wrong failure semantics are all
caught.

## Behaviours that were specifically checked because they are easy to get wrong

These are the places a translation would most plausibly diverge. Each was
confirmed identical rather than assumed.

1. **`scanf` failure leaves `x` untouched.** On matching failure (`abc`, `.`,
   `+`, `0x`, `infin`) and on input failure (empty stdin, whitespace-only,
   closed fd 0) nothing is stored, so the program prints `00000000` and still
   exits 0. A translation that defaulted to NaN, or that exited non-zero on a
   parse error, would diverge here. Covered by `empty_input_leaves_x_at_zero`
   and `matching_failure_paths`.
2. **Whitespace is skipped across newlines.** `%f` skips leading
   ` \t\n\v\f\r`, so `"\n\n\n\t\v\f\r  -2.5"` parses. Only the first token is
   read: `1.5 2.5` and `1.5\n2.5` both yield `1.5`. Covered by
   `whitespace_skipping_and_early_stop`.
3. **Incomplete exponents are pushed back, not consumed.** `1e`, `1e+`, `1.5e`,
   `0x1p`, `0x1p-` all parse as the significand alone and succeed; they are not
   matching failures. Covered by `incomplete_exponent_is_pushed_back`.
4. **`0x` with no significand is a matching failure, not the value 0.** Once
   the scanner commits to the hex prefix it requires a hex digit or a radix
   point, so `0x`, `0X`, `0xg`, `0xp1` store nothing and print `00000000` —
   which happens to look like a successful parse of `0`, but is reached by a
   different path. Covered by `matching_failure_paths` and `hex_float_forms`.
5. **`inf` vs `infinity` prefix commitment.** `inf` succeeds; `infi`, `infin`,
   `infini`, `infinit` are matching failures; `infinityx` succeeds and stops
   before `x`. Covered by `infinity_and_prefixes`.
6. **`nan` with an n-char-sequence payload.** `nan(1)`, `nan(abc_123)`,
   `nan(abc` (unterminated) and `-nan(0x7)` all yield the same default quiet
   NaN pattern as bare `nan`; the payload never reaches the stored value. The
   NaN sign bit is preserved for `-nan`. Covered by `nan_forms`.
7. **C99 hex floats are accepted by glibc's `%f`.** `0x1p3`, `0x1.8p1`,
   `0x.8p1`, `0x1.8` (no exponent), and 40-hex-digit significands with `p-200`
   all round identically. Covered by `hex_float_forms`.
8. **Correct rounding, including exact ties.** Exact midpoints between adjacent
   binary32 values were generated with big-decimal arithmetic (no floating
   point) for mantissa/exponent pairs spanning subnormals through the top
   binade, each tested bare (ties-to-even), with a `1` appended, with
   `0000000000000000001` appended, and negated. Also `16777215/16/17/18`,
   `16777216.5`, `16777218.5`, and `1.000000059604644775390625`. Covered by
   `exact_ties_across_the_exponent_range` and `rounding_boundaries`.
9. **Overflow / underflow edges.** `FLT_MAX` written out in full (39 digits),
   the exact overflow threshold `340282356779733661637539395458142568448` and
   the value one below it, smallest normal, largest subnormal, smallest
   subnormal, exactly half the smallest subnormal (ties to zero) and one ulp of
   decimal either side of it. `0x1p128`, `0x1p-150`, `0x1.0000001p-150`.
   Covered by `magnitude_extremes` and `hex_float_forms`.
10. **Signed zero.** `-0`, `-0.0`, `-0e5`, `-0x0p0` produce `00000080` (sign bit
    set), not `00000000`. Covered by `single_value_decimal`,
    `decimal_exponent_forms`, `hex_float_forms`.
11. **Exponents that overflow every integer width.** `1e2147483647`,
    `1e-2147483648`, `1e9223372036854775807`, `1e-9223372036854775808`,
    `1e+9999999999999999999999`, a 70-digit exponent, and `0e999999999999999999`
    (zero significand with an absurd exponent, which must stay zero rather than
    becoming inf). Covered by `absurd_exponents`.
12. **Very long significands.** 40 to 100 000 digits, including lengths
    straddling the 800-digit truncation point in the Rust decimal path
    (`800`, `801`, `900`), and 5000-hex-digit significands. Covered by
    `very_long_significands`.
13. **Bytes that are not valid UTF-8, and embedded NULs.** The C program is
    byte-oriented; the Rust one reads raw bytes and must not choke. `\xff`,
    `\x80\x81`, `\xc3\x28`, `1.5\xff`, `\x00`, `\x001.5`, a UTF-8 BOM before
    the number, and U+2009 (a Unicode space that is *not* C whitespace, so it
    is a matching failure). Covered by `non_utf8_and_nul_bytes`.
14. **Environmental edge cases** (checked manually, outside the test file, as
    they need shell redirection): stdin closed (`0<&-`), stdin `/dev/null`,
    stdout closed (`>&-`), stdout `/dev/full` (write always fails), and extra
    `argv` entries. All four produced identical exit status and stderr; neither
    program reports a write failure or inspects `argv`.

## Notes on the translation's approach

The Rust program reimplements the `%f` scanner and `strtof` rather than calling
libc. That is a correctness risk on paper, and it is why the tie-breaking,
truncation-boundary and absurd-exponent classes above were tested so heavily
rather than sampled. Two implementation details are worth flagging for a future
reader:

- The 800-digit truncation with a sticky digit in `decimal_to_f32` is only safe
  because 800 digits is far past the longest exact halfway case for binary32.
  Tests at 800/801/900/2000/5000 digits confirm it empirically.
- `parse_hex_float` does its own shift/round arithmetic in `u128`. The suite was
  run in the debug profile as well as release, so any arithmetic overflow on
  these paths would have panicked (debug overflow checks) and been reported as
  an exit-status mismatch. It did not.

## Residual risk

Verification is empirical, against glibc on this Linux/x86-64 host. Two things
are therefore *not* proven:

- `scanf`'s treatment of `nan(payload)`, and the exact NaN bit pattern, are
  implementation-defined. On a libc that forwards the payload, or a platform
  with a different default NaN encoding, the C output would change and the Rust
  program — which hardcodes `0x7fc00000` — would not follow.
- The C program uses the native byte order via `memcpy`; the Rust program uses
  `to_ne_bytes()`. These agree on any single target, including a big-endian one.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd translation && cargo build --release && cargo test
```

`tests/differential.rs` will build the C reference itself (out-of-source, under
`translation/target/`) if `c_src/build/driver` is absent, so `cargo test` is
self-contained. Nothing in `c_src/` is written to other than the `build/`
directory created by the documented CMake invocation; `c_src/src/main.c` and
`c_src/CMakeLists.txt` are unmodified.
