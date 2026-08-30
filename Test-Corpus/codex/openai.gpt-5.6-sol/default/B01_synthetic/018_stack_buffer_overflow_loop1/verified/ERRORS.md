# Differential Testing Errors

No mismatches were found between the C and Rust executables.

The comparison covers EOF, whitespace-only input, failed integer conversions,
zero and nonzero branches, signed `int` limits, out-of-range values, scanning
across newlines, trailing input, and a leading NUL byte. Every case compares
stdout, stderr, and exit status byte for byte.
