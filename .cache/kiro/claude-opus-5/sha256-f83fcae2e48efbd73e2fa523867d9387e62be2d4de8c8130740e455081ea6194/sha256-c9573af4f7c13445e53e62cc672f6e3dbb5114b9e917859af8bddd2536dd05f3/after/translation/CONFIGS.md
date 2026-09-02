# CONFIGS.md — configuration surface table (Phase B)

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from
`c_src/include/lib.h` + `c_src/src/lib.c` (and the gcc codegen for that source),
not from guesses about what matters.

## Axis enumeration

### Axis 1 — runtime options the public API can set

The public header exposes exactly one option: the first parameter of the only
public function.

```c
typedef enum cb_impairment { cbProtanopia, cbDeuteranopia, cbTritanopia, } cb_impairment;
void colourblind(cb_impairment Impairment, float *R, float *G, float *B);
```

`switch (Impairment)` in `src/lib.c:25` branches on it, giving **3 valid states**
(`0`, `1`, `2`), each selecting a different 3x3 matrix. There are no other flags,
no `#ifdef`s, no environment variables, no init/config struct and no global
state. (`grep -c '#ifdef\|#if ' src/lib.c` -> 0.)

### Axis 2 — public entry points, including the lowest level

| entry point | linkage | reachable across FFI? |
|-------------|---------|-----------------------|
| `colourblind` | exported (`T` in `nm -D`) | yes — the one-shot dispatcher |
| `Protanopia` | `static` | no |
| `Deuteranopia` | `static` | no |
| `Tritanopia` | `static` | no |

The three matrix routines **are** the low-level entry points, but `static`
linkage keeps them out of both `.so`s' dynamic symbol tables (verified in
`SYMBOLS.md`). They are therefore driven directly and individually by pinning
`Impairment` to `0` / `1` / `2`, which is the only way an external caller can
reach them; every row below names which one it exercises. Rows are not confined
to a single "convenience wrapper" — `colourblind` *is* the whole API.

### Axis 3 — input value shapes the code distinguishes

`src/lib.c` contains no value-dependent branch: each transform is nine
`mulss`/`addss`/`subss` instructions, straight-line. The distinctions that
matter are the ones the **hardware** draws, since they are what can diverge
between the C and Rust codegen:

| class | why the code/hardware treats it differently |
|-------|---------------------------------------------|
| V1 typical colour, each channel in `[0,1]` | the intended domain; ordinary normals |
| V2 arbitrary finite normal (full exponent range, random mantissa) | exercises rounding of every product and sum; catches a wrong coefficient in a low mantissa bit |
| V3 `+0.0` / `-0.0` | signed-zero rules: `-0.0 + 0.0 == +0.0`, `c * -0.0` sign, and `x - x == +0.0` |
| V4 subnormals incl. `f32::from_bits(1)` | gradual underflow; products of subnormals flush to zero |
| V5 `±FLT_MAX`, `±FLT_MIN` | products overflow to `inf` / underflow to subnormal, so the later adds see specials |
| V6 `±inf` | `inf * c`, `inf + inf`, and `inf - inf` -> invalid op |
| V7 quiet NaN, varying sign and payload | NaN payload/sign propagation; SSE forwards the **`dst`** operand when both are NaN, so operand roles must match |
| V8 signalling NaN, varying sign and payload | SSE quiets it (sets mantissa MSB) while preserving sign and the rest of the payload |
| V9 mixed — exactly one channel special, the other two ordinary normals (all 3 positions x V3..V8) | the case that actually pins the `dst`/`src` priority per instruction; a uniformly-special input hides it |
| V10 unit basis vectors `(1,0,0)`, `(0,1,0)`, `(0,0,1)` | recovers each matrix coefficient **bit-exactly**, so a mis-rounded literal is caught directly |

### Axis 4 — pointer/aliasing shapes

Each transform copies all three inputs into locals **before** writing any
output (`float R = *Red, G = *Green, B = *Blue;`), so aliasing is a real,
well-defined configuration, and a translation that wrote outputs incrementally
would diverge here.

| class | shape |
|-------|-------|
| A1 | three distinct `float`s |
| A2 | `R` and `G` are the same object (`p, p, q`) |
| A3 | `R` and `B` are the same object (`p, q, p`) |
| A4 | `G` and `B` are the same object (`p, q, q`) |
| A5 | all three are the same object (`p, p, p`) |
| A6 | three distinct, non-contiguous heap allocations (rules out any assumption of adjacency) |

### Axis 5 — feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the cross-product over
features is a single point: the default build. `--all-features`,
`--no-default-features` and the default are the same configuration; the harness
runs all three anyway. Verified mechanically:

```
$ cargo metadata --format-version 1 --no-deps | jq '.packages[].features'
{}
```

### Axis 6 — build profile

Both Rust `cdylib`s are loaded and compared in every row: `target/release/`
(optimised, `panic = "abort"`) and `target/debug/` (unoptimised, with
`debug-assertions`). They are compiled at different optimisation levels and could
in principle diverge from each other, so parity is required for each. Row counts
in the table below are per-Rust-`.so`; the actual number of bit-exact comparisons
is twice that, and each row asserts its own comparison count so a silently-empty
loop cannot pass.

