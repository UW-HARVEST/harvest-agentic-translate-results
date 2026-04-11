#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    static mut stdin: *mut _IO_FILE;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn fgets(
        __s: *mut libc::c_char,
        __n: libc::c_int,
        __stream: *mut FILE,
    ) -> *mut libc::c_char;
    fn atoi(__nptr: *const libc::c_char) -> libc::c_int;
    fn exit(__status: libc::c_int) -> !;
    fn strcpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn strncmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
        __n: size_t,
    ) -> libc::c_int;
    fn strcspn(
        __s: *const libc::c_char,
        __reject: *const libc::c_char,
    ) -> libc::c_ulong;
    fn strstr(
        __haystack: *const libc::c_char,
        __needle: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strtok(
        __s: *mut libc::c_char,
        __delim: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn time(__timer: *mut time_t) -> time_t;
    fn ctime(__timer: *const time_t) -> *mut libc::c_char;
}
pub type size_t = usize;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type __time_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct user_t {
    pub name: [libc::c_char; 32],
    pub password: [libc::c_char; 32],
    pub permission_level: libc::c_int,
    pub logged_in: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct file_t {
    pub filename: [libc::c_char; 64],
    pub content: [libc::c_char; 512],
    pub owner: [libc::c_char; 32],
    pub permissions: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct variable_t {
    pub name: [libc::c_char; 32],
    pub value: [libc::c_char; 128],
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const MAX_INPUT: libc::c_int = 256 as libc::c_int;
pub const MAX_COMMAND: libc::c_int = 64 as libc::c_int;
pub const MAX_ARGS: libc::c_int = 10 as libc::c_int;
pub const MAX_FILES: libc::c_int = 20 as libc::c_int;
pub const MAX_USERS: libc::c_int = 10 as libc::c_int;
pub const MAX_VARIABLES: libc::c_int = 20 as libc::c_int;
static mut users: [user_t; 10] = [user_t {
    name: [0; 32],
    password: [0; 32],
    permission_level: 0,
    logged_in: 0,
}; 10];
static mut user_count: libc::c_int = 0 as libc::c_int;
static mut current_user: *mut user_t = std::ptr::null::<user_t>() as *mut user_t;
static mut files: [file_t; 20] = [file_t {
    filename: [0; 64],
    content: [0; 512],
    owner: [0; 32],
    permissions: 0,
}; 20];
static mut file_count: libc::c_int = 0 as libc::c_int;
static mut variables: [variable_t; 20] = [variable_t {
    name: [0; 32],
    value: [0; 128],
}; 20];
static mut variable_count: libc::c_int = 0 as libc::c_int;
static mut debug_mode: libc::c_int = 0 as libc::c_int;
static mut verbose_mode: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn parse_command(
    mut input: *const libc::c_char,
    mut cmd: *mut libc::c_char,
    mut args: *mut [libc::c_char; 64],
    mut arg_count: *mut libc::c_int,
) {
    let mut temp: [libc::c_char; 256] = [0; 256];
    strncpy(
        &raw mut temp as *mut libc::c_char,
        input,
        (MAX_INPUT - 1 as libc::c_int) as size_t,
    );
    temp[(MAX_INPUT - 1 as libc::c_int) as usize] = '\0' as i32 as libc::c_char;
    *arg_count = 0 as libc::c_int;
    let mut token: *mut libc::c_char = strtok(
        &raw mut temp as *mut libc::c_char,
        b" \t\0" as *const u8 as *const libc::c_char,
    );
    if !token.is_null() {
        strncpy(
            cmd,
            token,
            (MAX_COMMAND - 1 as libc::c_int) as size_t,
        );
        *cmd.offset((MAX_COMMAND - 1 as libc::c_int) as isize) =
            '\0' as i32 as libc::c_char;
        loop {
            token = strtok(
                std::ptr::null_mut::<libc::c_char>(),
                b" \t\0" as *const u8 as *const libc::c_char,
            );
            if !(!token.is_null() && *arg_count < MAX_ARGS) {
                break;
            }
            strncpy(
                &raw mut *args.offset(*arg_count as isize) as *mut libc::c_char,
                token,
                (MAX_COMMAND - 1 as libc::c_int) as size_t,
            );
            (*args.offset(*arg_count as isize))[(MAX_COMMAND - 1 as libc::c_int) as usize] =
                '\0' as i32 as libc::c_char;
            *arg_count += 1;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn cmd_adduser(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 2 as libc::c_int {
        printf(
            b"Usage: adduser <username> <password> [permission_level]\n\0" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    if user_count >= MAX_USERS {
        printf(b"Error: Maximum users reached\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < user_count {
        if strcmp(
            &raw mut (*(&raw mut users as *mut user_t).offset(i as isize)).name
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            printf(
                b"Error: User '%s' already exists\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            );
            return;
        }
        i += 1;
    }
    strcpy(
        &raw mut (*(&raw mut users as *mut user_t).offset(user_count as isize)).name
            as *mut libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
    strcpy(
        &raw mut (*(&raw mut users as *mut user_t).offset(user_count as isize)).password
            as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
    );
    users[user_count as usize].permission_level = if arg_count >= 3 as libc::c_int {
        atoi(&raw mut *args.offset(2 as libc::c_int as isize) as *mut libc::c_char)
    } else {
        1 as libc::c_int
    };
    users[user_count as usize].logged_in = 0 as libc::c_int;
    user_count += 1;
    printf(
        b"User '%s' added with permission level %d\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        users[(user_count - 1 as libc::c_int) as usize].permission_level,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_login(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 2 as libc::c_int {
        printf(
            b"Usage: login <username> <password>\n\0" as *const u8 as *const libc::c_char,
        );
        return;
    }
    if !current_user.is_null() && (*current_user).logged_in != 0 {
        printf(
            b"Error: User '%s' already logged in. Use 'logout' first.\n\0" as *const u8
                as *const libc::c_char,
            &raw mut (*current_user).name as *mut libc::c_char,
        );
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < user_count {
        if strcmp(
            &raw mut (*(&raw mut users as *mut user_t).offset(i as isize)).name
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            if strcmp(
                &raw mut (*(&raw mut users as *mut user_t).offset(i as isize)).password
                    as *mut libc::c_char,
                &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
            ) == 0 as libc::c_int
            {
                users[i as usize].logged_in = 1 as libc::c_int;
                current_user = (&raw mut users as *mut user_t).offset(i as isize) as *mut user_t;
                printf(
                    b"Login successful. Welcome, %s!\n\0" as *const u8
                        as *const libc::c_char,
                    &raw mut (*current_user).name as *mut libc::c_char,
                );
                return;
            } else {
                printf(b"Error: Incorrect password\n\0" as *const u8 as *const libc::c_char);
                return;
            }
        }
        i += 1;
    }
    printf(b"Error: User not found\n\0" as *const u8 as *const libc::c_char);
}
#[no_mangle]
pub unsafe extern "C" fn cmd_logout() {
    if current_user.is_null() || (*current_user).logged_in == 0 {
        printf(b"Error: No user logged in\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(
        b"Goodbye, %s!\n\0" as *const u8 as *const libc::c_char,
        &raw mut (*current_user).name as *mut libc::c_char,
    );
    (*current_user).logged_in = 0 as libc::c_int;
    current_user = std::ptr::null_mut::<user_t>();
}
#[no_mangle]
pub unsafe extern "C" fn cmd_whoami() {
    if current_user.is_null() || (*current_user).logged_in == 0 {
        printf(b"Not logged in\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(
        b"Current user: %s\n\0" as *const u8 as *const libc::c_char,
        &raw mut (*current_user).name as *mut libc::c_char,
    );
    printf(
        b"Permission level: %d\n\0" as *const u8 as *const libc::c_char,
        (*current_user).permission_level,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_listusers() {
    if user_count == 0 as libc::c_int {
        printf(b"No users registered\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(b"Registered users:\n\0" as *const u8 as *const libc::c_char);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < user_count {
        printf(
            b"  %s (level %d) %s\n\0" as *const u8 as *const libc::c_char,
            &raw mut (*(&raw mut users as *mut user_t).offset(i as isize)).name
                as *mut libc::c_char,
            users[i as usize].permission_level,
            if users[i as usize].logged_in != 0 {
                b"[logged in]\0" as *const u8 as *const libc::c_char
            } else {
                b"\0" as *const u8 as *const libc::c_char
            },
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn cmd_createfile(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if current_user.is_null() || (*current_user).logged_in == 0 {
        printf(b"Error: Must be logged in\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    if arg_count < 1 as libc::c_int {
        printf(
            b"Usage: createfile <filename> [content]\n\0" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    if file_count >= MAX_FILES {
        printf(b"Error: Maximum files reached\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < file_count {
        if strcmp(
            &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).filename
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            printf(
                b"Error: File '%s' already exists\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            );
            return;
        }
        i += 1;
    }
    strcpy(
        &raw mut (*(&raw mut files as *mut file_t).offset(file_count as isize)).filename
            as *mut libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
    strcpy(
        &raw mut (*(&raw mut files as *mut file_t).offset(file_count as isize)).owner
            as *mut libc::c_char,
        &raw mut (*current_user).name as *mut libc::c_char,
    );
    files[file_count as usize].permissions = 755 as libc::c_int;
    if arg_count >= 2 as libc::c_int {
        strcpy(
            &raw mut (*(&raw mut files as *mut file_t).offset(file_count as isize)).content
                as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        );
    } else {
        files[file_count as usize].content[0 as libc::c_int as usize] =
            '\0' as i32 as libc::c_char;
    }
    file_count += 1;
    printf(
        b"File '%s' created\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_readfile(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 1 as libc::c_int {
        printf(b"Usage: readfile <filename>\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < file_count {
        if strcmp(
            &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).filename
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            printf(
                b"=== %s ===\n\0" as *const u8 as *const libc::c_char,
                &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).filename
                    as *mut libc::c_char,
            );
            printf(
                b"Owner: %s\n\0" as *const u8 as *const libc::c_char,
                &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).owner
                    as *mut libc::c_char,
            );
            printf(
                b"Permissions: %d\n\0" as *const u8 as *const libc::c_char,
                files[i as usize].permissions,
            );
            printf(
                b"Content: %s\n\0" as *const u8 as *const libc::c_char,
                &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).content
                    as *mut libc::c_char,
            );
            return;
        }
        i += 1;
    }
    printf(
        b"Error: File '%s' not found\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_writefile(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if current_user.is_null() || (*current_user).logged_in == 0 {
        printf(b"Error: Must be logged in\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    if arg_count < 2 as libc::c_int {
        printf(
            b"Usage: writefile <filename> <content>\n\0" as *const u8 as *const libc::c_char,
        );
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < file_count {
        if strcmp(
            &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).filename
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            if strcmp(
                &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).owner
                    as *mut libc::c_char,
                &raw mut (*current_user).name as *mut libc::c_char,
            ) == 0 as libc::c_int
                || (*current_user).permission_level >= 5 as libc::c_int
            {
                strcpy(
                    &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).content
                        as *mut libc::c_char,
                    &raw mut *args.offset(1 as libc::c_int as isize)
                        as *mut libc::c_char,
                );
                printf(
                    b"File '%s' updated\n\0" as *const u8 as *const libc::c_char,
                    &raw mut *args.offset(0 as libc::c_int as isize)
                        as *mut libc::c_char,
                );
                return;
            } else {
                printf(b"Error: Permission denied\n\0" as *const u8 as *const libc::c_char);
                return;
            }
        }
        i += 1;
    }
    printf(
        b"Error: File '%s' not found\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_deletefile(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if current_user.is_null() || (*current_user).logged_in == 0 {
        printf(b"Error: Must be logged in\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    if arg_count < 1 as libc::c_int {
        printf(b"Usage: deletefile <filename>\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < file_count {
        if strcmp(
            &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).filename
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            if strcmp(
                &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).owner
                    as *mut libc::c_char,
                &raw mut (*current_user).name as *mut libc::c_char,
            ) == 0 as libc::c_int
                || (*current_user).permission_level >= 9 as libc::c_int
            {
                let mut j: libc::c_int = i;
                while j < file_count - 1 as libc::c_int {
                    files[j as usize] = files[(j + 1 as libc::c_int) as usize];
                    j += 1;
                }
                file_count -= 1;
                printf(
                    b"File '%s' deleted\n\0" as *const u8 as *const libc::c_char,
                    &raw mut *args.offset(0 as libc::c_int as isize)
                        as *mut libc::c_char,
                );
                return;
            } else {
                printf(b"Error: Permission denied\n\0" as *const u8 as *const libc::c_char);
                return;
            }
        }
        i += 1;
    }
    printf(
        b"Error: File '%s' not found\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_listfiles() {
    if file_count == 0 as libc::c_int {
        printf(b"No files\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(b"Files:\n\0" as *const u8 as *const libc::c_char);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < file_count {
        printf(
            b"  %s (owner: %s, perm: %d)\n\0" as *const u8 as *const libc::c_char,
            &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).filename
                as *mut libc::c_char,
            &raw mut (*(&raw mut files as *mut file_t).offset(i as isize)).owner
                as *mut libc::c_char,
            files[i as usize].permissions,
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn cmd_set(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 2 as libc::c_int {
        printf(b"Usage: set <name> <value>\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < variable_count {
        if strcmp(
            &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).name
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            strcpy(
                &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).value
                    as *mut libc::c_char,
                &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
            );
            printf(
                b"Variable '%s' updated\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            );
            return;
        }
        i += 1;
    }
    if variable_count >= MAX_VARIABLES {
        printf(b"Error: Maximum variables reached\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    strcpy(
        &raw mut (*(&raw mut variables as *mut variable_t).offset(variable_count as isize)).name
            as *mut libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
    strcpy(
        &raw mut (*(&raw mut variables as *mut variable_t).offset(variable_count as isize)).value
            as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
    );
    variable_count += 1;
    printf(
        b"Variable '%s' set\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_get(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 1 as libc::c_int {
        printf(b"Usage: get <name>\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < variable_count {
        if strcmp(
            &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).name
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            printf(
                b"%s = %s\n\0" as *const u8 as *const libc::c_char,
                &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).name
                    as *mut libc::c_char,
                &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).value
                    as *mut libc::c_char,
            );
            return;
        }
        i += 1;
    }
    printf(
        b"Error: Variable '%s' not found\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_unset(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 1 as libc::c_int {
        printf(b"Usage: unset <name>\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < variable_count {
        if strcmp(
            &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).name
                as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            let mut j: libc::c_int = i;
            while j < variable_count - 1 as libc::c_int {
                variables[j as usize] = variables[(j + 1 as libc::c_int) as usize];
                j += 1;
            }
            variable_count -= 1;
            printf(
                b"Variable '%s' unset\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            );
            return;
        }
        i += 1;
    }
    printf(
        b"Error: Variable '%s' not found\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_listvars() {
    if variable_count == 0 as libc::c_int {
        printf(b"No variables set\n\0" as *const u8 as *const libc::c_char);
        return;
    }
    printf(b"Variables:\n\0" as *const u8 as *const libc::c_char);
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < variable_count {
        printf(
            b"  %s = %s\n\0" as *const u8 as *const libc::c_char,
            &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).name
                as *mut libc::c_char,
            &raw mut (*(&raw mut variables as *mut variable_t).offset(i as isize)).value
                as *mut libc::c_char,
        );
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn cmd_compare(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 2 as libc::c_int {
        printf(
            b"Usage: compare <string1> <string2>\n\0" as *const u8 as *const libc::c_char,
        );
        return;
    }
    let mut result: libc::c_int = strcmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
    );
    printf(
        b"strcmp('%s', '%s') = %d\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        result,
    );
    if result == 0 as libc::c_int {
        printf(b"Strings are equal\n\0" as *const u8 as *const libc::c_char);
    } else if result < 0 as libc::c_int {
        printf(
            b"'%s' < '%s'\n\0" as *const u8 as *const libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        );
    } else {
        printf(
            b"'%s' > '%s'\n\0" as *const u8 as *const libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn cmd_compareN(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 3 as libc::c_int {
        printf(
            b"Usage: compareN <string1> <string2> <n>\n\0" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    let mut n: libc::c_int =
        atoi(&raw mut *args.offset(2 as libc::c_int as isize) as *mut libc::c_char);
    let mut result: libc::c_int = strncmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        n as size_t,
    );
    printf(
        b"strncmp('%s', '%s', %d) = %d\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        n,
        result,
    );
    if result == 0 as libc::c_int {
        printf(
            b"First %d characters are equal\n\0" as *const u8 as *const libc::c_char,
            n,
        );
    } else if result < 0 as libc::c_int {
        printf(
            b"'%s' < '%s' (first %d chars)\n\0" as *const u8 as *const libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
            n,
        );
    } else {
        printf(
            b"'%s' > '%s' (first %d chars)\n\0" as *const u8 as *const libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
            n,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn cmd_startswith(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 2 as libc::c_int {
        printf(
            b"Usage: startswith <string> <prefix>\n\0" as *const u8 as *const libc::c_char,
        );
        return;
    }
    let mut prefix_len: size_t =
        strlen(&raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char);
    if strncmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        prefix_len,
    ) == 0 as libc::c_int
    {
        printf(
            b"'%s' starts with '%s'\n\0" as *const u8 as *const libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        );
    } else {
        printf(
            b"'%s' does not start with '%s'\n\0" as *const u8 as *const libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(1 as libc::c_int as isize) as *mut libc::c_char,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn cmd_match(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 2 as libc::c_int {
        printf(
            b"Usage: match <pattern> <string1> [string2] ...\n\0" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    printf(
        b"Matching pattern '%s':\n\0" as *const u8 as *const libc::c_char,
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
    );
    let mut matches: libc::c_int = 0 as libc::c_int;
    let mut i: libc::c_int = 1 as libc::c_int;
    while i < arg_count {
        if strcmp(
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
            &raw mut *args.offset(i as isize) as *mut libc::c_char,
        ) == 0 as libc::c_int
        {
            printf(
                b"  '%s' - EXACT MATCH\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(i as isize) as *mut libc::c_char,
            );
            matches += 1;
        } else if !strstr(
            &raw mut *args.offset(i as isize) as *mut libc::c_char,
            &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        )
        .is_null()
        {
            printf(
                b"  '%s' - contains pattern\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(i as isize) as *mut libc::c_char,
            );
            matches += 1;
        } else {
            printf(
                b"  '%s' - no match\n\0" as *const u8 as *const libc::c_char,
                &raw mut *args.offset(i as isize) as *mut libc::c_char,
            );
        }
        i += 1;
    }
    printf(
        b"Total matches: %d\n\0" as *const u8 as *const libc::c_char,
        matches,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_help() {
    printf(b"\n=== Command Interpreter Help ===\n\0" as *const u8 as *const libc::c_char);
    printf(b"User Management:\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"  adduser <user> <pass> [level] - Add new user\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  login <user> <pass>            - Login as user\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  logout                         - Logout current user\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  whoami                         - Show current user\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  listusers                      - List all users\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(b"\nFile Management:\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"  createfile <name> [content]    - Create file\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  readfile <name>                - Read file\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  writefile <name> <content>     - Write to file\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  deletefile <name>              - Delete file\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  listfiles                      - List all files\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(b"\nVariable Management:\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"  set <name> <value>             - Set variable\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  get <name>                     - Get variable\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  unset <name>                   - Unset variable\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  listvars                       - List all variables\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(b"\nString Operations:\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"  compare <str1> <str2>          - Compare strings\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  compareN <str1> <str2> <n>     - Compare first N chars\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  startswith <str> <prefix>      - Check if starts with\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  match <pattern> <str> ...      - Match pattern\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(b"\nSystem:\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"  debug [on|off]                 - Toggle debug mode\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  verbose [on|off]               - Toggle verbose mode\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  status                         - Show system status\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  time                           - Show current time\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  help                           - Show this help\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"  exit                           - Exit program\n\0" as *const u8
            as *const libc::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_debug(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 1 as libc::c_int {
        printf(
            b"Debug mode: %s\n\0" as *const u8 as *const libc::c_char,
            if debug_mode != 0 {
                b"ON\0" as *const u8 as *const libc::c_char
            } else {
                b"OFF\0" as *const u8 as *const libc::c_char
            },
        );
        return;
    }
    if strcmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"on\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        debug_mode = 1 as libc::c_int;
        printf(b"Debug mode enabled\n\0" as *const u8 as *const libc::c_char);
    } else if strcmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"off\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        debug_mode = 0 as libc::c_int;
        printf(b"Debug mode disabled\n\0" as *const u8 as *const libc::c_char);
    } else {
        printf(b"Usage: debug [on|off]\n\0" as *const u8 as *const libc::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn cmd_verbose(
    mut args: *mut [libc::c_char; 64],
    mut arg_count: libc::c_int,
) {
    if arg_count < 1 as libc::c_int {
        printf(
            b"Verbose mode: %s\n\0" as *const u8 as *const libc::c_char,
            if verbose_mode != 0 {
                b"ON\0" as *const u8 as *const libc::c_char
            } else {
                b"OFF\0" as *const u8 as *const libc::c_char
            },
        );
        return;
    }
    if strcmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"on\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        verbose_mode = 1 as libc::c_int;
        printf(b"Verbose mode enabled\n\0" as *const u8 as *const libc::c_char);
    } else if strcmp(
        &raw mut *args.offset(0 as libc::c_int as isize) as *mut libc::c_char,
        b"off\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        verbose_mode = 0 as libc::c_int;
        printf(b"Verbose mode disabled\n\0" as *const u8 as *const libc::c_char);
    } else {
        printf(b"Usage: verbose [on|off]\n\0" as *const u8 as *const libc::c_char);
    };
}
#[no_mangle]
pub unsafe extern "C" fn cmd_status() {
    printf(b"\n=== System Status ===\n\0" as *const u8 as *const libc::c_char);
    printf(
        b"Users: %d/%d\n\0" as *const u8 as *const libc::c_char,
        user_count,
        MAX_USERS,
    );
    printf(
        b"Files: %d/%d\n\0" as *const u8 as *const libc::c_char,
        file_count,
        MAX_FILES,
    );
    printf(
        b"Variables: %d/%d\n\0" as *const u8 as *const libc::c_char,
        variable_count,
        MAX_VARIABLES,
    );
    printf(
        b"Current user: %s\n\0" as *const u8 as *const libc::c_char,
        if !current_user.is_null() && (*current_user).logged_in != 0 {
            &raw mut (*current_user).name as *mut libc::c_char as *const libc::c_char
        } else {
            b"none\0" as *const u8 as *const libc::c_char
        },
    );
    printf(
        b"Debug mode: %s\n\0" as *const u8 as *const libc::c_char,
        if debug_mode != 0 {
            b"ON\0" as *const u8 as *const libc::c_char
        } else {
            b"OFF\0" as *const u8 as *const libc::c_char
        },
    );
    printf(
        b"Verbose mode: %s\n\0" as *const u8 as *const libc::c_char,
        if verbose_mode != 0 {
            b"ON\0" as *const u8 as *const libc::c_char
        } else {
            b"OFF\0" as *const u8 as *const libc::c_char
        },
    );
}
#[no_mangle]
pub unsafe extern "C" fn cmd_time() {
    let mut now: time_t = time(std::ptr::null_mut::<time_t>());
    printf(
        b"Current time: %s\0" as *const u8 as *const libc::c_char,
        ctime(&raw mut now),
    );
}
#[no_mangle]
pub unsafe extern "C" fn process_command(mut input: *const libc::c_char) {
    let mut command: [libc::c_char; 64] = [0; 64];
    let mut args: [[libc::c_char; 64]; 10] = [[0; 64]; 10];
    let mut arg_count: libc::c_int = 0 as libc::c_int;
    parse_command(
        input,
        &raw mut command as *mut libc::c_char,
        &raw mut args as *mut [libc::c_char; 64],
        &raw mut arg_count,
    );
    if strlen(&raw mut command as *mut libc::c_char) == 0 as size_t {
        return;
    }
    if debug_mode != 0 {
        printf(
            b"[DEBUG] Command: '%s', Args: %d\n\0" as *const u8 as *const libc::c_char,
            &raw mut command as *mut libc::c_char,
            arg_count,
        );
    }
    if strcmp(
        &raw mut command as *mut libc::c_char,
        b"adduser\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_adduser(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"login\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_login(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"logout\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_logout();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"whoami\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_whoami();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"listusers\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"users\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_listusers();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"createfile\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"touch\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_createfile(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"readfile\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"cat\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_readfile(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"writefile\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"write\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_writefile(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"deletefile\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"rm\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_deletefile(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"listfiles\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"ls\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_listfiles();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"set\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_set(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"get\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_get(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"unset\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_unset(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"listvars\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"vars\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_listvars();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"compare\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"cmp\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_compare(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"compareN\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"cmpn\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_compareN(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"startswith\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_startswith(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"match\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_match(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"debug\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_debug(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"verbose\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_verbose(&raw mut args as *mut [libc::c_char; 64], arg_count);
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"status\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_status();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"time\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
    {
        cmd_time();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"help\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"?\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        cmd_help();
    } else if strcmp(
        &raw mut command as *mut libc::c_char,
        b"exit\0" as *const u8 as *const libc::c_char,
    ) == 0 as libc::c_int
        || strcmp(
            &raw mut command as *mut libc::c_char,
            b"quit\0" as *const u8 as *const libc::c_char,
        ) == 0 as libc::c_int
    {
        printf(b"Goodbye!\n\0" as *const u8 as *const libc::c_char);
        exit(0 as libc::c_int);
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"add\0" as *const u8 as *const libc::c_char,
        3 as size_t,
    ) == 0 as libc::c_int
    {
        printf(b"Did you mean 'adduser'?\n\0" as *const u8 as *const libc::c_char);
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"log\0" as *const u8 as *const libc::c_char,
        3 as size_t,
    ) == 0 as libc::c_int
    {
        printf(b"Did you mean 'login' or 'logout'?\n\0" as *const u8 as *const libc::c_char);
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"list\0" as *const u8 as *const libc::c_char,
        4 as size_t,
    ) == 0 as libc::c_int
    {
        printf(
            b"Did you mean 'listusers', 'listfiles', or 'listvars'?\n\0" as *const u8
                as *const libc::c_char,
        );
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"create\0" as *const u8 as *const libc::c_char,
        6 as size_t,
    ) == 0 as libc::c_int
    {
        printf(b"Did you mean 'createfile'?\n\0" as *const u8 as *const libc::c_char);
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"read\0" as *const u8 as *const libc::c_char,
        4 as size_t,
    ) == 0 as libc::c_int
    {
        printf(b"Did you mean 'readfile'?\n\0" as *const u8 as *const libc::c_char);
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"write\0" as *const u8 as *const libc::c_char,
        5 as size_t,
    ) == 0 as libc::c_int
    {
        printf(b"Did you mean 'writefile'?\n\0" as *const u8 as *const libc::c_char);
    } else if strncmp(
        &raw mut command as *mut libc::c_char,
        b"delete\0" as *const u8 as *const libc::c_char,
        6 as size_t,
    ) == 0 as libc::c_int
    {
        printf(b"Did you mean 'deletefile'?\n\0" as *const u8 as *const libc::c_char);
    } else {
        printf(
            b"Unknown command: '%s'. Type 'help' for available commands.\n\0" as *const u8
                as *const libc::c_char,
            &raw mut command as *mut libc::c_char,
        );
    };
}
unsafe fn main_0() -> libc::c_int {
    printf(
        b"|----------------------------------------|\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"|   COMMAND INTERPRETER                  |\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"|   strcmp/strncmp demonstration         |\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(
        b"|----------------------------------------|\n\0" as *const u8
            as *const libc::c_char,
    );
    printf(b"Type 'help' for available commands\n\n\0" as *const u8 as *const libc::c_char);
    let mut input: [libc::c_char; 256] = [0; 256];
    loop {
        printf(b"> \0" as *const u8 as *const libc::c_char);
        if fgets(
            &raw mut input as *mut libc::c_char,
            MAX_INPUT,
            stdin as *mut FILE,
        )
        .is_null()
        {
            break;
        }
        input[strcspn(
            &raw mut input as *mut libc::c_char,
            b"\n\0" as *const u8 as *const libc::c_char,
        ) as usize] = 0 as libc::c_char;
        if verbose_mode != 0 {
            printf(
                b"[VERBOSE] Processing: '%s'\n\0" as *const u8 as *const libc::c_char,
                &raw mut input as *mut libc::c_char,
            );
        }
        process_command(&raw mut input as *mut libc::c_char);
    }
    return 0 as libc::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
