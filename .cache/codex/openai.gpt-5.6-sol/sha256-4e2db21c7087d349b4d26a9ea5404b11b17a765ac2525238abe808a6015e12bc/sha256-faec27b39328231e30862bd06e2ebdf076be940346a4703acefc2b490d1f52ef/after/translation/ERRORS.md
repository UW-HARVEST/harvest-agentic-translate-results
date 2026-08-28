# Differential Errors

## Runtime pointer bytes

- **Observed:** Successful add/remove/list/compare workflows print different
  hexadecimal values in `%p` fields.
- **Cause:** The C and Rust executables run as independent processes. ASLR and
  their different allocation layouts give their singleton shapes different
  virtual addresses.
- **Treatment:** Rust prints its real singleton addresses, preserving the C
  program's pointer identity and reuse behavior. Differential tests replace
  only hexadecimal pointer lexemes with first-seen identity tokens before
  comparing stdout; all remaining stdout bytes, all stderr bytes, and exit
  statuses are compared directly.

No deterministic translation mismatch was found in the enumerated input
classes.

## Non-input failure branches

The C executable also checks allocation failures and null pointers passed to
internal scene/shape functions. No stdin or file input can produce those
arguments, and forced allocator failure is environment-dependent, so these
branches are outside the executable input matrix. The subprocess suite does
exercise every input-reachable early return and error message.
