use std::ffi::{c_char, c_int};
use std::fmt::Write as FmtWrite;
use std::io::{self, Read, Write};
use std::process;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[repr(C)]
pub struct RoutingDirective {
    pub time_stamp: u32,
    pub luggage_id: [c_char; LUGGAGE_ID_LENGTH + 1],
    pub flight_id: [c_char; FLIGHT_ID_LENGTH + 1],
    pub departure: [c_char; AIRPORT_CODE_LENGTH + 1],
    pub arrival: [c_char; AIRPORT_CODE_LENGTH + 1],
    pub comments: [c_char; COMMENTS_LENGTH + 1],
    pub next_directive: *mut RoutingDirective,
}

unsafe fn c_strings_equal(mut left: *const c_char, mut right: *const c_char) -> bool {
    loop {
        let left_byte = unsafe { *left };
        let right_byte = unsafe { *right };
        if left_byte != right_byte {
            return false;
        }
        if left_byte == 0 {
            return true;
        }
        left = unsafe { left.add(1) };
        right = unsafe { right.add(1) };
    }
}

unsafe extern "C" {
    fn raise(signal: c_int) -> c_int;
    fn signal(signal: c_int, handler: usize) -> usize;
}

fn segmentation_fault() -> ! {
    unsafe {
        signal(11, 0);
        raise(11);
    }
    process::abort();
}

