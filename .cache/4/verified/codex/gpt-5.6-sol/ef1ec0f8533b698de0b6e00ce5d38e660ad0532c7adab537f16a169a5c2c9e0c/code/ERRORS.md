# Error Surface

Each row is one explicit rejection branch in `c_src/src/main.c`. The C API has
no enums, assertions, error-return macros, return-code errors, or explicit null
checks. Its command handlers report rejection through exact stdout bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `cmd_adduser` | `arg_count < 2` | `Usage: adduser <username> <password> [permission_level]\n` | [x] |
| 2 | `cmd_adduser` | `user_count >= MAX_USERS` (10) | `Error: Maximum users reached\n` | [x] |
| 3 | `cmd_adduser` | username equals an existing username | `Error: User '<name>' already exists\n` | [x] |
| 4 | `cmd_login` | `arg_count < 2` | `Usage: login <username> <password>\n` | [x] |
| 5 | `cmd_login` | `current_user && current_user->logged_in` | `Error: User '<name>' already logged in. Use 'logout' first.\n` | [x] |
| 6 | `cmd_login` | username exists but password differs | `Error: Incorrect password\n` | [x] |
| 7 | `cmd_login` | no existing username matches | `Error: User not found\n` | [x] |
| 8 | `cmd_logout` | no current user or current user is not logged in | `Error: No user logged in\n` | [x] |
| 9 | `cmd_whoami` | no current user or current user is not logged in | `Not logged in\n` | [x] |
| 10 | `cmd_listusers` | `user_count == 0` | `No users registered\n` | [x] |
| 11 | `cmd_createfile` | no current logged-in user | `Error: Must be logged in\n` | [x] |
| 12 | `cmd_createfile` | logged in and `arg_count < 1` | `Usage: createfile <filename> [content]\n` | [x] |
| 13 | `cmd_createfile` | logged in and `file_count >= MAX_FILES` (20) | `Error: Maximum files reached\n` | [x] |
| 14 | `cmd_createfile` | filename equals an existing filename | `Error: File '<name>' already exists\n` | [x] |
| 15 | `cmd_readfile` | `arg_count < 1` | `Usage: readfile <filename>\n` | [x] |
| 16 | `cmd_readfile` | no existing filename matches | `Error: File '<name>' not found\n` | [x] |
| 17 | `cmd_writefile` | no current logged-in user | `Error: Must be logged in\n` | [x] |
| 18 | `cmd_writefile` | logged in and `arg_count < 2` | `Usage: writefile <filename> <content>\n` | [x] |
| 19 | `cmd_writefile` | file exists but caller is neither owner nor level 5+ | `Error: Permission denied\n` | [x] |
| 20 | `cmd_writefile` | no existing filename matches | `Error: File '<name>' not found\n` | [x] |
| 21 | `cmd_deletefile` | no current logged-in user | `Error: Must be logged in\n` | [x] |
| 22 | `cmd_deletefile` | logged in and `arg_count < 1` | `Usage: deletefile <filename>\n` | [x] |
| 23 | `cmd_deletefile` | file exists but caller is neither owner nor level 9+ | `Error: Permission denied\n` | [x] |
| 24 | `cmd_deletefile` | no existing filename matches | `Error: File '<name>' not found\n` | [x] |
| 25 | `cmd_listfiles` | `file_count == 0` | `No files\n` | [x] |
| 26 | `cmd_set` | `arg_count < 2` | `Usage: set <name> <value>\n` | [x] |
| 27 | `cmd_set` | new name and `variable_count >= MAX_VARIABLES` (20) | `Error: Maximum variables reached\n` | [x] |
| 28 | `cmd_get` | `arg_count < 1` | `Usage: get <name>\n` | [x] |
| 29 | `cmd_get` | no existing variable name matches | `Error: Variable '<name>' not found\n` | [x] |
| 30 | `cmd_unset` | `arg_count < 1` | `Usage: unset <name>\n` | [x] |
| 31 | `cmd_unset` | no existing variable name matches | `Error: Variable '<name>' not found\n` | [x] |
| 32 | `cmd_listvars` | `variable_count == 0` | `No variables set\n` | [x] |
| 33 | `cmd_compare` | `arg_count < 2` | `Usage: compare <string1> <string2>\n` | [x] |
| 34 | `cmd_compareN` | `arg_count < 3` | `Usage: compareN <string1> <string2> <n>\n` | [x] |
| 35 | `cmd_startswith` | `arg_count < 2` | `Usage: startswith <string> <prefix>\n` | [x] |
| 36 | `cmd_match` | `arg_count < 2` | `Usage: match <pattern> <string1> [string2] ...\n` | [x] |
| 37 | `cmd_debug` | argument is neither `on` nor `off` | `Usage: debug [on|off]\n` | [x] |
| 38 | `cmd_verbose` | argument is neither `on` nor `off` | `Usage: verbose [on|off]\n` | [x] |
| 39 | `process_command` | command matches no exact route or partial-prefix route | `Unknown command: '<command>'. Type 'help' for available commands.\n` | [x] |
| 40 | `main` | `fgets(input, MAX_INPUT, stdin) == NULL` (EOF/read failure) | stop loop and return `0` | [x] |
