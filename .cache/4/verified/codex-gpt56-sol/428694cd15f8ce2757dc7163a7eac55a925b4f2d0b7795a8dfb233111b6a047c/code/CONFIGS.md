# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` has one
unconditional shared-library target with no options or preprocessor
definitions. There is exactly one valid feature combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (no named features) | default | [x] |

## Runtime Configurations

The public header declares only `void driver(char c)`. The function has no
runtime options and unconditionally selects the C locale, evaluates twelve
ctype classifications, performs lower/upper conversion, and prints the
results. These rows partition all 256 `char` bit patterns into the combinations
that those operations treat differently.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options; EOF representation (`c == -1`, byte `0xff`) | [x] |
| 2 | `driver` | no options; other negative signed chars (`-128..=-2`, bytes `0x80..=0xfe`) | [x] |
| 3 | `driver` | no options; NUL (`0x00`) | [x] |
| 4 | `driver` | no options; non-whitespace ASCII control (`0x01..=0x08`, `0x0e..=0x1f`, `0x7f`) | [x] |
| 5 | `driver` | no options; horizontal tab (`0x09`, control + space + blank) | [x] |
| 6 | `driver` | no options; other ASCII whitespace controls (`0x0a..=0x0d`, control + space) | [x] |
| 7 | `driver` | no options; space (`0x20`, space + blank + printable) | [x] |
| 8 | `driver` | no options; ASCII punctuation (`0x21..=0x2f`, `0x3a..=0x40`, `0x5b..=0x60`, `0x7b..=0x7e`) | [x] |
| 9 | `driver` | no options; decimal digit (`0` through `9`, also hexadecimal) | [x] |
| 10 | `driver` | no options; uppercase hexadecimal letter (`A` through `F`) | [x] |
| 11 | `driver` | no options; uppercase non-hexadecimal letter (`G` through `Z`) | [x] |
| 12 | `driver` | no options; lowercase hexadecimal letter (`a` through `f`) | [x] |
| 13 | `driver` | no options; lowercase non-hexadecimal letter (`g` through `z`) | [x] |
