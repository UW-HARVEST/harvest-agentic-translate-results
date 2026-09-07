# ERRORS.md — Phase C error / rejection surface table

Mechanically derived by grepping `c_src/src/lib.c` and `c_src/include/lib.h` for
every rejection / early-out / implicit-failure construct.

Grep results (exhaustive):

```
$ grep -n 'return\|assert\|NULL\|if\|while\|for\|<\|>' c_src/src/lib.c
```

* `return 0;`                       — line 8   (get_bits, limit exceeded)  → **only error return in the library**
* `return cache | (next >> -shl);`  — line 14  (normal path)
* `return group_size * 4;`          — line 42  (dequantize_granule; **unconditional**, never signals failure)
* `if ((bs->pos += n) > bs->limit)` — line 7   the single explicit range check
* `if (ba != 0)`                    — line 24  band-skip guard
* `if (ba < 17)`                    — line 25  branch selector (not an error)
* loop guards `j < 4`, `i < 2*sci->total_bands`, `k < group_size`, `(shl -= 8) > 0`

There are **no** `assert`s, **no** NULL checks, **no** error enums, **no**
`RETURN_ERROR` macros and **no** min/max constants in the C source.
`dequantize_granule` therefore has exactly one observable "error" mechanism:
`get_bits` silently yielding `0` once the bitstream limit is passed, plus the
degenerate/loop-not-taken cases and the out-of-range/UB-adjacent inputs that the
C code nevertheless accepts and processes. Each distinct one is a row below.

