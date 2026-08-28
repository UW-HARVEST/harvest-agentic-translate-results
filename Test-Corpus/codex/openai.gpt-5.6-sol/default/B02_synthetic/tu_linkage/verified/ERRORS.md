# Differential Verification Errors

## C-to-Rust mismatches

None observed. All enumerated cases matched stdout, stderr, and exit status.

## Test harness corrections

- The first test compile used a Unicode escape in a byte string for vertical
  tab. Rust byte strings require `\x0b`; correcting the literal allowed the
  intended stdin-delimiter comparison to run. This was not an executable
  behavior mismatch.

## Checked input classes

- Empty input, help, invalid arguments, stdin parsing, embedded NUL, and the
  4095-byte `fgets` boundary.
- Every VM opcode, every engine error return (`1` through `11` and `99`), jump
  and repeat branches, and partial second-pop behavior in opcode `9`.
- Integer and `long` boundaries, arithmetic wrapping, classifier buckets,
  stateful classifier calls, stream branches, and vector growth.