## Table

Cross-product of Axes 1 x 3 x 4, pruned to the combinations the code actually
distinguishes. Every row runs **many randomized inputs with a fixed seed**
(`SEED = 0x5EED_C01D_1234_5678`, SplitMix64), comparing the C `.so` and the Rust
`.so` bit-for-bit on all three outputs. `N` is the number of random draws per
row.

| # | entry point(s) | configuration (options set + input shape) | N | test | [x] |
|---|----------------|--------------------------------------------|---|------|-----|
| C1 | `colourblind` -> `Protanopia` | `Impairment=0`, V1 in-gamut `[0,1]`, A1 distinct | 20000 | `cfg_row` | [x] |
| C2 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V1 in-gamut `[0,1]`, A1 distinct | 20000 | `cfg_row` | [x] |
| C3 | `colourblind` -> `Tritanopia` | `Impairment=2`, V1 in-gamut `[0,1]`, A1 distinct | 20000 | `cfg_row` | [x] |
| C4 | `colourblind` -> `Protanopia` | `Impairment=0`, V2 arbitrary finite normals, A1 | 20000 | `cfg_row` | [x] |
| C5 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V2 arbitrary finite normals, A1 | 20000 | `cfg_row` | [x] |
| C6 | `colourblind` -> `Tritanopia` | `Impairment=2`, V2 arbitrary finite normals, A1 | 20000 | `cfg_row` | [x] |
| C7 | `colourblind` -> `Protanopia` | `Impairment=0`, V3 signed zeros (all 8 sign combos), A1 | 8 exhaustive | `cfg_row` | [x] |
| C8 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V3 signed zeros (all 8), A1 | 8 exhaustive | `cfg_row` | [x] |
| C9 | `colourblind` -> `Tritanopia` | `Impairment=2`, V3 signed zeros (all 8), A1 | 8 exhaustive | `cfg_row` | [x] |
| C10 | `colourblind` -> `Protanopia` | `Impairment=0`, V4 subnormals, A1 | 20000 | `cfg_row` | [x] |
| C11 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V4 subnormals, A1 | 20000 | `cfg_row` | [x] |
| C12 | `colourblind` -> `Tritanopia` | `Impairment=2`, V4 subnormals, A1 | 20000 | `cfg_row` | [x] |
| C13 | `colourblind` -> `Protanopia` | `Impairment=0`, V5 extremes `±FLT_MAX`/`±FLT_MIN`, A1 | 20000 | `cfg_row` | [x] |
| C14 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V5 extremes, A1 | 20000 | `cfg_row` | [x] |
| C15 | `colourblind` -> `Tritanopia` | `Impairment=2`, V5 extremes, A1 | 20000 | `cfg_row` | [x] |
| C16 | `colourblind` -> `Protanopia` | `Impairment=0`, V6 `±inf` (all 8 combos), A1 | 8 exhaustive | `cfg_row` | [x] |
| C17 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V6 `±inf` (all 8), A1 | 8 exhaustive | `cfg_row` | [x] |
| C18 | `colourblind` -> `Tritanopia` | `Impairment=2`, V6 `±inf` (all 8), A1 | 8 exhaustive | `cfg_row` | [x] |
| C19 | `colourblind` -> `Protanopia` | `Impairment=0`, V7 quiet NaNs, random sign+payload, A1 | 20000 | `cfg_row` | [x] |
| C20 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V7 quiet NaNs, A1 | 20000 | `cfg_row` | [x] |
| C21 | `colourblind` -> `Tritanopia` | `Impairment=2`, V7 quiet NaNs, A1 | 20000 | `cfg_row` | [x] |
| C22 | `colourblind` -> `Protanopia` | `Impairment=0`, V8 signalling NaNs, random sign+payload, A1 | 20000 | `cfg_row` | [x] |
| C23 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V8 signalling NaNs, A1 | 20000 | `cfg_row` | [x] |
| C24 | `colourblind` -> `Tritanopia` | `Impairment=2`, V8 signalling NaNs, A1 | 20000 | `cfg_row` | [x] |
| C25 | `colourblind` -> `Protanopia` | `Impairment=0`, V9 one special channel + two normals, A1 | 20000 | `cfg_row` | [x] |
| C26 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V9 one special channel + two normals, A1 | 20000 | `cfg_row` | [x] |
| C27 | `colourblind` -> `Tritanopia` | `Impairment=2`, V9 one special channel + two normals, A1 | 20000 | `cfg_row` | [x] |
| C28 | `colourblind` -> `Protanopia` | `Impairment=0`, V10 unit basis vectors (3 vectors) | 3 exhaustive | `cfg_row` | [x] |
| C29 | `colourblind` -> `Deuteranopia` | `Impairment=1`, V10 unit basis vectors | 3 exhaustive | `cfg_row` | [x] |
| C30 | `colourblind` -> `Tritanopia` | `Impairment=2`, V10 unit basis vectors | 3 exhaustive | `cfg_row` | [x] |
| C31 | `colourblind` -> all three | `Impairment=0,1,2`, mixed value classes drawn per-channel independently ("everything" generator), A1 | 60000 | `cfg_row` | [x] |
| C32 | `colourblind` -> `Protanopia` | `Impairment=0`, A2 `R`/`G` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C33 | `colourblind` -> `Protanopia` | `Impairment=0`, A3 `R`/`B` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C34 | `colourblind` -> `Protanopia` | `Impairment=0`, A4 `G`/`B` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C35 | `colourblind` -> `Protanopia` | `Impairment=0`, A5 all three aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C36 | `colourblind` -> `Deuteranopia` | `Impairment=1`, A2 `R`/`G` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C37 | `colourblind` -> `Deuteranopia` | `Impairment=1`, A3 `R`/`B` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C38 | `colourblind` -> `Deuteranopia` | `Impairment=1`, A4 `G`/`B` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C39 | `colourblind` -> `Deuteranopia` | `Impairment=1`, A5 all three aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C40 | `colourblind` -> `Tritanopia` | `Impairment=2`, A2 `R`/`G` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C41 | `colourblind` -> `Tritanopia` | `Impairment=2`, A3 `R`/`B` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C42 | `colourblind` -> `Tritanopia` | `Impairment=2`, A4 `G`/`B` aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C43 | `colourblind` -> `Tritanopia` | `Impairment=2`, A5 all three aliased, mixed values | 20000 | `cfg_alias_row` | [x] |
| C44 | `colourblind` -> all three | `Impairment=0,1,2`, A6 three separate heap boxes, mixed values | 30000 | `cfg_heap_row` | [x] |
| C45 | `colourblind` -> all three | idempotence/statelessness: the same call repeated 64x on fresh buffers, and calls interleaved across impairments, must give identical results in both `.so`s (no hidden state) | 3x64 + 30000 interleaved | `cfg_stateless_row` | [x] |
| C46 | `colourblind` -> all three | **exhaustive** over the top 16 bits of the `f32` space for a single varying channel with the other two pinned (all 65536 exponent/high-mantissa patterns x 3 channels x 3 impairments) | 589824 exhaustive | `cfg_exhaustive_high_bits` | [x] |

