# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no build
options or preprocessor definitions. There is exactly one valid feature
combination:

| # | Cargo feature set | C configuration | tested |
|---|-------------------|-----------------|-----|
| 1 | empty set (`--no-default-features`) | default CMake configuration | [x] |

## Runtime Configurations

Rows are derived from the branches, aliases, limits, and public entry points in
`c_src/src/main.c`. Error-only branches are cross-referenced to `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|-----|
| 1 | `parse_command` | empty or all-space/tab input: zero arguments and command storage unchanged | [x] |
| 2 | `parse_command` | one command token, no arguments | [x] |
| 3 | `parse_command` | mixed spaces/tabs, 1-10 arguments | [x] |
| 4 | `parse_command` | more than 10 arguments: retain first 10 | [x] |
| 5 | `parse_command` | command/argument tokens longer than 63 bytes: truncate each to 63 | [x] |
| 6 | `parse_command` | input longer than 255 bytes: tokenize only first 255 | [x] |
| 7 | `cmd_adduser` | new user with omitted permission: level defaults to 1 | [x] |
| 8 | `cmd_adduser` | new user with explicit numeric/non-numeric permission parsed by `atoi` | [x] |
| 9 | `cmd_login`, `cmd_logout`, `cmd_whoami` | valid credentials; observe logged-in state, then logout | [x] |
| 10 | `cmd_listusers` | one and many users; logged-out and logged-in display forms | [x] |
| 11 | `cmd_createfile` | logged-in owner; filename only gives empty content | [x] |
| 12 | `cmd_createfile` | logged-in owner; filename plus content | [x] |
| 13 | `cmd_readfile` | existing file: filename, owner, permissions 755, and content | [x] |
| 14 | `cmd_writefile` | owner updates existing file | [x] |
| 15 | `cmd_writefile` | non-owner with level exactly 5 updates existing file | [x] |
| 16 | `cmd_deletefile` | owner deletes existing file | [x] |
| 17 | `cmd_deletefile` | non-owner with level exactly 9 deletes existing file | [x] |
| 18 | `cmd_deletefile`, `cmd_listfiles` | delete first/middle file and verify remaining files shift in order | [x] |
| 19 | `cmd_listfiles` | one and many files | [x] |
| 20 | `cmd_set` | create new variable | [x] |
| 21 | `cmd_set` | update existing variable without increasing count | [x] |
| 22 | `cmd_get` | retrieve existing variable | [x] |
| 23 | `cmd_unset` | remove first/middle variable and shift remaining order | [x] |
| 24 | `cmd_listvars` | one and many variables | [x] |
| 25 | `cmd_compare` | equal strings | [x] |
| 26 | `cmd_compare` | first string lexicographically less | [x] |
| 27 | `cmd_compare` | first string lexicographically greater | [x] |
| 28 | `cmd_compareN` | `n == 0` | [x] |
| 29 | `cmd_compareN` | equal within first `n`, including `n` beyond both lengths | [x] |
| 30 | `cmd_compareN` | less within first `n` | [x] |
| 31 | `cmd_compareN` | greater within first `n` | [x] |
| 32 | `cmd_compareN` | negative `atoi` result converted to C `size_t` | [x] |
| 33 | `cmd_startswith` | prefix matches | [x] |
| 34 | `cmd_startswith` | prefix does not match | [x] |
| 35 | `cmd_startswith` | empty prefix through direct low-level API | [x] |
| 36 | `cmd_match` | exact match | [x] |
| 37 | `cmd_match` | substring match | [x] |
| 38 | `cmd_match` | no match | [x] |
| 39 | `cmd_match` | 2-10 candidate strings with mixed match classes | [x] |
| 40 | `cmd_help` | fixed help output | [x] |
| 41 | `cmd_debug` | query while off, turn on, query while on, turn off | [x] |
| 42 | `cmd_verbose` | query while off, turn on, query while on, turn off | [x] |
| 43 | `cmd_status` | empty state with both modes off | [x] |
| 44 | `cmd_status` | populated users/files/variables, current user, and modes on | [x] |
| 45 | `cmd_time` | current local-time text | [x] |
| 46 | `process_command` | leading/trailing repeated spaces and tabs around a nonempty command | [x] |
| 47 | `process_command` | exact user routes: `adduser`, `login`, `logout`, `whoami`, `listusers` | [x] |
| 48 | `process_command` | user alias route: `users` | [x] |
| 49 | `process_command` | exact file routes: `createfile`, `readfile`, `writefile`, `deletefile`, `listfiles` | [x] |
| 50 | `process_command` | file aliases: `touch`, `cat`, `write`, `rm`, `ls` | [x] |
| 51 | `process_command` | exact variable routes: `set`, `get`, `unset`, `listvars` | [x] |
| 52 | `process_command` | variable alias route: `vars` | [x] |
| 53 | `process_command` | exact string routes: `compare`, `compareN`, `startswith`, `match` | [x] |
| 54 | `process_command` | string aliases: `cmp`, `cmpn` | [x] |
| 55 | `process_command` | system routes: `debug`, `verbose`, `status`, `time`, `help` | [x] |
| 56 | `process_command` | help alias `?` | [x] |
| 57 | `process_command` | debug mode on prepends command/argument trace | [x] |
| 58 | `process_command` | partial prefix `add*` suggestion | [x] |
| 59 | `process_command` | partial prefix `log*` suggestion | [x] |
| 60 | `process_command` | partial prefix `list*` suggestion | [x] |
| 61 | `process_command` | partial prefix `create*` suggestion | [x] |
| 62 | `process_command` | partial prefix `read*` suggestion | [x] |
| 63 | `process_command` | partial prefix `write*` suggestion | [x] |
| 64 | `process_command` | partial prefix `delete*` suggestion | [x] |
| 65 | `process_command` | `exit` and `quit` print goodbye and terminate with status 0 | [x] |
| 66 | `main` | EOF immediately after startup | [x] |
| 67 | `main` | one/many input lines, newline removal, and 255-byte `fgets` chunks | [x] |
| 68 | `main` | verbose mode prints preprocessing trace before dispatch | [x] |
| 69 | `cmd_match` | empty pattern through direct low-level API: exact empty candidates and substring matches | [x] |
