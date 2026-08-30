# Translation Mismatches

No mismatches were found.

The C and Rust executables produced identical stdout, stderr, and exit status
for all enumerated input classes:

- EOF before either conversion, with and without leading whitespace
- one successful conversion followed by EOF
- two successful conversions separated by spaces or newlines
- matching failures at the first and second conversions
- a sign without decimal digits
- leading and trailing input
- zero, nonzero, signed, and all-one bit patterns
- `int` minimum and maximum values
- positive, negative, and very large out-of-range decimal input

The Rust implementation already delegates `%d` conversion to the same C
`scanf` function as the reference executable, including its failed-conversion
and out-of-range behavior, so no source correction was required.
