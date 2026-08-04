extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn scanf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn exit(__status: ::core::ffi::c_int) -> !;
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RoutingDirective {
    pub time_stamp: ::core::ffi::c_uint,
    pub luggage_id: [::core::ffi::c_char; 9],
    pub flight_id: [::core::ffi::c_char; 7],
    pub departure: [::core::ffi::c_char; 4],
    pub arrival: [::core::ffi::c_char; 4],
    pub comments: [::core::ffi::c_char; 81],
    pub next_directive: *mut RoutingDirective,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EOF: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
#[no_mangle]
pub unsafe extern "C" fn addRoutingDirectiveToList(
    mut previous_directive: *mut RoutingDirective,
    mut new_directive: *mut RoutingDirective,
) {
    let mut next_directive: *mut RoutingDirective =
        (*previous_directive).next_directive as *mut RoutingDirective;
    if next_directive.is_null() || (*next_directive).time_stamp > (*new_directive).time_stamp {
        (*new_directive).next_directive = next_directive as *mut RoutingDirective;
        (*previous_directive).next_directive = new_directive as *mut RoutingDirective;
    } else {
        addRoutingDirectiveToList(next_directive, new_directive);
    };
}
#[no_mangle]
pub unsafe extern "C" fn supersedes(
    mut directive: *mut RoutingDirective,
    mut luggage_id: *mut ::core::ffi::c_char,
    mut departure: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if directive.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    if strcmp(
        &raw mut (*directive).luggage_id as *mut ::core::ffi::c_char,
        luggage_id,
    ) != 0 as ::core::ffi::c_int
    {
        return supersedes(
            (*directive).next_directive as *mut RoutingDirective,
            luggage_id,
            departure,
        );
    }
    if strcmp(
        &raw mut (*directive).departure as *mut ::core::ffi::c_char,
        departure,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn superseded(mut directive: *mut RoutingDirective) -> ::core::ffi::c_int {
    return supersedes(
        (*directive).next_directive as *mut RoutingDirective,
        &raw mut (*directive).luggage_id as *mut ::core::ffi::c_char,
        &raw mut (*directive).departure as *mut ::core::ffi::c_char,
    );
}
#[no_mangle]
pub unsafe extern "C" fn matches(
    mut expected: *mut ::core::ffi::c_char,
    mut actual: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    return (*expected.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '-' as i32
        || strcmp(expected, actual) == 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn printMatchingDirectives(
    mut first_directive: *mut RoutingDirective,
    mut expected_luggage_id: *mut ::core::ffi::c_char,
    mut expected_flight_id: *mut ::core::ffi::c_char,
    mut expected_departure: *mut ::core::ffi::c_char,
    mut expected_arrival: *mut ::core::ffi::c_char,
) {
    let mut directive: *mut RoutingDirective = ::core::ptr::null_mut::<RoutingDirective>();
    directive = first_directive;
    while !directive.is_null() {
        if superseded(directive) == 0
            && matches(
                expected_luggage_id,
                &raw mut (*directive).luggage_id as *mut ::core::ffi::c_char,
            ) != 0
            && matches(
                expected_flight_id,
                &raw mut (*directive).flight_id as *mut ::core::ffi::c_char,
            ) != 0
            && matches(
                expected_departure,
                &raw mut (*directive).departure as *mut ::core::ffi::c_char,
            ) != 0
            && matches(
                expected_arrival,
                &raw mut (*directive).arrival as *mut ::core::ffi::c_char,
            ) != 0
        {
            printf(
                b"%010u %s %s %s %s %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*directive).time_stamp,
                &raw mut (*directive).luggage_id as *mut ::core::ffi::c_char,
                &raw mut (*directive).flight_id as *mut ::core::ffi::c_char,
                &raw mut (*directive).departure as *mut ::core::ffi::c_char,
                &raw mut (*directive).arrival as *mut ::core::ffi::c_char,
                &raw mut (*directive).comments as *mut ::core::ffi::c_char,
            );
        }
        directive = (*directive).next_directive as *mut RoutingDirective;
    }
}
unsafe fn main_0(
    mut argc: ::core::ffi::c_int,
    mut argv: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if argc != 5 as ::core::ffi::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Command line error: 4 arguments expected\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        exit(1 as ::core::ffi::c_int);
    }
    let mut directive_list_head: RoutingDirective = RoutingDirective {
        time_stamp: 0,
        luggage_id: [0; 9],
        flight_id: [0; 7],
        departure: [0; 4],
        arrival: [0; 4],
        comments: [0; 81],
        next_directive: ::core::ptr::null_mut::<RoutingDirective>(),
    };
    directive_list_head.time_stamp = 0 as ::core::ffi::c_uint;
    directive_list_head.next_directive = ::core::ptr::null_mut::<RoutingDirective>();
    loop {
        let mut time_stamp: ::core::ffi::c_uint = 0;
        let mut luggage_id: [::core::ffi::c_char; 9] = [0; 9];
        let mut flight_id: [::core::ffi::c_char; 7] = [0; 7];
        let mut departure: [::core::ffi::c_char; 4] = [0; 4];
        let mut arrival: [::core::ffi::c_char; 4] = [0; 4];
        let mut comments: [::core::ffi::c_char; 81] = [0; 81];
        comments[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_char;
        if scanf(
            b"%d \0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut time_stamp,
        ) == EOF
        {
            break;
        }
        if scanf(
            b"%8[A-Z0-9] %6[A-Z0-9] \0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut luggage_id as *mut ::core::ffi::c_char,
            &raw mut flight_id as *mut ::core::ffi::c_char,
        ) == EOF
        {
            break;
        }
        if scanf(
            b"%3[A-Z] %3[A-Z]\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut departure as *mut ::core::ffi::c_char,
            &raw mut arrival as *mut ::core::ffi::c_char,
        ) == EOF
        {
            break;
        }
        if scanf(
            b"%80[^\n]\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut comments as *mut ::core::ffi::c_char,
        ) == EOF
        {
            break;
        }
        let mut new_directive: *mut RoutingDirective = calloc(
            1 as size_t,
            ::core::mem::size_of::<RoutingDirective>() as size_t,
        ) as *mut RoutingDirective;
        (*new_directive).time_stamp = time_stamp;
        strcpy(
            &raw mut (*new_directive).luggage_id as *mut ::core::ffi::c_char,
            &raw mut luggage_id as *mut ::core::ffi::c_char,
        );
        strcpy(
            &raw mut (*new_directive).flight_id as *mut ::core::ffi::c_char,
            &raw mut flight_id as *mut ::core::ffi::c_char,
        );
        strcpy(
            &raw mut (*new_directive).departure as *mut ::core::ffi::c_char,
            &raw mut departure as *mut ::core::ffi::c_char,
        );
        strcpy(
            &raw mut (*new_directive).arrival as *mut ::core::ffi::c_char,
            &raw mut arrival as *mut ::core::ffi::c_char,
        );
        strcpy(
            &raw mut (*new_directive).comments as *mut ::core::ffi::c_char,
            &raw mut comments as *mut ::core::ffi::c_char,
        );
        (*new_directive).next_directive = ::core::ptr::null_mut::<RoutingDirective>();
        addRoutingDirectiveToList(&raw mut directive_list_head, new_directive);
    }
    printMatchingDirectives(
        directive_list_head.next_directive as *mut RoutingDirective,
        *argv.offset(1 as ::core::ffi::c_int as isize),
        *argv.offset(2 as ::core::ffi::c_int as isize),
        *argv.offset(3 as ::core::ffi::c_int as isize),
        *argv.offset(4 as ::core::ffi::c_int as isize),
    );
    exit(0 as ::core::ffi::c_int);
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut ::core::ffi::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut ::core::ffi::c_char)
        .chain(::core::iter::once(::core::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as ::core::ffi::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut ::core::ffi::c_char,
        ) as i32)
    }
}
