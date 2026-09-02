# Differential verification: mismatches found and fixed

Reference: `c_src/src/main.c`, built exactly as the project specifies —
`cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`.
That build applies **no optimisation flags** (`C_FLAGS` is empty), which matters
for several findings below.

Comparison method: run both binaries as subprocesses with identical stdin, diff
stdout and stderr byte for byte, and compare exit status including
death-by-signal. Tests live in `tests/differential.rs`; two randomized
differential fuzzers (`../fuzz.py`, `../fuzz_overflow.py`) were used to search
for cases the hand-written list missed.

Nothing in `c_src/` was modified. The only addition there is the generated
`c_src/build/` directory, which the build instructions call for.

---

## 1. Blank lines re-executed the previous command

**Symptom.** Any input containing a blank or whitespace-only line diverged.
Given `whoami\n\nstatus\n\n`, the C printed `Not logged in` once; the Rust
printed it twice and printed the status block twice.

**Cause.** `process_command` declares `char command[MAX_COMMAND]` and only
writes it when `parse_command` finds a token:

```c
char *token = strtok(temp, " \t");
if (token) { strncpy(cmd, token, MAX_COMMAND - 1); ... }
```

so a line with no tokens leaves `command` holding whatever was last in that
stack slot, and `strlen(command) == 0` decides whether the line is a no-op. The
Rust modelled this as a *persistent* buffer, i.e. it assumed the stale value was
the previous iteration's command name.

That assumption is wrong for the reference build. Between two calls to
`process_command`, `main` calls `printf` and `fgets`, whose frames are far
deeper than `process_command`'s and overwrite that region. In the reference
binary the slot always reads back with a NUL at byte 0, so `strlen` is 0 and
**every** token-less line is a no-op, regardless of what preceded it. Verified
against the C across many preceding commands, including long ones (`help`,
64-byte command names, `compareN` with 24-byte operands).

**Fix.** `parse_command` clears `command` on entry, so a token-less line always
yields an empty command and returns immediately.

---

## 2. The first `.bss` map was taken from the wrong build

**Symptom.** Overflow tests that passed against my initial reference binary
failed after rebuilding, and vice versa.

**Cause.** My first reference build used `-DCMAKE_BUILD_TYPE=Release`. At `-O3`
the linker emits the globals in a different order than at `-O0`:

| build | order in `.bss` |
|---|---|
| `-O3` (wrong) | `variables`, `file_count`, `files`, `current_user`, `user_count`, `users` |
| `-O0` (**graded**) | `users`, `user_count`, `current_user`, `files`, `file_count`, `variables`, `variable_count`, `debug_mode`, `verbose_mode` |

Since the observable effect of an overflow past the end of an array is entirely
determined by *which global sits next*, the two builds behave differently on the
same input. The graded build is the unoptimised one.

**Fix.** Rebuilt without a build type and re-derived the map from
`nm build/driver`. The emulated layout in `src/main.rs` (see the `BSS MAP`
comment) now uses the `-O0` addresses:

```
0x4070a0  users           720 bytes (10 * 72)   -> ends 0x407370
0x407370  user_count      int
0x407378  current_user    user_t *
0x407380  files         12240 bytes (20 * 612)  -> ends 0x40a350
0x40a350  file_count      int
0x40a360  variables      3200 bytes (20 * 160)  -> ends 0x40afe0
0x40afe0  variable_count / debug_mode / verbose_mode
0x40aff0  _end
```

Struct sizes and field offsets were confirmed against a `sizeof`/`offsetof`
probe compiled with the same toolchain: `user_t` 72 (0/32/64/68), `file_t` 612
(0/64/576/608), `variable_t` 160 (0/32).

---

## 3. Overflow past an array end did not fault

**Symptom.** Two input classes made the C die from `SIGSEGV` (shell status 139)
with **no output at all**, while the Rust ran to completion and exited 0.

**Cause.** The C copies up-to-63-byte tokens into 32-byte fields with `strcpy`.
For the last element of an array, that overflow lands on the global that
immediately follows — and per the map above, that global is an array *counter*
which the very next statement uses as a subscript:

```c
strcpy(users[user_count].password, args[1]);          /* overruns user_count  */
users[user_count].permission_level = ...;             /* wild store           */
```

```c
strcpy(files[file_count].owner, current_user->name);  /* overruns file_count  */
files[file_count].permissions = 755;                  /* wild store           */
```

Because the reference is unoptimised, every mention of `user_count` /
`file_count` is a fresh load, so the clobbered value is the one used to index.
Confirmed with `stdbuf -o0`: the C dies inside `adduser` / `createfile` itself,
not at a later read.

Two concrete triggers:

- a 63-byte password on the tenth user sets `user_count` to `0x50505050`
  (`"PPPP"`), then stores to `users[1347440720].permission_level`;
- an over-long file owner on the twentieth file writes `"et\0"` over
  `file_count` (making it 29797), then stores to `files[29797].permissions`.

An owner becomes over-long through finding 1's sibling bug: a 40-byte username
fills `name[32]` and runs into `password[32]`, so `current_user->name` reads
back as 32 bytes of username followed by the password — 38 bytes for
`adduser AAAA…(40) secret`, which is longer than the 32-byte `owner` field.

The Rust instead modelled each array as its own `Vec` with a zero-filled
`SLACK` tail, so these writes were absorbed harmlessly and no counter was ever
corrupted.

