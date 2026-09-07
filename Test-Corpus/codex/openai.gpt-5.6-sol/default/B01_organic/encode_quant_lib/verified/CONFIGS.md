# Configuration Surface

The only public entry point is the lowest-level function `encode_quant`.
Rows below are the mechanically derived, feasible cross-product of branches
in `src/lib.c`:

- `lsbit`: zero; exactly four; odd nonzero; even nonzero other than four.
- `uni & 7`: low octet edge (`0`); interior (`1..=6`); high octet edge (`7`).
- `uni & 8`: clear or set.
- final comparison outcomes: `(d1 < d0, d2 < d0)`.

Combinations omitted from the table are infeasible because candidate
normalization makes the relevant prediction identical to `p0`, or because the
two adjacent candidates cannot both improve on `p0`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `encode_quant` | `lsbit=0`; `uni&7=0`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 2 | `encode_quant` | `lsbit=0`; `uni&7=0`; `uni&8=0`; comparisons `(true,false)` | [x] |
| 3 | `encode_quant` | `lsbit=0`; `uni&7=0`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 4 | `encode_quant` | `lsbit=0`; `uni&7=0`; `uni&8!=0`; comparisons `(true,false)` | [x] |
| 5 | `encode_quant` | `lsbit=0`; `uni&7=1..=6`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 6 | `encode_quant` | `lsbit=0`; `uni&7=1..=6`; `uni&8=0`; comparisons `(false,true)` | [x] |
| 7 | `encode_quant` | `lsbit=0`; `uni&7=1..=6`; `uni&8=0`; comparisons `(true,false)` | [x] |
| 8 | `encode_quant` | `lsbit=0`; `uni&7=1..=6`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 9 | `encode_quant` | `lsbit=0`; `uni&7=1..=6`; `uni&8!=0`; comparisons `(false,true)` | [x] |
| 10 | `encode_quant` | `lsbit=0`; `uni&7=1..=6`; `uni&8!=0`; comparisons `(true,false)` | [x] |
| 11 | `encode_quant` | `lsbit=0`; `uni&7=7`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 12 | `encode_quant` | `lsbit=0`; `uni&7=7`; `uni&8=0`; comparisons `(false,true)` | [x] |
| 13 | `encode_quant` | `lsbit=0`; `uni&7=7`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 14 | `encode_quant` | `lsbit=0`; `uni&7=7`; `uni&8!=0`; comparisons `(false,true)` | [x] |
| 15 | `encode_quant` | `lsbit=4`; `uni&7=0`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 16 | `encode_quant` | `lsbit=4`; `uni&7=0`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 17 | `encode_quant` | `lsbit=4`; `uni&7=1..=6`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 18 | `encode_quant` | `lsbit=4`; `uni&7=1..=6`; `uni&8=0`; comparisons `(true,false)` | [x] |
| 19 | `encode_quant` | `lsbit=4`; `uni&7=1..=6`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 20 | `encode_quant` | `lsbit=4`; `uni&7=1..=6`; `uni&8!=0`; comparisons `(true,false)` | [x] |
| 21 | `encode_quant` | `lsbit=4`; `uni&7=7`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 22 | `encode_quant` | `lsbit=4`; `uni&7=7`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 23 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=0`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 24 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=0`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 25 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=1..=6`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 26 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=1..=6`; `uni&8=0`; comparisons `(true,false)` | [x] |
| 27 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=1..=6`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 28 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=1..=6`; `uni&8!=0`; comparisons `(true,false)` | [x] |
| 29 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=7`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 30 | `encode_quant` | `lsbit` odd and nonzero; `uni&7=7`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 31 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=0`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 32 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=0`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 33 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=1..=6`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 34 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=1..=6`; `uni&8=0`; comparisons `(true,false)` | [x] |
| 35 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=1..=6`; `uni&8!=0`; comparisons `(false,false)` | [x] |
| 36 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=1..=6`; `uni&8!=0`; comparisons `(true,false)` | [x] |
| 37 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=7`; `uni&8=0`; comparisons `(false,false)` | [x] |
| 38 | `encode_quant` | `lsbit` even, nonzero, and not four; `uni&7=7`; `uni&8!=0`; comparisons `(false,false)` | [x] |

Each row is tested with many fixed-seed randomized inputs. A separate
valid-path boundary corpus covers zero, negative values, and `int` extrema.
