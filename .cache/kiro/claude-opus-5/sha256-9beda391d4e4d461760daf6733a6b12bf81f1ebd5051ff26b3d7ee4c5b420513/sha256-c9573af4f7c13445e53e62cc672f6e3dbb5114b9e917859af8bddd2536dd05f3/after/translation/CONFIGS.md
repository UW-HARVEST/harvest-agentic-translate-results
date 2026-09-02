# CONFIGS.md — configuration surface table (Phase B gate)

## Mechanical derivation of the axes

Enumerated from the branches the C source actually takes (`c_src/src/lib.c`) and
the full public API (`c_src/include/lib.h` + non-`static` definitions in `lib.c`).

### Public entry points (FULL set, lowest-level first)

| level | symbol | signature |
|-------|--------|-----------|
| low   | `stbds_hash_bytes` | `size_t stbds_hash_bytes(void *p, size_t len, size_t seed)` — direct, parameterised access to the hash core (`lib.c:110`) |
| high  | `siphash`          | `void siphash(int init)` — one-shot convenience wrapper: builds a 64-byte buffer from `init` and prints 64 hashes (`lib.c:114`) |

(`stbds_siphash_bytes` is `static`, reachable only via `stbds_hash_bytes`.)

### Runtime options / modes

There are **no** flags, modes, globals, setters, or `#ifdef`s. Grep confirms:
0 `#if`/`#ifdef`, 0 global/`static` mutable state, 0 option struct. The only
runtime-selectable inputs are the function arguments themselves:

- `seed` (`size_t`) — mixed into all four of `v0..v3` **and** into `~seed` for
  `v1`/`v3`; a pure data axis with no branching.
- `len` (`size_t`) — the **branching** axis (see below).
- `p` (`void *`) — buffer contents; the branching axis for the signed-overflow
  paths.
- `init` (`int`, `siphash` only) — seeds the synthetic buffer contents.

### Input shapes the C special-cases

Branch points, exhaustively:

1. `for (i = 0; i + sizeof(size_t) <= len; ...)` → **block count** `nblocks = len / 8`
   ∈ {0, 1, many}.
2. `switch (len - i)` → **tail remainder** `len % 8` ∈ {0,1,2,3,4,5,6,7} — eight
   distinct fall-through entry points (`case 7` … `case 0`).
3. `data |= (d[3] << 24)` / `d[3]`, `d[7]` in the block path → **byte-value
   class**: top byte `< 0x80` (positive `int`) vs `>= 0x80` (signed overflow →
   sign-extension into the upper 32 bits of `size_t`). This is a *value-dependent*
   code path, not a size-dependent one.
4. `data = len << 56` → **length byte** `len & 0xFF`; lengths congruent mod 256
   collide in this term (e.g. 0 vs 256), so length must be varied past 255.
5. `seed` / `~seed` → **seed class**: 0, all-ones, low/high-half-only, random.
6. Byte order / element type: **none** — the C is byte-oriented and reads bytes
   individually (`d[0] | d[1]<<8 | ...`), i.e. it hard-codes little-endian
   assembly of `data` regardless of host endianness. One row records this.

