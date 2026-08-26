# CONFIGS.md — configuration-surface table (Phase A / gate for Phase B)

Derived mechanically from `c_src/src/main.c`: the axes below are exactly the
things the C branches on.

## Axes the C code actually distinguishes

1. **Runtime modes** (the only two settable flags, both `static int`):
   `debug_mode` (`if (debug_mode) printf("[DEBUG] Command: '%s', Args: %d\n"…)`
   in `process_command`) × `verbose_mode`
   (`if (verbose_mode) printf("[VERBOSE] Processing: '%s'\n"…)` in `main`,
   printed **before** the `[DEBUG]` line and also for lines that produce no
   command at all). 4 combinations, orthogonal to every command.
2. **Session state**: nobody logged in / logged in; and for a logged-in user the
   `permission_level` thresholds the code compares against:
   `>= 5` (`cmd_writefile`) and `>= 9` (`cmd_deletefile`), plus owner-vs-non-owner
   (`strcmp(files[i].owner, current_user->name)`).
3. **Table population** for each of the three tables: `0` / `1` / `many` /
   `full` (`MAX_USERS 10`, `MAX_FILES 20`, `MAX_VARIABLES 20`), and for
   `deletefile`/`unset` the *position* removed (first / middle / last), because
   both shift the array with struct assignment.
4. **Entry points** — the full command table of `process_command`, including
   every alias, which is the lowest level the program can be driven at:
   `adduser, login, logout, whoami, listusers|users, createfile|touch,
   readfile|cat, writefile|write, deletefile|rm, listfiles|ls, set, get, unset,
   listvars|vars, compare|cmp, compareN|cmpn, startswith, match, debug, verbose,
   status, time, help|?, exit|quit`, plus the 7 `strncmp` "did you mean" prefix
   branches and the unknown-command fallback.
5. **Input shape** produced by `fgets`/`strtok`/`strncpy`:
   token count `0,1,2,3,…,10,11+` (`MAX_ARGS 10`), token length
   `1,31,32,33,63,64,>64` (`MAX_COMMAND-1 = 63` truncation, and the 32-byte
   struct members that `strcpy` overruns), line length
   `0,1,254,255,256,>256` (`MAX_INPUT-1 = 255` `fgets` split), separator runs
   (`' '`, `'\t'`, repeated, leading, trailing), byte values (ASCII, `0x80..0xff`
   — `strcmp` is `unsigned char`), embedded `'\0'` (`strcspn`), `'\r'`, and
   final line with/without `'\n'`.
6. **Numeric inputs** parsed with `atoi` (permission level, `compareN` count):
   negative, `0`, threshold values `4,5,8,9`, `INT_MAX`, `INT_MAX+1`,
   `LONG_MAX+1`, non-numeric, `+`/`-` prefixes, digits after non-digits.
