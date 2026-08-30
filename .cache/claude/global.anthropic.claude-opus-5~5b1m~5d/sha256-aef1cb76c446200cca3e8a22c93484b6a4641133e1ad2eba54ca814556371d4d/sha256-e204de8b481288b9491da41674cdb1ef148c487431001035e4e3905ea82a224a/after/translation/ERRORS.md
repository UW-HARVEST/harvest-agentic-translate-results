# Differential testing report: `c_src` vs `translation`

The program under test reads a single `float` with `scanf("%f")` and prints the
four raw bytes of that `float` as lowercase hex followed by a newline:

```c
int main() {
    float x = 0.f;
    scanf("%f", &x);
    driver(x);      /* prints the object representation of x, then '\n' */
    return 0;
}
```

Because `x` is pre-initialised to `+0.0f` and `scanf`'s return value is ignored,
**a matching failure is observable**: it prints `00000000`. That makes the
distinction between "matched and produced zero" and "did not match at all"
visible only through the *sign bit* — `-0.0f` prints `00000080`. Several of the
bugs below live exactly in that gap.

The C program has no error paths of its own: it always `return 0`, never writes
to stderr, and the `print_hex` loop always runs `sizeof(float) == 4` times. So
all the interesting input classes are in glibc's `%f` grammar.

Reference behaviour was established by (a) running the built C binary and (b) a
scratch probe program that also printed `scanf`'s return value and the next
unread character, to distinguish matching failure from a successful conversion.

## Mismatches found and fixed

### 1. `-0x` (hex prefix with no hex digits) produced `-0.0` instead of `+0.0`

| input | C | Rust (before) |
|---|---|---|
| `-0x` | `00000000` | `00000080` |
| `-0xp3` | `00000000` | `00000080` |
| `-0X`, `-0XP-2`, `-0xz`, … | `00000000` | `00000080` |

**Cause.** `scan_hex` treated "`0x` with no hex digits" as a *successful*
conversion of the leading `0`, returning a signed zero:

```rust
if !any_digit {
    // "0x" with no hex digits: the leading '0' converts to zero.
    return if neg { -0.0f32 } else { 0.0f32 };
}
```

The probe shows glibc actually reports a **matching failure** here, so `x` is
never assigned and keeps its initial `+0.0`:

```
-0x    -> ret=0 bytes=00000000 nextchar=EOF
0x     -> ret=0 bytes=00000000 nextchar=EOF
```

Note this is invisible for the unsigned spelling `0x`, since a matching failure
and a `+0.0` conversion both print `00000000`. Only the negative form exposes it.

**Fix.** `scan_hex` now returns `Option<f32>` and yields `None` in this case, so
`main` leaves `x` at `+0.0`.

### 2. `-0x.` (hex prefix, no digits, but a `.`) must still convert, as signed zero

This is the counterpart to bug 1 and constrains the fix: glibc accepts a `.`
in place of a hex digit, and the conversion *succeeds* with a signed zero. It
does not even look at a following `p` exponent:

```
0x.     -> ret=1 bytes=00000000 nextchar=EOF
-0x.    -> ret=1 bytes=00000080 nextchar=EOF
-0x.g   -> ret=1 bytes=00000080 nextchar=103(g)
-0x.p3  -> ret=1 bytes=00000080 nextchar=112(p)     <- 'p' left unread
```

So the rule is: after `0x`, at least one hex digit **or** a `.` is required.
A naive "no digits ⇒ matching failure" fix would have broken `-0x.`
(`00000080`) into `00000000`. `scan_hex` now tracks `saw_dot` and distinguishes
the two.

### 3. Explicit exponent was clamped to ±10^6, which a long significand can cancel

| input | C | Rust (before) |
|---|---|---|
| `1` + 1000060 × `0` + `e-1000060` (= 1.0) | `0000803f` | `0000807f` (inf) |
| `1` + 2000000 × `0` + `e-2000000` (= 1.0) | `0000803f` | `0000807f` (inf) |
| `0x1` + 250010 × `0` + `p-1000041` (= 2^-1) | `0000003f` | `00008053` |
| `0x1` + 250010 × `0` + `p-1000040` (= 2^0) | `0000803f` | `00008053` |
| `0x0.` + 250010 × `0` + `1p1000045` | `00000040` | `00008029` |

**Cause.** Both the decimal and hex exponent readers saturated the exponent
literal at 10^6:

```rust
v = v.saturating_mul(10).saturating_add((ch - b'0') as i64);
if v > 1_000_000 { v = 1_000_000; }
```

A clamp is only sound if the clamped magnitude cannot be cancelled by the
digits of the significand. It can be: `1` followed by 1000060 zeros already
carries a factor of 10^1000060, so `e-1000060` is genuinely `1.0`, not
infinity. glibc computes the effective exponent against the digit count and
gets this right.

