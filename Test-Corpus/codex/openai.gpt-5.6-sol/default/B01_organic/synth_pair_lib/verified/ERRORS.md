# Error Surface

Mechanical search scope: `../c_src/include/` and `../c_src/src/`.

Search terms covered error-return macros/statements, `assert`, null checks,
range checks, comparisons, and min/max constants. The only comparisons found
are the valid-result conversion branches in `mp3d_scale_pcm`; the C API has no
error return, assertion, null check, enum, length argument, or explicit input
rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are zero source-derived rejection rows.

## Mandatory generic FFI boundaries

The C contract does not define these inputs and returns no rejection sentinel.
They are tracked separately to compare the observable process-level behavior
required by Phase C.

| boundary | condition | expected C behavior | status |
|----------|-----------|---------------------|--------|
| null `pcm` | `pcm == NULL`, valid readable `z`, `nch == 1` | invalid write; no API error result | [x] |
| null `z` | valid writable `pcm`, `z == NULL`, `nch == 1` | invalid read; no API error result | [x] |

Length and enum boundary probes are not applicable: `synth_pair` has no length
parameter and no enum parameter. `nch == 0` is accepted by C and is covered as
a valid aliasing configuration in `CONFIGS.md`.
