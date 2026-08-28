# Differential Errors

## Blank command repeated the prior command

- Input class: a blank or whitespace-only line after a valid command.
- Mismatch: C printed only the next prompt, while Rust executed the previous
  command again.
- Cause: Rust retained `previous_command` and used it when tokenization returned
  no command. C's observed executable returns from `process_command` for these
  inputs because the command buffer is empty.
- Resolution: removed the retained-command fallback; tokenless input now returns
  without dispatching a command.