Cross-product = {block count 0/1/many} × {tail 0..7} × {top-byte class} ×
{seed class}, pruned to the combinations the code actually distinguishes.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `stbds_hash_bytes` | `nblocks=0`, `tail=0` → `len == 0`; `seed = 0`. Empty message, loop skipped, `case 0`. | [x] |
| C2 | `stbds_hash_bytes` | `nblocks=0`, `tail=1..7` → `len ∈ 1..7`; `seed = 0`; randomized bytes. Tail-only, each `switch` arm. | [x] |
| C3 | `stbds_hash_bytes` | `nblocks=0`, `tail=1..7`; bytes forced so `d[3] >= 0x80` (`case 4` sign-extension path) and `d[3] < 0x80`, both. | [x] |
| C4 | `stbds_hash_bytes` | `nblocks=1`, `tail=0` → `len == 8`; randomized bytes; `seed = 0`. Exactly one loop iteration, `case 0`. | [x] |
| C5 | `stbds_hash_bytes` | `nblocks=1`, `tail=1..7` → `len ∈ 9..15`; randomized bytes. One block + every tail arm. | [x] |
| C6 | `stbds_hash_bytes` | `nblocks=many` (2..64), `tail=0..7` → `len ∈ 16..520`, all residues mod 8; randomized bytes. | [x] |
| C7 | `stbds_hash_bytes` | Block path, `d[3] < 0x80` **and** `d[7] < 0x80` for every block (no signed overflow anywhere). | [x] |
| C8 | `stbds_hash_bytes` | Block path, `d[3] >= 0x80`, `d[7] < 0x80` (low-half sign-extension only → upper 32 bits of `data` become all-ones before the `hi` OR). | [x] |
| C9 | `stbds_hash_bytes` | Block path, `d[3] < 0x80`, `d[7] >= 0x80` (high-half negative → `(size_t)neg << 16 << 16` drops the sign bits). | [x] |
| C10 | `stbds_hash_bytes` | Block path, `d[3] >= 0x80` **and** `d[7] >= 0x80` in every block (both overflow paths simultaneously). | [x] |
| C11 | `stbds_hash_bytes` | All-zero buffer, `len ∈ 0..80`, `seed = 0` — degenerate content, isolates the `len << 56` length term. | [x] |
| C12 | `stbds_hash_bytes` | All-`0xFF` buffer, `len ∈ 0..80` — maximal sign-extension in every position. | [x] |
| C13 | `stbds_hash_bytes` | `seed = 0` vs `SIZE_MAX` vs `0x00000000FFFFFFFF` vs `0xFFFFFFFF00000000` vs `1` vs `SIZE_MAX-1`, each over randomized `len ∈ 0..200` and bytes (seed-class × shape cross-product). | [x] |
| C14 | `stbds_hash_bytes` | Randomized `seed` (full 64-bit) × randomized `len ∈ 0..1024` × randomized bytes — the broad property-style sweep, fixed PRNG seed. | [x] |
| C15 | `stbds_hash_bytes` | `len & 0xFF` aliasing: `len ∈ {0,256,512}`, `{1,257,513}`, `{255,511}` with the same repeating content — checks the `len << 56` truncation is reproduced. | [x] |
| C16 | `stbds_hash_bytes` | Unaligned buffer start (`p = base+1 .. base+7`) with `len ∈ 0..64` — the C reads byte-wise so alignment must not matter; catches a Rust translation that used a `usize` load. | [x] |
| C17 | `stbds_hash_bytes` | Endianness/element-type row: multi-byte integers written as native `u16`/`u32`/`u64`/`f64` arrays then hashed as bytes; the C assembles `data` little-endian-by-construction. | [x] |
| C18 | `stbds_hash_bytes` | Exact-size heap allocations (no slack) for `len ∈ 0..24`, so any over-read is caught by the allocator/ASAN-style bounds. | [x] |
| C19 | `siphash` (wrapper) | `init = 0` — the documented/default invocation; compare all 64 printed lines byte-for-byte. | [x] |
| C20 | `siphash` (wrapper) | `init ∈ {1, -1, 42, 127, 128, 192, 255, 256, -256, 0x7A, 0xF9, 250}` — drives `mem[]` across the `0x80` byte-class boundary and through `unsigned char` truncation of a negative/large `int`. | [x] |
| C21 | `siphash` (wrapper) | `init = INT_MAX`, `INT_MIN` — `z++` signed-overflow wrap inside the fill loop. | [x] |
| C22 | `siphash` (wrapper) | Randomized `init` (full `int` range, fixed PRNG seed), full stdout compared. Exercises the wrapper's internal `len = 0..63` sweep, i.e. the composed pipeline `siphash → stbds_hash_bytes → stbds_siphash_bytes`. | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, therefore exactly one build configuration exists:

```
$ grep -c '\[features\]' Cargo.toml
0
```

The default (and only) combination is verified; `--no-default-features` is
equivalent to the default here. Both the **debug** and **release** Rust `cdylib`
are loaded and differentially tested, since `[profile.release] panic = "abort"`
and debug overflow-checks make them genuinely different builds of the same code.

## Test sensitivity (negative control)

Row coverage only means something if the tests can actually see a divergence.
`./mutation_check.sh` injects 17 deliberate behavioural changes into
`src/lib.rs` — dropped sign-extensions, wrong rotate constants, off-by-one tail
`switch` boundaries, wrong shift amounts, dropped finalisation steps, an altered
block-loop bound, `wrapping` → `saturating` in the `siphash` fill loop — rebuilds
both cdylib profiles and re-runs the suite for each. Result: **17 killed,
0 survived.**

One mutation is deliberately excluded as an *equivalent* mutant rather than a
coverage gap: removing the sign-extension in
`let hi_sext = (hi as i32) as i64 as u64 as usize` is bit-identical, because the
following `<< 16 << 16` discards every bit the sign extension sets. This is
proven exhaustively over the boundary values (and over 200k random ones) by
`d6_high_half_sign_extension_is_a_no_op` in `tests/phase_d_symbols.rs`.

## Reproducing

```
./verify_all.sh        # C build + symbol parity + all phases, all combinations
./mutation_check.sh    # negative control: the suite must kill every mutant
```
