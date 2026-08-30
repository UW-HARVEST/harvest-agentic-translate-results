# Differential Verification Errors

No mismatches were found between the C and Rust executables.

The comparison covered successful scans with zero and nonzero values, signed
integer boundaries, integer overflow and truncation, empty input, whitespace
followed by EOF, malformed tokens, an incomplete sign, input split across
newlines, and trailing input. Every case compared stdout, stderr, and exit
status.

The C `bad()` path dereferences an uninitialized pointer. In the required
default CMake build, inputs that leave `x` equal to zero consistently print
`0\n` and exit successfully. The Rust executable produces the same observable
result without dereferencing an uninitialized pointer.
