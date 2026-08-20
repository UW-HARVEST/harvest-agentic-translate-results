# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from the branches the C code actually takes.

## 1. Public entry points (the FULL set, lowest level included)

| entry point | level | signature | source |
|---|---|---|---|
| `stbds_hash_bytes` | **lowest-level** — the real hashing API; takes an arbitrary buffer, length and seed | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` | `src/lib.c:110` (not in `lib.h`, but exported — external linkage) |
| `siphash` | convenience / one-shot wrapper — builds a 64-byte buffer from `init`, then calls `stbds_hash_bytes` 64 times (`len` 0..63, `seed` 0) and `printf`s the digests | `void siphash(int init)` | `src/lib.c:114`, declared in `include/lib.h` |

`stbds_siphash_bytes` (`src/lib.c:6`) is `static`, so it is only reachable *through*
`stbds_hash_bytes`; driving `stbds_hash_bytes` directly **is** driving the lowest level.

## 2. Axes the C code branches on

There are **no runtime option/mode/flag setters** in this API (no context struct, no
`set_*` function, no global, no `enum`, no `#ifdef`). The configuration surface is
therefore entirely made of (a) the `seed` parameter, which is mixed into all four state
words *both* directly and as `~seed`, and (b) **input shape**, which the code
special-cases heavily:

| axis | values the C distinguishes | why (C source evidence) |
|---|---|---|
| **L** — length shape | `len == 0`; `len ∈ 1..=7`; `len == 8`; `len ∈ 9..=15`; `len == 16`; `len ∈ 17..=63`; `len ∈ 64..=255`; `len >= 256`; `len % 8 == 0` vs `!= 0` | line 18 `for (i = 0; i + sizeof(size_t) <= len; …)` controls the block count; line 48 `switch (len - i)` has 8 distinct fall-through entry points (`case 7`…`case 0`); line 47 `data = len << 56` keeps only `len & 0xFF` |
| **T** — tail remainder `len % 8` | 0,1,2,3,4,5,6,7 — **each is a different fall-through entry point** | lines 49–64: `case 7` ORs `d[6]<<48`, `case 6` `d[5]<<40`, `case 5` `d[4]<<32`, `case 4` `d[3]<<24` *(int, sign-extends)*, `case 3` `d[2]<<16`, `case 2` `d[1]<<8`, `case 1` `d[0]`, `case 0` nothing |
| **B** — byte-value pattern (sign-extension classes) | all `0x00`; all `0xFF`; block byte `d[3] >= 0x80`; block byte `d[3] < 0x80`; block byte `d[7] >= 0x80` with `d[3] < 0x80`; tail byte `d[3] >= 0x80`; single-bit-set; sequential; uniform random | line 20 evaluates `d[0]|…|(d[3]<<24)` in `int` then converts to `size_t` → **sign-extends** when `d[3] >= 0x80`; line 21 casts to `size_t` *before* `<<16<<16`, so its sign extension is **shifted out**; line 56 (`case 4`) sign-extends again |
| **S** — seed | `0`; `1`; `usize::MAX` (so `~seed == 0`); `1 << 63`; each single-bit seed `1<<k`, k∈0..63; uniform random | lines 10–17: `seed` XORs `v0`/`v2`, `~seed` XORs `v1`/`v3`, and each is applied **twice** (lines 10–13 then 14–17) |
| **A** — pointer alignment of `p` | 8-byte aligned; byte offsets 1..7 from an aligned base | line 20/21 load bytes individually via `unsigned char *`, so C permits any alignment; the Rust translation builds a slice from the raw pointer, which must be equally alignment-agnostic |
| **I** — `siphash`'s `init` | `0`; small positive; `0x7F`/`0x80`/`0x81` boundaries; negative (`-1`, `-128`, `-200`); `INT_MIN`; `INT_MAX` (line 118 `z++` overflows); random | line 117–118 `int z = init; … mem[i] = z;` truncates `int`→`unsigned char`, and `z++` past `INT_MAX` wraps |

## 3. The table (pruned cross-product — one row per combination the C treats differently)

