// By Jan Wrobel <wrr@mixedbit.org>
// Translation to Rust preserving byte-identical behavior.

use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_uint;
use std::ptr;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[repr(C)]
pub struct RoutingDirective {
    pub time_stamp: c_uint,
    pub luggage_id: [c_char; LUGGAGE_ID_LENGTH + 1],
    pub flight_id: [c_char; FLIGHT_ID_LENGTH + 1],
    pub departure: [c_char; AIRPORT_CODE_LENGTH + 1],
    pub arrival: [c_char; AIRPORT_CODE_LENGTH + 1],
    pub comments: [c_char; COMMENTS_LENGTH + 1],
    pub next_directive: *mut RoutingDirective,
}

extern "C" {
    fn scanf(format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut libc::FILE, format: *const c_char, ...) -> c_int;
    fn calloc(nmemb: usize, size: usize) -> *mut std::ffi::c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn exit(status: c_int) -> !;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn addRoutingDirectiveToList(
    previous_directive: *mut RoutingDirective,
    new_directive: *mut RoutingDirective,
) {
    let next_directive = (*previous_directive).next_directive;
    if next_directive.is_null()
        || (*next_directive).time_stamp > (*new_directive).time_stamp
    {
        (*new_directive).next_directive = next_directive;
        (*previous_directive).next_directive = new_directive;
    } else {
        addRoutingDirectiveToList(next_directive, new_directive);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn supersedes(
    directive: *mut RoutingDirective,
    luggage_id: *mut c_char,
    departure: *mut c_char,
) -> c_int {
    if directive.is_null() {
        return 0;
    }
    if strcmp((*directive).luggage_id.as_ptr(), luggage_id) != 0 {
        return supersedes((*directive).next_directive, luggage_id, departure);
    }
    if strcmp((*directive).departure.as_ptr(), departure) == 0 {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn superseded(directive: *mut RoutingDirective) -> c_int {
    supersedes(
        (*directive).next_directive,
        (*directive).luggage_id.as_mut_ptr(),
        (*directive).departure.as_mut_ptr(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matches(expected: *mut c_char, actual: *mut c_char) -> c_int {
    if *expected == b'-' as c_char || strcmp(expected, actual) == 0 {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printMatchingDirectives(
    first_directive: *mut RoutingDirective,
    expected_luggage_id: *mut c_char,
    expected_flight_id: *mut c_char,
    expected_departure: *mut c_char,
    expected_arrival: *mut c_char,
) {
    let mut directive = first_directive;
    while !directive.is_null() {
        if superseded(directive) == 0
            && matches(expected_luggage_id, (*directive).luggage_id.as_mut_ptr()) != 0
            && matches(expected_flight_id, (*directive).flight_id.as_mut_ptr()) != 0
            && matches(expected_departure, (*directive).departure.as_mut_ptr()) != 0
            && matches(expected_arrival, (*directive).arrival.as_mut_ptr()) != 0
        {
            let fmt = b"%010u %s %s %s %s %s\n\0".as_ptr() as *const c_char;
            printf(
                fmt,
                (*directive).time_stamp,
                (*directive).luggage_id.as_ptr(),
                (*directive).flight_id.as_ptr(),
                (*directive).departure.as_ptr(),
                (*directive).arrival.as_ptr(),
                (*directive).comments.as_ptr(),
            );
        }
        directive = (*directive).next_directive;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 5 {
        let msg = b"Command line error: 4 arguments expected\n\0".as_ptr() as *const c_char;
        let stderr = libc_stderr();
        fprintf(stderr, msg);
        exit(1);
    }

    let mut directive_list_head: RoutingDirective = RoutingDirective {
        time_stamp: 0,
        luggage_id: [0; LUGGAGE_ID_LENGTH + 1],
        flight_id: [0; FLIGHT_ID_LENGTH + 1],
        departure: [0; AIRPORT_CODE_LENGTH + 1],
        arrival: [0; AIRPORT_CODE_LENGTH + 1],
        comments: [0; COMMENTS_LENGTH + 1],
        next_directive: ptr::null_mut(),
    };

    loop {
        let mut time_stamp: c_uint = 0;
        let mut luggage_id: [c_char; LUGGAGE_ID_LENGTH + 1] = [0; LUGGAGE_ID_LENGTH + 1];
        let mut flight_id: [c_char; FLIGHT_ID_LENGTH + 1] = [0; FLIGHT_ID_LENGTH + 1];
        let mut departure: [c_char; AIRPORT_CODE_LENGTH + 1] = [0; AIRPORT_CODE_LENGTH + 1];
        let mut arrival: [c_char; AIRPORT_CODE_LENGTH + 1] = [0; AIRPORT_CODE_LENGTH + 1];
        let mut comments: [c_char; COMMENTS_LENGTH + 1] = [0; COMMENTS_LENGTH + 1];
        comments[0] = 0; // comments are optional.

        let fmt1 = b"%d \0".as_ptr() as *const c_char;
        if scanf(fmt1, &mut time_stamp as *mut c_uint) == libc::EOF {
            break;
        }
        let fmt2 = b"%8[A-Z0-9] %6[A-Z0-9] \0".as_ptr() as *const c_char;
        if scanf(
            fmt2,
            luggage_id.as_mut_ptr(),
            flight_id.as_mut_ptr(),
        ) == libc::EOF
        {
            break;
        }
        let fmt3 = b"%3[A-Z] %3[A-Z]\0".as_ptr() as *const c_char;
        if scanf(
            fmt3,
            departure.as_mut_ptr(),
            arrival.as_mut_ptr(),
        ) == libc::EOF
        {
            break;
        }
        let fmt4 = b"%80[^\n]\0".as_ptr() as *const c_char;
        if scanf(fmt4, comments.as_mut_ptr()) == libc::EOF {
            break;
        }

        let new_directive =
            calloc(1, std::mem::size_of::<RoutingDirective>()) as *mut RoutingDirective;
        (*new_directive).time_stamp = time_stamp;
        strcpy((*new_directive).luggage_id.as_mut_ptr(), luggage_id.as_ptr());
        strcpy((*new_directive).flight_id.as_mut_ptr(), flight_id.as_ptr());
        strcpy((*new_directive).departure.as_mut_ptr(), departure.as_ptr());
        strcpy((*new_directive).arrival.as_mut_ptr(), arrival.as_ptr());
        strcpy((*new_directive).comments.as_mut_ptr(), comments.as_ptr());
        (*new_directive).next_directive = ptr::null_mut();

        addRoutingDirectiveToList(&mut directive_list_head as *mut RoutingDirective, new_directive);
    }

    printMatchingDirectives(
        directive_list_head.next_directive,
        *argv.add(1),
        *argv.add(2),
        *argv.add(3),
        *argv.add(4),
    );
    exit(0);
}

// Helper to access stderr.
unsafe fn libc_stderr() -> *mut libc::FILE {
    extern "C" {
        // On glibc, stderr is a macro for `stderr` which is a global pointer.
        static mut stderr: *mut libc::FILE;
    }
    stderr
}
