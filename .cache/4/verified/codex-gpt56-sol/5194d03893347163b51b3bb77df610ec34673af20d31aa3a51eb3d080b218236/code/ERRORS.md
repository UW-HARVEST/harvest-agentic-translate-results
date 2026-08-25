# Error Surface

The complete C source was searched for error returns, error macros, assertions,
null checks, explicit range checks, enums, and min/max constants. It has no
input-rejection path. Both public entry points return normally for every value
their signatures accept. Failed `fscanf` input leaves the initialized space
character in place and is therefore covered as a valid configuration in
`CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

Generic FFI boundary audit:

| Boundary | Applicability |
|----------|---------------|
| null pointers | no public pointer parameters |
| zero or oversized lengths | no public length parameters |
| one past a documented range | `char` accepts the full target range |
| out-of-range enums | no public enum parameters |

Completion status: **0 rejection rows; generic boundary audit complete.**
