# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Derived mechanically from `c_src/src/main.c`. The program has **no** `assert`,
**no** `errno` use, **no** `return -1` / `return NULL` (every `cmd_*` returns
`void`) and **no** error enum: every rejection is an early `printf(...); return;`
or a fall-through message, and the two hard failure modes are `exit(0)` and
death by `SIGSEGV` from its own buffer overruns. `grep -c '#if' main.c` = 0.

Rejection constants: `MAX_INPUT 256`, `MAX_COMMAND 64`, `MAX_ARGS 10`,
`MAX_FILES 20`, `MAX_USERS 10`, `MAX_VARIABLES 20`, write-permission threshold
`>= 5`, delete-permission threshold `>= 9`.

"expected C result" is the exact byte sequence printed on stdout (all output goes
to stdout; stderr is always empty; the process exit status is 0 unless stated).
Every row is covered by a differential test in `tests/errors.rs`
(`E01`…`E62`); a row is checked off only when that test passes against both
binaries.

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| E01 | `cmd_adduser` | `arg_count < 2` (`adduser`, `adduser u`) | `Usage: adduser <username> <password> [permission_level]\n` | [x] |
| E02 | `cmd_adduser` | `user_count >= MAX_USERS` (11th user) — checked **before** the duplicate scan | `Error: Maximum users reached\n` | [x] |
| E03 | `cmd_adduser` | `strcmp(users[i].name, args[0]) == 0` (duplicate name, table not full) | `Error: User '<name>' already exists\n` | [x] |
| E04 | `cmd_adduser` | full table **and** duplicate name → E02 wins (ordering) | `Error: Maximum users reached\n` | [x] |
| E05 | `cmd_login` | `arg_count < 2` (`login`, `login u`) | `Usage: login <username> <password>\n` | [x] |
| E06 | `cmd_login` | `current_user && current_user->logged_in` — checked **before** the user scan, so it fires even for a nonexistent user | `Error: User '<cur>' already logged in. Use 'logout' first.\n` | [x] |
| E07 | `cmd_login` | name matches, `strcmp(users[i].password, args[1]) != 0` | `Error: Incorrect password\n` | [x] |
| E08 | `cmd_login` | no `users[i].name` matches (incl. empty user table) | `Error: User not found\n` | [x] |
| E09 | `cmd_login` | name matches an **earlier** duplicate-by-overflow entry: first match wins, later entries never consulted | first-match behaviour (E07/`Login successful`) | [x] |
| E10 | `cmd_logout` | `!current_user` (never logged in) | `Error: No user logged in\n` | [x] |
| E11 | `cmd_logout` | `current_user` set but `!current_user->logged_in` (after a previous logout the pointer is NULLed, so reachable only via overflow-cleared `logged_in`) | `Error: No user logged in\n` | [x] |
| E12 | `cmd_whoami` | not logged in | `Not logged in\n` | [x] |
| E13 | `cmd_listusers` | `user_count == 0` | `No users registered\n` | [x] |
| E14 | `cmd_createfile` | not logged in — checked **before** the arg-count check | `Error: Must be logged in\n` | [x] |
| E15 | `cmd_createfile` | logged in, `arg_count < 1` | `Usage: createfile <filename> [content]\n` | [x] |
| E16 | `cmd_createfile` | `file_count >= MAX_FILES` (21st file) — before the duplicate scan | `Error: Maximum files reached\n` | [x] |
| E17 | `cmd_createfile` | duplicate filename | `Error: File '<name>' already exists\n` | [x] |
| E18 | `cmd_createfile` | full table **and** duplicate name → E16 wins (ordering) | `Error: Maximum files reached\n` | [x] |
| E19 | `cmd_readfile` | `arg_count < 1` | `Usage: readfile <filename>\n` | [x] |
| E20 | `cmd_readfile` | filename not among `files[0..file_count]` (incl. empty table) | `Error: File '<name>' not found\n` | [x] |
| E21 | `cmd_readfile` | **no** login check exists — reading while logged out must succeed | full `=== … ===` dump | [x] |
| E22 | `cmd_writefile` | not logged in — before arg check | `Error: Must be logged in\n` | [x] |
| E23 | `cmd_writefile` | `arg_count < 2` (`writefile`, `writefile f`) | `Usage: writefile <filename> <content>\n` | [x] |
| E24 | `cmd_writefile` | file found, `strcmp(owner, cur) != 0` **and** `permission_level < 5` (0,1,4 and negatives) | `Error: Permission denied\n` | [x] |
| E25 | `cmd_writefile` | file not found (checked after the loop, so a missing file never reports "permission denied") | `Error: File '<name>' not found\n` | [x] |
| E26 | `cmd_writefile` | boundary: non-owner with `permission_level == 5` is allowed, `== 4` is not | `File '<n>' updated\n` / `Error: Permission denied\n` | [x] |
| E27 | `cmd_deletefile` | not logged in — before arg check | `Error: Must be logged in\n` | [x] |
| E28 | `cmd_deletefile` | `arg_count < 1` | `Usage: deletefile <filename>\n` | [x] |
| E29 | `cmd_deletefile` | non-owner with `permission_level < 9` (incl. 5..8, which *may* write but not delete) | `Error: Permission denied\n` | [x] |
| E30 | `cmd_deletefile` | file not found | `Error: File '<name>' not found\n` | [x] |
| E31 | `cmd_deletefile` | boundary: non-owner with `permission_level == 9` allowed, `== 8` denied | `File '<n>' deleted\n` / `Error: Permission denied\n` | [x] |
| E32 | `cmd_listfiles` | `file_count == 0` | `No files\n` | [x] |
| E33 | `cmd_set` | `arg_count < 2` (`set`, `set k`) | `Usage: set <name> <value>\n` | [x] |
| E34 | `cmd_set` | `variable_count >= MAX_VARIABLES` **and** name not already present (check is *after* the update scan) | `Error: Maximum variables reached\n` | [x] |
| E35 | `cmd_set` | table full **but** name already present → update succeeds (no rejection) | `Variable '<n>' updated\n` | [x] |
| E36 | `cmd_get` | `arg_count < 1` | `Usage: get <name>\n` | [x] |
| E37 | `cmd_get` | name not found (incl. empty table) | `Error: Variable '<name>' not found\n` | [x] |
| E38 | `cmd_unset` | `arg_count < 1` | `Usage: unset <name>\n` | [x] |
| E39 | `cmd_unset` | name not found | `Error: Variable '<name>' not found\n` | [x] |
| E40 | `cmd_listvars` | `variable_count == 0` | `No variables set\n` | [x] |
| E41 | `cmd_compare` | `arg_count < 2` | `Usage: compare <string1> <string2>\n` | [x] |
| E42 | `cmd_compareN` | `arg_count < 3` | `Usage: compareN <string1> <string2> <n>\n` | [x] |
| E43 | `cmd_compareN` | `n` non-numeric → `atoi` yields 0 → `strncmp(...,0) == 0` | `strncmp('a','b', 0) = 0\nFirst 0 characters are equal\n` | [x] |
| E44 | `cmd_compareN` | `n` negative → `(size_t)(-1)` (sign extension), compares to the first difference/NUL | `strncmp('a','b', -1) = -1\n'a' < 'b' (first -1 chars)\n` | [x] |
| E45 | `cmd_compareN` | `n` past `INT_MAX` / `LONG_MAX` → `atoi` saturation then truncation (`2147483648`→`-2147483648`, `99999999999999999999`→`-1`) | printed `%d` is the truncated value | [x] |
| E46 | `cmd_startswith` | `arg_count < 2` | `Usage: startswith <string> <prefix>\n` | [x] |
| E47 | `cmd_match` | `arg_count < 2` | `Usage: match <pattern> <string1> [string2] ...\n` | [x] |
| E48 | `cmd_debug` | `arg_count == 0` → not an error, prints state | `Debug mode: OFF\n` / `Debug mode: ON\n` | [x] |
| E49 | `cmd_debug` | `args[0]` is neither `on` nor `off` (incl. `ON`, `1`, `onx`) | `Usage: debug [on\|off]\n` | [x] |
| E50 | `cmd_verbose` | `arg_count == 0` → prints state | `Verbose mode: OFF\n` / `Verbose mode: ON\n` | [x] |
| E51 | `cmd_verbose` | `args[0]` neither `on` nor `off` | `Usage: verbose [on\|off]\n` | [x] |
| E52 | `process_command` | `strlen(command) == 0` (empty line, only spaces/tabs) → silent, **no** `[DEBUG]`/prompt output beyond `> ` | nothing | [x] |
| E53 | `process_command` | unknown command starting with `add` (`strncmp(...,3)`) | `Did you mean 'adduser'?\n` | [x] |
| E54 | `process_command` | unknown command starting with `log` | `Did you mean 'login' or 'logout'?\n` | [x] |
| E55 | `process_command` | unknown command starting with `list` (4) | `Did you mean 'listusers', 'listfiles', or 'listvars'?\n` | [x] |
| E56 | `process_command` | unknown command starting with `create` (6) / `read` (4) / `write` (5) / `delete` (6) | matching `Did you mean …?\n` | [x] |
| E57 | `process_command` | any other unknown command, incl. shorter-than-prefix (`ad`, `lo`, `lis`, `creat`, `writ`, `delet`) and non-ASCII | `Unknown command: '<cmd>'. Type 'help' for available commands.\n` | [x] |
| E58 | `process_command` | prefix rules are checked **after** every exact match, so `addusers`→`add…`, `logs`→`log…`, `lister`→`list…` | the "Did you mean" branch, never "Unknown command" | [x] |
| E59 | `main` | `fgets` returns NULL (EOF, empty stdin, closed pipe) | trailing `> ` then exit status 0 | [x] |
| E60 | `parse_command` | token longer than `MAX_COMMAND-1` = 63 → silently truncated by `strncpy` (both command and args) | truncated token echoed | [x] |
| E61 | `main` | input line longer than `MAX_INPUT-1` = 255 → `fgets` splits it; the tail is processed as the next command (extra `> `) | two command executions | [x] |
| E62 | `main` | embedded `'\0'` in a line → `strcspn` stops there, the rest of the line is dropped but consumed (so `compare ab<NUL>cd ab` becomes the one-argument `compare ab`) | command truncated at the NUL | [x] |

