# Differential Mismatches

No mismatches were found. The existing Rust executable matched the C
executable for stdout, stderr, and exit status in every differential case.

## Branch Audit

- The C executable ignores both stdin and command-line arguments.
- The only `if` checks whether `printLine` received a null pointer. This branch
  is not reachable through the executable because every call passes a non-null
  string literal.
- The C executable has no input validation, length limit, early error return,
  or other error path.

The differential suite covers empty stdin, a single item, multiline input,
arbitrary binary input, 1 MiB of input, and command-line arguments.
