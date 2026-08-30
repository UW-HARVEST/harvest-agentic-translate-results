# Differential verification log

Ground truth: `c_src/src/main.c`, built with cmake (`c_src/build/driver`).
Candidate: `translation/src/main.rs`, built with `cargo build --release`
(`translation/target/{debug,release}/driver`).

Comparison method: both programs are spawned as subprocesses, fed identical
bytes on stdin, and their **stdout, stderr and exit status** are compared byte
for byte (`translation/tests/differential.rs`). The Rust code is never loaded as
a library.

## What the C program actually does

```c
int main() { float x = 0.f; scanf("%f", &x); driver(x); return 0; }
```

There is no branching in the C file itself: `print_hex` always writes exactly
`sizeof(float)` == 4 bytes as `%02x` plus a `\n`, and `main` always returns 0.
Every input class therefore lives inside `scanf("%f")`. The observable surface is
exactly:

* stdout: 8 lowercase hex digits + `\n` — the object representation of the
  resulting `float` in host byte order (little-endian x86-64 here)
* stderr: always empty
* exit status: always 0

Consequently **no input can change the exit status or stderr**, and the only way
to diverge is to produce a different `float`. The tests still assert all three
channels on every input, as required.

## Mismatches found

**None.** Every input in the enumerated corpus produced identical stdout,
stderr and exit status. The corpus driven through both binaries during this
verification pass (integration suite plus ad-hoc sweeps) was roughly 30,000
distinct inputs, covering:

* empty input, whitespace-only input, EOF with no token
* leading whitespace of every C-locale kind, including newlines (`scanf` skips
  across them)
* single well-formed item, plus trailing junk after the item
* all decimal grammar edges: `.5`, `5.`, `.`, `-.`, `..`, `1.2.3`, leading zeros
* exponent edges: `1e`, `1e+`, `1e-x`, `1.e5`, `e5`, `1e 5`
* signs, including `--1`, `+ 1`, bare `-`, bare `+`, and signed zero
* `inf` / `infinity` in every case mix, plus every prefix of `infinity`
* `nan`, `nan()`, `nan(payload)`, unterminated `nan(abc`
* hex floats: `0x1p3`, `0x.8p1`, `0x8.p1`, `0x1p` with no exponent digits,
  `0xg`, and both no-digit forms `0x` and `0x.`
* overflow to `inf`, underflow to `0`, the whole subnormal range, and
  round-to-nearest-even ties at the 24-bit boundary
* exponents far outside `int` range in both the decimal and hex forms
* tokens up to 200,000 characters and 500,000 bytes of leading whitespace
* raw non-text bytes: `\0`, `\xff`, `\x80`, UTF-8 sequences, BOM
* every single non-whitespace ASCII byte alone; all 676 two-byte combinations
  over the alphabet the scanner branches on; ~3,900 pseudo-random tokens and
  fragment concatenations

## Behaviours that were checked because a naive translation gets them wrong

These are not mismatches — the translation already handles each one — but they
are the places where this program is easy to get wrong, so they are recorded
with the input that pins them down and are each covered by a test.

| Input | Correct output | Why it is a trap |
| --- | --- | --- |
| `infin` | `00000000` | glibc's `scanf` commits to `infinity` as soon as a 4th `i` follows `inf`; when the rest does not arrive it reports a *matching failure* and assigns nothing, so `x` stays `0.f`. A translation that calls `strtof`-like logic returns `inf` here. Same for `infi`, `infini`, `infinit`. |
| `-0x` vs `-0x.` | `00000000` vs `00000080` | `0x` with no hex digits is rejected outright (no assignment, sign lost). Adding a radix point makes the token acceptable to the converter, which converts the leading `0` — so the sign survives and the result is **negative** zero. |
| `-nan` | `0000c0ff` | The sign bit is applied to the NaN. |
| `nan(1)`, `nan(0x7fffff)` | `0000c07f` | glibc's `scanf` path ignores the payload; it does **not** fold it into the significand. A translation that implements the `strtod` payload extension would print a different mantissa. |
| `0x1.0000003p0` | `0000803f` | Round-to-nearest, **ties to even**, on the dropped low bits — the result is exactly `1.0`, not the next float up. |
| `1e` , `1e+` , `0x1p` | value of `1` / `1` / `0x1` | The exponent marker only participates when at least one digit follows it; otherwise it is not part of the subject sequence. |
| `1e400`, `0x1p128` | `0000807f` | Overflow yields `+inf` (`HUGE_VALF`), not a saturated finite value or a panic. |
| `0x1p-150` | `00000000` | Underflow below `2^-149` rounds to zero (ties to even on the subnormal boundary), it does not clamp to the smallest subnormal. |
| `1e99999999999999999999` | `0000807f` | The exponent does not fit in any integer type; it must be clamped rather than wrapped. Signed-overflow wrapping would flip it to an underflow and print `00000000`. |
| `\n\n\n3.25` | `0000503f` | `scanf` skips newlines. A `fgets`/read-a-line translation would see an empty line and print `00000000`. |
| `0.` + 200,000 `0`s + `1` | `00000000` | Long tokens must not overflow the significand accumulator into a wrong answer. |
| output encoding | `to_ne_bytes` | The C program dumps raw memory, so the byte order is the host's. `to_be_bytes` reverses every non-palindromic result. |

## Suite sensitivity check

To confirm the tests are not vacuous, the Rust source was temporarily mutated
and `cargo test` re-run (all mutations reverted afterwards; `src/main.rs` was
verified byte-identical to its original):

| Mutation | Result |
| --- | --- |
| return `inf` for `infin` instead of a matching failure | 3 tests fail |
| dump bytes big-endian (`to_be_bytes`) | 19 tests fail |
| round half-up instead of half-to-even | 2 tests fail |
| drop the sign on the `-0x.` path | 3 tests fail |
| return `Some(0.0)` for bare `0x` instead of a matching failure | *no failure — equivalent mutant.* `x` is initialised to `0.f`, so "assign nothing" and "assign +0.0" are indistinguishable on stdout. |
| stop consuming the `nan(...)` payload | *no failure — equivalent mutant.* Nothing reads stdin after the single conversion, so how much input was consumed is unobservable. |

The two survivors are genuinely unobservable through this program's interface,
not gaps in the tests.

## Files touched

* `translation/tests/differential.rs` — added (the differential suite)
* `translation/ERRORS.md` — this file
* `c_src/**` — **unmodified**; only the untracked cmake output directory
  `c_src/build/` was created, as the build instructions require.