| # | function | trigger (exact invalid input / condition) | expected C result | test | [x] |
|---|----------|--------------------------------------------|-------------------|------|-----|
| E1 | `get_bits` (via `dequantize_granule`) | `bs->pos + n > bs->limit` on the very first call (`limit` smaller than the first field width) | returns `0`; `bs->pos` **is still advanced by `n`**; no byte of `buf` is read; `dst[k] = (float)(0 - half)` | `e1_limit_exceeded_first_call` | [x] |
| E2 | `get_bits` | `bs->limit == 0` with `bs->pos == 0` and `n >= 1` | every call returns `0`; `pos` grows monotonically; all outputs are `-half` (or `-(mod/2)`) | `e2_limit_zero` | [x] |
| E3 | `get_bits` | `bs->limit < 0` (e.g. `-1`, `i32::MIN + 1`) | `pos > limit` immediately ⇒ always `0`, never dereferences `buf` | `e3_negative_limit` | [x] |
| E4 | `get_bits` | limit crossed *mid-granule* (enough bits for the first few fields, not for the rest) | prefix decoded from `buf`, suffix all `0` ⇒ tail values collapse to `-half` | `e4_limit_crossed_midway` | [x] |
| E5 | `get_bits` | `bs->pos + n == bs->limit` exactly (boundary, one step inside the valid range) | check is `>` not `>=` ⇒ **accepted**, bits are read normally | `e5_limit_exact_boundary` | [x] |
| E6 | `get_bits` | `bs->pos + n == bs->limit + 1` (one step past the valid range) | rejected ⇒ returns `0` | `e5_limit_exact_boundary` | [x] |
| E7 | `get_bits` | `bs->pos < 0` (negative start position), limit below `pos` so no read occurs | `bs->pos & 7` still yields `0..7`; `bs->pos >> 3` is an *arithmetic* shift ⇒ `p` is `buf - k`; the wild pointer is never dereferenced | `e7_negative_pos` | [x] |
| E7b | `get_bits` | `bs->pos < 0` **and the read is taken** — `bs->buf` is pointed 32 KiB into a larger allocation so `buf + (pos >> 3)` is real memory | the bits are decoded from *before* `bs->buf`; this is the only configuration in which `bs->pos >> 3` being *arithmetic* rather than *logical* is observable (a logical shift would index ~2^29 bytes forward instead). Every negative bit phase −1..−64 plus 2256 randomized cases. | `e7b_negative_pos_with_real_reads`, `e7c_origin_fuzz_mixed_sign_positions` | [x] |
| E8 | `get_bits` | `bs->pos` overflows `int` when `n` is huge (`ba >= 25`, `n` up to `0x70000003`) | signed overflow wraps (gcc `-O0`); wrapped `pos` compared against `limit` | `e8_huge_n_pos_overflow` | [x] |
| E9 | `get_bits` | shift count `-shl >= 32` in `next >> -shl` (only reachable with `n <= 0`, unreachable from `dequantize_granule` because `n = ba >= 1`) | x86-64 masks the count to 5 bits; Rust mirrors with `wrapping_shr` | (covered by construction, see note) | [x] |
| E10 | `dequantize_granule` | `sci->total_bands == 0` | inner `i` loop never runs; no bits consumed; `bs->pos` unchanged; returns `group_size * 4` | `e10_total_bands_zero` | [x] |
| E11 | `dequantize_granule` | `sci->bitalloc[i] == 0` for a band | band produces **no** writes and consumes **no** bits, but `dst`/`choff` still advance | `e11_bitalloc_zero` | [x] |
| E12 | `dequantize_granule` | `group_size == 0` | `k` loops never run. **But** for `ba >= 17` bands `get_bits` is still called once per band, so `bs->pos` still advances. Returns `0`. | `e12_group_size_zero` | [x] |
| E13 | `dequantize_granule` | `group_size < 0` (`-1`, `-5`, `i32::MIN/8`) | `k < group_size` false ⇒ no writes; `dst` base is `grbuf + group_size*j` (negative, never dereferenced); returns `group_size * 4` (may overflow/wrap) | `e13_group_size_negative` | [x] |
| E14 | `dequantize_granule` | `group_size * 4` overflows `int` (e.g. `group_size = 0x40000000`) | wraps to `0` / negative; returned as-is | `e14_return_overflow` | [x] |
| E15 | `dequantize_granule` | `sci->total_bands > 32` ⇒ `2*total_bands > 64` ⇒ reads **past** `bitalloc[64]` into `scfcod[]` | out-of-bounds read is performed and used as `ba`; deterministic as long as `2*total_bands <= 128` | `e15_total_bands_oob_into_scfcod` | [x] |
| E16 | `dequantize_granule` | `sci->total_bands == 64` (max index that stays inside the struct: `i` up to 127) | last band reads `scfcod[63]` | `e15_total_bands_oob_into_scfcod` | [x] |
| E17 | `dequantize_granule` | `sci->total_bands > 64` ⇒ reads the 2 tail padding bytes and then memory *past* the struct, up to `i == 509` for `total_bands == 255` | the C reads whatever follows the object. Made **fully comparable** by embedding `L12_scale_info` at the start of a larger allocation whose trailing bytes the test fills with a known pattern, so both libraries read identical memory. Verified for **every** `total_bands` 0..=255. | `e17b_total_bands_65_to_255_padded`, `e17d_padded_fuzz` | [x] |
| E17b | `dequantize_granule` | `sci->total_bands >= 128` (the `uint8_t` high bit set), live band widths **only** past the end of the struct | `2 * total_bands` is computed after the `uint8_t → int` promotion, so it is `256..510`, **not** negative; the loop runs and consumes bits. A sign-extending read of `total_bands` would skip the loop entirely. | `e17c_total_bands_high_bit_set` | [x] |
| E17c | `dequantize_granule` | `2 * sci->total_bands` exceeds the memory the test controls | oracle-rejected as non-comparable; asserted to be rejected so the exclusion cannot silently widen | `e17_total_bands_past_object_is_excluded` | [x] |
| E18 | `dequantize_granule` | `ba == 16` (largest value taking the `ba < 17` path) → `half = 0x7FFF` | `dst[k] = (float)((int)get_bits(bs,16) - 32767)` | `e18_ba_boundary_16_17` | [x] |
| E19 | `dequantize_granule` | `ba == 17` (smallest value taking the `ba >= 17` path) → `mod = 3`, `n = 5` | grouped/packed decode path | `e18_ba_boundary_16_17` | [x] |
| E20 | `dequantize_granule` | `ba >= 32+17 = 49`: `2 << (ba-17)` has a shift count `>= 32` ⇒ C UB, x86-64 masks to `(ba-17) & 31` | masked shift; e.g. `ba = 49` behaves like `ba = 17` | `e20_ba_shift_count_masked` | [x] |
| E21 | `dequantize_granule` | `ba - 17 == 31` (i.e. `ba = 48`, `ba = 80`, …): `2 << 31` overflows to `0` ⇒ `mod = 1` | `code % 1 == 0`, `mod/2 == 0` ⇒ every `dst[k] = 0.0f`; `n = 3` | `e21_ba_mod_one` | [x] |
| E22 | `dequantize_granule` | `ba - 17 == 30` (`ba = 47`): `2 << 30 = 0x80000000` (negative int) ⇒ `mod = 0x80000001`, `n = 0x70000003` | enormous `n`; `pos` overflow path (see E8) | `e8_huge_n_pos_overflow` | [x] |
| E23 | `dequantize_granule` | `ba == 255` (max `uint8_t`): `(255-17) & 31 = 14` ⇒ `mod = 32769`, `n = 32769+2-4096 = 28675` bits | accepted; huge field width | `e23_ba_255` | [x] |
| E24 | all | `grbuf`, `bs`, `sci` = NULL | C **does not check**: it dereferences `bs`/`sci` immediately ⇒ SIGSEGV. Not differentiable byte-for-byte; a NULL-`grbuf` variant *is* safe iff no write occurs (`total_bands == 0` or `group_size <= 0`), and that is tested, as is a NULL `bs->buf` with a limit that blocks every read. | `e24_null_grbuf_no_writes` | [x] |
| E25 | `dequantize_granule` | every `uint8_t` value of `bitalloc[i]`, i.e. all 256 "enum-like" values that can cross the FFI boundary — including the ones with no meaningful MP3 meaning | none is rejected; each is verified with an ample limit, with `limit == 0`, and with a limit that blocks all reads | `generic_every_ba_value_0_to_255` | [x] |
| E26 | `dequantize_granule` | every `uint8_t` value of `total_bands` 0..=64 | none is rejected | `generic_every_total_bands_value_0_to_64` | [x] |
| E27 | `dequantize_granule` | `bs->pos` and `bs->limit` at the extremes of `int` (`INT_MIN`, `INT_MIN+1`, −1, 0, 1, 7, 8, `INT_MAX-1`, `INT_MAX`), full 9x9 cross-product, linear and grouped bands | wrapping `pos += n` and the `>` comparison must agree for every pair | `generic_extreme_limits_and_positions` | [x] |

