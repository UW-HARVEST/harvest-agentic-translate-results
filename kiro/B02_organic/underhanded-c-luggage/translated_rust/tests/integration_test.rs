use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_uint};
use std::process::{Command, Stdio};
use std::io::Write;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libluggage.so");
const C_BIN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/driver");

// C struct layout — must match exactly
#[repr(C)]
struct RoutingDirective {
    time_stamp: c_uint,
    luggage_id: [c_char; 9],   // LUGGAGE_ID_LENGTH + 1
    flight_id: [c_char; 7],    // FLIGHT_ID_LENGTH + 1
    departure: [c_char; 4],    // AIRPORT_CODE_LENGTH + 1
    arrival: [c_char; 4],      // AIRPORT_CODE_LENGTH + 1
    comments: [c_char; 81],    // COMMENTS_LENGTH + 1
    next_directive: *mut RoutingDirective,
}

fn make_c_str_buf<const N: usize>(s: &str) -> [c_char; N] {
    let mut buf = [0i8; N];
    for (i, b) in s.bytes().enumerate() {
        if i >= N - 1 { break; }
        buf[i] = b as c_char;
    }
    buf
}

fn new_directive(ts: u32, lid: &str, fid: &str, dep: &str, arr: &str, com: &str) -> Box<RoutingDirective> {
    Box::new(RoutingDirective {
        time_stamp: ts,
        luggage_id: make_c_str_buf::<9>(lid),
        flight_id: make_c_str_buf::<7>(fid),
        departure: make_c_str_buf::<4>(dep),
        arrival: make_c_str_buf::<4>(arr),
        comments: make_c_str_buf::<81>(com),
        next_directive: std::ptr::null_mut(),
    })
}

// ---- Test: matches() ----
#[test]
fn test_matches() {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let c_matches: Symbol<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int> =
            lib.get(b"matches").unwrap();

        // Rust version
        fn rust_matches(expected: &str, actual: &str) -> bool {
            expected.starts_with('-') || expected == actual
        }

        let cases = vec![
            ("-", "ABC", true),
            ("-anything", "XYZ", true),
            ("ABC", "ABC", true),
            ("ABC", "DEF", false),
            ("", "", true),
            ("A", "B", false),
        ];

        for (expected, actual, want) in &cases {
            let e = CString::new(*expected).unwrap();
            let a = CString::new(*actual).unwrap();
            let c_result = c_matches(e.as_ptr(), a.as_ptr()) != 0;
            let r_result = rust_matches(expected, actual);
            assert_eq!(c_result, *want, "C matches({:?}, {:?})", expected, actual);
            assert_eq!(r_result, *want, "Rust matches({:?}, {:?})", expected, actual);
            assert_eq!(c_result, r_result, "C vs Rust mismatch for matches({:?}, {:?})", expected, actual);
        }
    }
}

// ---- Test: supersedes() on a linked list ----
#[test]
fn test_supersedes() {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let c_supersedes: Symbol<unsafe extern "C" fn(*mut RoutingDirective, *mut c_char, *mut c_char) -> c_int> =
            lib.get(b"supersedes").unwrap();

        // Build a linked list: d1 -> d2 -> null
        // d1: luggage_id=AAAA1111, departure=JFK
        // d2: luggage_id=AAAA1111, departure=LAX
        let mut d2 = new_directive(200, "AAAA1111", "UA100", "LAX", "SFO", "");
        let mut d1 = new_directive(100, "AAAA1111", "UA200", "JFK", "LAX", "");
        d1.next_directive = &mut *d2 as *mut RoutingDirective;

        // supersedes(d1, "AAAA1111", "JFK") should be true (d1 has same luggage_id and departure)
        let lid = CString::new("AAAA1111").unwrap();
        let dep_jfk = CString::new("JFK").unwrap();
        let dep_lax = CString::new("LAX").unwrap();
        let dep_ord = CString::new("ORD").unwrap();

        // From d1: luggage_id matches, departure JFK matches d1 -> true
        let r = c_supersedes(&mut *d1 as *mut _, lid.as_ptr() as *mut _, dep_jfk.as_ptr() as *mut _);
        assert_eq!(r, 1, "d1 should supersede AAAA1111/JFK");

        // From d1: luggage_id matches d1, but departure LAX != JFK for d1, so returns 0 (C returns 0 after first luggage_id match with wrong departure)
        // Wait - C supersedes: if luggage_id matches but departure doesn't, it returns 0 (not continuing)
        let r = c_supersedes(&mut *d1 as *mut _, lid.as_ptr() as *mut _, dep_lax.as_ptr() as *mut _);
        // C code: strcmp(d1->luggage_id, "AAAA1111") == 0, strcmp(d1->departure "JFK", "LAX") != 0, return 0
        assert_eq!(r, 0, "d1 should NOT supersede AAAA1111/LAX (first match has wrong departure)");

        // From d2: luggage_id matches, departure LAX matches -> true
        let r = c_supersedes(&mut *d2 as *mut _, lid.as_ptr() as *mut _, dep_lax.as_ptr() as *mut _);
        assert_eq!(r, 1, "d2 should supersede AAAA1111/LAX");

        // Unknown luggage_id
        let lid2 = CString::new("BBBB2222").unwrap();
        let r = c_supersedes(&mut *d1 as *mut _, lid2.as_ptr() as *mut _, dep_jfk.as_ptr() as *mut _);
        assert_eq!(r, 0, "no match for BBBB2222");

        // NULL directive
        let r = c_supersedes(std::ptr::null_mut(), lid.as_ptr() as *mut _, dep_jfk.as_ptr() as *mut _);
        assert_eq!(r, 0, "NULL should return 0");

        // Now test Rust supersedes
        // Rust supersedes(list, start, luggage_id, departure) iterates from start..
        // The Rust code's supersedes skips non-matching luggage_ids with continue,
        // then checks departure. If departure matches, returns true. If not, returns false.
        // This matches C behavior: C recurses to next on luggage_id mismatch, returns 0 on departure mismatch.

        // Build equivalent Rust data
        use std::io::Read;
        // We can't easily call the Rust functions from main.rs in integration tests
        // since they're in a binary. We'll test via binary output comparison instead.
    }
}

