# ERRORS.md — differential verification of `c_src` vs `translation`

Ground truth: `c_src/src/main.c`, built with CMake (`c_src/build/driver`).
Under test: `translation` (`target/{debug,release}/driver`).

The whole program is:

```c
typedef union { uint64_t x; double f; } raw_double_t;

void driver(double f) {
    raw_double_t u = {.f = f};
    printf("%llx %a %.4f\n", u.x, f, f);
}

int main() {
    double f = 0.0f;
    scanf("%lf", &f);
    driver(f);
    return 0;
}
```

So the observable surface is exactly three things: glibc's `scanf("%lf")`
conversion, glibc's `%llx`/`%a`/`%.4f` formatting, and the fact that `f` keeps
its `0.0` initialiser whenever `scanf` fails. `stderr` is always empty and the
exit status is always `0`, for every input — including empty input, junk input
and multi-kilobyte input. The tests assert all three streams anyway, since a
translation that exited non-zero on a parse failure would otherwise pass.

## Mismatches found and fixed

### 1. Shift-left overflow in `cscan::make_f64` (real defect, fixed)

**Symptom.** Any hexadecimal input whose significand rounds *up* and carries
into a new binade aborted the Rust program in a `debug` build:

```
$ printf '0x1.fffffffffffff8p0' | ./target/debug/driver
thread 'main' panicked at src/cscan.rs:462:27:
attempt to shift left with overflow
```

C output for the same input: `4000000000000000 0x1p+1 2.0000`, exit 0.
Rust `debug` output: empty stdout, a panic message on stderr, exit 101 — a
mismatch on all three of stdout, stderr and exit status.

**Cause.** `hex_to_f64` rounds the 53-bit quotient `q` to nearest/even. That
round-up can carry `q` all the way to `2^53`, i.e. 54 significant bits.
`make_f64` then computed

```rust
let q_bits = 64 - q.leading_zeros() as i64;   // 54
let significand = q << (53 - q_bits);         // q << -1
```

a shift by `-1`. In a `release` build the shift amount is masked to 63, `q <<
63` overflows `u64` to `0`, the fraction field comes out `0` and the answer is
*accidentally* correct (the carry case is always an exact power of two, so a
zero fraction is what was wanted). In a `debug` build the overflow check fires
and the process aborts. Either way the code was relying on wrap-around
behaviour that is not part of the contract.

**Fix.** Re-normalise before encoding, so `q` never exceeds 53 bits:

```rust
while 64 - q.leading_zeros() as i64 > 53 {
    q >>= 1;   // the carry can only produce a power of two, so no bits are lost
    e2 += 1;
}
```

**Inputs that reproduced it**, now covered by
`phase_c_hex_rounding_boundaries` and `phase_c_exponent_range_sweep_shard_4`:
`0x1.fffffffffffff8p0`, `0x1.fffffffffffff8pN` and `0x1.ffffffffffffffpN` for
essentially every exponent `N`, plus the subnormal/normal crossover cases
`0x0.fffffffffffff8p-1022` and `0x1.fffffffffffff8p-1023`.

## Behaviours verified as already matching

These were each candidates for a mismatch and were checked explicitly; no
difference was found, so no code changed. They are listed because "checked and
identical" is the useful record.

