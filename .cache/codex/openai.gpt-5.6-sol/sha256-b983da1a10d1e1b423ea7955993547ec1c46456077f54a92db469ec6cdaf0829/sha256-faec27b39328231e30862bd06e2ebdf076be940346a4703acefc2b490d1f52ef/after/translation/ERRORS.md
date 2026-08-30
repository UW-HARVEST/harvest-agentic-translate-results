# Differential Verification Errors

## Mismatches

No mismatches were found. The initial Rust translation matched the C executable
for every enumerated input class, so no translation fix was required.

## Audited Behavior

The differential suite covers both `scanf` failure outcomes, successful
conversion and token termination, whitespace across lines, extra input,
32-bit signed limits and conversion overflow, and the multiplication and
addition overflow boundaries in `2*x + 300`.
