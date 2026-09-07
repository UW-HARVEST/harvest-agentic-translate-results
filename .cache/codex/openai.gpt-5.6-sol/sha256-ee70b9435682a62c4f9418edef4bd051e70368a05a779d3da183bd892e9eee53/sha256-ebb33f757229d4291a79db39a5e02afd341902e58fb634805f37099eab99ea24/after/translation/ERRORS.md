# Error Surface

Mechanical scans of `../c_src/include/` and `../c_src/src/` found no rejection
branches, error-return macros, negative or null sentinel returns, assertions,
explicit range checks, null checks, error enums, or min/max input constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|----------------------------------------------|-------------------|----------|

`rev16` accepts one `uint32_t` by value, so every possible argument bit pattern
is valid. Generic pointer, length, and enum error boundaries are not applicable
to this API.

- [x] Phase C complete: the empty rejection surface was confirmed, and scalar
  boundaries (`u32::MIN`, the 16-bit cutoff and adjacent values, and
  `u32::MAX`) match through both dynamically loaded libraries.
