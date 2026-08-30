# Configuration Surface

The only runtime mode is C truthiness of `driver(useGood)`. There are no
compile-time feature flags, element types, lengths, formats, or byte-order
options. All four dynamic exports are included, including the three
implementation-level entry points omitted from the public header.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printIntPtrLine` | valid pointer to any representable C `int`; output varies with the pointed-to value | [x] |
| C2 | `good` | no input; local `int` is initialized to `5` and printed through `printIntPtrLine` | [x] |
| C3 | `bad` | no input; local pointer is uninitialized and then dereferenced; isolate and compare exact observed behavior because the C language leaves it undefined | [x] |
| C4 | `driver` | `useGood == 0`; dispatches to `bad`, requiring an isolated exact-behavior comparison | [x] |
| C5 | `driver` | `useGood != 0`, including positive and negative C `int` values; dispatches to `good` | [x] |
