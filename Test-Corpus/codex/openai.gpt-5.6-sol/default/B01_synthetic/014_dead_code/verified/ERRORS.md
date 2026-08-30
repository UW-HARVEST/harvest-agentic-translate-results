# Differential Verification

## Input Classes

The C program does not read stdin or inspect command-line arguments. It has no
finite input-size limit and no input-dependent error path. The differential
suite therefore covers empty stdin, one item, multiline text, arbitrary binary
bytes, 64 KiB of input as a representative large input, and multiple command-line
arguments.

## Branch Audit

`printLine` checks whether its pointer argument is null. Every call in this
executable passes a non-null string literal, so the false branch cannot be
reached through process input. `main` executes one unconditional sequence and
returns zero. `helperBad` is not called by the C program.

## Mismatches

No stdout, stderr, or exit-status mismatches were found. The existing Rust
translation already matched the C executable for every enumerated process input.
