# Differential Verification Errors

No C/Rust output mismatches were observed.

This file is the mismatch ledger for the differential test run. Each discovered
mismatch will record its triggering input, observed difference, cause, and fix.

## Executables

- C: `c_src/build/driver`
- Rust release: `translation/target/release/driver`

## Reachability Notes

The integration suite covers every branch reachable through the executable's
stdin interface, including all operation modes, validation failures, operation
failures, empty buffers, and maximum accepted counts and lengths.

Branches requiring a null pointer, an externally corrupted checksum or buffer
length, direct calls to unused helper functions, or an allocator failure cannot
be reached by running this executable with input. They are therefore outside
the subprocess comparison surface.