#[no_mangle]
pub unsafe extern "C" fn addRoutingDirectiveToList(
    previous_directive: *mut RoutingDirective,
    new_directive: *mut RoutingDirective,
) {
    if previous_directive.is_null() || new_directive.is_null() {
        segmentation_fault();
    }
    let next_directive = unsafe { (*previous_directive).next_directive };
    if next_directive.is_null()
        || unsafe { (*next_directive).time_stamp > (*new_directive).time_stamp }
    {
        unsafe {
            (*new_directive).next_directive = next_directive;
            (*previous_directive).next_directive = new_directive;
        }
    } else {
        unsafe { addRoutingDirectiveToList(next_directive, new_directive) };
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
    if luggage_id.is_null() {
        segmentation_fault();
    }
    if !unsafe { c_strings_equal((*directive).luggage_id.as_ptr(), luggage_id) } {
        return unsafe { supersedes((*directive).next_directive, luggage_id, departure) };
    }
    if departure.is_null() {
        segmentation_fault();
    }
    if unsafe { c_strings_equal((*directive).departure.as_ptr(), departure) } {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn superseded(directive: *mut RoutingDirective) -> c_int {
    if directive.is_null() {
        segmentation_fault();
    }
    unsafe {
        supersedes(
            (*directive).next_directive,
            (*directive).luggage_id.as_mut_ptr(),
            (*directive).departure.as_mut_ptr(),
        )
    }
}

#[no_mangle]
pub unsafe extern "C" fn matches(expected: *mut c_char, actual: *mut c_char) -> c_int {
    if expected.is_null() {
        segmentation_fault();
    }
    if unsafe { *expected == b'-' as c_char } {
        return 1;
    }
    if actual.is_null() {
        segmentation_fault();
    }
    if unsafe { c_strings_equal(expected, actual) } {
        1
    } else {
        0
    }
}

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[no_mangle]
pub unsafe extern "C" fn printMatchingDirectives(
    mut directive: *mut RoutingDirective,
    expected_luggage_id: *mut c_char,
    expected_flight_id: *mut c_char,
    expected_departure: *mut c_char,
    expected_arrival: *mut c_char,
) {
    static FORMAT: &[u8] = b"%010u %s %s %s %s %s\n\0";
    while !directive.is_null() {
        if unsafe {
            superseded(directive) == 0
                && matches(expected_luggage_id, (*directive).luggage_id.as_mut_ptr()) != 0
                && matches(expected_flight_id, (*directive).flight_id.as_mut_ptr()) != 0
                && matches(expected_departure, (*directive).departure.as_mut_ptr()) != 0
                && matches(expected_arrival, (*directive).arrival.as_mut_ptr()) != 0
        } {
            unsafe {
                printf(
                    FORMAT.as_ptr().cast(),
                    (*directive).time_stamp,
                    (*directive).luggage_id.as_ptr(),
                    (*directive).flight_id.as_ptr(),
                    (*directive).departure.as_ptr(),
                    (*directive).arrival.as_ptr(),
                    (*directive).comments.as_ptr(),
                );
            }
        }
        directive = unsafe { (*directive).next_directive };
    }
}

struct OwnedDirective {
    time_stamp: u32,
    luggage_id: Vec<u8>,
    flight_id: Vec<u8>,
    departure: Vec<u8>,
    arrival: Vec<u8>,
    comments: Vec<u8>,
}

struct Scanner {
    input: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn new(input: Vec<u8>) -> Self {
        Self { input, position: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self
            .input
            .get(self.position)
            .is_some_and(|byte| is_c_whitespace(*byte))
        {
            self.position += 1;
        }
    }

    fn scan_timestamp(&mut self, value: &mut u32) -> c_int {
        self.skip_whitespace();
        if self.position == self.input.len() {
            return -1;
        }

        let start = self.position;
        let negative = match self.input[self.position] {
            b'-' => {
                self.position += 1;
                true
            }
            b'+' => {
                self.position += 1;
                false
            }
            _ => false,
        };
        let digits_start = self.position;
        let mut magnitude = 0_u128;
        while let Some(byte @ b'0'..=b'9') = self.input.get(self.position).copied() {
            magnitude = magnitude
                .saturating_mul(10)
                .saturating_add(u128::from(byte - b'0'));
            self.position += 1;
        }
        if self.position == digits_start {
            self.position = start;
            return 0;
        }

        let signed = if negative {
            if magnitude >= (i64::MAX as u128) + 1 {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if magnitude > i64::MAX as u128 {
            i64::MAX
        } else {
            magnitude as i64
        };
        *value = signed as u32;
        self.skip_whitespace();
        1
    }

    fn scan_set<F>(&mut self, maximum: usize, output: &mut Vec<u8>, accepts: F) -> c_int
    where
        F: Fn(u8) -> bool,
    {
        if self.position == self.input.len() {
            return -1;
        }
        if !accepts(self.input[self.position]) {
            return 0;
        }

        output.clear();
        while output.len() < maximum {
            let Some(&byte) = self.input.get(self.position) else {
                break;
            };
            if !accepts(byte) {
                break;
            }
            output.push(byte);
            self.position += 1;
        }
        1
    }

    fn scan_ids(&mut self, luggage_id: &mut Vec<u8>, flight_id: &mut Vec<u8>) -> c_int {
        let first = self.scan_set(LUGGAGE_ID_LENGTH, luggage_id, is_uppercase_or_digit);
        if first <= 0 {
            return first;
        }
        self.skip_whitespace();
        let second = self.scan_set(FLIGHT_ID_LENGTH, flight_id, is_uppercase_or_digit);
        if second <= 0 {
            return 1;
        }
        self.skip_whitespace();
        2
    }

    fn scan_airports(&mut self, departure: &mut Vec<u8>, arrival: &mut Vec<u8>) -> c_int {
        let first = self.scan_set(AIRPORT_CODE_LENGTH, departure, is_uppercase);
        if first <= 0 {
            return first;
        }
        self.skip_whitespace();
        let second = self.scan_set(AIRPORT_CODE_LENGTH, arrival, is_uppercase);
        if second <= 0 {
            return 1;
        }
        2
    }

    fn scan_comments(&mut self, comments: &mut Vec<u8>) -> c_int {
        self.scan_set(COMMENTS_LENGTH, comments, |byte| byte != b'\n')
    }
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_uppercase(byte: u8) -> bool {
    byte.is_ascii_uppercase()
}

fn is_uppercase_or_digit(byte: u8) -> bool {
    byte.is_ascii_uppercase() || byte.is_ascii_digit()
}

fn add_owned_directive(directives: &mut Vec<OwnedDirective>, new_directive: OwnedDirective) {
    let insertion_point = directives
        .iter()
        .position(|directive| directive.time_stamp > new_directive.time_stamp)
        .unwrap_or(directives.len());
    directives.insert(insertion_point, new_directive);
}

fn owned_superseded(directives: &[OwnedDirective], index: usize) -> bool {
    let directive = &directives[index];
    for later in &directives[index + 1..] {
        if later.luggage_id == directive.luggage_id {
            return later.departure == directive.departure;
        }
    }
    false
}

unsafe fn owned_matches(expected: *mut c_char, actual: &[u8]) -> bool {
    if expected.is_null() {
        segmentation_fault();
    }
    if unsafe { *expected } == b'-' as c_char {
        return true;
    }
    let mut cursor = expected;
    for &actual_byte in actual {
        let expected_byte = unsafe { *cursor };
        if expected_byte == 0 || expected_byte as u8 != actual_byte {
            return false;
        }
        cursor = unsafe { cursor.add(1) };
    }
    unsafe { *cursor == 0 }
}

fn run_main(arguments: &[*mut c_char]) -> c_int {
    let mut input = Vec::new();
    let _ = io::stdin().read_to_end(&mut input);
    let mut scanner = Scanner::new(input);
    let mut directives = Vec::new();

    loop {
        let mut time_stamp = 0_u32;
        let mut luggage_id = Vec::new();
        let mut flight_id = Vec::new();
        let mut departure = Vec::new();
        let mut arrival = Vec::new();
        let mut comments = Vec::new();

        if scanner.scan_timestamp(&mut time_stamp) == -1 {
            break;
        }
        if scanner.scan_ids(&mut luggage_id, &mut flight_id) == -1 {
            break;
        }
        if scanner.scan_airports(&mut departure, &mut arrival) == -1 {
            break;
        }
        if scanner.scan_comments(&mut comments) == -1 {
            break;
        }

        add_owned_directive(
            &mut directives,
            OwnedDirective {
                time_stamp,
                luggage_id,
                flight_id,
                departure,
                arrival,
                comments,
            },
        );
    }

    let mut output = Vec::new();
    for (index, directive) in directives.iter().enumerate() {
        if !owned_superseded(&directives, index)
            && unsafe { owned_matches(arguments[1], &directive.luggage_id) }
            && unsafe { owned_matches(arguments[2], &directive.flight_id) }
            && unsafe { owned_matches(arguments[3], &directive.departure) }
            && unsafe { owned_matches(arguments[4], &directive.arrival) }
        {
            let mut timestamp = String::new();
            write!(&mut timestamp, "{:010}", directive.time_stamp).unwrap();
            output.extend_from_slice(timestamp.as_bytes());
            output.push(b' ');
            output.extend_from_slice(&directive.luggage_id);
            output.push(b' ');
            output.extend_from_slice(&directive.flight_id);
            output.push(b' ');
            output.extend_from_slice(&directive.departure);
            output.push(b' ');
            output.extend_from_slice(&directive.arrival);
            output.push(b' ');
            output.extend_from_slice(&directive.comments);
            output.push(b'\n');
        }
    }
    let _ = io::stdout().write_all(&output);
    0
}

#[cfg(not(test))]
#[export_name = "main"]
pub unsafe extern "C" fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 5 {
        let _ = io::stderr().write_all(b"Command line error: 4 arguments expected\n");
        process::exit(1);
    }
    if argv.is_null() {
        segmentation_fault();
    }
    let arguments = unsafe { std::slice::from_raw_parts(argv, argc as usize) };
    let result = run_main(arguments);
    process::exit(result);
}
