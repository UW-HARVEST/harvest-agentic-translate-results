# Configuration-surface table

Mechanical source scan found no runtime options, modes, flags, enums,
preprocessor feature branches, element-type choices, byte-order choices, or
length/count parameters. `Cargo.toml` declares no features and CMake declares
one shared library and no executable.

The public dynamic entry points are `foo` (the lowest-level counter, exported
despite not appearing in `driver.h`) and `driver` (the header-declared
composed operation). The meaningful valid-input shapes come from the
`strchr`-controlled loop in `foo` and the cross-product of the fixed `'A'` and
`'x'` searches in `driver`. Inputs are NUL-terminated byte strings; embedded
NUL ends the input.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `foo` | empty string; non-NUL target byte; zero matches | [x] |
| 2 | `foo` | nonempty string; target absent | [x] |
| 3 | `foo` | nonempty string; exactly one match at start, middle, or end | [x] |
| 4 | `foo` | nonempty string; multiple separated matches | [x] |
| 5 | `foo` | nonempty string; multiple consecutive matches | [x] |
| 6 | `foo` | non-ASCII target byte (`0x80..=0xff`) passed through signed C `char` | [x] |
| 7 | `driver` | empty string or nonempty string containing neither `'A'` nor `'x'` | [x] |
| 8 | `driver` | one or more `'A'`, no `'x'` | [x] |
| 9 | `driver` | no `'A'`, one or more `'x'` | [x] |
| 10 | `driver` | one or more of both `'A'` and `'x'`, including adjacent and separated occurrences | [x] |

Feature/build combinations:

| # | combination | status |
|---|-------------|--------|
| 1 | default / `--no-default-features` (equivalent because no features are declared) | [x] |
