# Configuration Surface

The configuration inventory is derived from the sole public declaration in
`../c_src/include/lib.h` and every conditional/indexing operation in
`../c_src/src/lib.c`.

There are no compile-time features, runtime options, modes, flags, element
types, byte-order choices, pointers, lengths, or formats. The complete public
API is `ldexp_q2(float y, int exp_q2)`.

The C implementation distinguishes:

- loop shape: one iteration (`exp_q2 <= 120`), exactly two iterations
  (`121..=240`), or many iterations (`exp_q2 > 240`);
- scale-table selection on each terminal exponent's low two bits
  (`e & 3` equals 0, 1, 2, or 3);
- all shift quotients through `e >> 2`.

For positive multi-iteration inputs, leading iterations use `e = 120` and
table index 0. The terminal table index is `exp_q2 & 3`, including exact
multiples of 120.

Each row is exercised with reproducibly randomized raw `float` bit patterns
(zeros, subnormals, normals, infinities, and NaNs can all occur) and exponents
spanning the row's shift quotients. Scalar boundaries are included explicitly.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `ldexp_q2` | one iteration; `exp_q2 <= 120`; terminal `e & 3 == 0` | [x] |
| 2 | `ldexp_q2` | one iteration; `exp_q2 <= 120`; terminal `e & 3 == 1` | [x] |
| 3 | `ldexp_q2` | one iteration; `exp_q2 <= 120`; terminal `e & 3 == 2` | [x] |
| 4 | `ldexp_q2` | one iteration; `exp_q2 <= 120`; terminal `e & 3 == 3` | [x] |
| 5 | `ldexp_q2` | exactly two iterations; `121 <= exp_q2 <= 240`; terminal `e & 3 == 0` | [x] |
| 6 | `ldexp_q2` | exactly two iterations; `121 <= exp_q2 <= 240`; terminal `e & 3 == 1` | [x] |
| 7 | `ldexp_q2` | exactly two iterations; `121 <= exp_q2 <= 240`; terminal `e & 3 == 2` | [x] |
| 8 | `ldexp_q2` | exactly two iterations; `121 <= exp_q2 <= 240`; terminal `e & 3 == 3` | [x] |
| 9 | `ldexp_q2` | three or more iterations; `exp_q2 > 240`; terminal `e & 3 == 0` | [x] |
| 10 | `ldexp_q2` | three or more iterations; `exp_q2 > 240`; terminal `e & 3 == 1` | [x] |
| 11 | `ldexp_q2` | three or more iterations; `exp_q2 > 240`; terminal `e & 3 == 2` | [x] |
| 12 | `ldexp_q2` | three or more iterations; `exp_q2 > 240`; terminal `e & 3 == 3` | [x] |
