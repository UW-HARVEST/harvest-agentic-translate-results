# Differential Testing Errors

No C/Rust output, error, or exit-status mismatches were found during the
initial validation-path probes.

The programs were invoked with the same synthetic `argv[0]` in differential
tests because the argument-count error intentionally includes `argv[0]` in its
message.
