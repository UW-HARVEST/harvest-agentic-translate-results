# Configuration Surface

Mechanical branch inventory:

- Public entry points: `synth_pair` only.
- Runtime options, modes, flags, feature conditionals, and enums: none.
- Input layout: `z` must expose indices through `898`; `pcm` must make offsets
  `0` and `16 * nch` writable. The implementation branches only on each
  computed sample passed to `mp3d_scale_pcm`.
- Address shapes exercised in every row: negative `nch` with a centered
  output pointer, zero `nch` (both writes alias), and positive `nch`.
- Scaling outcomes: low saturation (`a <= -32767.5`), interior with negative
  converted `s` (`-32767.5 < a <= -1.5`), interior with nonnegative converted
  `s` (`-1.5 < a < 32766.5`), and high saturation (`a >= 32766.5`).

The rows are the full cross-product of the scaling outcome for `pcm[0]` and
the scaling outcome for `pcm[16 * nch]`.

| # | entry point(s) | configuration (first output; second output) | [ ] |
|---|----------------|---------------------------------------------|-----|
| 1 | `synth_pair` | low saturation; low saturation | [x] |
| 2 | `synth_pair` | low saturation; interior `s < 0` | [x] |
| 3 | `synth_pair` | low saturation; interior `s >= 0` | [x] |
| 4 | `synth_pair` | low saturation; high saturation | [x] |
| 5 | `synth_pair` | interior `s < 0`; low saturation | [x] |
| 6 | `synth_pair` | interior `s < 0`; interior `s < 0` | [x] |
| 7 | `synth_pair` | interior `s < 0`; interior `s >= 0` | [x] |
| 8 | `synth_pair` | interior `s < 0`; high saturation | [x] |
| 9 | `synth_pair` | interior `s >= 0`; low saturation | [x] |
| 10 | `synth_pair` | interior `s >= 0`; interior `s < 0` | [x] |
| 11 | `synth_pair` | interior `s >= 0`; interior `s >= 0` | [x] |
| 12 | `synth_pair` | interior `s >= 0`; high saturation | [x] |
| 13 | `synth_pair` | high saturation; low saturation | [x] |
| 14 | `synth_pair` | high saturation; interior `s < 0` | [x] |
| 15 | `synth_pair` | high saturation; interior `s >= 0` | [x] |
| 16 | `synth_pair` | high saturation; high saturation | [x] |
