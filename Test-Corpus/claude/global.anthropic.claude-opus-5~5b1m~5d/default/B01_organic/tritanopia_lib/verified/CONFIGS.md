# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branches the C in `c_src/src/lib.c` actually
takes. Every row is a differential test that calls **both** `.so`s through
`libloading` and compares the 3 result bytes exactly.

## Axis derivation

### Runtime options / modes / flags: NONE

`c_src/include/lib.h` exposes one function and one struct. There is no init
call, no context/handle, no options struct, no setter, no global, no
`#ifdef`/`#if` in the source, and no `[features]` in `translation/Cargo.toml`.
So the configuration surface is entirely made of **input shapes**, not flags.

### Public entry points

`tritanopia` is the only exported symbol (see `SYMBOLS.md`), so it is
simultaneously the highest- and lowest-level public entry point. The five
`static` helpers (`cbNorm`, `cbRemoveGammaRGB`, `Tritanopia`, `cbApplyGammaRGB`,
`cbDenorm`) are **not** reachable from outside the `.so` and cannot be called
directly by any consumer. They are therefore exercised *through* the composed
pipeline, which is what makes the whole-pipeline rows below (C9–C17) the real
test — a per-helper test is impossible here, and a single happy-path call would
miss the value-dependent branches the helpers contain.

### Input-shape axes the C branches on

1. **`cbRemoveGammaRGB` arm, per channel** (3 independent ternaries,
   `lib.c:13-18`): `RGB.c > 0.04045` where `c = byte/255`.
   `0.04045 * 255 = 10.31...`, so **byte <= 10 -> linear arm (`/12.92`)**,
   **byte >= 11 -> `pow` arm**. 2 states x 3 channels = 8 combinations.
2. **`cbApplyGammaRGB` arm, per channel** (3 ternaries, `lib.c:36-44`):
   `v > 0.00313080495356037151702786377709`. Reachable both ways: the matrix's
   red row `R + 0.1274*G - 0.1274*B` goes **negative** whenever `B > G` and `R`
   is small, which sends red down the `*12.92` linear arm; the green/blue rows
   (`~0.874*G + 0.126*B`) only fall below the threshold when both `G` and `B`
   are near zero.
3. **`cbDenorm` conversion outcome, per channel** (`lib.c:29-31`, lowered to
   `cvttss2si %xmm0,%eax; mov %al,..`): the pre-cast value `v*255+0.5` lands
   **in `[0,255]`**, **below 0** (wraps mod 256), or **above 255** (wraps mod
   256). All three are reachable — see `ERRORS.md` E1/E2.
4. **Byte-value shape**: `0x00`, `0x01`, the branch boundary `10`/`11`, `0xfe`,
   `0xff`; one/two/three non-zero channels; grayscale `R==G==B`; pure primaries.
5. **ABI shape**: the 3-byte struct rides in a 64-bit register, so the upper 5
   bytes are unspecified — a real caller can pass garbage there.

Axes 2 and 3 are *consequences* of axis 1 plus the specific byte values, not
independently settable, so the cross-product is pruned to the combinations the
code actually distinguishes. Row **C9** is an exhaustive sweep of the entire
2^24 input domain, which by construction covers every reachable point of the
full cross-product; the targeted rows above and below it exist so that a
divergence is *localised* to a named branch rather than just "some input fails".

## Table