7. **Overflow shape** (`strcpy` into 32-byte members with 63-byte tokens — the
   C's dominant *observable* behaviour): which member overruns which neighbour,
   and whether the overrun stays inside the array or reaches the trailing
   `static` (`user_count` / `file_count`) — i.e. slot `0..8` vs slot `9` for
   `users`, slot `0..18` vs slot `19` for `files`.
8. **stdio state**: total output below / above a 4096-byte `st_blksize` block
   (glibc only writes whole blocks, so this decides how much survives a crash),
   the buffering *mode* glibc picks for the stream (fully buffered pipe/file vs
   line-buffered terminal, plus the line-buffer flush glibc performs before
   reading from an interactive stdin), and the three ways the program ends:
   `exit(0)` via `exit|quit`, `fgets` returning NULL (EOF), and death by
   `SIGSEGV`.

## Rows (cross-product, pruned to what the C treats differently)

Each row is exercised by `tests/configs.rs` with **many randomized inputs**
(fixed seeds, so failures reproduce) unless the row is inherently a single
shape. Checked off only after passing.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C01 | *program* | empty stdin (immediate EOF): banner + one `> ` + exit 0 | [x] |
| C02 | *program* | last line without trailing `'\n'`; and a lone `'\n'` | [x] |
| C03 | `process_command` | lines that tokenize to nothing: `""`, `" "`, `"\t"`, `"   \t \t "` × modes off/on | [x] |
| C04 | `help`, `?` | both aliases, in all 4 mode combinations | [x] |
| C05 | `status` | all population levels: empty / 1 / many / full users+files+vars × logged in/out × debug/verbose on/off | [x] |
| C06 | `debug` | `debug`, `debug on`, `debug off`, `debug on` twice, then any command (checks the `[DEBUG]` line and its `Args:` count) | [x] |
| C07 | `verbose` | `verbose`, `verbose on/off`, then lines incl. blank ones (the `[VERBOSE]` line is printed even when no command runs) | [x] |
| C08 | `debug`+`verbose` | both ON: ordering `[VERBOSE]` then `[DEBUG]`, over a random command stream | [x] |
| C09 | `adduser` | 1..10 users, random short names/passwords, level omitted / given | [x] |
| C10 | `adduser` | level values `-5,-1,0,1,4,5,8,9,10,2147483647,2147483648,99999999999999999999,abc,+7,-0` | [x] |
| C11 | `adduser`,`listusers`,`users` | exactly `MAX_USERS` users, listed via both aliases, some logged in | [x] |
| C12 | `login`,`logout`,`whoami` | random login/logout/relogin sequences over a random user table (correct + wrong passwords, unknown users) | [x] |
| C13 | `whoami` | each session state: never logged in / logged in / after logout | [x] |
| C14 | `listusers` | 0 / 1 / many users, with and without a `[logged in]` flag | [x] |
| C15 | `createfile`,`touch` | both aliases, with and without content, random filenames/contents, 1..20 files | [x] |
| C16 | `readfile`,`cat` | both aliases: existing file, after `write`, after `delete`, while logged out | [x] |
| C17 | `writefile`,`write` | owner path; non-owner with level `0,1,4` (denied) and `5,8,9,10` (allowed); random contents | [x] |
| C18 | `deletefile`,`rm` | owner path; non-owner level `5,8` (denied) / `9,10` (allowed); delete first / middle / last of 20 (array shifting); re-create afterwards | [x] |
| C19 | `listfiles`,`ls` | 0 / 1 / many / full(20) files, both aliases, after deletions | [x] |
| C20 | `set`,`get`,`unset`,`listvars`,`vars` | new + update, get hit/miss, unset first/middle/last of 20 (shifting), both list aliases | [x] |
| C21 | `compare`,`cmp` | random pairs: equal, prefix-of, differing first byte, one empty-ish, high bytes (`0x80..0xff`), 1..63 bytes — checks the exact `strcmp` return value | [x] |
| C22 | `compareN`,`cmpn` | random pairs × `n ∈ {0,1,2,len-1,len,len+1,63,64,2147483647,2147483648,-1,-5,abc}` | [x] |
| C23 | `startswith` | prefix / equal / longer-than-string / differing / high-byte prefixes | [x] |
| C24 | `match` | 1..9 candidates: exact match, substring, non-match, pattern longer than candidate, repeated candidates (counts) | [x] |
| C25 | *all commands* | token count sweep 0..11 per command (`MAX_ARGS` truncation: the 11th+ token is dropped) | [x] |
| C26 | *all commands* | token lengths `1,31,32,33,62,63,64,80` (63-byte `strncpy` truncation) in every argument position | [x] |
| C27 | *program* | line lengths `1,100,254,255,256,257,300,600` → `fgets` splits at 255 and the tail becomes the next command | [x] |
| C28 | `compare`,`set`,`adduser` | tokens of arbitrary bytes `0x01..0xff` (never `' '`,`'\t'`,`'\n'`,NUL) — `unsigned char` comparison semantics | [x] |
| C29 | *program* | embedded NUL byte in a line (`strcspn`), `'\r'`-terminated (CRLF) lines, NUL as the first byte | [x] |
| C30 | `match`,`adduser` | 10, 11, 12 tokens: `MAX_ARGS` cut-off (the loop consumes the 11th token then stops) | [x] |
| C31 | `adduser`,`listusers`,`whoami`,`status` | **name overflow**: names of 32..63 bytes on slots 0..8 → `name[32]` overruns into `password`, then the password `strcpy` rewrites it; read back through every printer | [x] |
| C32 | `adduser`,`login`,`listusers` | **password overflow**: passwords of 32..39 bytes on slots 0..8 → overruns into `users[i+1].name`/`permission_level`; log in with the resulting artifact strings | [x] |
| C33 | `adduser` (slot 9) | **last-slot overflow into `user_count`**: password length exactly 40 (count zeroed → 1) and 41 with final byte 1..224 (count = that byte); then `listusers`/`status`/`login`/`adduser` on the corrupted state | [x] |
| C34 | `createfile`,`listfiles`,`readfile` | **owner overflow**: owner name 33..63 bytes on files 0..18 → `owner[32]` overruns into `permissions` and `files[i+1].filename`/`content`; read back | [x] |
| C35 | `createfile` (slot 19) | **last-slot overflow into `file_count`**: owner string length exactly 36 (count zeroed → 1) / 37..40 (count = owner bytes) | [x] |
| C36 | `createfile`,`readfile` | filename exactly 63 bytes (fills `filename[64]`), content exactly 63 bytes, empty-ish content | [x] |
| C37 | `set`,`get`,`listvars` | variable name 32..63 bytes → `name[32]` overruns into `value[128]`, which the following `strcpy` then rewrites; read back via `get` using the artifact name | [x] |
| C38 | `exit`,`quit` | both aliases: `Goodbye!`, flush of buffered stdout, exit code 0, trailing input ignored; also `exit` as the very first command | [x] |
| C39 | *program* | total output crossing 4096-byte stdio blocks (repeated `help`/`listusers`) then EOF | [x] |
| C40 | *program* | > 4096 bytes of output followed by a `SIGSEGV` (C33/C35 shapes): only whole 4096-byte blocks survive | [x] |
| C41 | `time` | `TZ=UTC`, `LC_ALL=C`; `ctime` line shape (length-checked, value normalized) | [x] |
| C42 | `process_command` | every prefix branch (`add*`, `log*`, `list*`, `create*`, `read*`, `write*`, `delete*`) and unknown commands interleaved with valid ones (state must survive) | [x] |
| C43 | *all aliases* | alias-equivalence sweep: for identical state, `users≡listusers`, `touch≡createfile`, `cat≡readfile`, `write≡writefile`, `rm≡deletefile`, `ls≡listfiles`, `vars≡listvars`, `cmp≡compare`, `cmpn≡compareN`, `?≡help`, `quit≡exit` | [x] |
| C44 | *all commands* | randomized command soup (uniform over the whole command+alias table, random token counts/lengths, random modes), 12 seeds × ~60 commands | [x] |
| C45 | *all commands* | randomized soup restricted to long tokens (32..63 bytes) so overflows interact across all three tables, 12 seeds | [x] |
| C46 | *all commands* | mode cross-product: every command executed once under each of the 4 `debug`/`verbose` combinations | [x] |
| C47 | `deletefile` | 20 files then repeated random-index deletions until empty (repeated struct-shifting), interleaved `ls`/`cat` | [x] |
| C48 | `unset` | 20 variables then repeated random-index unsets until empty, interleaved `listvars`/`get` | [x] |
| C49 | `adduser`,`login` | 63-byte name **and** 63-byte password (max truncation + overflow together) on slots 0..8, then login with the artifact name/password | [x] |
| C50 | `login`,`logout`,`whoami`,`createfile` | login state built on an overflow-corrupted `logged_in`/`current_user` (still inside the mapping): `logout`, `whoami`, `createfile` on that state | [x] |
| C51 | *program* | stdio buffering mode: stdout/stdin as pipe vs pseudo-terminal (all 4 combinations), with and without a `SIGSEGV` — line buffering makes far more output survive a crash, and glibc flushes line-buffered stdout before reading from a tty stdin but does *not* flush a fully buffered one | [x] |
