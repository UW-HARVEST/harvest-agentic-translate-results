# CONFIGS.md — Phase B configuration-surface table

## Axes derived mechanically from the C source

There are **no** runtime option flags, no modes, no `#ifdef`s and no enums in
this library (`grep -cE 'enum|#if|switch' c_src/src/lib.c` → 0). The
"configuration" is therefore entirely (a) the mutable state carried in
`struct tflac` / `struct tflac_md5` on entry, and (b) the shape/values of the
inputs. Every branch the C takes:

* `tflac_md5_addsample`: `if (m->pos >= 64)` (fold-down) and `while (bytes--)`
  (copy count `= pos % 64`, may be 0).
* `update_md5`: fixed 5-iteration loop, no data-dependent branch; but the
  *state* it feeds forward (`md5_ctx.pos`, `.total`, `.buffer`) and the
  unsigned wraparound of `b` make the result state-dependent.

### Axis list

| axis | values the code distinguishes |
|------|-------------------------------|
| `A1` entry point | `tflac_pack_u64le`, `tflac_md5_addsample`, `update_md5` (all three are public exports; the lowest-level two are driven **directly**, not only through `update_md5`) |
| `A2` destination offset for `pack_u64le` | `0`, mid (`1..56`), `63` (spans into the 8-byte tail), `64` (largest offset the library itself generates, exactly fills `buffer[72]`) |
| `A3` `n` / `val` value shape | `0`, `u64::MAX`, single-bit sweep (all 64 positions), byte-pattern, random |
| `A4` initial `md5.pos` | `0`; interior `1..55`; near-boundary `56`,`57`,`63`; at-boundary `64`; past-boundary `65`,`71`,`127`; extreme `0xFFFFFFFF` |
| `A5` `bits` argument | `0`; `8` (1 byte); `64` (the value `update_md5` uses); multiples of 8 (`16,24,32,40,48,56`); **non**-multiples of 8 (`1,7,9,63,65`); oversized `0xFFFFFF00`, `0xFFFFFFFF` |
| `A6` initial `md5.total` | `0`, small, `u64::MAX - k` (wraparound) |
| `A7` initial `md5.buffer` contents | all-zero, `0xAA`/`0x55` pattern, random — matters because the fold-down copies from the tail region |
| `A8` fold-down sub-case | branch not taken (`pos+bytes < 64`); taken with `pos%64 == 0` (zero-length copy); taken with `pos%64 in 1..63` (real copy) |
| `A9` `cur_blocksize` × `channels` | `(0,0)`, `(0,n)`, `(n,0)`, `(1,1)`, product `<40`, `==40`, `>40`, product overflowing `u32` (`0x10000*0x10000`), `(u32::MAX,u32::MAX)` |
| `A10` sample values | all `0`, all `-1`, `i32::MIN`, `i32::MAX`, low-byte-only, random full-range (sign-extension + `&0xFF`) |
| `A11` sample buffer shape | exactly 136 elements (the minimum `update_md5` reads: index `4*32+7`), and 136 + guard padding to prove nothing past 135 is read |
| `A12` call sequencing | single call vs. long randomized *sequence* of mixed `addsample`/`update_md5` calls on one shared context (composed pipeline; state carried forward) |

