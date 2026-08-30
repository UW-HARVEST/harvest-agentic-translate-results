# Configuration Surface

Mechanical scans of the public header, exported definitions, and all C branch
conditions find four public dynamic entry points and two runtime branch axes:

- `printLine`: `line == NULL` versus `line != NULL`.
- `bad`: the uninitialized local contains null versus a valid non-null C string.
- `driver`: `useGood == 0` versus `useGood != 0`; its nested `bad` call has the
  same null/non-null uninitialized-local states.

The null `printLine` case is rejection row 1 in `ERRORS.md`. The non-null row
uses randomized C strings spanning empty, one-byte, and multi-byte shapes.
There are no Cargo features or C feature macros, so the sole build
configuration is the default/no-feature crate.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `printLine` | Non-null valid C string; randomized empty, one-byte, and multi-byte contents | [x] |
| 2 | `bad` | Direct call; uninitialized local contains null | [x] |
| 3 | `bad` | Direct call; uninitialized local contains a randomized valid non-null C string | [x] |
| 4 | `good` | Direct call; fixed non-null `"string"` input | [x] |
| 5 | `driver`, `bad` | `useGood == 0`; nested uninitialized local contains null | [x] |
| 6 | `driver`, `bad` | `useGood == 0`; nested uninitialized local contains a randomized valid non-null C string | [x] |
| 7 | `driver`, `good` | Randomized positive and negative `useGood != 0`; selects `good` | [x] |
