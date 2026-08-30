# Differential Verification Errors

## Mismatches Found

None. The Rust translation matched the C executable for stdout, stderr, and
exit status on every tested input.

## Input Coverage

- Empty input, which makes `getchar()` return EOF.
- Every possible single input byte from `0x00` through `0xFF`.
- Multiple-byte inputs beginning with a letter, newline, NUL, DEL, and
  `0xFF`, confirming that bytes after the first are ignored.

The C source contains no conditional statements, early returns, explicit error
paths, or length checks. Exhausting all first-byte values covers every C-locale
character classification and case-conversion result used by the program.
