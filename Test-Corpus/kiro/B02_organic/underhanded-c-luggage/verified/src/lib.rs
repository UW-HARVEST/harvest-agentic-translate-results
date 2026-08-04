use std::ffi::c_char;
use std::os::raw::c_uint;

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

unsafe fn c_strcmp(a: *const c_char, b: *const c_char) -> i32 {
    let mut i = 0;
    loop {
        let ca = *a.add(i) as u8;
        let cb = *b.add(i) as u8;
        if ca != cb || ca == 0 {
            return (ca as i32) - (cb as i32);
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn addRoutingDirectiveToList(
    previous_directive: *mut RoutingDirective,
    new_directive: *mut RoutingDirective,
) {
    let next_directive = (*previous_directive).next_directive;
    if next_directive.is_null() || (*next_directive).time_stamp > (*new_directive).time_stamp {
        (*new_directive).next_directive = next_directive;
        (*previous_directive).next_directive = new_directive;
    } else {
        addRoutingDirectiveToList(next_directive, new_directive);
    }
}

#[no_mangle]
pub unsafe extern "C" fn supersedes(
    directive: *mut RoutingDirective,
    luggage_id: *mut c_char,
    departure: *mut c_char,
) -> i32 {
    if directive.is_null() {
        return 0;
    }
    if c_strcmp((*directive).luggage_id.as_ptr(), luggage_id) != 0 {
        return supersedes((*directive).next_directive, luggage_id, departure);
    }
    if c_strcmp((*directive).departure.as_ptr(), departure) == 0 {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn superseded(directive: *mut RoutingDirective) -> i32 {
    supersedes(
        (*directive).next_directive,
        (*directive).luggage_id.as_mut_ptr(),
        (*directive).departure.as_mut_ptr(),
    )
}

#[no_mangle]
pub unsafe extern "C" fn matches(expected: *mut c_char, actual: *mut c_char) -> i32 {
    if *expected == b'-' as c_char || c_strcmp(expected, actual) == 0 {
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
            // Use libc printf to match C output exactly
            extern "C" {
                fn printf(fmt: *const c_char, ...) -> i32;
            }
            printf(
                b"%010u %s %s %s %s %s\n\0".as_ptr() as *const c_char,
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
