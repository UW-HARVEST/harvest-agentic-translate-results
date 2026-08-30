# Differential Mismatches

No stdout, stderr, or exit-status mismatch was found between the C and Rust
executables.

## Audit Notes

- The C program has no explicit input error path. It ignores the return value
  from `scanf`, retains zero-initialized values after failed conversions, and
  returns status 0.
- Inputs with positive `x` and negative `y` do not terminate: after entering
  `foo`, the C program decrements negative `y` away from zero indefinitely.
  The Rust translation has the same behavior, so completion-based subprocess
  tests exclude this input class.
- An initial harness probe using `-2147483649 0` was removed because this
  platform converts the first token to `INT_MAX`, requiring billions of output
  lines. Other overflow probes cover the same truncation and signedness rules
  with terminating converted values. This was a test-harness issue, not a
  translation mismatch.
