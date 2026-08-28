# Differential verification log

The C in `c_src/` is the ground truth. This file records every input class where
the Rust translation disagreed with the C reference, what caused it, and how it
was fixed. Nothing in `c_src/` was modified.

## How the two programs are compared

Both are built and run as executables, driven exactly the way a shell drives
them:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test                                           # the differential suite
```

The suite (`translation/tests/`) spawns both binaries as subprocesses, pipes
identical bytes into stdin, and asserts on **stdout byte for byte, stderr byte
for byte, and the exit status including death by signal**. Nothing is loaded as
a library.

Coverage of the C was measured objectively by replaying the suite's whole input
corpus through a `gcc --coverage` build of a copy of `c_src/src/main.c`:

```
Lines executed:       100.00% of 392
Branches executed:    100.00% of 262
Taken at least once:  100.00% of 262
Calls executed:       100.00% of 167
```

---

## Mismatch 1 — `strcpy` overruns past the end of a record array were not modelled, so the Rust exited 0 where the C dies of `SIGSEGV`

**Severity: this was the big one.** It produced *both* wrong output and a wrong
exit status, and randomised testing hits it readily.

### Symptom

```
adduser u0 p0 0
... (9 users total)
adduser LAST PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP 3    # 43-byte password
status
```

| | C reference | Rust (before) |
|---|---|---|
| exit status | killed by signal 11 (`SIGSEGV`, shell status 139) | exited 0 |
| stdout | 0 bytes | 4075 bytes |

### Cause

`parse_command` truncates tokens to `MAX_COMMAND - 1` = **63** bytes, but
`cmd_adduser` copies them with `strcpy` into 32-byte fields:

```c
strcpy(users[user_count].name,     args[0]);   /* char name[32]     */
strcpy(users[user_count].password, args[1]);   /* char password[32] */
users[user_count].permission_level = ...;
users[user_count].logged_in = 0;
user_count++;
```

For `users[9]` (offset 648 in the 720-byte array) the `password` field starts at
byte 680, so a 40-byte password writes bytes 680..721 — **past the end of
`users`**. In the reference build the globals are laid out contiguously in
`.bss`:

```
0x4070a0  users            720 bytes  (10 x 72)
0x407370  user_count         4        <-- immediately after users
0x407378  current_user       8
0x407380  files          12240 bytes  (20 x 612)
0x40a350  file_count         4        <-- immediately after files
0x40a360  variables       3200 bytes  (20 x 160)
0x40afe0  variable_count     4
0x40afe4  debug_mode         4
0x40afe8  verbose_mode       4
```

so the overrun lands on `user_count`. Because the build is unoptimised (the
supplied `CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`), every `users[user_count]`
in the lines that follow **reloads the corrupted global**. The result is exactly
reproducible and depends only on the password length:

| password length | bytes written over `user_count` | resulting `user_count` | effect |
|---|---|---|---|
| ≤ 39 | none | 9 | normal; `Users: 10/10` |
| 40 | terminating NUL on byte 0 | `0` | `users[0]` is rewritten; `Users: 1/10` |
| 41 | `'P'`, NUL | `0x50` = 80 | stores land inside `files`; `Users: 81/10` |
| 42 | `'P','P'`, NUL | `0x5050` = 20560 | store address ≈ 0x570be0, unmapped → **SIGSEGV** |
| 43 | `'P','P','P'`, NUL | `0x505050` = 5263440 | **SIGSEGV** |
| ≥ 44 | `'P'`×4 | `0x50505050` | **SIGSEGV** |

The original translation modelled each array as an isolated flat buffer with
1024 bytes of slack, so an overrun harmlessly smeared into padding: the counters
were plain Rust fields that nothing could corrupt.

The same thing happens one array along. `cmd_createfile` does

```c
strcpy(files[file_count].owner, current_user->name);   /* char owner[32] */
```

and `current_user->name` can itself be a long string (a 32-byte `name` field has
no room for its terminator, so the stored name runs on into `password`). Writing
that into `files[19].owner` overruns into `file_count`, and the following
`files[file_count].permissions = 755` faults.

### Fix

`translation/src/main.rs` now models the reference build's whole writable segment
as one flat byte array at the real link addresses, with `user_count`,
`current_user`, `file_count`, `variable_count`, `debug_mode` and `verbose_mode`
living *inside* it, so an overrun corrupts them the same way. Every counter is
re-read from that memory at exactly the points where the unoptimised C reloads
it. `current_user` is stored as a raw 8-byte address rather than an index, so it
can dangle.

Any access that leaves the mapped window `[0x406000, 0x40b000)` calls `segv()`,
which restores the default `SIGSEGV` disposition (the Rust runtime installs its
own handler for stack-overflow reporting, which would otherwise turn this into a
`SIGABRT`) and raises it — terminating the process with signal 11 and discarding
the buffered output, as the C does.

Tests: `differential.rs::users_array_overrun_corrupts_following_globals`,
`differential.rs::files_array_overrun_corrupts_file_count`,
`stress.rs::crash_paths_are_really_crash_paths` (which asserts the C *really* is
killed by `SIGSEGV`, so the comparison cannot pass vacuously).

---

## Mismatch 2 — buffered stdout was flushed at the wrong points, so the bytes surviving a crash differed

### Symptom

Only visible together with Mismatch 1, and only once the output was large:
after the fix above both programs died of `SIGSEGV`, but the number of bytes that
had reached the pipe differed.

### Cause

When stdout is not a terminal, glibc buffers it fully in `st_blksize`-sized
chunks (4096 for both a pipe and a regular file here). A `SIGSEGV` runs no
cleanup, so **everything still in the buffer is lost** and the consumer sees only
whole 4096-byte chunks.

The original translation used `BufWriter<Stdout>` and additionally called
`flush()` after every `printf("> ")` prompt. That pushed partial chunks out
early, so on a crash the Rust program had already emitted bytes the C never
did.

Wrapping Rust's `io::stdout()` would not have been enough either: `Stdout` is
itself a `LineWriter`, so the tail of a chunk after the last newline would stay
in *Rust's* buffer and be lost, in a place glibc would not lose it.

### Fix

`Out` now writes straight to fd 1 with `write(2)` through its own buffer sized
from `fstat(1).st_blksize`, emitting in exact `cap`-sized units, and is line
buffered instead when `isatty(1)` — mirroring glibc's
`_IO_file_doallocate`. The per-prompt flush was removed. Verified against the C
for output volumes straddling several buffer boundaries, over both a pipe and a
regular file.

Tests: `stress.rs::buffer_flush_boundaries_on_crash`,
`stress.rs::buffer_boundaries_on_clean_exit`.

---

## Latent branch found by coverage, no mismatch

Replaying the corpus through the instrumented C showed 7 branch outcomes never
taken, all the same condition: **`current_user != NULL` while
`current_user->logged_in == 0`** — the third outcome of every
`if (!current_user || !current_user->logged_in)` guard, plus the `status`
ternary.

`cmd_logout` can never produce it, because it clears the flag *and* NULLs the
pointer. It is reachable only through Mismatch 1's corruption: once `user_count`
has been reset to 0, `cmd_adduser`'s `users[user_count].logged_in = 0` clears the
flag on the record `current_user` still points at. The C then reports
`Not logged in`, `Error: No user logged in`, `Error: Must be logged in`, and
`Current user: none` while `listusers` still shows the user.

The Rust matched here on the first try — the address-based `current_user` model
already reproduced it. Added as a regression test anyway:
`differential.rs::dangling_current_user_with_cleared_flag`. With it, C branch
coverage reaches 100 % of branch *outcomes*.

---

## Behaviours confirmed correct (checked, no change needed)

These are the quirks most likely to be "fixed" by mistake; each is pinned by a
test.

- **`fgets`, not `scanf`.** At most `MAX_INPUT - 1` = 255 bytes per read,
  stopping after a newline, so a 300-byte line is split into two commands and
  the tail is executed on its own. A final line without a newline is still
  processed.
- **Blank lines are no-ops.** With no token, `parse_command` never writes
  `command`, and `process_command` then reads that uninitialised buffer — formally
  UB. The reference `-O0` build always observes it empty, so blank and
  whitespace-only lines do nothing. (At `-O2` the previous command can survive
  there and be re-run; the translation matches the reference build.)
- **The input is a C string.** An embedded NUL truncates the line before
  `strcspn` looks for the newline, so `sta\0tus` is the command `sta`.
- **Separators are only space and tab.** `\r`, `\v` and `\f` stay inside tokens,
  so `status\r` is an unknown command.
- **Token truncation and the argument cap.** Tokens are cut to 63 bytes;
  arguments past `MAX_ARGS` = 10 are dropped silently.
- **`strcmp`/`strncmp` return the raw difference of the first differing bytes,
  compared as `unsigned char`** — `compare` prints that number, so `0x80` is
  greater than `0x7f`.
- **`compareN` converts its `int` to `size_t`.** A negative `n` becomes a huge
  unsigned count, so `compareN abc abd -1` compares the whole strings while
  still printing `-1`.
- **`atoi` is `(int)strtol(s, NULL, 10)`**: `abc` → 0, `12abc` → 12, `0x10` → 0,
  and out-of-range values saturate to `LONG_MAX`/`LONG_MIN` and are then
  truncated to `int` (`9223372036854775808` → `-1`).
- **Validation order.** `createfile`/`writefile`/`deletefile` check the login
  *before* the argument count, so `createfile` with no arguments and nobody
  logged in prints `Error: Must be logged in`, not the usage line. `readfile`
  needs no login at all. `cmd_set` scans for an existing name *before* the
  capacity check, so an existing variable can still be updated when the table is
  full.
- **Asymmetric permission thresholds.** Writing someone else's file needs level
  ≥ 5, deleting it needs ≥ 9.
- **`printf` spacing.** `listusers` prints `"  %s (level %d) %s\n"` with an empty
  final `%s` when the user is not logged in, leaving a trailing space before the
  newline. `help` and `status` open with a bare `\n`. `time` prints
  `"Current time: %s"` with no newline of its own, because `ctime` supplies one.
- **Exact dispatch beats prefix suggestion.** `strcmp` is tried for every command
  and alias before the `strncmp` prefix hints, and the hints are ordered
  `add`, `log`, `list`, `create`, `read`, `write`, `delete`. So `write` is
  `writefile` but `writex` is a suggestion; `read` is a suggestion because the
  `readfile` alias is `cat`.
- **`exit`/`quit` call `exit(0)`** mid-loop; the rest of stdin is never read.

## Test inventory

| File | Tests | Focus |
|---|---|---|
| `tests/common/mod.rs` | — | harness: spawn, pipe, compare stdout/stderr/status; optional corpus dump via `DIFFTEST_DUMP_INPUTS` |
| `tests/differential.rs` | 29 | one test per command and per branch: argument-count guards, capacity limits, permission levels, aliases, prefix hints, parsing and `fgets` edges, byte-level input, struct-field and array overruns |
| `tests/stress.rs` | 10 | meta-assertions that the crash paths really crash, stdio buffer boundaries (pipe and file), the clock-dependent `time` command, and four families of randomised sessions |

No test is `#[ignore]`d, skipped or disabled.
