# Differential Verification Errors

No C/Rust output, stderr, or exit-status mismatches were found.

The existing Rust translation matched the C executable for EOF and failed
conversions, valid positive and negative integers, both signed 32-bit integer
limits, input spanning newlines, numeric prefixes, and trailing input. No Rust
source changes were required.
