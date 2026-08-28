# Verification evidence

Supporting tooling for the differential verification. Nothing here is part of
the shipped crate.

## `mutate.py` + `lib.rs.pristine` — mutation testing of the test suite

A differential suite that passes proves nothing unless it can *catch* a real
divergence. `mutate.py` injects a single-line change into `src/lib.rs` and the
suite is re-run to confirm it turns red.

It anchors the substitution on `^    <line>$` (four-space indent, start of line)
so it can only hit **code**. This matters: an unanchored
`str.replace(old, new, 1)` silently rewrites the copy of the C source quoted in
the doc comment above `rev16` instead, producing a fake "the suite caught
nothing" result.

Usage:

```
cd translation
python3 verification/mutate.py '<old code line>' '<new code line>'
cargo build --release --offline && cargo test --offline
python3 verification/mutate.py restore
```

### Results

| # | mutation (code line) | suite outcome | verdict |
|---|----------------------|---------------|---------|
| M1 | stmt 4 `>> 8` → `>> 4` | **8 tests failed** | caught |
| M2 | stmt 1 masks `0xAAAA`/`0x5555` → `0xAAAAAAAA`/`0x55555555` | 0 failed | correct — see below |
| M3 | stmt 2 masks swapped (`0xCCCC`↔`0x3333`) | **8 tests failed** | caught |
| M4 | stmt 3 `<< 4` → `<< 5` | **8 tests failed** | caught |
| M5 | `a` OR-ed with `0x00010000` on entry | 0 failed | correct — see below |

Sample caught divergence (M1):

```
divergence for rev16(0xFFFFFFFF): C returned 0x0000FFFF but Rust returned 0x0000FFF0
```

Mutation score: **3/3 behaviour-changing mutations caught; 2/2 behaviour-
preserving mutations correctly passed.**

## `equiv_check.c` — proving M2 and M5 are true equivalences

M2 and M5 did not turn the suite red. That is either a blind spot in the suite or
a genuine semantic equivalence, and the difference matters, so it was settled by
brute force rather than by argument.

`equiv_check.c` implements the original and both mutants and compares all 2^32
inputs:

```
cc -O2 -o equiv_check verification/equiv_check.c && ./equiv_check
```

Output:

```
checked 2^32 = 4294967296 inputs
M2 mismatches: 0
M5 mismatches: 0
```

Both mutants are **equivalent to the original across the entire input domain**,
so the suite was right to pass them.

Why they are equivalent:

* **M2** — widening statement 1's masks moves the point at which bits 16..31 are
  discarded from statement 1 to statement 2 (whose masks are still 16-bit).
  No bit crosses the bit-15/bit-16 boundary in the process: bit 15 is not in
  `0x55555555`, so nothing shifts up out of the low half, and bit 16 is not in
  `0xAAAAAAAA`, so nothing shifts down into it.
* **M5** — bit 16 is set on entry, but statement 1's 16-bit masks discard bits
  16..31 immediately, so the injected bit can never be observed.

## Exhaustive proof

Independently of the mutation work, `tests/valid_paths.rs::
exhaustive_all_2pow32_arguments` drives **every one of the 4 294 967 296 `u32`
values** through both `.so` exports and requires byte-identical results
(~26 s). Since `rev16`'s entire input domain is one `u32`, this is a *complete*
verification, not a sample:

```
cargo test --offline --test valid_paths -- --ignored --nocapture
[EXHAUSTIVE] verified all 4294967296 u32 arguments identical
```