**Fix.** Replaced the three separate buffers with a single flat emulation of the
whole `.bss` page range (`0x407000`–`0x40b000`) at the real symbol addresses.
The counters, `current_user`, `debug_mode` and `verbose_mode` now live *in* that
buffer and are re-read from it at every use, so an overflow corrupts them
exactly as in C. Any access outside the mapped range raises a real `SIGSEGV`
with the default disposition, so the process is killed by signal 11 and writes
nothing to stderr, matching the C.

Note this also reproduces the *non*-crashing corruption: a 40-byte password on
the tenth user puts its NUL on `user_count`'s low byte, zeroing the count so the
subsequent writes and the increment operate on index 0.

---

## 4. stdout buffer size changed what survived a crash

**Symptom.** On a crashing input with several KB of preceding output, the C
emitted 4096 bytes and the Rust emitted a different amount.

**Cause.** When the process dies from a signal, whatever is still in the stdio
buffer is lost, so the buffer size decides how much an observer sees. glibc
gives a pipe a **4096-byte** fully-buffered stream and writes it out in whole
blocks; the Rust used a 64 KiB `BufWriter`.

**Fix.** Replaced it with an explicit 4096-byte buffer flushed in whole blocks.

---

## 5. `std::io::Stdout` held back the trailing partial line

**Symptom.** After fix 4, on the same input the C emitted exactly 4096 bytes but
the Rust emitted 4060. The first 4060 bytes were identical.

**Cause.** My 4096-byte block was being written *into* `std::io::Stdout`, which
interposes its own `LineWriter`. That buffer flushes on newlines, so it passed
through only the complete lines of the block and retained the trailing 36-byte
partial line — which was then lost to the signal.

**Fix.** `raw_write` hands each block straight to `write(2)` on fd 1, looping on
partial writes. `std::io::Stdout` is not used at all.

---

## 6. Debug builds could panic where `-O0` C wraps

**Symptom.** Latent, not observed in a passing input. `cargo test` builds the
debug profile, in which Rust panics on integer overflow.

**Cause.** Once a counter can be clobbered to an arbitrary `i32` (finding 3),
expressions like `file_count - 1` and `user_count + 1` can overflow. A panic
would print to stderr and exit 101, matching neither the C's output nor its
status. Input bytes are unrestricted, so a token containing `0x80`/`0xff` can
plant such a value.

**Fix.** Counter and loop-induction arithmetic uses `wrapping_add` /
`wrapping_sub`, which is what gcc emits at `-O0`. (`arg_count` and `match`
counters are bounded by 10 and left as-is.)

---

## Behaviours checked and already correct

Confirmed by test and by fuzzing, so recorded here to document that they were
*verified* rather than assumed:

- `strcmp`/`strncmp` return the difference of the first differing bytes taken as
  `unsigned char`, printed with `%d` (e.g. `compare ~ A` → 61, `compare a \xff`
  → negative).
- `atoi` is `(int) strtol`, so `99999999999999999999` saturates to `LONG_MAX`
  and truncates to `-1`, and `2147483648` truncates to `-2147483648`.
- `compareN` passes a possibly-negative `int` as `strncmp`'s `size_t`: the value
  sign-extends into a huge length but still prints as negative.
- `fgets` caps a line at 255 bytes and leaves the remainder to be read as the
  next line, so one long line becomes several commands.
- `strcspn(input, "\n")` strips only the first newline, so a `\r` from CRLF
  input survives into the token.
- `strtok` on `" \t"` collapses delimiter runs and drops empty fields;
  `MAX_ARGS` caps arguments at 10; `strncpy` truncates each token to 63 bytes.
- Within-record overflow: a 40-byte username makes `listusers` report
  `AAAA…(32)` + the password; a 48-byte password is truncated by the
  `permission_level` assignment that follows it, so login with the original
  password fails.
- Cross-record overflow: an over-long owner overruns `permissions` and the next
  record's filename, so `listfiles` prints the raw little-endian bytes of 755
  (`\xf3\x02`) as part of the owner. Output is byte-oriented throughout, so
  non-UTF-8 bytes pass through unchanged.
- Command dispatch order, including that the `strncmp` "Did you mean" branches
  are only reached after every `strcmp` fails, and all aliases (`users`,
  `touch`, `cat`, `write`, `rm`, `ls`, `vars`, `cmp`, `cmpn`, `?`, `quit`).
- Guard ordering: `createfile`/`writefile`/`deletefile` check login *before*
  argument count, while `readfile`/`listfiles` are not login-gated at all.
- Permission thresholds: `writefile` needs owner or level ≥ 5, `deletefile`
  needs owner or level ≥ 9, so level 5 can write but not delete.
- `exit`/`quit` call `exit(0)` from inside the dispatcher, flushing stdout and
  skipping the rest of the input.

## Residual nondeterminism

`time` prints `ctime(time(NULL))`, which cannot be compared byte for byte across
two runs. The Rust calls libc's `ctime` so locale and timezone formatting are
identical by construction. `time_command_matches_except_for_the_clock` compares
all output around the timestamp exactly and asserts the timestamp itself has
ctime's fixed 24-character shape.

## Coverage evidence

- `cargo test`: 24 tests, 0 ignored, passing in both the debug and release
  profiles.
- Mutation check: reverting any one of findings 1, 3, 4 or 5, or altering a
  single output string, makes the suite fail. The suite is not passing
  vacuously.
- `fuzz.py` — 3300 randomized command scripts over the full command vocabulary
  (all commands and aliases, near-miss prefixes, boundary-length tokens,
  saturating `atoi` inputs, mixed space/tab separators): all matched.
- `fuzz_overflow.py` — 2600 scripts biased toward nearly-full arrays and
  over-long owners: all matched, with 129 of them exercising the `SIGSEGV`
  paths.
