# Differential Verification Errors

No mismatches were found.

The C and Rust executables produced identical stdout, stderr, and exit status
for:

- empty input;
- single `A`, single `x`, and a single unrelated byte;
- absent, single, and repeated matches for both searched bytes;
- matches separated by newlines;
- non-UTF-8 bytes;
- leading, embedded, and repeated NUL bytes;
- 999-byte and 1000-byte inputs; and
- input extending beyond the 1000-byte read.

The C source has no input validation branch, diagnostic path, or nonzero return
path.
