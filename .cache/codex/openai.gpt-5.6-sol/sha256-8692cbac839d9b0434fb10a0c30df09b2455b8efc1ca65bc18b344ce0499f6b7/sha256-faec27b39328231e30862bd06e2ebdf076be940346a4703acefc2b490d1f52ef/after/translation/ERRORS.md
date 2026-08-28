# Differential Test Results

No C/Rust output, error, or exit-status mismatches were found.

The Phase C branch audit also identified C returns that cannot be reached by
any executable input:

- `apply_permissions`: the `permission_value != 6` path inside `read && write`
  is impossible because that condition fixes the value at 6.
- `evaluate_conditions`: XOR result 90 is impossible for three booleans, and
  NAND result 100 is preempted by the checks for each false condition.
- `configure_flags`: after detecting exactly one true or exactly one false,
  the corresponding search loop must find that value.
- `validate_sequence`: its direct zero-length return is preempted by
  `process_decisions`, and long-sequence result 40 is impossible because more
  than nine values with fewer than three transitions necessarily contains four
  consecutive equal values and returns -12 first.
