# Differential Verification Errors

No mismatches were found.

## Audited Input Classes

- Empty input: `fscanf` reaches EOF and the initialized space byte is used.
- Every possible single byte (`0x00` through `0xff`), including signed `char`
  conversion boundaries.
- Whitespace bytes, including space and newline; `%c` does not skip them.
- Additional bytes on the same line and across lines; only the first byte is
  consumed.

The C implementation has no explicit conditional branches, null checks, or
error exit paths. It ignores the return value from `fscanf` and always exits
with status zero after printing one line.
