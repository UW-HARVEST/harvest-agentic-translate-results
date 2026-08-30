# Configuration Surface

Mechanical inspection of the only public header and source found:

- one public entry point, `driver`;
- no runtime options, modes, flags, element types, formats, or counts;
- one fixed input shape, a by-value C `int`;
- a fixed `sizeof(int)`-iteration loop that prints the native-order object
  bytes as two lowercase hexadecimal digits each, followed by one newline.

There are no Cargo features, C preprocessor feature branches, or alternative
public entry points.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; every by-value `c_int` bit pattern, including zero, positive/negative values, and `INT_MIN`/`INT_MAX`; output is the native-order `sizeof(c_int)` object representation plus newline. | [x] |
