# CONFIGS.md — Configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

Grep for branching constructs:

```
grep -nE 'if|switch|case|for|while|#if' c_src/src/lib.c
```

* `for (i = 0; i + sizeof(size_t) <= len; i += 8, d += 8)` (lib.c:18)
  → **Axis L (block count)**: `len/8` = 0, 1, 2, many.
* `switch (len - i)` with fall-through arms `7,6,5,4,3,2,1,0` (lib.c:48-65)
  → **Axis T (tail length)**: `len % 8` ∈ {0,1,2,3,4,5,6,7}.
* `data = len << (STBDS_SIZE_T_BITS - 8)` (lib.c:47)
  → **Axis L2 (len magnitude)**: only the low 8 bits of `len` survive the
    shift, so `len == 256` and `len == 0` inject the *same* length byte; a
    length ≥ 256 is a distinct shape.
* `d[3] << 24` / `d[7] << 24` on `int` (lib.c:20-22, 56)
  → **Axis S (high-bit bytes)**: byte value < 0x80 vs ≥ 0x80 at offsets
    3 and 7 of each block, and at offset 3 of the tail. Selects
    sign-extension vs not.
* `^ seed` / `^ ~seed` (lib.c:10-17)
  → **Axis D (seed)**: 0, 1, all-ones, half-ones, random 64-bit.
* `siphash(int init)` fill loop `mem[i] = z; z++` (lib.c:118)
  → **Axis I (init)**: any `int`; only `init & 0xff` and the wrap point matter.
* `#ifdef` / `#if`: **none**. No compile-time configuration.
* Cargo features: **none declared** in `translation/Cargo.toml` → the only
  feature combination is the default (empty) one.

## Full set of public entry points

| entry point | level | signature |
|---|---|---|
| `stbds_hash_bytes` | LOW-LEVEL primitive (thin wrapper over the `static stbds_siphash_bytes`) | `size_t (void*, size_t, size_t)` |
| `siphash`          | HIGH-LEVEL driver / one-shot table dumper (calls `stbds_hash_bytes` 64×, prints) | `void (int)` |

`stbds_siphash_bytes` is `static` and reachable only through
`stbds_hash_bytes`; driving `stbds_hash_bytes` directly *is* driving the
lowest-level entry point.

## Configuration rows (cross-product, pruned to distinguished combinations)