Randomized rows use a fixed seed (SplitMix64, seed `0x5EED_1234_ABCD_0001`) for
reproducibility; N is the number of random inputs drawn for that row.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `tritanopia` | remove-gamma **linear** on R,G,B (all bytes in `0..=10`); exhaustive over all 11^3 = 1331 such triples | [x] |
| C2 | `tritanopia` | remove-gamma **pow** on R,G,B (all bytes in `11..=255`); N=20000 random | [x] |
| C3 | `tritanopia` | remove-gamma linear on **R** only; G,B in `11..=255`; N=20000 random | [x] |
| C4 | `tritanopia` | remove-gamma linear on **G** only; R,B in `11..=255`; N=20000 random | [x] |
| C5 | `tritanopia` | remove-gamma linear on **B** only; R,G in `11..=255`; N=20000 random | [x] |
| C6 | `tritanopia` | remove-gamma linear on **R,G**; B in `11..=255`; N=20000 random | [x] |
| C7 | `tritanopia` | remove-gamma linear on **R,B**; G in `11..=255`; N=20000 random | [x] |
| C8 | `tritanopia` | remove-gamma linear on **G,B**; R in `11..=255`; N=20000 random | [x] |
| C9 | `tritanopia` | **exhaustive**: all 2^24 = 16777216 inputs `(R,G,B) in 0..=255^3`, byte-for-byte compare | [x] |
| C10 | `tritanopia` | apply-gamma **linear (negative)** arm on red: `B > G`, `R` small — post-matrix red `< 0.0031308`; N=20000 random over `R in 0..=20, G < B` | [x] |
| C11 | `tritanopia` | apply-gamma pow arm with post-matrix red **> 1.0** (`G > B`, `R` large) -> `cbDenorm` **overflows past 255 and wraps**; N=20000 random over `R in 235..=255, G > B` | [x] |
| C12 | `tritanopia` | apply-gamma **linear** arm on green **and** blue simultaneously (`G,B in 0..=1`), R free — the near-zero green/blue corner; exhaustive over R x {0,1} x {0,1} | [x] |
| C13 | `tritanopia` | `cbDenorm` **negative wrap** actually observed (result byte != 0 for a channel whose pre-cast value was negative), e.g. pure blue `(0,0,255)`; N=20000 random over `G=0, B in 128..=255` | [x] |
| C14 | `tritanopia` | remove-gamma **threshold crossing**: every channel independently in `{9,10,11,12}` (strict `>` boundary at 10/11); exhaustive over 4^3 = 64 triples | [x] |
| C15 | `tritanopia` | **ABI**: identical low 3 bytes, arbitrary garbage in the upper 5 register bytes (called through a `u64 -> u64` signature); N=20000 random | [x] |
| C16 | `tritanopia` | **grayscale** `R == G == B`; exhaustive over all 256 | [x] |
| C17 | `tritanopia` | **sparse shapes**: exactly one non-zero channel, exactly two non-zero channels, and the 8 corners `{0,255}^3` (pure primaries/secondaries, black, white); exhaustive | [x] |

Binary executable: **none**. `c_src/CMakeLists.txt` builds only
`add_library(... SHARED src/lib.c)` — there is no `add_executable`, and the Rust
crate is `crate-type = ["cdylib"]` with no `src/main.rs`. So the
"compare C and Rust binary stdout" gate is not applicable; the `.so`-level
differential comparison in C9 covers the whole input domain instead.

## Harness soundness (why these check-marks can be trusted)

Two things were verified about the harness itself, because a differential suite
that silently tests the wrong binary produces check-marks that mean nothing:

1. **Staleness guard.** `cargo test` does **not** rebuild a
   `crate-type = ["cdylib"]` artifact — it only builds the test harness. The
   `.so` on disk can therefore lag `src/lib.rs`. This was not hypothetical: an
   injected mutation initially passed all 29 tests because the tests were
   loading a 7-minute-old `.so`. `tests/common/mod.rs::find_rust_so` now asserts
   `mtime(.so) >= mtime(src/lib.rs)` and aborts with `STALE ARTIFACT` otherwise
   (confirmed to fire by `touch src/lib.rs` without rebuilding), and
   `run_all.sh` always runs `cargo build` before `cargo test`.
2. **Mutation check (the suite has teeth).** Replacing the careful x86
   `cvttss2si` emulation in `f32_to_u8_c_cast` with the naive, *saturating*
   `v as u8` — the single most likely translation mistake here — is caught by
   **11 of the 17** Phase B rows, with the exhaustive row reporting the exact
   divergence (`(0,0,12)`: C gives `R=255`, naive Rust gives `R=0`). The
   original code was then restored byte-for-byte and re-verified. This confirms
   the passes are real signal rather than a harness that agrees with anything.
