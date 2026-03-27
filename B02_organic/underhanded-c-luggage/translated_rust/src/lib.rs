use std::os::raw::{c_char, c_int, c_uint};
use std::ffi::CStr;

#[repr(C)]
pub struct RoutingDirective {
    pub time_stamp: c_uint,
    pub luggage_id: [c_char; 9],
    pub flight_id: [c_char; 7],
    pub departure: [c_char; 4],
    pub arrival: [c_char; 4],
    pub comments: [c_char; 81],
    pub next_directive: *mut RoutingDirective,
}

#[no_mangle]
pub unsafe extern "C" fn addRoutingDirectiveToList(
    previous_directive: *mut RoutingDirective,
    new_directive: *mut RoutingDirective,
) {
    let next = (*previous_directive).next_directive;
    if next.is_null() || (*next).time_stamp > (*new_directive).time_stamp {
        (*new_directive).next_directive = next;
        (*previous_directive).next_directive = new_directive;
    } else {
        addRoutingDirectiveToList(next, new_directive);
    }
}

#[no_mangle]
pub unsafe extern "C" fn supersedes(
    directive: *mut RoutingDirective,
    luggage_id: *mut c_char,
    departure: *mut c_char,
) -> c_int {
    if directive.is_null() {
        return 0;
    }
    if libc_strcmp((*directive).luggage_id.as_ptr(), luggage_id) != 0 {
        return supersedes((*directive).next_directive, luggage_id, departure);
    }
    if libc_strcmp((*directive).departure.as_ptr(), departure) == 0 {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn superseded(directive: *mut RoutingDirective) -> c_int {
    supersedes(
        (*directive).next_directive,
        (*directive).luggage_id.as_mut_ptr(),
        (*directive).departure.as_mut_ptr(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn matches(expected: *mut c_char, actual: *mut c_char) -> c_int {
    if *expected == b'-' as c_char || libc_strcmp(expected, actual) == 0 {
        1
    } else {
        0
    }
}

#[no_mangle]
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
            let lid = CStr::from_ptr((*directive).luggage_id.as_ptr()).to_str().unwrap_or("");
            let fid = CStr::from_ptr((*directive).flight_id.as_ptr()).to_str().unwrap_or("");
            let dep = CStr::from_ptr((*directive).departure.as_ptr()).to_str().unwrap_or("");
            let arr = CStr::from_ptr((*directive).arrival.as_ptr()).to_str().unwrap_or("");
            let com = CStr::from_ptr((*directive).comments.as_ptr()).to_str().unwrap_or("");
            // C uses printf("%010u %s %s %s %s %s\n", ...)
            print!("{:010} {} {} {} {} {}\n", (*directive).time_stamp, lid, fid, dep, arr, com);
        }
        directive = (*directive).next_directive;
    }
}

unsafe fn libc_strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0isize;
    loop {
        let ca = *a.offset(i) as u8;
        let cb = *b.offset(i) as u8;
        if ca != cb {
            return (ca as c_int) - (cb as c_int);
        }
        if ca == 0 {
            return 0;
        }
        i += 1;
    }
}
