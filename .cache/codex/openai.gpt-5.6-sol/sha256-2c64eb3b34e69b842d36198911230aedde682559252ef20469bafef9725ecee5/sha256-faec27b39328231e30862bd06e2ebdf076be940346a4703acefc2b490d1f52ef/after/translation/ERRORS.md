# Differential Errors

No C/Rust output, error, or exit-status mismatches were found.

The initial empty-stdin comparison produced identical 1,499-byte stdout and
72-byte stderr streams, with exit status 0 from both executables. The stderr
stream contains the two expected built-in error-path messages for a duplicate
node ID and a parent exceeding `MAX_CHILDREN`.

The executable does not read stdin or command-line arguments. Differential
tests therefore cover empty, single-item, multiline, 256-byte, and malformed
stdin payloads to verify that both programs ignore them identically while
running the same built-in scenarios.

## Test Harness Correction

The first piped-stdin harness could race these short-lived programs and receive
`EPIPE` after a child exited. This was not a C/Rust output mismatch. The harness
now redirects stdin from a fully written temporary file and removes that file
after each run.
