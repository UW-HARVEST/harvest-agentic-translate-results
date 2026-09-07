# Configuration Surface

The public API has one entry point and no compile-time or runtime feature
flags. `synth_pair` computes two independent floating-point accumulators and
passes each through `mp3d_scale_pcm`. Mechanical branch analysis yields five
paths for each accumulator:

- `P`: `sample >= 32766.5f` (positive saturation)
- `L`: `sample <= -32767.5f` (negative saturation)
- `I+`: neither saturation comparison; converted sample is nonnegative
- `I-`: neither saturation comparison; converted sample is negative, so the
  `(s < 0)` adjustment executes
- `N`: NaN makes both comparisons false and reaches the float-to-integer cast

The output-address shape differs for the public positive channel counts:
`nch == 1` writes samples 0 and 16, while `nch == 2` writes samples 0 and 32.
The full mechanically distinguished cross-product is below. Generic zero,
negative, and large channel-stride boundaries are tracked in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `synth_pair` | `nch=1`; first=P; second=P | [x] |
| 2 | `synth_pair` | `nch=1`; first=P; second=L | [x] |
| 3 | `synth_pair` | `nch=1`; first=P; second=I+ | [x] |
| 4 | `synth_pair` | `nch=1`; first=P; second=I- | [x] |
| 5 | `synth_pair` | `nch=1`; first=P; second=N | [x] |
| 6 | `synth_pair` | `nch=1`; first=L; second=P | [x] |
| 7 | `synth_pair` | `nch=1`; first=L; second=L | [x] |
| 8 | `synth_pair` | `nch=1`; first=L; second=I+ | [x] |
| 9 | `synth_pair` | `nch=1`; first=L; second=I- | [x] |
| 10 | `synth_pair` | `nch=1`; first=L; second=N | [x] |
| 11 | `synth_pair` | `nch=1`; first=I+; second=P | [x] |
| 12 | `synth_pair` | `nch=1`; first=I+; second=L | [x] |
| 13 | `synth_pair` | `nch=1`; first=I+; second=I+ | [x] |
| 14 | `synth_pair` | `nch=1`; first=I+; second=I- | [x] |
| 15 | `synth_pair` | `nch=1`; first=I+; second=N | [x] |
| 16 | `synth_pair` | `nch=1`; first=I-; second=P | [x] |
| 17 | `synth_pair` | `nch=1`; first=I-; second=L | [x] |
| 18 | `synth_pair` | `nch=1`; first=I-; second=I+ | [x] |
| 19 | `synth_pair` | `nch=1`; first=I-; second=I- | [x] |
| 20 | `synth_pair` | `nch=1`; first=I-; second=N | [x] |
| 21 | `synth_pair` | `nch=1`; first=N; second=P | [x] |
| 22 | `synth_pair` | `nch=1`; first=N; second=L | [x] |
| 23 | `synth_pair` | `nch=1`; first=N; second=I+ | [x] |
| 24 | `synth_pair` | `nch=1`; first=N; second=I- | [x] |
| 25 | `synth_pair` | `nch=1`; first=N; second=N | [x] |
| 26 | `synth_pair` | `nch=2`; first=P; second=P | [x] |
| 27 | `synth_pair` | `nch=2`; first=P; second=L | [x] |
| 28 | `synth_pair` | `nch=2`; first=P; second=I+ | [x] |
| 29 | `synth_pair` | `nch=2`; first=P; second=I- | [x] |
| 30 | `synth_pair` | `nch=2`; first=P; second=N | [x] |
| 31 | `synth_pair` | `nch=2`; first=L; second=P | [x] |
| 32 | `synth_pair` | `nch=2`; first=L; second=L | [x] |
| 33 | `synth_pair` | `nch=2`; first=L; second=I+ | [x] |
| 34 | `synth_pair` | `nch=2`; first=L; second=I- | [x] |
| 35 | `synth_pair` | `nch=2`; first=L; second=N | [x] |
| 36 | `synth_pair` | `nch=2`; first=I+; second=P | [x] |
| 37 | `synth_pair` | `nch=2`; first=I+; second=L | [x] |
| 38 | `synth_pair` | `nch=2`; first=I+; second=I+ | [x] |
| 39 | `synth_pair` | `nch=2`; first=I+; second=I- | [x] |
| 40 | `synth_pair` | `nch=2`; first=I+; second=N | [x] |
| 41 | `synth_pair` | `nch=2`; first=I-; second=P | [x] |
| 42 | `synth_pair` | `nch=2`; first=I-; second=L | [x] |
| 43 | `synth_pair` | `nch=2`; first=I-; second=I+ | [x] |
| 44 | `synth_pair` | `nch=2`; first=I-; second=I- | [x] |
| 45 | `synth_pair` | `nch=2`; first=I-; second=N | [x] |
| 46 | `synth_pair` | `nch=2`; first=N; second=P | [x] |
| 47 | `synth_pair` | `nch=2`; first=N; second=L | [x] |
| 48 | `synth_pair` | `nch=2`; first=N; second=I+ | [x] |
| 49 | `synth_pair` | `nch=2`; first=N; second=I- | [x] |
| 50 | `synth_pair` | `nch=2`; first=N; second=N | [x] |

There are no Cargo features in `Cargo.toml`; the only build configurations are
the equivalent default and `--no-default-features` modes.

Every row passed 256 fixed-seed randomized inputs in both build modes. Separate
tests also covered saturation thresholds, rounding transitions, infinities, and
channel-stride boundaries.
