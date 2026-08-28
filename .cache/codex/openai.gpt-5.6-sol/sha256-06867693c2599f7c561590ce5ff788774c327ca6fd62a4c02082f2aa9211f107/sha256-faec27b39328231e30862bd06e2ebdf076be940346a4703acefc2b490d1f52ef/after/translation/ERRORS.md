# Differential Verification

## Input classes

The C source has one execution path. It does not read stdin, branch on input,
check a length or pointer, or return through an error path. The differential
suite therefore verifies empty input, a single item, multiline and binary
input, and a 64 KiB input. There is no maximum accepted input or input-driven
error path in this implementation.

## Mismatches

No mismatches were found.
