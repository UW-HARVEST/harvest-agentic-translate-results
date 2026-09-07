# CONFIGS.md — Phase B configuration surface table

## Axes derived from the C source

`c_src/include/lib.h` declares exactly one public entry point, which is also the
lowest-level entry point (there are no convenience wrappers to skip past):

```c
void gaussian_kernel(float *dest, int size, float radius);
```

There are **no** runtime options, modes, flags, global state, byte-order
handling, element-type choices or `#ifdef`s in the library. The only axes the C
code branches on are therefore the shapes/values of its three arguments:

* **Axis `size` (parity)** — `hsize = size / 2` is C truncating integer
  division, and the write loop is the *inclusive* range `[-hsize, hsize]`, i.e.
  `2*hsize + 1` iterations. So:
  * `size` odd → exactly `size` elements written;
  * `size` even and `> 0` → `size + 1` elements written (one past the end);
  * `size == 0` → exactly `1` element written;
  * `size < 0` → `0` elements written.
* **Axis `size` (magnitude)** — `0`, `1`, `2`, small, large, `INT_MIN`,
  and values around the clamp radius (see below).
* **Axis `radius`** — controls `rs = sigma / radius` and hence `x = r * rs`:
  * `radius` large → `rs → 0`, every `v = 1 - s2 > 0`, no clamping;
  * `radius` moderate → some `|x| < 2.4` (kept) and some `|x| >= 2.4` clamped
    (`v > 0` fails because `1/expf(x*x) < s2 = 1/expf(5.76)`);
  * `radius` tiny / `0` / `NaN` → every element clamped to `+0.0f`;
  * `radius` negative → sign-symmetric via `x*x`;
  * `radius` `±inf` → `rs = ±0.0f`, uniform kernel.
* **Axis clamp branch (line 18)** — `v > 0` taken vs not taken, per element.
* **Axis normalisation branch (line 23)** — `sum > 0.0f` true (second loop runs,
  `size` iterations) vs false (buffer left as written).
* **Interaction axis** — the second loop runs over `[0, size)` while the first
  wrote `[0, 2*hsize]`; for even `size` those ranges differ, so the *composition*
  of the two loops (not either loop alone) determines the output.

## Configuration table

Every row is exercised with many randomized inputs (fixed seed, xorshift PRNG)
inside the stated shape, comparing the *entire* buffer including the
out-of-bounds slot and the untouched guard region, bit-for-bit (`to_bits()`),
between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `gaussian_kernel` | `size` odd small (1,3,5,7,9,11), `radius` random in `[0.5, 8.0]` — exactly `size` writes, mixed clamped/unclamped, normalisation runs | [x] |
| 2 | `gaussian_kernel` | `size` even small (2,4,6,8,10), `radius` random in `[0.5, 8.0]` — `size+1` writes, trailing element left un-normalised | [x] |
| 3 | `gaussian_kernel` | `size == 0`, `radius` random positive — single write, `sum > 0`, normalisation loop runs 0 times → `dest[0]` un-normalised `1 - s2` | [x] |
| 4 | `gaussian_kernel` | `size == 1`, `radius` random positive — exactly one write, normalised to `1.0f` | [x] |
| 5 | `gaussian_kernel` | `size` odd large (65, 127, 255, 511, 1023), `radius` random in `[0.5, 64.0]` | [x] |
| 6 | `gaussian_kernel` | `size` even large (64, 128, 256, 512, 1024), `radius` random in `[0.5, 64.0]` | [x] |
| 7 | `gaussian_kernel` | `radius` very large (`1e6 .. 1e30`, `f32::MAX`) so `rs ≈ 0` — no element clamped, kernel is uniform `1/n` | [x] |
| 8 | `gaussian_kernel` | `radius == +inf` → `rs = +0.0f`, uniform kernel | [x] |
| 9 | `gaussian_kernel` | `radius == -inf` → `rs = -0.0f`, uniform kernel | [x] |
| 10 | `gaussian_kernel` | `radius` negative finite (mirrors of row 1/2 magnitudes) — must equal the `+radius` result exactly | [x] |
| 11 | `gaussian_kernel` | `radius` small (`0.01 .. 0.5`) so **most** elements clamp to `+0.0f` but the centre survives → `sum > 0`, sparse kernel | [x] |
| 12 | `gaussian_kernel` | `radius` at/below the clamp threshold such that even `r = ±1` clamps: `1.6/radius >= 2.4` ⇒ `radius <= 0.6667` — boundary sweep around `0.6666`, `0.66667`, `0.6667` | [x] |
| 13 | `gaussian_kernel` | `radius` denormal / tiny (`f32::MIN_POSITIVE`, `1e-40`, `1e-30`, `1e-20`) → `rs` overflows to `inf`, centre becomes `NaN` → clamped, `sum == 0`, no normalisation | [x] |
| 14 | `gaussian_kernel` | `radius == 0.0f` and `radius == -0.0f` — `NaN` centre, whole buffer `+0.0f`, no normalisation | [x] |
| 15 | `gaussian_kernel` | `radius = NaN` (both quiet-NaN bit patterns), any `size` | [x] |
| 16 | `gaussian_kernel` | pre-poisoned `dest` buffer (filled with `NaN`/`inf`/random garbage before the call) — verifies the C leaves the same slots untouched (esp. `size <= 0` and even-`size` tail) | [x] |
| 17 | `gaussian_kernel` | fully randomized fuzz: `size` random in `[-8, 64]`, `radius` from a random `u32` bit pattern (any float incl. NaN/inf/denormal), 20000 iterations | [x] |
| 18 | `gaussian_kernel` | repeated calls on the same buffer (idempotence / no hidden global state between invocations) | [x] |
| 19 | `gaussian_kernel` | non-zero-offset `dest` (unaligned-in-buffer start, pointer arithmetic path) — call with `base.add(3)` | [x] |
| 20 | `gaussian_kernel` | `size` values around the `size/2` truncation boundary for negatives: `-1` (`hsize = 0` ⇒ loop *does* run once!), `-2`, `-3`, `-4` | [x] |

### Note on row 20

`size == -1` gives `hsize = -1/2 == 0` in C (truncation toward zero), so
`-hsize == 0 <= hsize == 0` and the loop **does** execute once, writing
`dest[0]`. `sum > 0` then holds but the normalisation loop `r < -1` runs 0
times. This is a genuinely surprising branch and is verified explicitly.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`; there is
no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. **No driver binary exists**, so the
stdout-comparison item of the completion gate is not applicable.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default one. Verified by:

```
$ grep -n '\[features\]' translation/Cargo.toml   # no match
```
