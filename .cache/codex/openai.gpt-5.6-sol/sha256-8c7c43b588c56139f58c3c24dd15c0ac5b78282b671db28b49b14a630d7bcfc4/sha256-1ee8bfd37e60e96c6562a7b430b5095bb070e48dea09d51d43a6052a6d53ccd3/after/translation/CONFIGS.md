# Configuration Surface

The public header declares one entry point. The C source contains no runtime
options, flags, modes, conditional branches, switches, or feature-dependent
paths. Its input is one by-value `double`; the single row therefore covers the
complete binary64 input space, including signed zero, normal and subnormal
finite values, infinities, and NaNs with varying signs and payloads.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | No options; every binary64 bit pattern, observed through raw-bit (`%llx`), hexadecimal-float (`%a`), and four-decimal (`%.4f`) output | [x] |
