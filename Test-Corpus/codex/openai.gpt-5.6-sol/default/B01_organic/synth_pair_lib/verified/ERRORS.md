# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`, error
enums, assertions, null checks, range checks, and min/max constants in
`../c_src/include` and `../c_src/src`.

The C implementation contains **no input validation and no explicit rejection
path**. The comparisons against `32766.5f` and `-32767.5f` are output
saturation paths, not errors, and are covered in `CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

## Generic ABI boundary cases

These are required boundary probes even though they are not C rejection rows.
Pointer faults are isolated in subprocesses because dereferencing null is not a
recoverable library error.

| # | function | boundary input | expected C behavior | verified |
|---|----------|----------------|---------------------|----------|
| G1 | `synth_pair` | `pcm == NULL`, valid `z` | process receives `SIGSEGV` on output write | [x] |
| G2 | `synth_pair` | valid `pcm`, `z == NULL` | process receives `SIGSEGV` on input read | [x] |
| G3 | `synth_pair` | `nch == 0` | accepted; second output aliases and overwrites `pcm[0]` | [x] |
| G4 | `synth_pair` | `nch == -1` with centered writable `pcm` | accepted; second output is written 16 samples before `pcm` | [x] |
| G5 | `synth_pair` | large `nch == 4096` with sufficient writable storage | accepted; second output is written at sample `65536` | [x] |

There are no length arguments, public enums, documented numeric input ranges,
or error return values in this API, so zero/oversized lengths and out-of-range
enum probes are not applicable.

All generic boundary probes passed against both shared libraries in default and
`--no-default-features` modes.
