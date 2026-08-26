# Configuration Surface

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` declares no
options. There is one valid build-time combination:
`--no-default-features` with an empty feature set.

The C header exposes one entry point, `my_pow(double, double)`. It has no
runtime options, modes, flags, state, pointers, lengths, enums, or element
formats. The valid-input rows below partition the `double` input shapes handled
by the delegated C `pow` operation while taking the C function's non-error
return branch (`errno != EDOM && errno != ERANGE`).

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `my_pow` | No options; positive finite base and finite exponent, with an ordinary finite result | [x] |
| 2 | `my_pow` | No options; identity boundaries (`exponent` is signed zero, or `base` is `1.0`) | [x] |
| 3 | `my_pow` | No options; signed-zero base and positive finite exponent, without a range error | [x] |
| 4 | `my_pow` | No options; negative finite base and integral finite exponent, with an ordinary finite result | [x] |
| 5 | `my_pow` | No options; finite inputs producing an exact or representable boundary result (including signed results) | [x] |
| 6 | `my_pow` | No options; IEEE-754 `NaN` or infinity combinations for which C `pow` reports no domain/range error | [x] |
