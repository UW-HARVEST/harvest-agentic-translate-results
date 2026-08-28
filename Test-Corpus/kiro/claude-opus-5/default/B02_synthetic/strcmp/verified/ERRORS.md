# Differential verification of the Rust translation

The C program in `../c_src` is the ground truth. `tests/differential.rs` runs
both executables as subprocesses over the same stdin and requires byte-identical
stdout, byte-identical stderr and the same termination status (exit code *or*
fatal signal).

- C build: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust build: `cd translation && cargo build --release`
  → `translation/target/release/driver`
- Run both as `./driver < input`. Neither program takes arguments and neither
  writes to stderr.

`cargo test` builds the C program itself if `c_src/build/driver` is missing, so
the suite is self-contained. No test is `#[ignore]`d, skipped or disabled, and
nothing in `c_src/` was modified.

## Mismatches found and fixed

### 1. Overflowing struct fields did not clobber the neighbouring globals

**Symptom.** For `adduser <name> <40-byte-password> 5` as the tenth user, the C
program printed a single user with a rewritten permission level and
`Users: 1/10`, while Rust printed all ten users and `Users: 10/10`.

```
adduser u1 p1 1 ... adduser u9 p9 9
adduser LAST PPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPPP 5
listusers
status
```

| | C | Rust (before) |
|---|---|---|
| `listusers` | `  u1 (level 5)` only | all ten users |
| `status` | `Users: 1/10` | `Users: 10/10` |

**Cause.** `cmd_adduser` does `strcpy(users[user_count].password, args[1])` into
a `char[32]`, but `parse_command` hands it tokens of up to 63 bytes. In the
compiled C program the globals are laid out consecutively in BSS:

```
users            rel      0   size   720
user_count       rel    720   size     4
current_user     rel    728   size     8
files            rel    736   size 12240
file_count       rel  12976   size     4
variables        rel  12992   size  3200
variable_count   rel  16192
debug_mode       rel  16196
verbose_mode     rel  16200
```

`users[9].password` starts at rel 680, so a 40-byte password writes its NUL
terminator exactly onto byte 0 of `user_count`, turning 9 into 0. The following
statements re-read `user_count`, so `users[0].permission_level = 5` and then
`user_count++` leaves the table holding one user. A 41-byte password stores
`'P'` there instead, giving `user_count == 80`.

The original translation modelled `users`, `files` and `variables` as three
separate `Vec<u8>`s with slack bytes, and kept the counters as ordinary Rust
`i32` fields, so an overflow could never reach them.

**Fix.** All mutable global state is now one flat byte array (`struct Mem`)
using the exact offsets above, and `user_count`, `file_count`,
`variable_count`, `debug_mode`, `verbose_mode` and the `current_user` pointer
are read and written through it. Every statement re-reads its counter from
memory the way the C does, e.g.

```rust
let uc = self.user_count();
self.mem.strcpy(Self::user_at(uc) + U_PASS, &pass);
let uc = self.user_count();          // may have just been overwritten
self.mem.set_i32(Self::user_at(uc) + U_PERM, level);
```

Covered by `last_user_slot_overflow_clobbers_globals` (sweeps password lengths
0..=63), `last_user_slot_name_overflow`, `continuing_after_a_clobbered_user_count`
and `clobbering_counters_with_high_bytes` (0xff bytes drive the counter
negative rather than positive).

### 2. `files[i].owner` overflow did not clobber `file_count`

**Symptom.** After logging in as a user whose stored name is longer than 35
bytes and creating twenty files, C printed `Files: 1/20` and listed one file;
Rust printed `Files: 20/20`.

**Cause.** The same class of bug one array over. `cmd_createfile` does
`strcpy(files[file_count].owner, current_user->name)` into a `char[32]`, and
`current_user->name` is itself longer than 31 bytes whenever the user was added
with a name of 32 bytes or more (see mismatch 3). `files[19].owner` starts at
rel 12940 and `file_count` is at rel 12976, so an owner string of 36 bytes or
more writes into it; from 52 bytes it reaches `variables[0]`.

**Fix.** Same flat-memory model. Covered by
`owner_field_overflow_clobbers_file_count`, which sweeps stored-owner lengths
33..=63 both with a full table (overflow leaves `files`) and with two files
(overflow lands inside `files`).

Reaching this path needs care in the test itself: because `name[32]` overflows
into `password`, a user added as `adduser NNN…(32) PPP` is *stored* under the
name `NNN…(32)PPP`, so `login NNN…(32) PPP` reports `Error: User not found`.
The test logs in with the concatenated name.

### 3. A fatal fault was not reproduced

**Symptom.** With a password of 42 bytes or more in the last user slot, the C
program was killed by SIGSEGV (shell status 139) and produced **no** stdout at
all; Rust exited 0 after printing everything.

**Cause.** A 42-byte password leaves `user_count == 0x4e4e`, so
`users[20046].permission_level = …` addresses roughly 1.4 MB past the end of the
BSS, outside the process's writable mapping. The writable pages of the C program
are `[0x406000, 0x40b000)`, i.e. `[-4256, 16224)` relative to `&users`.

**Fix.** Every access through `Mem` is bounds checked against that window; an
access outside it restores the default `SIGSEGV` disposition and `raise`s it, so
the Rust process dies by the same signal with no Rust destructors run.

```rust
fn segv() -> ! {
    unsafe { signal(SIGSEGV, SIG_DFL); raise(SIGSEGV); }
    std::process::abort()
}
```

Covered by the length sweeps above (lengths ≥ 42 crash, ≤ 41 do not) and by
`output_buffered_before_a_fatal_fault_matches`.