## Harness sensitivity (why a passing row means something)

Passing rows only prove correctness if the harness can actually detect a wrong
answer. `../mutate.py` injects 19 specific bug classes into `src/lib.rs`, rebuilds
both cdylibs, and reports which phase catches each. **All 19 are caught:**

| mutation | caught by |
|----------|-----------|
| `dispatch-swap-0-1` — swap the Protanopia/Deuteranopia discriminants | B, C |
| `dispatch-add-default` — add a `default:` arm applying Protanopia | C |
| `dispatch-clamp-out-of-range` — clamp out-of-range to Tritanopia | C |
| `dispatch-mod3-normalise` — `impairment.rem_euclid(3)` | C |
| `coeff-1ulp-P_RR` / `coeff-1ulp-P_GG` — 1-ULP coefficient error | B, C |
| `coeff-sign-T_GR` — sign flip on a `4.5e-11` coefficient | B, C |
| `coeff-round-T_RB` — re-round a pooled Tritanopia literal | B, C |
| `coeff-drop-tiny-P_RB` — drop a `2.9e-9` term as "negligible" | B, C |
| `op-sub-to-add` — turn `subss` into `addss` | B, C |
| `op-reassociate-add` — swap `addss` operand roles (4 sites) | B, C |
| `nan-drop-sse-emulation` — use plain `f32` operators | B, C |
| `nan-src-before-dst` — invert SSE's NaN operand priority | B, C |
| `nan-quiet-noop` — stop quieting forwarded NaNs | B, C |
| `nan-quiet-drops-payload` — quiet to a canonical NaN, losing the payload | B, C |
| `nan-default-qnan-sign` — `0x7FC00000` instead of `0xFFC00000` | B, C |
| `alias-reverse-write-order` — write `*blue` before `*red` (3 sites) | B (rows C32–C43 only) |
| `symbol-drop-no-mangle` — remove the export | B, C, D |
| `symbol-export-static-helper` — also export `Protanopia` | D |

Two mutations were investigated and found to be **equivalent mutants** rather
than suite holes, and were replaced:

* `if impairment < 0 { return; }` before the `match` — every negative `int` is
  already out of range in C, so the early return is behaviourally identical.
* `T_GG = 0.8739093` — rounds to the same `f32` (`0x3F5FB885`) as
  `0.87390929928361`, which is also the exact value in the C `.rodata` at offset
  `0x38`.

## Cross-check against the C object's constant pool

`objdump -r` on `lib.c.o` shows gcc pooled 17 distinct `float` constants for the
24 source literals, revealing which literal pairs round to the same `f32`:
`P_RR≡P_GR`, `P_RG≡P_GG`, `D_RR≡D_GR`, `D_RG≡D_GG`, `T_RG≡T_RB`, `T_GG≡T_BG`,
`T_GB≡T_BB`. Rows C28–C30 (unit basis vectors) recover every coefficient
bit-exactly through the FFI boundary and confirm the Rust constants agree, so the
translation's choice to spell each literal out separately is verified rather than
assumed.