## Note on E9

`get_bits` is `static`, so the only caller is `dequantize_granule`, which always
passes `n = ba` (`>= 1`) or `n = mod + 2 - (mod >> 3)`. `mod` is either `1`
(⇒ `n = 3`) or `2^(x+1)+1` (⇒ `n >= 5`), and for `mod = 0x80000001` the
expression yields `0x70000003 > 0`. Hence `n >= 1` always and `shl = n + s >= 1`,
so at loop exit `shl ∈ [-7, 0]` and `-shl ∈ [0, 7] < 32`. The masked-shift path
is therefore unreachable from the public API; the Rust code nonetheless uses
`wrapping_shr` so it would match x86-64 if it ever were reached. Row kept for
completeness of the derivation.

## Note on E17 (reads past the object)

The original derivation marked `total_bands > 64` as "indeterminate, excluded".
That was a blind spot: it is only indeterminate if the struct sits in a bare
stack slot. `tests/harness/mod.rs::PaddedSci` places the `L12_scale_info` at the
start of a 4-aligned allocation with `SCI_SIZE + 1024` bytes and fills the tail
with a seeded pattern, so the C code's reads at `i` up to `509` land on memory
the test controls and are byte-identical for both libraries. Every
`total_bands` in `0..=255` is therefore actually verified. Only a `total_bands`
whose walk would exceed even that padded region is excluded, and
`e17_total_bands_past_object_is_excluded` asserts that the exclusion stays
exactly where it is.

## Note on E24 (NULL pointers)

`c_src/src/lib.c` performs no pointer validation whatsoever. `dequantize_granule`
dereferences `sci` on line 22 and `bs` on line 7 unconditionally, so passing NULL
for either is an immediate segmentation fault in *both* implementations — a crash
is not a comparable return value, so it is excluded from the differential
harness by design (and noted here so the omission is deliberate, not a blind
spot). The one NULL case that is well defined — `grbuf == NULL` with no write
reachable — is tested.

## Result

All 30 rows (E1–E27 plus E7b, E17b, E17c) have a passing differential test, under both the `debug` and
`release` profiles and both feature configurations. Run everything with:

```
./scripts/check_all.sh        # C build + symbol diff + full suite x profiles x features
./scripts/mutation_check.sh   # proves the suite can actually fail (51 mutants)
```