// ---- Test: addRoutingDirectiveToList ----
#[test]
fn test_add_routing_directive_to_list() {
    unsafe {
        let lib = Library::new(C_LIB_PATH).expect("Failed to load C library");
        let c_add: Symbol<unsafe extern "C" fn(*mut RoutingDirective, *mut RoutingDirective)> =
            lib.get(b"addRoutingDirectiveToList").unwrap();

        // Create head with time_stamp=0, next=null
        let mut head = new_directive(0, "", "", "", "", "");

        // Add directives with timestamps 300, 100, 200
        let mut d1 = new_directive(300, "AAAA1111", "UA100", "JFK", "LAX", "first");
        let mut d2 = new_directive(100, "BBBB2222", "DL200", "ORD", "SFO", "second");
        let mut d3 = new_directive(200, "CCCC3333", "AA300", "LAX", "JFK", "third");

        c_add(&mut *head as *mut _, &mut *d1 as *mut _);
        c_add(&mut *head as *mut _, &mut *d2 as *mut _);
        c_add(&mut *head as *mut _, &mut *d3 as *mut _);

        // Walk the list and collect timestamps - should be sorted: 100, 200, 300
        let mut timestamps = Vec::new();
        let mut cur = head.next_directive;
        while !cur.is_null() {
            timestamps.push((*cur).time_stamp);
            cur = (*cur).next_directive;
        }
        assert_eq!(timestamps, vec![100, 200, 300], "C list should be sorted by time_stamp");
    }
}

// ---- Test: binary output comparison ----
fn run_c_driver(input: &str, args: &[&str]) -> Vec<u8> {
    let mut child = Command::new(C_BIN_PATH)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn C driver");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    output.stdout
}

fn run_rust_driver(input: &str, args: &[&str]) -> Vec<u8> {
    // Find the Rust binary - cargo test builds it
    let rust_bin = env!("CARGO_BIN_EXE_driver");
    let mut child = Command::new(rust_bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn Rust driver");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    output.stdout
}

#[test]
fn test_binary_single_directive() {
    let input = "100 AAAA1111 UA1234 JFK LAX checked bag\n";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "Single directive output mismatch"
    );
}

#[test]
fn test_binary_multiple_directives_sorted() {
    let input = "\
300 AAAA1111 UA1234 JFK LAX first bag
100 BBBB2222 DL5678 ORD SFO second bag
200 CCCC3333 AA9012 LAX JFK third bag
";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "Multiple directives output mismatch"
    );
}

#[test]
fn test_binary_superseded_directive() {
    // Two directives for same luggage_id and departure, different timestamps
    // The earlier one should be superseded by the later one
    let input = "\
100 AAAA1111 UA1234 JFK LAX first
200 AAAA1111 DL5678 JFK SFO second
";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "Superseded directive output mismatch"
    );
}

#[test]
fn test_binary_filter_by_luggage_id() {
    let input = "\
100 AAAA1111 UA1234 JFK LAX bag1
200 BBBB2222 DL5678 ORD SFO bag2
";
    let args = ["AAAA1111", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "Filter by luggage_id mismatch"
    );
}

#[test]
fn test_binary_filter_by_departure() {
    let input = "\
100 AAAA1111 UA1234 JFK LAX bag1
200 BBBB2222 DL5678 ORD SFO bag2
";
    let args = ["-", "-", "JFK", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "Filter by departure mismatch"
    );
}

#[test]
fn test_binary_empty_input() {
    let input = "";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "Empty input output mismatch"
    );
}

#[test]
fn test_binary_no_comments() {
    let input = "100 AAAA1111 UA1234 JFK LAX\n";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "No comments output mismatch"
    );
}

#[test]
fn test_binary_complex_superseding() {
    let input = "\
100 AAAA1111 UA1234 JFK LAX first
200 AAAA1111 DL5678 JFK SFO supersedes first
300 BBBB2222 AA9012 ORD LAX standalone
400 AAAA1111 UA4444 LAX JFK different departure
";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(c_out, r_out, "Complex superseding output mismatch");
}

#[test]
fn test_binary_supersedes_stops_at_first_match() {
    // d1(ts=100, AAAA1111, JFK), d2(ts=200, AAAA1111, LAX), d3(ts=300, AAAA1111, JFK)
    // superseded(d1) checks from d2: d2 has same luggage_id but different departure -> return 0
    // So d1 is NOT superseded even though d3 has same luggage_id and departure
    let input = "\
100 AAAA1111 UA1234 JFK LAX first
200 AAAA1111 DL5678 LAX SFO middle
300 AAAA1111 AA9012 JFK ORD third
";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(c_out, r_out, "Supersedes stop-at-first-match mismatch");
}

#[test]
fn test_binary_extra_whitespace() {
    let input = "  100   AAAA1111   UA1234   JFK   LAX   extra spaces\n";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(c_out, r_out, "Extra whitespace output mismatch");
}

#[test]
fn test_binary_no_trailing_newline() {
    let input = "100 AAAA1111 UA1234 JFK LAX first\n200 BBBB2222 DL5678 ORD SFO second";
    let args = ["-", "-", "-", "-"];
    let c_out = run_c_driver(input, &args);
    let r_out = run_rust_driver(input, &args);
    assert_eq!(c_out, r_out, "No trailing newline output mismatch");
}
