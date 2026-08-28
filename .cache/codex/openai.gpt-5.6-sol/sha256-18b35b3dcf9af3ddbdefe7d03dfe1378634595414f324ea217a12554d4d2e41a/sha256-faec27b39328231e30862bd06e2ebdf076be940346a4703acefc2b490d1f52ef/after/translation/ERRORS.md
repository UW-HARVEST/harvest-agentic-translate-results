# Differential Mismatches

## Case-sensitive pattern longer than text

- Input class: operation 4 with flag bit `0x02` set and a null-terminated
  pattern longer than the null-terminated text.
- C result: no output; process terminated by `SIGSEGV`.
- Original Rust result: printed `1034` and exited successfully.
- Cause: `text_len - pattern_len` wraps as `size_t` in C. The resulting
  unbounded scan reaches unmapped memory. Rust instead searched its finite
  modeled storage and found the pattern in the adjacent reference region.
- Fix: Rust now terminates with `SIGSEGV` when this input-reachable underflow
  condition occurs.
