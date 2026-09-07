# Configuration Surface

There are no Cargo features, C preprocessor feature switches, runtime options,
mode enums, byte-order choices, or element-format flags. The mechanically
visible axes are:

- entry point: low-level exported `run` or public-header `driver`;
- `run` state shapes: signed integer arithmetic, `double` addition, and
  `printf("%.1f")` formatting;
- `driver` input shapes accepted by `strtol(..., 10)`: canonical decimal,
  leading space/sign, accepted trailing suffix, and exact `int` boundaries.

Rows are pruned to combinations that lead to distinct arithmetic, parsing, or
formatting behavior in the C implementation.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| C1 | `run` | ordinary finite house fields; negative `extra_bedrooms`; no signed overflow | [x] |
| C2 | `run` | ordinary finite house fields; zero `extra_bedrooms` | [x] |
| C3 | `run` | ordinary finite house fields; positive `extra_bedrooms`; no signed overflow | [x] |
| C4 | `run` | floor at/near signed-int boundaries, including the compiled C behavior of `INT_MAX + 1` | [x] |
| C5 | `run` | bedroom and `extra_bedrooms` values at/near signed-int boundaries, including compiled signed wraparound cases | [x] |
| C6 | `run` | finite fractional bathroom values, including rounding ties, signed zero, and values whose `+ 1.0` changes representability | [x] |
| C7 | `run` | non-finite bathroom values: positive/negative infinity and NaNs with varied payloads | [x] |
| C8 | `driver` | canonical in-range decimal strings spanning negative, zero, and positive values | [x] |
| C9 | `driver` | accepted decimal strings with leading whitespace and/or explicit sign | [x] |
| C10 | `driver` | accepted in-range decimal prefix followed by a trailing suffix (the C code does not require `*endp == '\0'`) | [x] |
| C11 | `driver` | exact `INT_MIN` and `INT_MAX` decimal boundaries | [x] |
