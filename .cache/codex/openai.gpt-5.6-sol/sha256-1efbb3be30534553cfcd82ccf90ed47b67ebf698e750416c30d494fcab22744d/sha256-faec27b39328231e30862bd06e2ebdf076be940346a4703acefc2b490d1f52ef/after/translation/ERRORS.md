# Differential Errors

No C/Rust output, stderr, or exit-status mismatches were found during Phases B
and C, so no translation fixes were required.

The differential suite covers EOF, every menu choice, both user-visible error
messages, integer parsing boundaries and overflow, embedded NUL bytes, and the
254/255/256-byte `fgets` boundaries. It also covers multiple commands and
overlong physical lines whose remaining bytes are read as another command.

Null pointers and allocation failures in the generic container helpers are not
stdin-controlled paths of this executable. The fixed demo data exercises the
normal allocation paths and both outcomes of its item, price, quantity, and
order predicates.
