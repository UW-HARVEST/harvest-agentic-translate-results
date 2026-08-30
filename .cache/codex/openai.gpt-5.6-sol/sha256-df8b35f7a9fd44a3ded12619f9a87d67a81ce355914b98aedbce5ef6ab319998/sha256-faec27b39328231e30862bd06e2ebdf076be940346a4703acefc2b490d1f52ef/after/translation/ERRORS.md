# Differential Verification Errors

No mismatches were found.

The subprocess comparisons cover stdout, stderr, and exit status for EOF,
stdin read failure, a blank line, zero, one item, the largest copy count
(`99`), the `100` boundary, a value above the boundary, newline and 13-byte
`fgets` termination, nonnumeric and signed input, an embedded NUL, 32-bit
integer truncation, and negative copy counts.

`printLine(NULL)` is not reachable from program input: both call sites pass a
non-null string literal or array.
