# CONFIGS.md — Phase B configuration surface table

Mechanically derived from `c_src/include/lib.h` + `c_src/src/lib.c`.

## Axes the C code actually branches on

The public header exposes exactly one entry point and no options:

```c
void flip_horizontal(cp_image_t *img);   /* the ONLY public symbol */
```

* **Runtime options / modes / flags:** none. There is no context struct, no
  setter, no flag word, no format/byte-order/element-type selector, no
  `#ifdef`, no `switch`. The complete input is the three struct fields
  `w`, `h`, `pix`.
* **Compile-time features:** the Rust crate declares **no** `[features]` at all
  (`translation/Cargo.toml`), so there is exactly one feature combination
  (the empty/default one). No `#ifdef`/`#if` in the C either.
* **Binary / driver:** the project builds **no executable** — `CMakeLists.txt`
  has a single `add_library(... SHARED src/lib.c)` and `[lib] crate-type =
  ["cdylib"]`. There is no stdout to compare.

Everything the code distinguishes is therefore the *shape of the input*, along
these axes read straight out of the source:

| axis | values the code distinguishes | where |
|------|-------------------------------|-------|
| A1 `h` parity | even (middle row absent) vs odd (middle row `h/2` must be left untouched) | `int flips = h / 2;` |
| A2 `h` magnitude | `0`, `1` (→ `flips == 0`, no-op), `2`, `3`, small, large | `flips = h/2`, `i < flips` |
| A3 `h` sign | `> 0` vs `<= 0` (`flips <= 0` → no-op), `INT_MIN` | `i < flips` |
| A4 `w` magnitude | `0` (inner loop no-op), `1` (single column), `> 1` (many) | `j < w` |
| A5 `w` sign | `> 0` vs `<= 0` (inner loop no-op) | `j < w` |
| A6 row stride vs width | the code assumes stride == `w`; buffer must be exactly `w*h` pixels — so `w*h == 0`, `== 1`, and `>> 1` are distinct shapes | `pix + w*i` |
| A7 pixel content | all four channels `r,g,b,a` are copied by struct assignment `*a = *b` — per-channel content must round-trip, incl. `0x00`/`0xFF` extremes | `cp_pixel_t t = *a; *a = *b; *b = t;` |
| A8 aliasing of the swapped rows | `i` vs `h-i-1`: distinct rows for every `i < h/2`; they can never alias, but `h==2` is the boundary where the two rows are adjacent | pointer setup |
| A9 idempotence / involution | applying the function twice restores the original (a property the C satisfies for every valid shape) | whole loop |
| A10 buffer alignment | `cp_pixel_t` is align-1, so `pix` may be at any byte offset | `pix + w*i` |

Rows below are the cross-product of these axes pruned to the combinations the C
actually treats differently. Every row is driven through **both** `.so`
exports with **many randomized pixel payloads** (fixed seed `0x5EED_1234`,
xorshift64* PRNG) and the two output buffers compared byte-for-byte, plus the
whole 16-byte `cp_image_t` struct compared (the C must not mutate `w`/`h`/`pix`).

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|-------------------------------------------|-----|
| C1  | `flip_horizontal` | `w=0, h=0` — empty image, both dims zero, `pix` = valid 0-length alloc | [x] |
| C2  | `flip_horizontal` | `w=1, h=1` — single pixel, odd `h`, `flips=0` | [x] |
| C3  | `flip_horizontal` | `w=1, h=2` — single column, even `h`, minimal swap, adjacent rows (A8 boundary) | [x] |
| C4  | `flip_horizontal` | `w=1, h=3` — single column, odd `h`, middle row must stay put | [x] |
| C5  | `flip_horizontal` | `w=2, h=1` — one row only, `flips=0`, no-op with valid buffer | [x] |
| C6  | `flip_horizontal` | `w=3, h=2` — even `h`, multi-column | [x] |
| C7  | `flip_horizontal` | `w=4, h=5` — odd `h`, multi-column, middle row `2` untouched | [x] |
| C8  | `flip_horizontal` | `w=1, h=64` — tall & thin, even | [x] |
| C9  | `flip_horizontal` | `w=1, h=65` — tall & thin, odd | [x] |
| C10 | `flip_horizontal` | `w=64, h=1` — wide & flat (`flips=0`) | [x] |
| C11 | `flip_horizontal` | `w=97, h=2` — wide, even `h`, non-power-of-two width | [x] |
| C12 | `flip_horizontal` | `w=97, h=97` — large square, odd `h`, non-power-of-two | [x] |
| C13 | `flip_horizontal` | `w=128, h=128` — large square, even `h`, power-of-two | [x] |
| C14 | `flip_horizontal` | `w=0, h=7` — zero width with `flips=3 > 0`: outer loop runs, inner never (A4/A6 interaction) | [x] |
| C15 | `flip_horizontal` | `w=5, h=0` — zero height with non-zero width | [x] |
| C16 | `flip_horizontal` | randomized `w in 1..=40`, `h in 1..=40` (200 random shapes × random payload) — full shape sweep incl. both parities | [x] |
| C17 | `flip_horizontal` | pixel payload extremes: every pixel `{0,0,0,0}`, every pixel `{255,255,255,255}`, per-channel one-hot patterns, at `w=7,h=6` (A7) | [x] |
| C18 | `flip_horizontal` | payload is a channel-index ramp `r=x, g=y, b=x^y, a=x+y` at `w=17,h=9` — detects channel swaps and row/column transposition (A7) | [x] |
| C19 | `flip_horizontal` | double application (`flip` twice) at randomized shapes — involution property must hold identically in both libs (A9) | [x] |
| C20 | `flip_horizontal` | unaligned `pix` (base pointer offset by 1, 2, 3 bytes inside a `u8` allocation) at `w=6,h=4` (A10) | [x] |
| C21 | `flip_horizontal` | struct-preservation: after the call, `w`, `h`, `pix` are unchanged, checked as raw 16 bytes, over the randomized sweep | [x] |
| C22 | `flip_horizontal` | called repeatedly (10×) on the same buffer — parity of the number of applications must match | [x] |
| C23 | `flip_horizontal` | `w=2, h=3` with padding canaries around the pixel buffer — asserts neither lib writes outside `[pix, pix + w*h)` | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table → the only combination
is the default (empty) one. `cargo test --no-default-features` is equivalent to
`cargo test` here and both are run by `run_all.sh`.

## Row → test mapping (`tests/phase_b_valid.rs`)

| rows | test |
|------|------|
| C1–C15 | `c01_w0_h0_empty` … `c15_w5_h0_zero_height` (one test per row, `REPS = 32` random payloads each) |
| C16 | `c16_random_shape_sweep` (200 random `w,h` in `1..=40`) |
| C17 | `c17_payload_extremes` (8 payload patterns) |
| C18 | `c18_coordinate_ramp` |
| C19 | `c19_double_application_is_identity` (60 random shapes) |
| C20 | `c20_unaligned_pix` (byte shifts 0..8) |
| C21 | `c21_struct_preserved` (80 random shapes, raw 16-byte struct compare) |
| C22 | `c22_repeated_application_parity` (20 shapes × 10 applications) |
| C23 | `c23_no_out_of_bounds_writes` (5 shapes × 3 canary values) |
| layout | `layout_c_struct_layout_assumptions` |

Every one of these compares the *entire* allocation (payload + 4 KiB canary
padding either side) byte-for-byte between the two `.so` files, and additionally
against an independent reference model of the C loop, so the two libraries
cannot pass by agreeing on the same wrong answer.

Status: **24/24 tests pass**, covering all 23 rows.