## Combination table (cross-product pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) against both `.so`s and compared byte-for-byte over the whole
88/96-byte context plus the return value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `tflac_pack_u64le` | offset 0, `n` = 0 / `u64::MAX` / all 64 single-bit values | [x] |
| 2 | `tflac_pack_u64le` | offset swept `0..=64` (every in-bounds offset), random `n` × many | [x] |
| 3 | `tflac_pack_u64le` | offset 64 (buffer-filling boundary), random `n`, full 72-byte buffer compared | [x] |
| 4 | `tflac_pack_u64le` | called repeatedly on one buffer with overlapping offsets (composed writes) | [x] |
| 5 | `tflac_md5_addsample` | `pos=0`, `bits=64`, random `val`, zeroed buffer — no fold-down (A8 not-taken) | [x] |
| 6 | `tflac_md5_addsample` | `pos` swept `0..=63`, `bits=64`, random `val`/buffer — covers not-taken, taken-with-copy, taken-with-`pos%64==0` (`pos=0`→64→0) | [x] |
| 7 | `tflac_md5_addsample` | `pos=56` (`+8 == 64` exactly → fold-down with `pos%64==0`, zero-length copy) | [x] |
| 8 | `tflac_md5_addsample` | `pos=57..63`, `bits=64` → fold-down with a real 1..7-byte tail copy; random tail bytes | [x] |
| 9 | `tflac_md5_addsample` | `pos ∈ {64,65,71,127,255,0xFFFFFFFF}` (out of documented range) × `bits ∈ {0,8,64}` | [x] |
| 10 | `tflac_md5_addsample` | `bits ∈ {0,1,7,8,9,16,24,32,40,48,56,63,64,65}` (incl. non-multiples of 8) × random `pos`/`val` | [x] |
| 11 | `tflac_md5_addsample` | `bits ∈ {0xFFFFFF00, 0xFFFFFFF8, 0xFFFFFFFF}` (oversized; `pos` add wraps u32) | [x] |
| 12 | `tflac_md5_addsample` | `total ∈ {0, 1, u64::MAX, u64::MAX-63}` × `bits` large → `total` wraparound | [x] |
| 13 | `tflac_md5_addsample` | buffer preloaded all-zero / `0xAA` / `0x55` / random, `pos=60`, fold-down copy source verified | [x] |
| 14 | `tflac_md5_addsample` | **sequence**: 500 randomized calls on one shared context, state carried forward, compared after every call | [x] |
| 15 | `update_md5` | `(cur_blocksize,channels) = (1,1)`, `pos=0`, `total=0`, samples all zero | [x] |
| 16 | `update_md5` | product `>40` (e.g. `4096×2`, `1152×8`), random samples, min 136-element buffer | [x] |
| 17 | `update_md5` | product `==40` → returns exactly 0 | [x] |
| 18 | `update_md5` | product `<40` (`1..39`) → return-value underflow | [x] |
| 19 | `update_md5` | `(0,0)`, `(0,n)`, `(n,0)` → `b==0` → returns `0xFFFFFFD8` | [x] |
| 20 | `update_md5` | product overflows u32: `(0x10000,0x10000)`, `(u32::MAX,u32::MAX)`, `(0xFFFF,0x10001)` | [x] |
| 21 | `update_md5` | samples all `-1`, all `i32::MIN`, all `i32::MAX`, alternating sign (sign-extend + `&0xFF`) | [x] |
| 22 | `update_md5` | samples fully random `i32` (many seeds), all 136 elements distinct | [x] |
| 23 | `update_md5` | initial `md5.pos ∈ {0,8,56,57,60,63,64,65,127,0xFFFFFFFF}` — 5 folds inside one call | [x] |
| 24 | `update_md5` | initial `md5.total ∈ {0, u64::MAX-100}` (wraparound across the 5 `+=64`) | [x] |
| 25 | `update_md5` | initial `md5.buffer` random / patterned (fold-down reads the tail) | [x] |
| 26 | `update_md5` | 136-element buffer + guard region: proves stride is 32 elements and index 135 is the last read | [x] |
| 27 | `update_md5` | **sequence**: repeated calls on one context with advancing sample window (composed pipeline) | [x] |
| 28 | mixed | **sequence**: randomized interleaving of `pack_u64le`, `addsample` and `update_md5` on one shared `tflac`, full-state compare after each step | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default (empty) feature set. Verified by
`cargo metadata`/grep; `--no-default-features` is therefore identical to the
default build. Both were run (see `scripts`-style loop in the test log).

## Test mapping and result

Every row above maps 1:1 onto a test in `tests/phase_b_valid.rs`
(`row01_…` … `row28_…`), plus `abi_layout_matches_c` and
`both_libraries_export_all_three_symbols`.

**Status: 30/30 valid-path tests pass.** Each is driven by the fixed-seed
xorshift64* PRNG in `tests/common/mod.rs` with hundreds to thousands of inputs
per row (≈300k differential calls in total), comparing the *entire* 88/96-byte
context image — including struct padding and a 192-byte trailing guard — plus
the `u32` return value of `update_md5`.

The lowest-level entry points (`tflac_pack_u64le`, `tflac_md5_addsample`) are
driven **directly** through their `.so` exports, not only via `update_md5`;
rows 4, 14, 27 and 28 additionally drive long randomized *sequences* on a
single shared context so the composed pipeline is covered.

## Binary / driver executable

There is none (`c_src/CMakeLists.txt` has no `add_executable`;
`translation/Cargo.toml` has no `[[bin]]`), so the "compare stdout" gate is N/A.

## Cross-optimisation matrix (run by `./run_tests.sh`)

| C build | Rust build | result |
|---|---|---|
| CMake default (`-O0`) | `--release` | 50/50 pass |
| CMake default (`-O0`) | debug (`-O0`) | 50/50 pass |
| `-O2` (`CMAKE_BUILD_TYPE=Release`) | `--release` | 50/50 pass |

Feature combinations exercised: default, `--no-default-features`,
`--all-features` — identical, because the crate declares no features.
