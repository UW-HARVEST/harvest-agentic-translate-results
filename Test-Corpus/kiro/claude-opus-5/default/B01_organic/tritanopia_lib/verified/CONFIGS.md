# CONFIGS.md — configuration-surface table (Phase B gate)

## Mechanical derivation of the axes

The library has **no runtime options, no modes, no flags, no `#ifdef`, no
`enum`, no build features** — grepped for and confirmed absent (see
`ERRORS.md`). `translation/Cargo.toml` declares no `[features]` table, so the
Cargo feature cross-product is the single default configuration.

The only public entry point is `tritanopia` (`c_src/include/lib.h:7`); it is also
the *lowest-level* entry point, because every other function in the library is
file-local `static` and therefore not reachable by an external caller. There is
no convenience-wrapper / low-level split to worry about.

So the configuration surface is entirely **input shape**: which of the C's
value-dependent branches each of the three channels takes. Those branches were
enumerated from the source and their boundaries measured with an instrumented
build of the real C code (`/tmp/probe.c`, which `#include`s the same logic):

```
A1 removeGamma: linear for bytes 0..10, pow for 11..255
A2 R: post-matrix min=-0.127398863 at (0,0,255)  max=1.12739885 at (255,255,0)
      applyLinear=1796020  applyPow=14981196  negative=1666521  denormOutOfRange=1814886
A2 G: post-matrix min=-4.48600011e-11 at (255,0,0) max=1 at (0,255,255)
      applyLinear=88320    applyPow=16688896  negative=255      denormOutOfRange=0
A2 B: post-matrix min=0 at (0,0,0)             max=1 at (0,255,255)
      applyLinear=88320    applyPow=16688896  negative=0        denormOutOfRange=0
```

### The axes

| axis | source site | values it can take |
|---|---|---|
| **X1** `cbRemoveGammaRGB` branch, per channel | `lib.c:11` `RGB.x > 0.04045` | `linear` (byte ≤ 10) / `pow` (byte ≥ 11) — boundary **10 / 11**, measured above |
| **X2** `cbApplyGammaRGB` branch, per channel | `lib.c:35` `RGB.x > 0.0031308…` | `linear` / `pow`; both reachable on all three channels |
| **X3** `cbDenorm` conversion range, per channel | `lib.c:28` `(unsigned char)(x*255.f+0.5f)` | `in range [0,256)` / `negative → wraps` / `≥ 256 → wraps`; the out-of-range states are reachable **only on the red channel** (1 814 886 of 16 777 216 inputs) |
| **X4** sign of the red row | `lib.c:50` `R + 0.1274*G − 0.1274*B` | `G > B` (pushes red up, can exceed 1.0) / `G == B` (red unchanged) / `G < B` (drives red down, can go negative) |
| **X5** channel-symmetry / aliasing shape | `Tritanopia` reads all three inputs into locals *before* writing any pointee (`lib.c:48-53`) | `R=G=B` (grey axis) / two equal / all distinct; plus the pure primaries and secondaries where two channels are 0 |
| **X6** per-channel extremes | `cbNorm` `byte/255.f` | `0`, `1`, `10`, `11`, `127`, `128`, `254`, `255` |

X1–X6 are *per-channel*, so a "configuration" is a point in the 3-channel
cross-product. The full cross-product **is** the 2^24 input domain, and the
domain is small enough to enumerate, so row R0 below is an exhaustive sweep that
subsumes every other row. The remaining rows exist because the task requires one
row per meaningful combination with randomized inputs, and because a targeted
failing row localises a bug far better than "somewhere in 16.7 M inputs".

## Configuration table

Every row calls **both** `.so`s through `libloading` and compares the three
returned bytes exactly. Rows R1–R16 use a fixed-seed (`0x5EED_C0DE_1234_5678`)
xorshift generator with **≥ 4096 randomized inputs each** (R0 uses all of them).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| R0 | `tritanopia` | **EXHAUSTIVE**: all 2^24 = 16 777 216 `(R,G,B)` byte triples. Subsumes every axis combination below, including all X1×X2×X3×X4×X5 interactions. | [x] |
| R1 | `tritanopia` | X1 = `linear` on **all three** channels (every channel ≤ 10) — the pre-gamma linear regime | [x] |
| R2 | `tritanopia` | X1 = `pow` on **all three** channels (every channel ≥ 11) — the pre-gamma power regime | [x] |
| R3 | `tritanopia` | X1 mixed: R `linear`, G/B `pow` (R ≤ 10, G,B ≥ 11) | [x] |
| R4 | `tritanopia` | X1 mixed: G `linear`, R/B `pow` | [x] |
| R5 | `tritanopia` | X1 mixed: B `linear`, R/G `pow` | [x] |
| R6 | `tritanopia` | X1 boundary sweep: each channel drawn from `{9,10,11,12}` — straddles the measured 10/11 threshold in all 64 combinations | [x] |
| R7 | `tritanopia` | X4 = `G > B` with X3 overflow: `G` large, `B` small ⇒ red row exceeds 1.0 ⇒ `cbDenorm` converts ≥ 256 and **wraps** | [x] |
| R8 | `tritanopia` | X4 = `G < B` with X3 negative: `B` large, `G` small, `R` small ⇒ red row negative ⇒ `cbDenorm` converts a negative float and **wraps** | [x] |
| R9 | `tritanopia` | X4 = `G == B` (red row reduces to `R` plus the tiny coefficient difference `0.12739886310880 − 0.12739886341072`, which is *not* exactly zero — a value-dependent 1-ulp trap) | [x] |
| R10 | `tritanopia` | X2 = `linear` on the red output (post-matrix red ≤ 0.0031308…): small `R`, `G ≈ B` | [x] |
| R11 | `tritanopia` | X2 = `linear` on green **and** blue outputs (both `G` and `B` small) while red takes `pow` | [x] |
| R12 | `tritanopia` | X5 = grey axis `R = G = B`, all 256 values | [x] |
| R13 | `tritanopia` | X5 = two channels equal (`R=G`, `G=B`, `R=B`), randomized | [x] |
| R14 | `tritanopia` | X5 = one channel 0, the other two randomized (the three coordinate planes of the RGB cube) | [x] |
| R15 | `tritanopia` | X5 = one channel 255, the other two randomized (the three far faces of the cube) | [x] |
| R16 | `tritanopia` | X6 = all 8^3 = 512 combinations of the extreme/boundary set `{0,1,10,11,127,128,254,255}`, including all 8 corners of the cube | [x] |
| R17 | `tritanopia` | ABI shape: the same 3-byte struct passed with **non-zero junk in the unused high bytes** of the argument eightbyte, across randomized inputs (see `ERRORS.md` E6) | [x] |
| R18 | `tritanopia` | Repeatability / statelessness: the same input called many times, interleaved between the two `.so`s, in a shuffled order — the C keeps no state, so order must not matter | [x] |

**All 19 rows are checked off** by passing differential tests in
`tests/differential.rs`.

## Feature combinations

| combo | command | status |
|---|---|---|
| default (the only one) | `cargo test --release` | pass |
| `--no-default-features` (identical, no features declared) | `cargo test --release --no-default-features` | pass |
