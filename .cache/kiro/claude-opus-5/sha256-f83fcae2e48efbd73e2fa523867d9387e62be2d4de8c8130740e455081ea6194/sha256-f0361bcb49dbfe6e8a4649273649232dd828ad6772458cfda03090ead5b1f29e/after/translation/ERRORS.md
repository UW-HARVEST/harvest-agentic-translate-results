# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep evidence

```
$ grep -n 'return' src/lib.c include/lib.h          -> 0 matches
$ grep -n 'assert'  src/lib.c include/lib.h          -> 0 matches
$ grep -n 'NULL'    src/lib.c include/lib.h          -> 0 matches
$ grep -n 'ERROR\|errno' src/lib.c include/lib.h     -> 0 matches
$ grep -n 'default'  src/lib.c                       -> 0 matches
$ grep -n 'if *(\|while\|for *(\|#if' src/lib.c       -> 0 matches
$ grep -n 'switch\|case' src/lib.c                   -> 1 switch, 3 cases
```

The library has **no error-return path at all**: all four functions return
`void`, there is no error enum, no sentinel, no `errno` use, no `assert`, no
null check, and no explicit range check. The whole rejection surface is
therefore the single `switch (Impairment)` in `colourblind`, which carries
**no `default:` label** — so any `Impairment` value that is not `0`, `1` or `2`
falls straight through to the function epilogue.

Consequence to replicate exactly: for an unhandled `Impairment`, the C
- writes nothing,
- **does not dereference `R`, `G` or `B` at all** (confirmed in the disassembly:
  the `cmpl`/`ja` chain at `colourblind+0x17` jumps to the `nop; leave; ret` at
  `+0x7c` before any `movss (%rax)` is reached),
- so it is safe even with null / dangling pointers in that case.

Undefined behaviour in C that is therefore NOT a testable rejection: passing a
null pointer with a *valid* impairment. The C dereferences unconditionally there
and segfaults; that is UB, not a defined rejection, so it gets no row. (It is
still probed out-of-process in `tests/error_paths.rs::null_pointer_*` to confirm
C and Rust agree that the *valid*-impairment case is a crash in both and the
*invalid*-impairment case is a no-op in both.)

## The table

One row per distinct rejection / non-handled condition the C actually
distinguishes.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|----------------------------------------------|-------------------|-----|
| E1 | `colourblind` | `Impairment == 3` (first value one past the last enumerator `cbTritanopia`); `switch` has no `default:` | no-op: `*R`, `*G`, `*B` unchanged; pointers never dereferenced; no return value | [x] |
| E2 | `colourblind` | `Impairment == -1` (first value one before the first enumerator `cbProtanopia`); `cmpl $0x2` + `ja` treats it as unsigned > 2 | no-op, as E1 | [x] |
| E3 | `colourblind` | `Impairment == INT_MAX` (2147483647) — extreme positive out-of-range enum | no-op, as E1 | [x] |
| E4 | `colourblind` | `Impairment == INT_MIN` (-2147483648) — extreme negative out-of-range enum | no-op, as E1 | [x] |
| E5 | `colourblind` | `Impairment == 4 .. 255` and a randomized sweep of arbitrary `int` values with no valid variant | no-op, as E1 | [x] |
| E6 | `colourblind` | `Impairment` out of range **and** `R`/`G`/`B` are all `NULL` — checks the "no dereference" claim, i.e. that the no-op path is reached before any load | no-op, no crash | [x] |
| E7 | `colourblind` | `Impairment` out of range **and** `R`/`G`/`B` point at NaN / `inf` payloads | no-op: bit patterns preserved byte-for-byte, including NaN payload and sign | [x] |
| E8 | `colourblind` | `Impairment == 0/1/2` (valid) with `R == G == B` (fully aliased pointers) — the C loads all three inputs into locals *before* the first store, so aliasing does not corrupt later reads; a translation that stored eagerly would diverge | last store wins: `*p` receives the `*Blue` row of the matrix computed from `(x,x,x)` | [x] |
| E9 | `colourblind` | `Impairment == 0/1/2` (valid) with *partial* aliasing (`R == G`, `B` distinct; and `R == B`, `G` distinct) | same load-then-store ordering; deterministic result | [x] |
| E10 | `colourblind` | `Impairment == 0/1/2` (valid) with unaligned `float*` (byte offset 1 into a buffer) — `movss` permits unaligned | normal computation, no fault | [x] |
| E11 | `colourblind` | `Impairment == 0/1/2` (valid) with a signalling NaN input | SSE quiets it and forwards it per the `dst`-then-`src` operand rule; no trap (MXCSR invalid-op masked by default) | [x] |
| E12 | `colourblind` | `Impairment == 0/1/2` (valid) with inputs whose product overflows `f32` (`±FLT_MAX`, `±inf`) | `±inf` propagation, or the x86 "real indefinite" QNaN `0xFFC00000` when the operation itself is invalid (`inf - inf`, `0 * inf`) | [x] |

All rows are exercised in `translation/tests/error_paths.rs`; every row asserts
C and Rust agree on the *same* bit pattern / same no-op, not merely "both did
something".