Every row is exercised with **many randomized inputs** (fixed seed
`0x5eed_1234_abcd_0001`, `N = 256` iterations per row unless noted), comparing
the C `.so` and Rust `.so` return values bit-for-bit.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C01 | `stbds_hash_bytes` | L=0 blocks, T=0 (`len==0`), seed=0; empty input | `cfg_c01_len0_seed0` | [x] |
| C02 | `stbds_hash_bytes` | L=0, T=0 (`len==0`), seed ∈ {1, MAX, MAX/2, random×N} | `cfg_c02_len0_seed_sweep` | [x] |
| C03 | `stbds_hash_bytes` | L=0, T=1 (`len==1`), random byte×N, seed=0 | `cfg_c03_len1` | [x] |
| C04 | `stbds_hash_bytes` | L=0, T=2 (`len==2`), random bytes×N, seed=0 | `cfg_c04_len2` | [x] |
| C05 | `stbds_hash_bytes` | L=0, T=3 (`len==3`), random bytes×N, seed=0 | `cfg_c05_len3` | [x] |
| C06 | `stbds_hash_bytes` | L=0, T=4 (`len==4`), random bytes×N, seed=0 — first arm reaching `d[3]<<24` | `cfg_c06_len4` | [x] |
| C07 | `stbds_hash_bytes` | L=0, T=5 (`len==5`), random bytes×N, seed=0 | `cfg_c07_len5` | [x] |
| C08 | `stbds_hash_bytes` | L=0, T=6 (`len==6`), random bytes×N, seed=0 | `cfg_c08_len6` | [x] |
| C09 | `stbds_hash_bytes` | L=0, T=7 (`len==7`), random bytes×N, seed=0 — deepest fall-through | `cfg_c09_len7` | [x] |
| C10 | `stbds_hash_bytes` | L=1, T=0 (`len==8`) — exactly one body block, no tail; random×N | `cfg_c10_len8` | [x] |
| C11 | `stbds_hash_bytes` | L=1, T∈1..7 (`len==9..15`) — one block + each tail arm; random×N per len | `cfg_c11_one_block_plus_each_tail` | [x] |
| C12 | `stbds_hash_bytes` | L=2, T=0 (`len==16`); random×N | `cfg_c12_len16` | [x] |
| C13 | `stbds_hash_bytes` | L=2..8, T∈0..7 — full cross-product of block count × tail length (`len` 16..71), random×N per len | `cfg_c13_blocks_x_tails_crossproduct` | [x] |
| C14 | `stbds_hash_bytes` | S: byte at offset 3 forced ≥ 0x80 (tail arm 4..7) — sign-extension path | `cfg_c14_tail_highbit_offset3` | [x] |
| C15 | `stbds_hash_bytes` | S: byte at offset 3 forced < 0x80 (tail arm 4..7) — no sign-extension | `cfg_c15_tail_lowbit_offset3` | [x] |
| C16 | `stbds_hash_bytes` | S: body block bytes 3 and 7 forced ≥ 0x80 (all 4 combinations of {b3<0x80, b3≥0x80} × {b7<0x80, b7≥0x80}) across L=1..4 | `cfg_c16_body_highbit_combos` | [x] |
| C17 | `stbds_hash_bytes` | all-zero buffer, L=0..8, every T; seed=0 | `cfg_c17_all_zero_buffer` | [x] |
| C18 | `stbds_hash_bytes` | all-`0xff` buffer, L=0..8, every T; seed=0 | `cfg_c18_all_ff_buffer` | [x] |
| C19 | `stbds_hash_bytes` | L2: `len` ≥ 256 (`len` ∈ {255, 256, 257, 511, 512, 1024, 4096}) — `len << 56` keeps only low byte; random data | `cfg_c19_large_len` | [x] |
| C20 | `stbds_hash_bytes` | seed × shape interaction: random seed AND random len ∈ 0..=200 AND random data, N=4096 property iterations | `cfg_c20_property_full_random` | [x] |
| C21 | `stbds_hash_bytes` | unaligned `p`: buffer offset by 1..7 bytes from an 8-aligned base, all lens 0..=40 (`d` walks unaligned) | `cfg_c21_unaligned_pointer` | [x] |
| C22 | `siphash` | `init == 0` — the canonical stb_ds table; stdout byte-compared | `cfg_c22_siphash_init0_stdout` | [x] |
| C23 | `siphash` | `init` ∈ {1, 7, 42, 127, 128, 200, 255, 256, 1000, -1, -128, -1000, INT_MIN, INT_MAX} + randomized ints; stdout byte-compared each | `cfg_c23_siphash_init_sweep_stdout` | [x] |
| C24 | `siphash` | composed-pipeline check: `siphash(init)` output must also equal the 64 values obtained by driving the low-level `stbds_hash_bytes` directly on the same `mem` fill — verifies the wrapper composes identically in both libs | `cfg_c24_siphash_vs_lowlevel_composition` | [x] |
| C25 | both | feature combination sweep: only the default (no features) combination exists; `cargo test` with `--no-default-features` and with default both run the full suite, in BOTH the `dev` and `release` profiles | `scripts/check_features.sh` | [x] |
| C26 | `stbds_hash_bytes` | **seed-invariance** (see finding below): every seed must give the *same* value as seed 0, in both libraries, for `len` 0..=80 | `c_hash_is_seed_invariant_and_rust_matches` | [x] |
| C27 | `stbds_hash_bytes` | exhaustive: every tail byte POSITION (0..len) × every byte VALUE (0..=255) × every `len` 1..=7 — each tail offset uses a *different* shift expression in the C | `exh_tail_every_byte_position_and_value` | [x] |
| C28 | `stbds_hash_bytes` | exhaustive: every body-block byte position (0..8) × every byte value (0..=255) at `len==8` | `exh_body_every_byte_position_and_value` | [x] |
| C29 | `stbds_hash_bytes` | body×tail interaction: byte position × boundary values {0,1,0x7f,0x80,0x81,0xfe,0xff} × tail 1..=7 | `exh_body_plus_tail_interaction` | [x] |
| C30 | `stbds_hash_bytes` | every `len` 0..=200, 8 random buffers × 5 seeds each | `exh_every_len_0_to_200` | [x] |
| C31 | `stbds_hash_bytes` | seed single-bit sweep: each of the 64 bits set alone and cleared alone, × `len` ∈ {0,1,7,8,9,15,16,33} | `exh_seed_single_bit_sweep` | [x] |
| C32 | `stbds_hash_bytes` | known-answer vector pinning the init constants + finalisation: `hash(empty) == 0x726fdb47dd0e0e31` | `known_answer_empty_input` | [x] |

## Finding: `stbds_hash_bytes` ignores its `seed` argument

`lib.c:10-17` XORs the seed into each state word **twice**:

```c
v0 = (C0) ^  seed;                       // :10
v0 ^= 0x0706050403020100ull ^  seed;     // :14   -> seed cancels (x ^ s ^ k ^ s == x ^ k)
v1 = (C1) ^ ~seed;                       // :11
v1 ^= 0x0f0e0d0c0b0a0908ull ^ ~seed;     // :15   -> ~seed cancels
v2 = (C2) ^  seed;  v2 ^= 0x0706...  ^  seed;   // :12,:16
v3 = (C3) ^ ~seed;  v3 ^= 0x0f0e...  ^ ~seed;   // :13,:17
```

So the seed has **no effect whatsoever** on the result.  This was confirmed by
probing the compiled C `.so` directly (all seeds give an identical hash for
every length 0..20).  It is a genuine property of the C ground truth and is
asserted as such in row C26 — it is *not* "fixed" in the Rust.  A
mistranslation of any of lines 10-17 would break this invariant, so C26 is a
strong structural check.

## Result

Rows: 32. STATUS: **PASS** (see test output).
