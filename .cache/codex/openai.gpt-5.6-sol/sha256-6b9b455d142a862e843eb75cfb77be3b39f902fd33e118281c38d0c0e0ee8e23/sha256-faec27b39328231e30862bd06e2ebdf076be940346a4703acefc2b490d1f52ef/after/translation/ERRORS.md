# Differential Verification Errors

No mismatches were found. The initial Rust executable matched the C executable
for every tested input class, so no implementation fix was required.

## Audited Input Classes

- Empty input and whitespace-only input (`scanf` returns EOF, `len == 0`).
- One successfully converted item (`len > 0` and both array loops execute).
- Multiple items separated by spaces, tabs, CRLF, and newlines.
- Conversion failure before the first item and after successful items.
- A numeric prefix followed by an invalid suffix.
- Signed `int` limits and values outside the signed `int` range.
- Exactly 100 items (the maximum accepted length).
- More than 100 items (input after the 100th conversion is ignored).

Each case compares stdout bytes, stderr bytes, and exit status.
