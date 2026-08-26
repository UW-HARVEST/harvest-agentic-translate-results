# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` has no
options, conditional sources, compile definitions, or preprocessor feature
selection. There is exactly one valid build configuration:

| # | Cargo features | C configuration | Check command | Status |
|---|----------------|-----------------|---------------|--------|
| 1 | none (`--no-default-features`) | default | `cargo check --no-default-features` | [x] |

## Runtime Axes

The C source has two public entry points and no explicit `if`, `switch`, or
preprocessor branches. Its observable data-shape distinctions come from:

- `driver(double)`: raw IEEE-754 bits plus `%a` and `%.4f` formatting.
- `main(void)`: `%lf` scanning into an initialized `+0.0`, followed by
  `driver`.

The following is the pruned cross-product of entry point, IEEE-754 class, and
scanner subject-sequence form. Scanner failure and EOF are included because C
continues successfully with initialized state rather than rejecting them.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | direct call; positive and negative zero bit patterns | [x] |
| 2 | `driver` | direct call; positive and negative finite subnormals, including boundary payloads | [x] |
| 3 | `driver` | direct call; positive and negative finite normals, including exponent/mantissa boundaries | [x] |
| 4 | `driver` | direct call; positive and negative infinity | [x] |
| 5 | `driver` | direct call; positive/negative quiet and signaling NaNs with varied payloads | [x] |
| 6 | `main` -> `driver` | decimal finite normal/zero subject sequences, with signs and exponents | [x] |
| 7 | `main` -> `driver` | decimal finite subnormal and signed-zero subject sequences | [x] |
| 8 | `main` -> `driver` | hexadecimal finite normal/subnormal subject sequences | [x] |
| 9 | `main` -> `driver` | case-varied infinity subject sequences | [x] |
| 10 | `main` -> `driver` | case-varied NaN subject sequences, with accepted payload forms | [x] |
| 11 | `main` -> `driver` | decimal overflow subject sequences | [x] |
| 12 | `main` -> `driver` | decimal underflow subject sequences | [x] |
| 13 | `main` -> `driver` | accepted number with varied leading whitespace and trailing bytes | [x] |
| 14 | `main` -> `driver` | nonempty conversion-failure subject sequence; initialized `+0.0` is used | [x] |
| 15 | `main` -> `driver` | empty stdin/EOF; initialized `+0.0` is used | [x] |
