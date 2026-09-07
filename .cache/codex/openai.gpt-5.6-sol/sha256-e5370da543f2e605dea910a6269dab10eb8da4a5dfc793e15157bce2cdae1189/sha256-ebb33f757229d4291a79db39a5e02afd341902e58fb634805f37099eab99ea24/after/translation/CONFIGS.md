# Configuration Surface

Mechanical enumeration of the public header and all C control-flow constructs
found:

- one public entry point: `next_double`;
- one API-visible input shape: a non-null pointer to `cn_rnd_t`, containing
  exactly two `uint64_t` state words;
- no runtime options, modes, flags, enums, formats, element types, lengths,
  switches, conditionals, or compile-time feature branches;
- one internal static helper, `cn_rnd_next`, which is not a public entry point;
- stateful behavior across successive calls.

Because the C code contains no configuration-dependent branches, its pruned
configuration cross-product has one row. The row covers arbitrary state words,
boundary state values, and both single and repeated calls.

| # | entry point(s) | configuration (options set + input shape) | Status |
|---|----------------|--------------------------------------------|--------|
| 1 | `next_double` | no options; valid `cn_rnd_t*`; two arbitrary `uint64_t` words; one and many successive calls | [x] |

## Cargo feature combinations

`Cargo.toml` declares no features. The required build invocations are:

- [x] default feature selection (empty);
- [x] `--no-default-features` (also empty, checked separately).