| Behaviour | Inputs used | Result |
| --- | --- | --- |
| `scanf` failure leaves `f == 0.0` — output is `0 0x0p+0 0.0000`, exit `0`, empty stderr | `""`, `" "`, `"\n"`, `abc`, `.`, `-`, `+`, `e5`, `--1`, `\xff`, `\0` | identical |
| `scanf` skips whitespace *across newlines* (unlike `fgets`) | `"\n\n\n\n\n-7.5"`, `" \r\n\x0b\x0c\t 0x1.8p3"` | identical |
| Only the first item is consumed; the rest of stdin is ignored | `1 2`, `1\n2`, `1.5abc`, `9.5\0trailing` | identical |
| `%a` leading digit and exponent for subnormals (`0x0.…p-1022`, not a normalised form) | full binade sweep `-1080..=1024` | identical |
| `%a` for zero is special-cased to `0x0p+0` / `-0x0p+0` | `0`, `-0`, `-0.0e10`, `-0x0p0` | identical |
| `%a` trailing-zero suppression, including dropping the `.` entirely | `0x1p0`, `0x1.8p1`, `0x1.0000000000001p0` | identical |
| `%a`/`%.4f` spell non-finites `inf`/`-inf`/`nan`/`-nan` from the sign bit | `inf`, `-inf`, `nan`, `-nan`, `INFINITY`, `iNfInItY` | identical |
| `%.4f` ties round half-to-even (values `odd/32` terminate exactly at the 5th fractional digit, so a true tie is reachable) | `0.03125`, `0.09375`, `0.15625`, `0.53125`, `1.03125`, `2.03125` | identical |
| NaN payload from `nan(n-char-seq)` is **not** propagated into the bit pattern | `nan(1)`, `nan(1234)`, `nan(0xfffff)`, `-nan(1)` | identical (`7ff8000000000000` / `fff8000000000000`) |
| Partially-matched `inf`/`infinity`/`nan` words are matching failures, with the consumed bytes not restored | `infinit`, `infini`, `inft`, `na`, `nan(abc`, `i`, `n` (× sign/suffix combinations) | identical |
| A bare `0x` prefix with no hex digit and no radix character is a matching failure, but `0x.` / `0x.p5` convert the leading `0` | `0x`, `0X`, `-0x`, `0xp0`, `0xz` vs `0x.`, `0x.p5`, `0x.5` | identical |
| Truncated exponents leave the mantissa converted | `1e`, `1e+`, `1e-`, `1.5e`, `1e5e5`, `1ee5` | identical |
| Absurd exponents saturate to `inf` / `0` rather than wrapping | `1e999999999999999999999`, `1e-999999999999999999999`, `0e999999999999999999999`, `0x1p16384`, `0x1p-16384` | identical |
| Overflow / underflow / subnormal boundaries | `1e309`, `1e-324`, `5e-324`, `2.2250738585072009e-308`, `1.7976931348623159e308`, `0x1p-1075` | identical |
| 53-bit integer significand rounding | `9007199254740993`, `9007199254740995`, `18014398509481985` | identical |
| Inputs longer than any stdio buffer | 5 000-digit mantissa, 5 000-char fraction, 20 000 newlines, 10 000 junk bytes, 1 000+1 000-digit value | identical |
| Non-UTF-8 and embedded-NUL bytes | `\xff\xfe\xfd`, `\x801.5`, `\xc3\x28`, `1\x002` | identical |

## Coverage of the C's branch structure

`main`/`driver` contain no `if`, so the input classes come from the library
calls the C makes:

* `scanf` returns `1` → `f` is the converted value.
* `scanf` returns `0` (matching failure) → `f` stays `0.0`.
* `scanf` returns `EOF` (empty / whitespace-only input) → `f` stays `0.0`.
* `%a` has four shapes: zero, subnormal, normal, non-finite — each × sign bit.
* `%.4f` has three shapes: non-finite, and finite rounding down / up / at a tie.
* `%llx` is a plain 64-bit hex dump, covered by the full binade sweep.

The suite in `tests/differential.rs` covers all of them. The two random sweeps
(`phase_c_deterministic_random_sweep`, seeded xorshift, no external crates) and
the four binade shards add roughly 15 000 further input pairs. During
development the same comparison was additionally driven from an external
harness for ~200 000 randomly generated inputs (random bit patterns re-fed as
exact hex literals, random decimal and hex-float strings, and random byte soup
over the alphabet the scanner branches on), against both the `debug` and
`release` Rust builds, with no remaining differences.

## Status

* Both programs build without errors.
* `cargo test` and `cargo test --release` pass: 22 tests, 0 failed, 0 ignored.
* No test is `#[ignore]`d, skipped or disabled.
* Nothing under `c_src/` was modified; only `c_src/build/` (CMake output) was
  added, as the build instructions require.