## Undefined-behaviour rejections (no explicit check in the C — verified empirically)

These are the "one step past the valid range" paths of this program: the C never
range-checks the `strcpy` destinations, so out-of-range input is "rejected" by
memory corruption or `SIGSEGV`. All rows verified against the reference binary
and reproduced by the translation (`tests/errors.rs`, `U01`…`U11`).

Note that every `users[user_count]` / `files[file_count]` /
`variables[variable_count]` subscript in `main.c` re-loads the counter from
memory (the reference build is `-O0`), so a `strcpy` that clobbers the counter
*mid-statement* redirects the following writes of the same function — that is
where most of the rows below come from.

| # | function | trigger | expected C result | [x] |
|---|----------|---------|-------------------|-----|
| U01 | `cmd_adduser` | name 32..63 bytes → `strcpy` overruns `name[32]` into `password[32]`, then the password `strcpy` overwrites it | name renders as `name[0..32] + password` | [x] |
| U02 | `cmd_adduser` | password 32..39 bytes on `users[i<9]` → overruns into `users[i+1].name` | later `adduser` overwrites, `listusers` shows the artifact | [x] |
| U03 | `cmd_adduser` | 10th user (`users[9]`), password **exactly 40** bytes → the NUL lands on `user_count` byte 0 → count becomes 0, so `permission_level`/`logged_in` are written to `users[0]`, then `user_count++` → 1 | `Users: 1/10`, `listusers` shows `u0` with the *new* level | [x] |
| U04 | `cmd_adduser` | 10th user, password **41** bytes, last byte `b` ∈ 1..224 → `user_count = b`, the `users[b].permission_level` write still lands inside the mapping, count becomes `b+1` | `Users: <b+1>/10` + `b+1` rows of `.bss` garbage | [x] |
| U05 | `cmd_adduser` | 10th user, password 41 bytes with last byte ≥ 225, or password ≥ 42 bytes (bytes 40..43 are non-NUL, so `user_count` ≥ 0x01010101) → the `users[user_count]` int write leaves the RW mapping | **killed by `SIGSEGV`**, buffered stdout lost (only whole 4096-byte blocks survive) | [x] |
| U06 | `cmd_createfile` | 20th file (`files[19]`) created by a user whose *name string* is ≥ 36 bytes → the owner `strcpy` overruns into `file_count`; length 36 zeroes it (→ 1 after `++`), length 37 with a small byte gives in-range garbage, length ≥ 37 with printable bytes → out-of-range write | count reset / garbage rows / `SIGSEGV` | [x] |
| U07 | `cmd_listvars`, `cmd_get`, `cmd_set`, `cmd_unset` | `variable_count` clobbered by `users[224].permission_level` (same address, reached via U04 with `b = 224`): value ≥ 21 makes `listvars` read `variables[20].value` past the mapping, ≥ 22 makes the name-only scans walk off | `listvars` dies at ≥ 21, `get`/`set`/`unset` at ≥ 22, all **`SIGSEGV`**; ≤ 20 survives with garbage rows | [x] |
| U08 | `cmd_adduser` | 10th user, password 44 bytes with bytes 40..43 = `0xff` → `user_count` = −1, but the name/password `strcpy`s already used the old count and the two `int` writes land in the 24-byte padding below `users` | **survives**: `Users: 0/10` after the `++`, exit status 0 | [x] |
| U09 | `cmd_adduser` | `user_count` left at −1 by the previous command (bytes 40..43 = `fe ff ff ff` → −2, `++` → −1) → the *next* `adduser` `strcpy`s into `users[-1]`, i.e. `.got.plt` | **killed by `SIGSEGV`** at the next libc call | [x] |
| U10 | `cmd_adduser` | same, but `user_count` = −4 (`fb ff ff ff`) → `users[-4]` lies in `.dynamic`/`.fini_array` | **killed by `SIGSEGV`**, buffered output lost | [x] |
| U11 | `cmd_deletefile` | `file_count` corrupted to 24/25 (U06 with a control byte) → the `files[j] = files[j+1]` shift loop copies garbage but never leaves the mapping | **survives**, exit status 0, garbage listing | [x] |

## Not applicable

* **NULL pointers / oversized lengths across an FFI boundary** — there is no
  library API: every entry point is a text command on stdin, and all C
  parameters are stack arrays (`char args[10][64]`), never caller pointers.
  The equivalent generic boundaries *are* covered: empty input (E52), EOF (E59),
  oversized tokens (E60), oversized lines (E61), embedded NUL (E62).
* **Out-of-range enum values** — `main.c` declares no enum; the equivalent
  "integer with no valid variant" inputs are the `atoi`-parsed permission level
  and `compareN` count, covered by E43/E44/E45 and CONFIGS rows C10/C22
  (negative, 0, boundary, `INT_MAX`, `INT_MAX+1`, `LONG_MAX+1`, non-numeric,
  leading `+`/`-`/whitespace).