The decimal path compounded the problem by formatting the raw exponent into a
string for `str::parse::<f32>()`, so the clamped value was what got parsed.

**Fix.**
* The saturation bound is now `EXP_CAP = 10^18`, far above any achievable digit
  count, so a saturated exponent can never be cancelled back into range.
* The decimal path renormalises the exponent against the digit count *before*
  formatting: the value is rewritten as `0.<significant digits>e<E>` where
  `E = exp - frac_len + ndigits`, which is always small (`|E| ≤ 61` for anything
  that is not a definite overflow/underflow). Long significands stay exact and
  the formatted exponent is always tiny.
* The hex path accumulates `exp_adj - 4*frac_count + pexp` in `i128`. The final
  clamp to ±100000 is retained but is now provably safe: `m` holds at most 128
  bits, so any exponent beyond that range is unambiguously overflow or flush to
  zero.

### 4. Broken stdout pipe: C dies from `SIGPIPE`, Rust exited 0

With stdout a pipe whose read end is already closed:

```
C returncode = -13   (killed by SIGPIPE)
R returncode = 0
```

**Cause.** The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so the
write fails with `EPIPE`, which the translation discards (`let _ = ...`) and
exits 0. The C program inherits the default disposition and is killed.

**Fix.** `restore_default_sigpipe()` resets `SIGPIPE` to `SIG_DFL` at the top of
`main`. Verified that writing to `/dev/full` still exits 0 in both programs
(neither checks `printf`'s return value), so only the signal case changed.

## Behaviours confirmed identical (no change needed)

These were checked because they looked like plausible divergences:

* **`nan(...)` payloads.** `strtod` sets a NaN mantissa from `nan(n-char-seq)`,
  but glibc's `scanf("%f")` does **not**: it matches the bare `nan` and leaves
  `(` unread, always yielding `0x7fc00000`. The translation's paren-consuming
  loop could not change the returned value, but it was removed to match glibc.
  `-nan` correctly gives `0xffc00000`.
* **Truncated `inf`/`nan` spellings.** `i`, `in`, `infi`, `infin`, `infinit`,
  `n`, `na` are all matching failures (glibc commits to the full `infinity`
  spelling once an `i` follows `inf`), so all print `00000000`.
* **Dangling exponents.** `1e`, `1e+`, `1ex`, `0x1p`, `0x1p+`, `0x1pz` all
  convert using exponent 0 (`1.0`), with the offending characters left unread.
* **Leading-`.` failures.** `.`, `-.`, `.e5`, `e5`, `-e5`, bare `+`/`-` are
  matching failures.
* **`scanf` reads across newlines** (unlike `fgets`): leading `\n`, `\t`, `\r`,
  `\v`, `\f` are skipped, so `"\n\n\n   \t 1.5\n2.5"` reads `1.5`.
* **Rounding.** All exact halfway cases agree, including the FLT_MAX↔infinity
  midpoint (ties to even ⇒ infinity), the 0↔2^-149 midpoint (ties to even ⇒
  zero), subnormals, and hex ties with sticky bits set far below the rounded
  digit.
* **Signed zero on underflow.** `-1e-46` gives `-0.0` (`00000080`) in both.
* **Other I/O modes.** Empty stdin, closed stdin, `/dev/null` stdin, and
  stdout to `/dev/full` all agree (exit 0, `00000000` where applicable).
* **Binary input.** Embedded NULs and bytes ≥ 0x80 agree; every possible single
  leading byte was tested, alone and followed by `1.5`.

## Test coverage

`tests/differential.rs` runs both binaries as subprocesses and compares stdout,
stderr, exit code **and terminating signal** (the signal is compared explicitly,
since a signalled process reports `code() == None` and comparing only exit codes
would have hidden bug 4).

Input classes covered: empty input, whitespace-only, simple values, whitespace
skipping across lines, trailing junk, matching failures, sign-only input, all
hex forms including the `0x`/`0x.` corner cases and dangling `p`, hex precision
and subnormal boundaries, every `inf`/`nan` spelling plus all truncated ones,
`nan(...)`, overflow/underflow, exact ties, many-digit significands, the
exponent-clamp regressions from bug 3, significands up to 2 MB, every single
leading byte 0x00–0xff, embedded NULs, the broken-pipe case, and five
deterministic fuzzers (token soup, random bytes, structured decimals,
structured hex, subnormal/overflow boundaries).

Beyond the committed suite, roughly 60,000 additional cases were compared
during investigation via an out-of-tree Python harness (random byte strings,
grammar-directed forms, and exact tie corpora generated with `Fraction`/
`Decimal` and `float.hex`). After the four fixes, zero mismatches remain.

Each fix was mutation-checked: reverting it individually makes the corresponding
test fail, confirming none of the regression tests are vacuous.
