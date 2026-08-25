# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert\s*\(|if\s*\(|switch\s*\(|MIN|MAX|NULL' c_src
```

The public `synth_pair` API returns `void` and contains no rejection branch,
error code, sentinel, assertion, null check, enum, or length parameter. Its
private scaling helper has the following two explicit range checks. They
saturate output rather than rejecting the call, but are included because they
are the complete range-check surface in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `mp3d_scale_pcm`, through `synth_pair` | synthesized `sample >= 32766.5f` | return `INT16_MAX` (`32767`) for that output [x] |
| 2 | `mp3d_scale_pcm`, through `synth_pair` | synthesized `sample <= -32767.5f` | return `INT16_MIN` (`-32768`) for that output [x] |

## Generic FFI Boundaries

These are tracked separately because the C API does not reject them:

| # | public function | boundary | expected C behavior |
|---|-----------------|----------|---------------------|
| G1 | `synth_pair` | null `pcm` with valid `z` | no source-level check; process faults on the first output store [x] |
| G2 | `synth_pair` | valid `pcm` with null `z` | no source-level check; process faults on the first input load [x] |
| G3 | `synth_pair` | `nch == 0` | both stores address `pcm[0]`; the second result replaces the first [x] |
| G4 | `synth_pair` | `nch == 3` (one beyond the practical mono/stereo strides) | second result is stored at `pcm[48]` [x] |
| G5 | `synth_pair` | oversized `nch == 268435456` | compiled C integer multiplication wraps to zero; second result replaces `pcm[0]` [x] |
| G6 | `synth_pair` | negative `nch == -1` with addressable preceding storage | second result is stored 16 samples before `pcm` [x] |

Zero/oversized lengths and out-of-range enum values are inapplicable: the API
has neither a length parameter nor an enum parameter. `nch` is a destination
stride multiplier, not a source or destination length.
