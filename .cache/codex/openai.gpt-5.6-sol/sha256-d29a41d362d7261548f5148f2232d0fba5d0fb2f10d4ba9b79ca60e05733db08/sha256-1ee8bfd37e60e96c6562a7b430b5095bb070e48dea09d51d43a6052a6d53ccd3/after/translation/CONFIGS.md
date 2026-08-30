# Configuration Surface

Mechanical inspection found no runtime options, modes, flags, enums, lengths,
element types, formats, byte-order settings, compile-time feature branches, or
Cargo features. `printLine` is the only entry point accepting input and has one
valid branch: a non-null pointer to a NUL-terminated C string.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printLine` | no options; non-null NUL-terminated byte string, randomized across empty, one-byte, and many-byte payloads | [x] |
| 2 | `bad` | no options or inputs; fixed one-line operation | [x] |
| 3 | `good` | no options or inputs; fixed operation including the internal helper call | [x] |
| 4 | `driver` | no options or inputs; full composed operation through `good` and `bad` | [x] |
