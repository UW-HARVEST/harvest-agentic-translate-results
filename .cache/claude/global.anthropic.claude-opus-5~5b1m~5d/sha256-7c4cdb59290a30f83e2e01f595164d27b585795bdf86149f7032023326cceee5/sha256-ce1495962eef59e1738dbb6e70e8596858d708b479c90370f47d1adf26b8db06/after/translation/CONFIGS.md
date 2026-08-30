# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source and the public header.

## Axis derivation

### Axis 1 — runtime options / modes / flags: **NONE**

Grepping the public header and the implementation for option state finds nothing:
there are no setters, no context/handle struct, no global or `static` variables,
no `#ifdef`/`#if` compile-time switches in the implementation, and no flags
parameter. `driver` takes one `double` by value and returns `void`. The library
is entirely stateless, so there is no option cross-product to enumerate.

```
$ grep -nE 'static|extern|global|_flag|_opt|#ifdef|#if |set[A-Z_]' src/driver.c
(no matches outside the licence header)
```

### Axis 2 — full set of public entry points

The header declares exactly one function, and it is simultaneously the
highest-level and the lowest-level entry point — there is no convenience wrapper
layered over a lower-level core, so "test the low-level entry points too"
collapses to "test `driver`". There are no internal helpers with external
linkage (`nm -D` confirms `driver` is the only exported symbol).

| entry point | signature | kind |
|---|---|---|
| `driver` | `void driver(double f)` | sole public API; both the top and the bottom of the call graph |

### Axis 3 — input shapes the code special-cases

`driver`'s own body has zero branches. But it forwards the value to three
`printf` conversions — `%llx` on the type-punned bits, `%a`, and `%.4f` — and
*glibc's formatter branches heavily* on the IEEE-754 class of the value. That is
where the real configuration surface lives, so the shapes below are derived from
the value classes those three conversions distinguish:

* sign bit: clear / set (affects `%a` and `%.4f` sign, and `-nan` spelling)
* exponent field: all-zero (zero & subnormal), all-ones (inf & NaN), in between
  (normal) — `%a` switches between `0x0.…p-1022` and `0x1.…p±N` forms
* mantissa: zero (powers of two → `%a` trims to `0x1p+N`), trailing-zero runs
  (partial trimming), full 52 significant bits (no trimming), NaN payloads
* magnitude vs `%.4f`: huge (hundreds of integer digits), moderate, tiny
  (underflows to `0.0000` / `-0.0000`)
* decimal rounding position: exact ties at the 4th fractional digit
  (round-half-to-even off the *exact* binary value)

### Axis 4 — environmental state `printf` branches on

Both implementations share the process's libc and `stdout`, so two further axes
are observable through the API even though `driver` never mentions them:

* the C locale's `LC_NUMERIC` decimal point, which `%.4f` honours
* `stdout`'s buffer state — ordering of `driver`'s output relative to output the
  caller itself writes to `stdout`

## Configuration table

One row per combination the C actually treats differently. Every row is driven
with **many randomized inputs (fixed seed `0x5EED_D1FF_C0FFEE01`)**, not a single
hand-picked value, and asserts byte-identical stdout from the C `.so` and the
Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | No options (none exist). Positive **normal** doubles, randomized sign-free mantissa + exponent across the whole normal range. | [x] |
| C2 | `driver` | Negative **normal** doubles, randomized — sign bit set on every field of C1. | [x] |
| C3 | `driver` | **Zero mantissa** (exact powers of two), every exponent `2^-1074 … 2^1023`, both signs — drives `%a`'s trailing-zero trimming to the fully-trimmed `0x1p+N` form. | [x] |
| C4 | `driver` | **Full 52-bit mantissa** (all mantissa bits set / randomized with low bit set), both signs — `%a` emits all 13 hex mantissa digits, no trimming. | [x] |
| C5 | `driver` | **Partial trailing-zero mantissa runs**: mantissa randomized then masked to leave 1…12 trailing zero hex digits — exercises every intermediate `%a` trim length. | [x] |
| C6 | `driver` | **Signed zeros**: `+0.0` (`0x0`) and `-0.0` (`0x8000000000000000`). Distinguishes `%llx` from the `%a`/`%.4f` sign rendering. | [x] |
| C7 | `driver` | **Infinities**: `+inf`, `-inf` — glibc prints `inf`/`-inf` for both `%a` and `%.4f`. | [x] |
| C8 | `driver` | **NaN family**: quiet/signalling × positive/negative × randomized payloads — `%llx` must preserve payload bits exactly while `%a`/`%.4f` collapse to `nan`/`-nan`. | [x] |
| C9 | `driver` | **Subnormals**: smallest (`0x1`), largest (`0x000fffffffffffff`), and randomized subnormal mantissas, both signs — `%a` switches to the `0x0.…p-1022` form. | [x] |
| C10 | `driver` | **Class boundaries**, each value and its `nextafter` neighbour on both sides: `±DBL_MIN`, `±DBL_MAX`, subnormal↔normal transition, `±1.0`, `±2.0`, `±0.5`. | [x] |
| C11 | `driver` | **Huge magnitudes** → longest `%.4f` output: randomized values with exponents in `[1e50, DBL_MAX]`, both signs; `%.4f` emits up to ~310 integer digits. | [x] |
| C12 | `driver` | **Tiny magnitudes** → `%.4f` underflows: randomized values in `(0, 1e-5)`, both signs, asserting `0.0000` vs `-0.0000` sign retention. | [x] |
| C13 | `driver` | **`%.4f` rounding ties**: values at/near the 4th-fractional-digit halfway point (`0.00005`, `0.00015`, `0.00025`, `x.12345`), plus each tie's `nextafter` neighbours, at several magnitudes — round-half-to-even off the exact binary value. | [x] |
| C14 | `driver` | **`%a` exponent sign flip** around `1.0`: values with exponents just above and below `p+0`, so `%a` prints `p+N`, `p+0`, and `p-N`. | [x] |
| C15 | `driver` | **Exhaustive exponent-field sweep**: all 2048 exponent encodings × both signs × a randomized mantissa each — every `%a`/`%.4f` code path glibc selects on the exponent. | [x] |
| C16 | `driver` | **Full-domain randomized raw bit-pattern sweep**: uniformly random `u64` values reinterpreted as `double` and passed through the ABI, covering all classes simultaneously (large volume). | [x] |
| C17 | `driver` | **Repeated / sequential invocation**: many calls in one process, values from a randomized mixed-class stream — verifies statelessness and that N calls produce exactly N identical lines in the same order. | [x] |
| C18 | `driver` | **Interleaving with caller's own `stdout` writes**: caller emits its own text via libc `fputs`/`printf` before and after each `driver` call — verifies the Rust side writes through the *same* libc `stdout` (a `println!`-based translation would reorder here). | [x] |
| C19 | `driver` | **`LC_NUMERIC` locale set to a comma-decimal locale** (e.g. `de_DE.UTF-8`, skipped if absent), randomized values — `%.4f`'s decimal point must match between C and Rust under a non-`C` locale. | [x] |
| C20 | `driver` | Build-configuration axis: default features, `--no-default-features`, and `--all-features`, each in **debug and release** profile (`panic = "abort"` applies to release only). Rows C1–C19 re-run under each. | [x] |

All 20 rows are checked off only because they pass across their randomized
inputs; see `tests/differential.rs` and `run_all_configs.sh`.
