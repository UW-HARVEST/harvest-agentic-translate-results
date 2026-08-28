# Differential Verification Errors

No mismatches were observed.

The C program does not inspect stdin and has no conditional or error paths. It
always writes `Hello World!\n` to stdout, writes nothing to stderr, and exits
with status 0. Differential coverage includes empty, single-item, multiline,
and 1 MiB inputs.
