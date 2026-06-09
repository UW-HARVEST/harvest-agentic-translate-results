// Translated from c_src/src/main.c
// Library exposing the C-compatible command interpreter.

use std::ffi::{c_char, c_int, c_void};

const MAX_INPUT: usize = 256;
const MAX_COMMAND: usize = 64;
const MAX_ARGS: usize = 10;
const MAX_FILES: usize = 20;
const MAX_USERS: usize = 10;
const MAX_VARIABLES: usize = 20;

#[repr(C)]
#[derive(Copy, Clone)]
struct UserT {
    name: [c_char; 32],
    password: [c_char; 32],
    permission_level: c_int,
    logged_in: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FileT {
    filename: [c_char; 64],
    content: [c_char; 512],
    owner: [c_char; 32],
    permissions: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct VariableT {
    name: [c_char; 32],
    value: [c_char; 128],
}

// Global state
static mut USERS: [UserT; MAX_USERS] = [UserT {
    name: [0; 32],
    password: [0; 32],
    permission_level: 0,
    logged_in: 0,
}; MAX_USERS];
static mut USER_COUNT: c_int = 0;
static mut CURRENT_USER: *mut UserT = std::ptr::null_mut();

static mut FILES: [FileT; MAX_FILES] = [FileT {
    filename: [0; 64],
    content: [0; 512],
    owner: [0; 32],
    permissions: 0,
}; MAX_FILES];
static mut FILE_COUNT: c_int = 0;

static mut VARIABLES: [VariableT; MAX_VARIABLES] = [VariableT {
    name: [0; 32],
    value: [0; 128],
}; MAX_VARIABLES];
static mut VARIABLE_COUNT: c_int = 0;

static mut DEBUG_MODE: c_int = 0;
static mut VERBOSE_MODE: c_int = 0;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fgets(s: *mut c_char, size: c_int, stream: *mut c_void) -> *mut c_char;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strlen(s: *const c_char) -> usize;
    fn strstr(haystack: *const c_char, needle: *const c_char) -> *mut c_char;
    fn strtok(str: *mut c_char, delim: *const c_char) -> *mut c_char;
    fn strcspn(s: *const c_char, reject: *const c_char) -> usize;
    fn atoi(s: *const c_char) -> c_int;
    fn time(t: *mut i64) -> i64;
    fn ctime(t: *const i64) -> *mut c_char;
    fn exit(status: c_int) -> !;
    static stdin: *mut c_void;
}

// Helper macro to create C string literal
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// Parse command and arguments
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_command(
    input: *const c_char,
    cmd: *mut c_char,
    args: *mut [c_char; MAX_COMMAND],
    arg_count: *mut c_int,
) {
    unsafe {
        let mut temp: [c_char; MAX_INPUT] = [0; MAX_INPUT];
        strncpy(temp.as_mut_ptr(), input, MAX_INPUT - 1);
        temp[MAX_INPUT - 1] = 0;

        *arg_count = 0;
        let delim = cstr!(" \t");
        let mut token = strtok(temp.as_mut_ptr(), delim);

        if !token.is_null() {
            strncpy(cmd, token, MAX_COMMAND - 1);
            *cmd.add(MAX_COMMAND - 1) = 0;

            loop {
                token = strtok(std::ptr::null_mut(), delim);
                if token.is_null() || *arg_count >= MAX_ARGS as c_int {
                    break;
                }
                let arg_slot = args.add(*arg_count as usize);
                strncpy((*arg_slot).as_mut_ptr(), token, MAX_COMMAND - 1);
                (*arg_slot)[MAX_COMMAND - 1] = 0;
                *arg_count += 1;
            }
        }
    }
}

// User management commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_adduser(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 2 {
            printf(cstr!("Usage: adduser <username> <password> [permission_level]\n"));
            return;
        }

        if USER_COUNT >= MAX_USERS as c_int {
            printf(cstr!("Error: Maximum users reached\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();

        // Check if user already exists using strcmp
        for i in 0..USER_COUNT as usize {
            if strcmp(USERS[i].name.as_ptr(), arg0) == 0 {
                printf(cstr!("Error: User '%s' already exists\n"), arg0);
                return;
            }
        }

        let idx = USER_COUNT as usize;
        strcpy(USERS[idx].name.as_mut_ptr(), arg0);
        strcpy(USERS[idx].password.as_mut_ptr(), arg1);
        USERS[idx].permission_level = if arg_count >= 3 {
            atoi((*args.add(2)).as_ptr())
        } else {
            1
        };
        USERS[idx].logged_in = 0;
        USER_COUNT += 1;

        printf(
            cstr!("User '%s' added with permission level %d\n"),
            arg0,
            USERS[(USER_COUNT - 1) as usize].permission_level,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_login(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 2 {
            printf(cstr!("Usage: login <username> <password>\n"));
            return;
        }

        if !CURRENT_USER.is_null() && (*CURRENT_USER).logged_in != 0 {
            printf(
                cstr!("Error: User '%s' already logged in. Use 'logout' first.\n"),
                (*CURRENT_USER).name.as_ptr(),
            );
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();

        // Find user and verify password using strcmp
        for i in 0..USER_COUNT as usize {
            if strcmp(USERS[i].name.as_ptr(), arg0) == 0 {
                if strcmp(USERS[i].password.as_ptr(), arg1) == 0 {
                    USERS[i].logged_in = 1;
                    CURRENT_USER = &raw mut USERS[i];
                    printf(
                        cstr!("Login successful. Welcome, %s!\n"),
                        (*CURRENT_USER).name.as_ptr(),
                    );
                    return;
                } else {
                    printf(cstr!("Error: Incorrect password\n"));
                    return;
                }
            }
        }

        printf(cstr!("Error: User not found\n"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_logout() {
    unsafe {
        if CURRENT_USER.is_null() || (*CURRENT_USER).logged_in == 0 {
            printf(cstr!("Error: No user logged in\n"));
            return;
        }

        printf(cstr!("Goodbye, %s!\n"), (*CURRENT_USER).name.as_ptr());
        (*CURRENT_USER).logged_in = 0;
        CURRENT_USER = std::ptr::null_mut();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_whoami() {
    unsafe {
        if CURRENT_USER.is_null() || (*CURRENT_USER).logged_in == 0 {
            printf(cstr!("Not logged in\n"));
            return;
        }

        printf(cstr!("Current user: %s\n"), (*CURRENT_USER).name.as_ptr());
        printf(
            cstr!("Permission level: %d\n"),
            (*CURRENT_USER).permission_level,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_listusers() {
    unsafe {
        if USER_COUNT == 0 {
            printf(cstr!("No users registered\n"));
            return;
        }

        printf(cstr!("Registered users:\n"));
        for i in 0..USER_COUNT as usize {
            let logged_str = if USERS[i].logged_in != 0 {
                cstr!("[logged in]")
            } else {
                cstr!("")
            };
            printf(
                cstr!("  %s (level %d) %s\n"),
                USERS[i].name.as_ptr(),
                USERS[i].permission_level,
                logged_str,
            );
        }
    }
}

// File management commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_createfile(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if CURRENT_USER.is_null() || (*CURRENT_USER).logged_in == 0 {
            printf(cstr!("Error: Must be logged in\n"));
            return;
        }

        if arg_count < 1 {
            printf(cstr!("Usage: createfile <filename> [content]\n"));
            return;
        }

        if FILE_COUNT >= MAX_FILES as c_int {
            printf(cstr!("Error: Maximum files reached\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        // Check if file exists using strcmp
        for i in 0..FILE_COUNT as usize {
            if strcmp(FILES[i].filename.as_ptr(), arg0) == 0 {
                printf(cstr!("Error: File '%s' already exists\n"), arg0);
                return;
            }
        }

        let idx = FILE_COUNT as usize;
        strcpy(FILES[idx].filename.as_mut_ptr(), arg0);
        strcpy(FILES[idx].owner.as_mut_ptr(), (*CURRENT_USER).name.as_ptr());
        FILES[idx].permissions = 755;

        if arg_count >= 2 {
            strcpy(FILES[idx].content.as_mut_ptr(), (*args.add(1)).as_ptr());
        } else {
            FILES[idx].content[0] = 0;
        }

        FILE_COUNT += 1;
        printf(cstr!("File '%s' created\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_readfile(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 1 {
            printf(cstr!("Usage: readfile <filename>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        // Find file using strcmp
        for i in 0..FILE_COUNT as usize {
            if strcmp(FILES[i].filename.as_ptr(), arg0) == 0 {
                printf(cstr!("=== %s ===\n"), FILES[i].filename.as_ptr());
                printf(cstr!("Owner: %s\n"), FILES[i].owner.as_ptr());
                printf(cstr!("Permissions: %d\n"), FILES[i].permissions);
                printf(cstr!("Content: %s\n"), FILES[i].content.as_ptr());
                return;
            }
        }

        printf(cstr!("Error: File '%s' not found\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_writefile(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if CURRENT_USER.is_null() || (*CURRENT_USER).logged_in == 0 {
            printf(cstr!("Error: Must be logged in\n"));
            return;
        }

        if arg_count < 2 {
            printf(cstr!("Usage: writefile <filename> <content>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();

        // Find file and check ownership
        for i in 0..FILE_COUNT as usize {
            if strcmp(FILES[i].filename.as_ptr(), arg0) == 0 {
                if strcmp(FILES[i].owner.as_ptr(), (*CURRENT_USER).name.as_ptr()) == 0
                    || (*CURRENT_USER).permission_level >= 5
                {
                    strcpy(FILES[i].content.as_mut_ptr(), arg1);
                    printf(cstr!("File '%s' updated\n"), arg0);
                    return;
                } else {
                    printf(cstr!("Error: Permission denied\n"));
                    return;
                }
            }
        }

        printf(cstr!("Error: File '%s' not found\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_deletefile(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if CURRENT_USER.is_null() || (*CURRENT_USER).logged_in == 0 {
            printf(cstr!("Error: Must be logged in\n"));
            return;
        }

        if arg_count < 1 {
            printf(cstr!("Usage: deletefile <filename>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        // Find and delete file
        let mut i = 0usize;
        while i < FILE_COUNT as usize {
            if strcmp(FILES[i].filename.as_ptr(), arg0) == 0 {
                if strcmp(FILES[i].owner.as_ptr(), (*CURRENT_USER).name.as_ptr()) == 0
                    || (*CURRENT_USER).permission_level >= 9
                {
                    // Shift remaining files
                    let mut j = i;
                    while j < (FILE_COUNT - 1) as usize {
                        FILES[j] = FILES[j + 1];
                        j += 1;
                    }
                    FILE_COUNT -= 1;
                    printf(cstr!("File '%s' deleted\n"), arg0);
                    return;
                } else {
                    printf(cstr!("Error: Permission denied\n"));
                    return;
                }
            }
            i += 1;
        }

        printf(cstr!("Error: File '%s' not found\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_listfiles() {
    unsafe {
        if FILE_COUNT == 0 {
            printf(cstr!("No files\n"));
            return;
        }

        printf(cstr!("Files:\n"));
        for i in 0..FILE_COUNT as usize {
            printf(
                cstr!("  %s (owner: %s, perm: %d)\n"),
                FILES[i].filename.as_ptr(),
                FILES[i].owner.as_ptr(),
                FILES[i].permissions,
            );
        }
    }
}

// Variable commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_set(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 2 {
            printf(cstr!("Usage: set <name> <value>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();

        // Check if variable exists
        for i in 0..VARIABLE_COUNT as usize {
            if strcmp(VARIABLES[i].name.as_ptr(), arg0) == 0 {
                strcpy(VARIABLES[i].value.as_mut_ptr(), arg1);
                printf(cstr!("Variable '%s' updated\n"), arg0);
                return;
            }
        }

        // Create new variable
        if VARIABLE_COUNT >= MAX_VARIABLES as c_int {
            printf(cstr!("Error: Maximum variables reached\n"));
            return;
        }

        let idx = VARIABLE_COUNT as usize;
        strcpy(VARIABLES[idx].name.as_mut_ptr(), arg0);
        strcpy(VARIABLES[idx].value.as_mut_ptr(), arg1);
        VARIABLE_COUNT += 1;
        printf(cstr!("Variable '%s' set\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_get(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 1 {
            printf(cstr!("Usage: get <name>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        for i in 0..VARIABLE_COUNT as usize {
            if strcmp(VARIABLES[i].name.as_ptr(), arg0) == 0 {
                printf(
                    cstr!("%s = %s\n"),
                    VARIABLES[i].name.as_ptr(),
                    VARIABLES[i].value.as_ptr(),
                );
                return;
            }
        }

        printf(cstr!("Error: Variable '%s' not found\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_unset(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 1 {
            printf(cstr!("Usage: unset <name>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        for i in 0..VARIABLE_COUNT as usize {
            if strcmp(VARIABLES[i].name.as_ptr(), arg0) == 0 {
                let mut j = i;
                while j < (VARIABLE_COUNT - 1) as usize {
                    VARIABLES[j] = VARIABLES[j + 1];
                    j += 1;
                }
                VARIABLE_COUNT -= 1;
                printf(cstr!("Variable '%s' unset\n"), arg0);
                return;
            }
        }

        printf(cstr!("Error: Variable '%s' not found\n"), arg0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_listvars() {
    unsafe {
        if VARIABLE_COUNT == 0 {
            printf(cstr!("No variables set\n"));
            return;
        }

        printf(cstr!("Variables:\n"));
        for i in 0..VARIABLE_COUNT as usize {
            printf(
                cstr!("  %s = %s\n"),
                VARIABLES[i].name.as_ptr(),
                VARIABLES[i].value.as_ptr(),
            );
        }
    }
}

// String comparison commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_compare(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 2 {
            printf(cstr!("Usage: compare <string1> <string2>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();

        let result = strcmp(arg0, arg1);

        printf(cstr!("strcmp('%s', '%s') = %d\n"), arg0, arg1, result);

        if result == 0 {
            printf(cstr!("Strings are equal\n"));
        } else if result < 0 {
            printf(cstr!("'%s' < '%s'\n"), arg0, arg1);
        } else {
            printf(cstr!("'%s' > '%s'\n"), arg0, arg1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_compareN(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 3 {
            printf(cstr!("Usage: compareN <string1> <string2> <n>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();
        let arg2 = (*args.add(2)).as_ptr();

        let n = atoi(arg2);
        let result = strncmp(arg0, arg1, n as usize);

        printf(
            cstr!("strncmp('%s', '%s', %d) = %d\n"),
            arg0,
            arg1,
            n,
            result,
        );

        if result == 0 {
            printf(cstr!("First %d characters are equal\n"), n);
        } else if result < 0 {
            printf(cstr!("'%s' < '%s' (first %d chars)\n"), arg0, arg1, n);
        } else {
            printf(cstr!("'%s' > '%s' (first %d chars)\n"), arg0, arg1, n);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_startswith(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 2 {
            printf(cstr!("Usage: startswith <string> <prefix>\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();
        let arg1 = (*args.add(1)).as_ptr();

        let prefix_len = strlen(arg1);

        if strncmp(arg0, arg1, prefix_len) == 0 {
            printf(cstr!("'%s' starts with '%s'\n"), arg0, arg1);
        } else {
            printf(cstr!("'%s' does not start with '%s'\n"), arg0, arg1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_match(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 2 {
            printf(cstr!("Usage: match <pattern> <string1> [string2] ...\n"));
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        printf(cstr!("Matching pattern '%s':\n"), arg0);
        let mut matches = 0;

        for i in 1..arg_count as usize {
            let argi = (*args.add(i)).as_ptr();
            if strcmp(arg0, argi) == 0 {
                printf(cstr!("  '%s' - EXACT MATCH\n"), argi);
                matches += 1;
            } else if !strstr(argi, arg0).is_null() {
                printf(cstr!("  '%s' - contains pattern\n"), argi);
                matches += 1;
            } else {
                printf(cstr!("  '%s' - no match\n"), argi);
            }
        }

        printf(cstr!("Total matches: %d\n"), matches);
    }
}

// System commands
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_help() {
    unsafe {
        printf(cstr!("\n=== Command Interpreter Help ===\n"));
        printf(cstr!("User Management:\n"));
        printf(cstr!("  adduser <user> <pass> [level] - Add new user\n"));
        printf(cstr!("  login <user> <pass>            - Login as user\n"));
        printf(cstr!("  logout                         - Logout current user\n"));
        printf(cstr!("  whoami                         - Show current user\n"));
        printf(cstr!("  listusers                      - List all users\n"));
        printf(cstr!("\nFile Management:\n"));
        printf(cstr!("  createfile <name> [content]    - Create file\n"));
        printf(cstr!("  readfile <name>                - Read file\n"));
        printf(cstr!("  writefile <name> <content>     - Write to file\n"));
        printf(cstr!("  deletefile <name>              - Delete file\n"));
        printf(cstr!("  listfiles                      - List all files\n"));
        printf(cstr!("\nVariable Management:\n"));
        printf(cstr!("  set <name> <value>             - Set variable\n"));
        printf(cstr!("  get <name>                     - Get variable\n"));
        printf(cstr!("  unset <name>                   - Unset variable\n"));
        printf(cstr!("  listvars                       - List all variables\n"));
        printf(cstr!("\nString Operations:\n"));
        printf(cstr!("  compare <str1> <str2>          - Compare strings\n"));
        printf(cstr!("  compareN <str1> <str2> <n>     - Compare first N chars\n"));
        printf(cstr!("  startswith <str> <prefix>      - Check if starts with\n"));
        printf(cstr!("  match <pattern> <str> ...      - Match pattern\n"));
        printf(cstr!("\nSystem:\n"));
        printf(cstr!("  debug [on|off]                 - Toggle debug mode\n"));
        printf(cstr!("  verbose [on|off]               - Toggle verbose mode\n"));
        printf(cstr!("  status                         - Show system status\n"));
        printf(cstr!("  time                           - Show current time\n"));
        printf(cstr!("  help                           - Show this help\n"));
        printf(cstr!("  exit                           - Exit program\n"));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_debug(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 1 {
            let mode_str = if DEBUG_MODE != 0 {
                cstr!("ON")
            } else {
                cstr!("OFF")
            };
            printf(cstr!("Debug mode: %s\n"), mode_str);
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        if strcmp(arg0, cstr!("on")) == 0 {
            DEBUG_MODE = 1;
            printf(cstr!("Debug mode enabled\n"));
        } else if strcmp(arg0, cstr!("off")) == 0 {
            DEBUG_MODE = 0;
            printf(cstr!("Debug mode disabled\n"));
        } else {
            printf(cstr!("Usage: debug [on|off]\n"));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_verbose(args: *mut [c_char; MAX_COMMAND], arg_count: c_int) {
    unsafe {
        if arg_count < 1 {
            let mode_str = if VERBOSE_MODE != 0 {
                cstr!("ON")
            } else {
                cstr!("OFF")
            };
            printf(cstr!("Verbose mode: %s\n"), mode_str);
            return;
        }

        let arg0 = (*args.add(0)).as_ptr();

        if strcmp(arg0, cstr!("on")) == 0 {
            VERBOSE_MODE = 1;
            printf(cstr!("Verbose mode enabled\n"));
        } else if strcmp(arg0, cstr!("off")) == 0 {
            VERBOSE_MODE = 0;
            printf(cstr!("Verbose mode disabled\n"));
        } else {
            printf(cstr!("Usage: verbose [on|off]\n"));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_status() {
    unsafe {
        printf(cstr!("\n=== System Status ===\n"));
        printf(cstr!("Users: %d/%d\n"), USER_COUNT, MAX_USERS as c_int);
        printf(cstr!("Files: %d/%d\n"), FILE_COUNT, MAX_FILES as c_int);
        printf(
            cstr!("Variables: %d/%d\n"),
            VARIABLE_COUNT,
            MAX_VARIABLES as c_int,
        );
        let user_str = if !CURRENT_USER.is_null() && (*CURRENT_USER).logged_in != 0 {
            (*CURRENT_USER).name.as_ptr()
        } else {
            cstr!("none")
        };
        printf(cstr!("Current user: %s\n"), user_str);
        let dbg_str = if DEBUG_MODE != 0 {
            cstr!("ON")
        } else {
            cstr!("OFF")
        };
        printf(cstr!("Debug mode: %s\n"), dbg_str);
        let vrb_str = if VERBOSE_MODE != 0 {
            cstr!("ON")
        } else {
            cstr!("OFF")
        };
        printf(cstr!("Verbose mode: %s\n"), vrb_str);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cmd_time() {
    unsafe {
        let now: i64 = time(std::ptr::null_mut());
        let t_ptr: *const i64 = &now;
        printf(cstr!("Current time: %s"), ctime(t_ptr));
    }
}

// Main command processor using extensive strcmp/strncmp
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_command(input: *const c_char) {
    unsafe {
        let mut command: [c_char; MAX_COMMAND] = [0; MAX_COMMAND];
        let mut args: [[c_char; MAX_COMMAND]; MAX_ARGS] = [[0; MAX_COMMAND]; MAX_ARGS];
        let mut arg_count: c_int = 0;

        parse_command(
            input,
            command.as_mut_ptr(),
            args.as_mut_ptr(),
            &mut arg_count,
        );

        if strlen(command.as_ptr()) == 0 {
            return;
        }

        if DEBUG_MODE != 0 {
            printf(
                cstr!("[DEBUG] Command: '%s', Args: %d\n"),
                command.as_ptr(),
                arg_count,
            );
        }

        let cmd_ptr = command.as_ptr();
        let args_ptr = args.as_mut_ptr();

        // Command routing using strcmp and strncmp
        // User commands
        if strcmp(cmd_ptr, cstr!("adduser")) == 0 {
            cmd_adduser(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("login")) == 0 {
            cmd_login(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("logout")) == 0 {
            cmd_logout();
        } else if strcmp(cmd_ptr, cstr!("whoami")) == 0 {
            cmd_whoami();
        } else if strcmp(cmd_ptr, cstr!("listusers")) == 0 || strcmp(cmd_ptr, cstr!("users")) == 0 {
            cmd_listusers();
        }
        // File commands
        else if strcmp(cmd_ptr, cstr!("createfile")) == 0 || strcmp(cmd_ptr, cstr!("touch")) == 0 {
            cmd_createfile(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("readfile")) == 0 || strcmp(cmd_ptr, cstr!("cat")) == 0 {
            cmd_readfile(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("writefile")) == 0 || strcmp(cmd_ptr, cstr!("write")) == 0 {
            cmd_writefile(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("deletefile")) == 0 || strcmp(cmd_ptr, cstr!("rm")) == 0 {
            cmd_deletefile(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("listfiles")) == 0 || strcmp(cmd_ptr, cstr!("ls")) == 0 {
            cmd_listfiles();
        }
        // Variable commands
        else if strcmp(cmd_ptr, cstr!("set")) == 0 {
            cmd_set(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("get")) == 0 {
            cmd_get(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("unset")) == 0 {
            cmd_unset(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("listvars")) == 0 || strcmp(cmd_ptr, cstr!("vars")) == 0 {
            cmd_listvars();
        }
        // String comparison commands
        else if strcmp(cmd_ptr, cstr!("compare")) == 0 || strcmp(cmd_ptr, cstr!("cmp")) == 0 {
            cmd_compare(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("compareN")) == 0 || strcmp(cmd_ptr, cstr!("cmpn")) == 0 {
            cmd_compareN(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("startswith")) == 0 {
            cmd_startswith(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("match")) == 0 {
            cmd_match(args_ptr, arg_count);
        }
        // System commands
        else if strcmp(cmd_ptr, cstr!("debug")) == 0 {
            cmd_debug(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("verbose")) == 0 {
            cmd_verbose(args_ptr, arg_count);
        } else if strcmp(cmd_ptr, cstr!("status")) == 0 {
            cmd_status();
        } else if strcmp(cmd_ptr, cstr!("time")) == 0 {
            cmd_time();
        } else if strcmp(cmd_ptr, cstr!("help")) == 0 || strcmp(cmd_ptr, cstr!("?")) == 0 {
            cmd_help();
        } else if strcmp(cmd_ptr, cstr!("exit")) == 0 || strcmp(cmd_ptr, cstr!("quit")) == 0 {
            printf(cstr!("Goodbye!\n"));
            exit(0);
        }
        // Check for partial matches using strncmp
        else if strncmp(cmd_ptr, cstr!("add"), 3) == 0 {
            printf(cstr!("Did you mean 'adduser'?\n"));
        } else if strncmp(cmd_ptr, cstr!("log"), 3) == 0 {
            printf(cstr!("Did you mean 'login' or 'logout'?\n"));
        } else if strncmp(cmd_ptr, cstr!("list"), 4) == 0 {
            printf(cstr!("Did you mean 'listusers', 'listfiles', or 'listvars'?\n"));
        } else if strncmp(cmd_ptr, cstr!("create"), 6) == 0 {
            printf(cstr!("Did you mean 'createfile'?\n"));
        } else if strncmp(cmd_ptr, cstr!("read"), 4) == 0 {
            printf(cstr!("Did you mean 'readfile'?\n"));
        } else if strncmp(cmd_ptr, cstr!("write"), 5) == 0 {
            printf(cstr!("Did you mean 'writefile'?\n"));
        } else if strncmp(cmd_ptr, cstr!("delete"), 6) == 0 {
            printf(cstr!("Did you mean 'deletefile'?\n"));
        } else {
            printf(
                cstr!("Unknown command: '%s'. Type 'help' for available commands.\n"),
                cmd_ptr,
            );
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main() -> c_int {
    unsafe {
        printf(cstr!("|----------------------------------------|\n"));
        printf(cstr!("|   COMMAND INTERPRETER                  |\n"));
        printf(cstr!("|   strcmp/strncmp demonstration         |\n"));
        printf(cstr!("|----------------------------------------|\n"));
        printf(cstr!("Type 'help' for available commands\n\n"));

        let mut input: [c_char; MAX_INPUT] = [0; MAX_INPUT];

        loop {
            printf(cstr!("> "));

            if fgets(input.as_mut_ptr(), MAX_INPUT as c_int, stdin).is_null() {
                break;
            }

            // Remove newline
            let nl_idx = strcspn(input.as_ptr(), cstr!("\n"));
            input[nl_idx] = 0;

            if VERBOSE_MODE != 0 {
                printf(cstr!("[VERBOSE] Processing: '%s'\n"), input.as_ptr());
            }

            process_command(input.as_ptr());
        }

        0
    }
}
