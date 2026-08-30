# Differential Verification Errors

No C/Rust mismatches were found.

The existing Rust implementation delegates `%d` parsing to the same C `scanf`
interface and prints the native-endian bytes of the resulting `int`, matching
the C implementation for successful conversions, conversion failures, EOF,
integer boundaries, and out-of-range inputs.
