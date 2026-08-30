# Differential findings: C (`c_src/`) vs Rust (`translation/`)

The C program is the ground truth:

```c
int main() { float x = 0.f; scanf("%f", &x); driver(x); return 0; }
```

`driver` `memcpy`s the 4 bytes of the `float` and prints them as `%02x` plus a
newline. So the *entire* observable behaviour is "what bit pattern does
`scanf("%f", &x)` leave in `x`", with the crucial detail that **`scanf`'s return
value is ignored**: on EOF or a matching failure `x` keeps its initial `+0.f`
and the program still prints `00000000` and exits `0`.

## How it was tested

`tests/differential.rs` spawns both executables as subprocesses, feeds each the
same bytes on stdin, and compares stdout, stderr and exit status byte for byte.
32 tests cover ~50,000 inputs, including exhaustive enumeration of all strings
of length ≤ 3 over the float alphabet, all strings of length 4 over a core
alphabet, all 256 single bytes, all 256 second bytes after each of 9 prefixes,
and exact decimal midpoints between adjacent `f32` values.

---

## Mismatch 1 — a bare signed hex prefix loses its sign

**Status: found and fixed.**

| input  | C (`printf`) | Rust before fix | Rust after fix |
| ------ | ------------ | --------------- | -------------- |
| `-0x`  | `00000000`   | `00000080`      | `00000000`     |
| `-0X`  | `00000000`   | `00000080`      | `00000000`     |
| `-0xg` | `00000000`   | `00000080`      | `00000000`     |
| `-0xp` | `00000000`   | `00000080`      | `00000000`     |
| `-0xx` | `00000000`   | `00000080`      | `00000000`     |
| `-0x ` | `00000000`   | `00000080`      | `00000000`     |

`00000080` is little-endian `-0.0f`; `00000000` is `+0.0f`.

**Cause.** The Rust translation modelled `%f` the way `strtof` is specified:
scan the longest prefix that forms a valid subject sequence, and if that prefix
is shorter than what was consumed, fall back to it. For `-0x` the longest valid
prefix is `-0`, so `strtof` semantics give `-0.0` — and indeed
`strtof("-0x", NULL)` really does return `-0.0`.

glibc's `scanf` does **not** do that. It accumulates the candidate characters
into a buffer and hands the buffer to `strtof`, but *before* that it explicitly
rejects the case where the buffer is nothing but an (optionally signed) `0x` /
`0X` hex prefix, and reports a **matching failure** instead of converting. A
matching failure means `x` is never written, so the printed value is the
initial `+0.f` — the sign the user typed is silently dropped.

The boundary is exactly "did anything get added after the `0x`":

- `-0x`, `-0xg`, `-0xp` → matching failure → `00000000` (`+0.0`)
- `-0x.`, `-0x..`, `-0x.g`, `-0x.p` → converted → `00000080` (`-0.0`)

A single `.` is enough to make glibc accept, because the `.` *is* appended to
the buffer and `strtof("-0x.")` then converts the `-0` prefix normally.
`+0x` and `0x` are unaffected only by coincidence: `+0.0` and the untouched
`+0.f` have the same bit pattern.

**Fix** (`src/main.rs`, in `scan_float`): after running the subject-sequence
DFA, if the DFA never left the hex-prefix state, return a matching failure
rather than falling back to the `0` prefix:

```rust
if state == St::HexPrefix {
    return None;
}
```

Regression test: `signed_bare_hex_prefix_loses_its_sign`, which pins both the
failing forms and the `-0x.` forms that must still convert to `-0.0`.

---

## C behaviours that look like bugs and were deliberately preserved

These produced no mismatch, but they are the non-obvious parts of the C that
the translation has to keep reproducing, so they are recorded here with the
tests that lock them down.

### A truncated exponent backs out and keeps the mantissa

`1e` → `0000803f` (= `1.0f`), `1e+` → `0000803f`, `0x1p` → `0000803f`,
`0x1p+` → `0000803f`.

Despite the usual claim that `scanf` can only push back one character, glibc
retreats over the whole incomplete exponent and converts just the mantissa.
It does *not* report a matching failure. Test:
`exponent_forms_including_truncated_ones`, `hex_forms_including_truncated_ones`.

### A truncated `infinity` is a matching failure, not `inf`

`inf` → `0000807f` (`+inf`), but `infi`, `infin`, `infini`, `infinit` all →
`00000000`.

Once glibc has seen the `i` after `inf` it is committed to matching the rest of
`infinity`; when that fails it cannot fall back to the `inf` it already had, so
the whole conversion fails and `x` keeps `+0.f`. `strtof` would have returned
`inf` here. This asymmetry with the exponent case above is genuine glibc
behaviour and is reproduced explicitly (`St::Infi | St::Infin | St::Infini |
St::Infinit` → matching failure). Test:
`truncated_infinity_prefixes_are_matching_failures`.

### `0x` with no hex digits converts the leading `0`

`0x` → `00000000`, `0xg` → `00000000`, `0x.` → `00000000`. Unsigned, this is
indistinguishable from a matching failure; see Mismatch 1 for how the signed
case reveals which it actually is. Test:
`hex_prefix_without_digits_falls_back_to_zero`.

### `scanf` reads across newlines; `fgets` would not

`%f` skips *any* leading whitespace, newlines included, so `"\n\n\t 3.25"`
converts `3.25` and `"\n\n\n\n7"` converts `7`. Conversely a complete number
stops at the first byte that cannot extend it, and the rest is simply never
read: `"1 2"`, `"1\n2"` and `"1abc"` all yield `1.0f`. Tests:
`leading_whitespace_is_skipped_and_newlines_crossed`,
`stops_at_first_non_matching_byte`.

### Signed zero and NaN bit patterns

`-0` → `00000080`, `nan` → `0000c07f` (`0x7fc00000`), `-nan` → `0000c0ff`
(`0x ffc00000`). The sign bit of a NaN is observable through the hexdump, so
`-nan` must negate rather than produce a canonical NaN. `nan(...)` with an
n-char-sequence, and the truncated `nan(` (which still converts to NaN), are
covered too. Tests: `signed_zeros`, `nan_forms`.

### Range errors are silent

Overflow saturates to `±inf` (`1e40` → `0000807f`) and underflow decays through
the subnormals to `±0` (`0x1p-150` → `00000000`); `errno`/`ERANGE` is never
inspected, so nothing appears on stderr and the exit status stays `0`. Tests:
`overflow_and_underflow_range_errors`,
`subnormal_and_overflow_boundary_literals`.

### Rounding must be correctly rounded to `f32`, with no double rounding

Inputs that sit exactly halfway between two `f32` values must round to even,
including in the subnormal range. Naively parsing to `f64` and casting to `f32`
would round twice and get some of these wrong, so the hex path in
`hex_to_f32` rounds to odd at 53 significant bits (with a sticky bit for
digits beyond the 128-bit accumulator) before the final `as f32`. Tests:
`rounding_boundaries`, `exact_decimal_midpoints_between_adjacent_floats`,
`exact_float_values_and_hex_round_trips`.

---

## Result

No remaining differences. Every input tested — including all ~21,000 exhaustive
short strings, all 256 single bytes, and ~26,000 rounding-boundary literals —
gives identical stdout, stderr and exit status from both programs.
`cargo test` passes with 32 tests, none ignored or skipped. Nothing in
`c_src/` was modified.
