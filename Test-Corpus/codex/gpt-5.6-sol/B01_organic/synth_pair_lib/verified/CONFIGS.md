# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options or conditional compilation. There is one valid feature combination:

| # | Cargo feature combination | CMake configuration | Status |
|---|---------------------------|---------------------|-----|
| B1 | empty set (`--no-default-features`) | default | [x] |

## Runtime Axes

The public surface consists only of `synth_pair`. The source always reads a
fixed 899-float span (`z[0]` through `z[898]`) and emits two samples at
`pcm[0]` and `pcm[16 * nch]`. The practical public channel strides are one and
two. Each output independently reaches four source-distinct scaling paths:

- `high`: synthesized value `>= 32766.5f`
- `low`: synthesized value `<= -32767.5f`
- `negative`: interior value where the `(s < 0)` adjustment executes
- `nonnegative`: interior value where the `(s < 0)` adjustment does not execute

The table is the source-derived cross-product of those axes.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|-----|
| 1 | `synth_pair` | `nch=1`; first=high; second=high; `z[899]`, `pcm[17+]` | [x] |
| 2 | `synth_pair` | `nch=1`; first=high; second=low; `z[899]`, `pcm[17+]` | [x] |
| 3 | `synth_pair` | `nch=1`; first=high; second=negative; `z[899]`, `pcm[17+]` | [x] |
| 4 | `synth_pair` | `nch=1`; first=high; second=nonnegative; `z[899]`, `pcm[17+]` | [x] |
| 5 | `synth_pair` | `nch=1`; first=low; second=high; `z[899]`, `pcm[17+]` | [x] |
| 6 | `synth_pair` | `nch=1`; first=low; second=low; `z[899]`, `pcm[17+]` | [x] |
| 7 | `synth_pair` | `nch=1`; first=low; second=negative; `z[899]`, `pcm[17+]` | [x] |
| 8 | `synth_pair` | `nch=1`; first=low; second=nonnegative; `z[899]`, `pcm[17+]` | [x] |
| 9 | `synth_pair` | `nch=1`; first=negative; second=high; `z[899]`, `pcm[17+]` | [x] |
| 10 | `synth_pair` | `nch=1`; first=negative; second=low; `z[899]`, `pcm[17+]` | [x] |
| 11 | `synth_pair` | `nch=1`; first=negative; second=negative; `z[899]`, `pcm[17+]` | [x] |
| 12 | `synth_pair` | `nch=1`; first=negative; second=nonnegative; `z[899]`, `pcm[17+]` | [x] |
| 13 | `synth_pair` | `nch=1`; first=nonnegative; second=high; `z[899]`, `pcm[17+]` | [x] |
| 14 | `synth_pair` | `nch=1`; first=nonnegative; second=low; `z[899]`, `pcm[17+]` | [x] |
| 15 | `synth_pair` | `nch=1`; first=nonnegative; second=negative; `z[899]`, `pcm[17+]` | [x] |
| 16 | `synth_pair` | `nch=1`; first=nonnegative; second=nonnegative; `z[899]`, `pcm[17+]` | [x] |
| 17 | `synth_pair` | `nch=2`; first=high; second=high; `z[899]`, `pcm[33+]` | [x] |
| 18 | `synth_pair` | `nch=2`; first=high; second=low; `z[899]`, `pcm[33+]` | [x] |
| 19 | `synth_pair` | `nch=2`; first=high; second=negative; `z[899]`, `pcm[33+]` | [x] |
| 20 | `synth_pair` | `nch=2`; first=high; second=nonnegative; `z[899]`, `pcm[33+]` | [x] |
| 21 | `synth_pair` | `nch=2`; first=low; second=high; `z[899]`, `pcm[33+]` | [x] |
| 22 | `synth_pair` | `nch=2`; first=low; second=low; `z[899]`, `pcm[33+]` | [x] |
| 23 | `synth_pair` | `nch=2`; first=low; second=negative; `z[899]`, `pcm[33+]` | [x] |
| 24 | `synth_pair` | `nch=2`; first=low; second=nonnegative; `z[899]`, `pcm[33+]` | [x] |
| 25 | `synth_pair` | `nch=2`; first=negative; second=high; `z[899]`, `pcm[33+]` | [x] |
| 26 | `synth_pair` | `nch=2`; first=negative; second=low; `z[899]`, `pcm[33+]` | [x] |
| 27 | `synth_pair` | `nch=2`; first=negative; second=negative; `z[899]`, `pcm[33+]` | [x] |
| 28 | `synth_pair` | `nch=2`; first=negative; second=nonnegative; `z[899]`, `pcm[33+]` | [x] |
| 29 | `synth_pair` | `nch=2`; first=nonnegative; second=high; `z[899]`, `pcm[33+]` | [x] |
| 30 | `synth_pair` | `nch=2`; first=nonnegative; second=low; `z[899]`, `pcm[33+]` | [x] |
| 31 | `synth_pair` | `nch=2`; first=nonnegative; second=negative; `z[899]`, `pcm[33+]` | [x] |
| 32 | `synth_pair` | `nch=2`; first=nonnegative; second=nonnegative; `z[899]`, `pcm[33+]` | [x] |
