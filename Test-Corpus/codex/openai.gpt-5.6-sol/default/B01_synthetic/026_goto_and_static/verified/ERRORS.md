# Differential Verification Errors

No mismatches were observed between the C and Rust executables.

The comparison covered empty and partial input, every ordered validation
failure, successful input, malformed conversions at each field, numeric-prefix
conversion, C whitespace classes, extra trailing input, signed integer
boundaries, integer narrowing, and decimal overflow.

In addition to the integration cases, a deterministic sweep of 2,800 arbitrary
byte inputs produced identical stdout, stderr, and exit status.