### 4. Wrong base address shrank the valid memory window

**Symptom.** After introducing the flat-memory model the Rust program segfaulted
on *every* input, including a bare `status`.

**Cause.** I wrote `&users` as `0x407ce0` in the layout constants from a
miscalculation; the real address is `0x4070a0` (`nm -td c_src/build/driver`
reports 4223136). `MEM_HI` is derived from it, so the window came out 3136 bytes
too short and `variable_count` at rel 16192 fell outside it.

**Fix.** `BASE_ADDR = 0x0040_70a0`, taken from the binary. This constant also
has to be right for its own sake: `current_user` stores an absolute address, so
the base is what maps a clobbered pointer value back onto a struct offset.

### 5. Buffered stdout lost on a crash did not match

**Symptom.** Once the crash was reproduced, the crashing runs still differed:
for a fault preceded by thirty `help` calls the C program had already written
36864 bytes, while Rust wrote a different amount.

**Cause.** Two separate issues.

- glibc fully buffers stdout to a pipe or file with a 4096 byte buffer (the
  `st_blksize` of the descriptor) and flushes only when the buffer is exactly
  full, so surviving output is always a multiple of 4096. Rust's `BufWriter`
  defaults to 8192 bytes and flushes whatever happens to be buffered when the
  next write does not fit, giving flush boundaries that are not multiples of
  4096.
- Rust's `io::stdout()` is a `LineWriter`; it flushes on every newline
  regardless of whether the destination is a terminal. That would have flushed
  nearly everything before the crash.

**Fix.** `struct Out` writes straight to fd 1 through a `ManuallyDrop<File>` and
reimplements glibc's `_IO_new_file_xsputn`: fill the 4096 byte buffer, and only
if bytes remain flush the now-full buffer, write whole blocks directly, and
buffer the tail. `exit`/end-of-input flush explicitly, matching the C exit path;
a `segv()` does not.

Covered by `output_buffered_before_a_fatal_fault_matches`, which checks faults
after 0, 1, 3, 7, 10 and 30 `help` calls (0 → 0 bytes survive, 3 → 4096,
10 → 12288, 30 → 36864).

## Behaviours confirmed to already match

Verified, not assumed — each has a test.

- **glibc `strcmp`/`strncmp` return the byte difference**, not a normalised
  -1/0/1: `strcmp('abc','abcd')` prints `-100`, `strcmp('zzz','a')` prints `25`.
  A translation using Rust's `Ord` would print -1 and 1.
- **`strncmp`'s `n` is an `int` widened to `size_t`.** `compareN abc abc -5`
  sign extends to a huge length, so the comparison still stops at the NUL and
  returns 0, and the count is printed back as `-5`:
  `First -5 characters are equal`.
- **`atoi` is `(int) strtol`**: it saturates in `long` and then truncates, so
  `adduser a p 99999999999999999999` yields level `-1`, the negative form yields
  `0`, and `4294967296` yields `0`.
- **`fgets` reads at most 255 bytes per call**, so a longer line is split and
  each chunk is processed as its own command with its own `> ` prompt. A
  300-byte `set` line becomes `set k vvv…` followed by a second command made of
  the remaining `v`s.
- **`strtok` splits on `' '` and `'\t'` only**; `'\r'` stays inside the token,
  runs of separators are skipped, and `MAX_ARGS` is 10 so later tokens are
  dropped.
- **Tokens are truncated to 63 bytes** by `strncpy(…, MAX_COMMAND - 1)`.
- **`input[strcspn(input, "\n")] = 0`** stops at the terminating NUL as well as
  at a newline, so a NUL embedded in a line truncates the command:
  `sta\0tus` runs `sta`, which falls through to the `Unknown command` branch.
- **`printf` spacing**, including the trailing space `listusers` prints when a
  user is not logged in (`"  %s (level %d) %s\n"` with an empty final `%s`), the
  leading blank line before `=== System Status ===`, and `ctime`'s own trailing
  newline after `Current time: `.
- **Order of validation.** `cmd_createfile`, `cmd_writefile` and
  `cmd_deletefile` check "must be logged in" *before* checking the argument
  count; `cmd_set` searches for an existing variable *before* checking
  `MAX_VARIABLES`, so an update still succeeds with a full table.
- **`exit`/`quit` call `exit(0)`** after flushing, so the remainder of stdin is
  never read.
- **The suggestion ladder** is only reached after every exact match fails, and
  is ordered, so `list…` is answered before `login`-style prefixes get a chance.
- **`time`** uses `time()`/`ctime()` from libc, so the local timezone is applied
  identically; the test masks the timestamp and checks the rendered length.
- The C program never writes to stderr, and stderr is compared on every case.

## Coverage beyond the test suite

Alongside `cargo test`, the translation was checked against roughly 3000
randomly generated sessions and ~270 hand-built cases, including runs seeded
into states that sit on the overflow boundaries (nine users then a long
password, an overflowed owner name then a full file table, a full variable
table). No remaining differences in stdout, stderr or termination status.

## Known limits of the emulation

The overflow fidelity above depends on the layout of the compiled C program:
the BSS offsets, the `[0x406000, 0x40b000)` writable window and the 4096 byte
stdout buffer are read from `c_src/build/driver` as built by the CMake file in
this repository (non-PIE, x86-64, glibc). Those are properties of a specific
binary, not of the C source, so rebuilding with a different compiler,
optimisation level or libc could move them. Every input whose behaviour is
well-defined in C is unaffected; only the memory-corruption cases are tied to
the layout. The constants are gathered in one block at the top of
`src/main.rs`, next to the `nm` and `readelf` output they came from.