Every row is driven with **many randomized inputs** from a fixed-seed splitmix64 RNG
(see `tests/common/mod.rs`), not a single hand-picked value. Buffer contents are also
snapshot-compared before/after each call to prove neither implementation writes to `p`.

Rows 1–30 are `#[test]` fns in `tests/differential_hash.rs`. Rows 31–38 are fns in
`tests/differential_siphash.rs`, driven in sequence by its single
`#[test] siphash_configuration_rows_31_to_38`: `siphash` writes to fd 1, which is
process-global and also carries libtest's own result lines, so that binary must contain
exactly one `#[test]` for the stdout capture to be race-free.

| #  | entry point(s) | configuration (options set + input shape) | test fn | ✔ |
|----|----------------|-------------------------------------------|---------|---|
| 1  | `stbds_hash_bytes` | L=`len==0`, B=any (buffer must be ignored), S=`0` | `row01_len0_seed0` | [x] |
| 2  | `stbds_hash_bytes` | L=`len==0`, S=`usize::MAX` (`~seed==0`) | `row02_len0_seed_max` | [x] |
| 3  | `stbds_hash_bytes` | L=`len==0`, S=random ×256 | `row03_len0_seed_random` | [x] |
| 4  | `stbds_hash_bytes` | L/T=`len==1` (tail `case 1`), B=random, S=`0` | `row04_len1` | [x] |
| 5  | `stbds_hash_bytes` | T=`len==2` (`case 2`), B=random, S=random | `row05_len2` | [x] |
| 6  | `stbds_hash_bytes` | T=`len==3` (`case 3`, `d[2]` up to `0xFF`), B=random, S=random | `row06_len3` | [x] |
| 7  | `stbds_hash_bytes` | T=`len==4` (`case 4`), B with `d[3] < 0x80` → **no** sign-extension, S=random | `row07_len4_no_signext` | [x] |
| 8  | `stbds_hash_bytes` | T=`len==4` (`case 4`), B with `d[3] >= 0x80` → **sign-extension floods bits 31..63**, S=random | `row08_len4_signext` | [x] |
| 9  | `stbds_hash_bytes` | T=`len==5` (`case 5` → `d[4]<<32`), B=random incl. both `d[3]` classes, S=random | `row09_len5` | [x] |
| 10 | `stbds_hash_bytes` | T=`len==6` (`case 6` → `d[5]<<40`), B=random, S=random | `row10_len6` | [x] |
| 11 | `stbds_hash_bytes` | T=`len==7` (`case 7` → `d[6]<<48`), B=random, S=random | `row11_len7` | [x] |
| 12 | `stbds_hash_bytes` | T=`len==7`, B=all `0xFF` and all `0x00` (extremes of the tail path), S∈{0,1,MAX,1<<63} | `row12_len7_extremes` | [x] |
| 13 | `stbds_hash_bytes` | L=`len==8`: exactly one block, T=0, B=random, S=random | `row13_len8_random` | [x] |
| 14 | `stbds_hash_bytes` | L=`len==8`, B with `d[3] < 0x80` **and** `d[7] < 0x80` (neither word sign-extends) | `row14_len8_no_signext` | [x] |
| 15 | `stbds_hash_bytes` | L=`len==8`, B with `d[3] >= 0x80` (low word sign-extends → high 32 bits stay all-ones, `d[4..8]` swallowed) | `row15_len8_low_signext` | [x] |
| 16 | `stbds_hash_bytes` | L=`len==8`, B with `d[3] < 0x80` **and** `d[7] >= 0x80` (high word sign-extends but the `<<16<<16` must discard it) | `row16_len8_high_signext` | [x] |
| 17 | `stbds_hash_bytes` | L=`len==8`, B=all `0x00` / all `0xFF`, S∈{0,1,MAX,1<<63} | `row17_len8_extremes` | [x] |
| 18 | `stbds_hash_bytes` | L=`len ∈ 9..=15`: one block **+ every** tail case 1..7, B=random, S=random | `row18_len9_15` | [x] |
| 19 | `stbds_hash_bytes` | L=`len==16`: two blocks, T=0, B=random, S=random | `row19_len16` | [x] |
| 20 | `stbds_hash_bytes` | L=`len ∈ 17..=63`: 2–7 blocks × all tails, B=random, S=random | `row20_len17_63` | [x] |
| 21 | `stbds_hash_bytes` | L=`len ∈ 64..=255`, B=random, S=random | `row21_len64_255` | [x] |
| 22 | `stbds_hash_bytes` | L=`len >= 256` (`len << 56` truncation), `len ∈ 256..=1024` all tails, B=random, S=random | `row22_len_ge_256` | [x] |
| 23 | `stbds_hash_bytes` | L=`len % 8 == 0` and large (`len ∈ {512, 1024, 2048, 4096}`), B=random, S=random | `row23_len_multiple_of_8_large` | [x] |
| 24 | `stbds_hash_bytes` | A=**unaligned** `p` (base+1 … base+7) × L=`len ∈ 0..=40` × B=random, S=random | `row24_unaligned_pointer` | [x] |
| 25 | `stbds_hash_bytes` | S=`usize::MAX` × L=`len ∈ 0..=72` × B=random | `row25_seed_max_all_lens` | [x] |
| 26 | `stbds_hash_bytes` | S=`1 << 63` and `S=1` × L=`len ∈ 0..=72` × B=random | `row26_seed_highbit_all_lens` | [x] |
| 27 | `stbds_hash_bytes` | S=each single-bit seed `1<<k`, k∈0..=63, fixed random buffer, L=`len ∈ {0,7,8,15,16,33}` | `row27_seed_single_bit_sweep` | [x] |
| 28 | `stbds_hash_bytes` | B=single-bit-set buffers (avalanche): for L=`len ∈ 1..=32`, every bit position set alone, S=`0` | `row28_single_bit_data_avalanche` | [x] |
| 29 | `stbds_hash_bytes` | B=sequential `z` pattern exactly as `siphash` builds it, L=`len ∈ 0..=63`, S=`0`, over many `init` values — the composed pipeline driven through the **low-level** entry point | `row29_sequential_pattern_all_lens` | [x] |
| 30 | `stbds_hash_bytes` | Full randomized sweep: L=`len ∈ 0..=520` (every length), B=random, S=random, fresh per length | `row30_full_length_sweep` | [x] |
| 31 | `siphash` | I=`0` (stdout compared byte-for-byte) | `row31_siphash_init_0` | [x] |
| 32 | `siphash` | I=small positive `{1,2,3,42,127}` | `row32_siphash_small_positive` | [x] |
| 33 | `siphash` | I=`0x7F`/`0x80`/`0x81`/`0xC0`/`0xFF` (drives `mem` bytes across the `0x80` sign-extension boundary) | `row33_siphash_signext_boundary` | [x] |
| 34 | `siphash` | I=negative `{-1,-2,-64,-128,-200,-255,-256}` (`int`→`unsigned char` truncation) | `row34_siphash_negative` | [x] |
| 35 | `siphash` | I=`INT_MIN` | `row35_siphash_int_min` | [x] |
| 36 | `siphash` | I=`INT_MAX`, `INT_MAX-1`, `INT_MAX-63` (line 118 `z++` overflows mid-loop) | `row36_siphash_int_max_overflow` | [x] |
| 37 | `siphash` | I=64 uniform-random `int` values | `row37_siphash_random_inits` | [x] |
| 38 | `siphash` + `stbds_hash_bytes` | Interleaved / repeated calls in one process (proves there is no hidden state and that both entry points agree with each other) | `row38_interleaved_no_hidden_state` | [x] |

## 4. Feature / build configurations

`Cargo.toml` declares no `[features]`; `c_src/CMakeLists.txt` declares no options or
`#ifdef`s. The complete set of feature combinations is `{default}` = `{--no-default-features}`
= `{--all-features}` = ∅. `./verify_all.sh` runs the whole table under all three
invocations, plus against the **release** `.so` (which additionally uses
`panic = "abort"` and optimizations).
