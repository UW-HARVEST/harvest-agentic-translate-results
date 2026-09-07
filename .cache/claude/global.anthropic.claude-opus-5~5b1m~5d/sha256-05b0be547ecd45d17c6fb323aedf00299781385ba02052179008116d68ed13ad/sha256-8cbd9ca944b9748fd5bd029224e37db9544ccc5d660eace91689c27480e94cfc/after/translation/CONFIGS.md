# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

`bin2hex` is the **only** public entry point (`c_src/include/lib.h` declares
exactly one function; it is also the lowest-level entry point — there is no
convenience wrapper layer to bypass). Grep for branch/option constructs:

```sh
grep -n 'if\|while\|for\|switch\|#if\|#ifdef\|#ifndef' c_src/src/lib.c
# 12:    if (bin_len >= (18446744073709551615UL) / 2 || hex_maxlen <= bin_len * 2U) {
# 15:    while (i < bin_len) {
```

There are **no runtime options, modes, flags, global state, `switch`es, or
`#ifdef`s** — no `set*`/`init*`/`ctx` functions exist, so there is no option
cross-product. The axes are purely input SHAPE + input VALUE:

* **A. `bin_len`** (drives the `while` trip count): `0` (loop body never
  executes, only the terminator store), `1`, `2`, odd, even, "many",
  large-but-valid.
* **B. `hex_maxlen` relative to `bin_len * 2`** (drives the accept/reject
  boundary at line 12 — the only accepting side here): exactly
  `bin_len*2 + 1` (minimum accepted), `bin_len*2 + 2`, much larger.
  (`<= bin_len*2` is the reject side → `ERRORS.md`.)
* **C. per-byte nibble value classes** (drive the branch-free
  `((n - 10U) >> 8) & ~38U` correction term, which behaves differently for
  `n < 10` vs `n >= 10`) — independently for the HIGH nibble (`bin[i] >> 4`)
  and the LOW nibble (`bin[i] & 0xf`), giving 4 distinct combinations:
  `hi<10/lo<10`, `hi<10/lo>=10`, `hi>=10/lo<10`, `hi>=10/lo>=10`.
* **D. output-buffer aliasing/alignment shape**: `hex` at offset 0 of an
  allocation vs. an unaligned offset (the C writes byte-at-a-time; an
  autovectorized Rust build must not change results), and surrounding
  guard bytes to prove no byte past `hex[bin_len*2]` is written.
* **E. returned pointer**: must be exactly the incoming `hex`.

## Rows (cross-product of A x B x C x D, pruned to what the code distinguishes)

Every row is driven with MANY randomized inputs (fixed seed `0x5EED_1234`,
xorshift64* PRNG defined in `tests/differential.rs`) unless the row is a
single exact shape. Both `.so`s are loaded with `libloading` and the full
output buffer (including guard bytes) plus the returned pointer offset are
compared byte-for-byte.

| #  | entry point | configuration (options set + input shape)                                                                                              | [x] |
|----|-------------|----------------------------------------------------------------------------------------------------------------------------------------|-----|
| 1  | `bin2hex`   | no options exist. `bin_len = 0`, `hex_maxlen = 1` (minimum accepted for empty input): only the NUL terminator is written                | [x] |
| 2  | `bin2hex`   | `bin_len = 0`, `hex_maxlen` randomized in `1..=4096`                                                                                    | [x] |
| 3  | `bin2hex`   | `bin_len = 1`, all 256 byte values, `hex_maxlen = 3` (== `bin_len*2+1`, minimum accepted)                                               | [x] |
| 4  | `bin2hex`   | `bin_len = 1`, all 256 byte values x `hex_maxlen` in `{3, 4, 64}` (exhaustive nibble-class coverage: axis C all 4 combos)               | [x] |
| 5  | `bin2hex`   | `bin_len = 2`, exhaustive over all 65536 two-byte inputs, `hex_maxlen = 5`                                                              | [x] |
| 6  | `bin2hex`   | axis C forced: every byte's HIGH nibble `< 10` AND LOW nibble `< 10` (digits only), randomized `bin_len` in `1..=256`                   | [x] |
| 7  | `bin2hex`   | axis C forced: HIGH nibble `< 10`, LOW nibble `>= 10`, randomized `bin_len` in `1..=256`                                               | [x] |
| 8  | `bin2hex`   | axis C forced: HIGH nibble `>= 10`, LOW nibble `< 10`, randomized `bin_len` in `1..=256`                                              | [x] |
| 9  | `bin2hex`   | axis C forced: HIGH nibble `>= 10` AND LOW nibble `>= 10` (letters only), randomized `bin_len` in `1..=256`                            | [x] |
| 10 | `bin2hex`   | fully random bytes, randomized `bin_len` in `1..=512`, `hex_maxlen = bin_len*2 + 1` (minimum accepted — axis B boundary)                | [x] |
| 11 | `bin2hex`   | fully random bytes, randomized `bin_len` in `1..=512`, `hex_maxlen = bin_len*2 + 2` (one past the minimum)                              | [x] |
| 12 | `bin2hex`   | fully random bytes, randomized `bin_len` in `1..=512`, `hex_maxlen` randomized in `bin_len*2+1 ..= bin_len*2+4096` (much larger)        | [x] |
| 13 | `bin2hex`   | odd vs. even `bin_len` (axis A parity), randomized bytes, minimum `hex_maxlen`                                                          | [x] |
| 14 | `bin2hex`   | axis D: `hex` at each unaligned offset `0..=15` inside a larger allocation, randomized bytes, `bin_len` in `1..=128`                    | [x] |
| 15 | `bin2hex`   | axis D: guard bytes `0xAA` around the output; assert byte `hex[bin_len*2 + 1]` and beyond are UNTOUCHED by both implementations         | [x] |
| 16 | `bin2hex`   | axis D: `bin` at each unaligned offset `0..=15` inside a larger allocation (read-side alignment), randomized bytes                      | [x] |
| 17 | `bin2hex`   | axis E: returned pointer identity — assert both return exactly the `hex` argument, across all shapes above                              | [x] |
| 18 | `bin2hex`   | "many": `bin_len` large (`4096`, `65536`), randomized bytes, `hex_maxlen = bin_len*2 + 1`                                               | [x] |
| 19 | `bin2hex`   | `bin_len` large + `hex_maxlen` = `SIZE_MAX/2 - 1` (huge but non-overflowing, term-2 passes) with a correctly sized buffer               | [x] |
| 20 | `bin2hex`   | repeated calls into the SAME `hex` buffer with decreasing `bin_len` (no hidden state; stale bytes must remain where C leaves them)      | [x] |
| 21 | `bin2hex`   | `hex` and `bin` in the same allocation, non-overlapping (C has no aliasing annotations; Rust must not assume `noalias` beyond C's)      | [x] |
