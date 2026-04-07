use libloading::{Library, Symbol};
use std::ffi::c_char;
use std::os::raw::c_uint;
use std::path::PathBuf;
use std::ptr;

const LUGGAGE_ID_LENGTH: usize = 8;
const FLIGHT_ID_LENGTH: usize = 6;
const AIRPORT_CODE_LENGTH: usize = 3;
const COMMENTS_LENGTH: usize = 80;

#[repr(C)]
struct RoutingDirective {
    time_stamp: c_uint,
    luggage_id: [c_char; LUGGAGE_ID_LENGTH + 1],
    flight_id: [c_char; FLIGHT_ID_LENGTH + 1],
    departure: [c_char; AIRPORT_CODE_LENGTH + 1],
    arrival: [c_char; AIRPORT_CODE_LENGTH + 1],
    comments: [c_char; COMMENTS_LENGTH + 1],
    next_directive: *mut RoutingDirective,
}

impl RoutingDirective {
    fn new(ts: u32, lid: &str, fid: &str, dep: &str, arr: &str, com: &str) -> Box<Self> {
        let mut d = Box::new(RoutingDirective {
            time_stamp: ts,
            luggage_id: [0; LUGGAGE_ID_LENGTH + 1],
            flight_id: [0; FLIGHT_ID_LENGTH + 1],
            departure: [0; AIRPORT_CODE_LENGTH + 1],
            arrival: [0; AIRPORT_CODE_LENGTH + 1],
            comments: [0; COMMENTS_LENGTH + 1],
            next_directive: ptr::null_mut(),
        });
        copy_str(&mut d.luggage_id, lid);
        copy_str(&mut d.flight_id, fid);
        copy_str(&mut d.departure, dep);
        copy_str(&mut d.arrival, arr);
        copy_str(&mut d.comments, com);
        d
    }
}

fn copy_str(dst: &mut [c_char], src: &str) {
    for (i, b) in src.bytes().enumerate() {
        if i >= dst.len() - 1 { break; }
        dst[i] = b as c_char;
    }
}

fn c_str(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libluggage.so");
    p
}

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libluggage_c.so");
    p
}

type MatchesFn = unsafe extern "C" fn(*mut c_char, *mut c_char) -> i32;
type SupersedesFn = unsafe extern "C" fn(*mut RoutingDirective, *mut c_char, *mut c_char) -> i32;
type SupersededFn = unsafe extern "C" fn(*mut RoutingDirective) -> i32;
type AddFn = unsafe extern "C" fn(*mut RoutingDirective, *mut RoutingDirective);

struct Libs {
    c_lib: Library,
    rs_lib: Library,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            Libs {
                c_lib: Library::new(c_lib_path()).expect("load C .so"),
                rs_lib: Library::new(rust_lib_path()).expect("load Rust .so"),
            }
        }
    }
}

// ==================== matches ====================

#[test]
fn test_matches_exact() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<MatchesFn> = libs.c_lib.get(b"matches").unwrap();
        let rs_fn: Symbol<MatchesFn> = libs.rs_lib.get(b"matches").unwrap();

        let cases: &[(&str, &str)] = &[
            ("ABC", "ABC"),
            ("ABC", "DEF"),
            ("-", "ABC"),
            ("-XY", "ABC"),
            ("", ""),
            ("A", ""),
            ("", "A"),
        ];
        for (exp, act) in cases {
            let mut e = c_str(exp);
            let mut a = c_str(act);
            let c_res = c_fn(e.as_mut_ptr(), a.as_mut_ptr());
            let mut e = c_str(exp);
            let mut a = c_str(act);
            let rs_res = rs_fn(e.as_mut_ptr(), a.as_mut_ptr());
            assert_eq!(c_res, rs_res, "matches({:?}, {:?}): C={} Rust={}", exp, act, c_res, rs_res);
        }
    }
}

// ==================== supersedes ====================

#[test]
fn test_supersedes_null() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<SupersedesFn> = libs.c_lib.get(b"supersedes").unwrap();
        let rs_fn: Symbol<SupersedesFn> = libs.rs_lib.get(b"supersedes").unwrap();

        let mut lid = c_str("ABCD1234");
        let mut dep = c_str("JFK");
        let c_res = c_fn(ptr::null_mut(), lid.as_mut_ptr(), dep.as_mut_ptr());
        let mut lid = c_str("ABCD1234");
        let mut dep = c_str("JFK");
        let rs_res = rs_fn(ptr::null_mut(), lid.as_mut_ptr(), dep.as_mut_ptr());
        assert_eq!(c_res, rs_res, "supersedes(NULL): C={} Rust={}", c_res, rs_res);
    }
}

#[test]
fn test_supersedes_match_and_nomatch() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<SupersedesFn> = libs.c_lib.get(b"supersedes").unwrap();
        let rs_fn: Symbol<SupersedesFn> = libs.rs_lib.get(b"supersedes").unwrap();

        // Create a single-node list: same luggage_id, same departure => supersedes=1
        let mut d1 = RoutingDirective::new(100, "ABCD1234", "FL1234", "JFK", "LAX", "test");
        d1.next_directive = ptr::null_mut();

        let mut lid = c_str("ABCD1234");
        let mut dep = c_str("JFK");
        let c_res = c_fn(&mut *d1 as *mut _, lid.as_mut_ptr(), dep.as_mut_ptr());
        let mut lid = c_str("ABCD1234");
        let mut dep = c_str("JFK");
        let rs_res = rs_fn(&mut *d1 as *mut _, lid.as_mut_ptr(), dep.as_mut_ptr());
        assert_eq!(c_res, rs_res, "supersedes same lid+dep: C={} Rust={}", c_res, rs_res);
        assert_eq!(c_res, 1);

        // Same luggage_id, different departure => 0
        let mut dep2 = c_str("SFO");
        let c_res2 = c_fn(&mut *d1 as *mut _, lid.as_mut_ptr(), dep2.as_mut_ptr());
        let mut lid = c_str("ABCD1234");
        let mut dep2 = c_str("SFO");
        let rs_res2 = rs_fn(&mut *d1 as *mut _, lid.as_mut_ptr(), dep2.as_mut_ptr());
        assert_eq!(c_res2, rs_res2, "supersedes same lid diff dep: C={} Rust={}", c_res2, rs_res2);
        assert_eq!(c_res2, 0);

        // Different luggage_id => 0 (no more nodes)
        let mut lid3 = c_str("XXXX9999");
        let mut dep3 = c_str("JFK");
        let c_res3 = c_fn(&mut *d1 as *mut _, lid3.as_mut_ptr(), dep3.as_mut_ptr());
        let mut lid3 = c_str("XXXX9999");
        let mut dep3 = c_str("JFK");
        let rs_res3 = rs_fn(&mut *d1 as *mut _, lid3.as_mut_ptr(), dep3.as_mut_ptr());
        assert_eq!(c_res3, rs_res3, "supersedes diff lid: C={} Rust={}", c_res3, rs_res3);
        assert_eq!(c_res3, 0);
    }
}

// ==================== superseded ====================

#[test]
fn test_superseded() {
    let libs = Libs::load();
    unsafe {
        let c_fn: Symbol<SupersededFn> = libs.c_lib.get(b"superseded").unwrap();
        let rs_fn: Symbol<SupersededFn> = libs.rs_lib.get(b"superseded").unwrap();

        // d1 -> d2 (same luggage_id, same departure) => d1 is superseded
        let mut d2 = RoutingDirective::new(200, "ABCD1234", "FL5678", "JFK", "ORD", "");
        d2.next_directive = ptr::null_mut();
        let mut d1 = RoutingDirective::new(100, "ABCD1234", "FL1234", "JFK", "LAX", "");
        d1.next_directive = &mut *d2 as *mut _;

        let c_res = c_fn(&mut *d1 as *mut _);
        let rs_res = rs_fn(&mut *d1 as *mut _);
        assert_eq!(c_res, rs_res, "superseded (yes): C={} Rust={}", c_res, rs_res);
        assert_eq!(c_res, 1);

        // d1 alone => not superseded
        d1.next_directive = ptr::null_mut();
        let c_res2 = c_fn(&mut *d1 as *mut _);
        let rs_res2 = rs_fn(&mut *d1 as *mut _);
        assert_eq!(c_res2, rs_res2, "superseded (no): C={} Rust={}", c_res2, rs_res2);
        assert_eq!(c_res2, 0);
    }
}

// ==================== addRoutingDirectiveToList ====================

fn collect_timestamps(head: *mut RoutingDirective) -> Vec<u32> {
    let mut v = Vec::new();
    let mut p = unsafe { (*head).next_directive };
    while !p.is_null() {
        v.push(unsafe { (*p).time_stamp });
        p = unsafe { (*p).next_directive };
    }
    v
}

#[test]
fn test_add_routing_directive_ordering() {
    let libs = Libs::load();
    unsafe {
        let c_add: Symbol<AddFn> = libs.c_lib.get(b"addRoutingDirectiveToList").unwrap();
        let rs_add: Symbol<AddFn> = libs.rs_lib.get(b"addRoutingDirectiveToList").unwrap();

        // Build two identical lists by inserting in same order
        let timestamps = [50u32, 10, 30, 20, 40, 30];

        let mut c_head = RoutingDirective::new(0, "", "", "", "", "");
        c_head.next_directive = ptr::null_mut();
        let mut c_nodes: Vec<Box<RoutingDirective>> = Vec::new();

        let mut rs_head = RoutingDirective::new(0, "", "", "", "", "");
        rs_head.next_directive = ptr::null_mut();
        let mut rs_nodes: Vec<Box<RoutingDirective>> = Vec::new();

        for &ts in &timestamps {
            let mut cn = RoutingDirective::new(ts, "LUGGAGE1", "FL0001", "JFK", "LAX", "");
            cn.next_directive = ptr::null_mut();
            c_nodes.push(cn);
            let cp = &mut *c_nodes.last_mut().unwrap() as &mut RoutingDirective as *mut _;
            c_add(&mut *c_head as *mut _, cp);

            let mut rn = RoutingDirective::new(ts, "LUGGAGE1", "FL0001", "JFK", "LAX", "");
            rn.next_directive = ptr::null_mut();
            rs_nodes.push(rn);
            let rp = &mut *rs_nodes.last_mut().unwrap() as &mut RoutingDirective as *mut _;
            rs_add(&mut *rs_head as *mut _, rp);
        }

        let c_ts = collect_timestamps(&mut *c_head as *mut _);
        let rs_ts = collect_timestamps(&mut *rs_head as *mut _);
        assert_eq!(c_ts, rs_ts, "addRoutingDirectiveToList ordering: C={:?} Rust={:?}", c_ts, rs_ts);
    }
}

// ==================== printMatchingDirectives (capture stdout) ====================

#[test]
fn test_print_matching_directives() {
    use std::process::Command;

    // We test printMatchingDirectives indirectly by running both C and Rust
    // executables with the same input and comparing output.
    // Build a small helper that calls printMatchingDirectives via the .so.
    // Actually, the simplest approach: use the driver binaries with identical input.

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Build C executable
    let c_exe = manifest_dir.join("c_src/build/driver");
    if !c_exe.exists() {
        let _ = Command::new("cmake")
            .args(&["--build", "."])
            .current_dir(manifest_dir.join("c_src/build"))
            .output();
    }

    let rust_exe = manifest_dir.join("target/debug/driver");

    let input = "100 ABCD1234 FL1234 JFK LAX first route\n\
                 200 ABCD1234 FL5678 JFK ORD supersedes JFK\n\
                 150 EFGH5678 FL9999 SFO SEA another bag\n";

    // Test with wildcard luggage_id
    let args_sets: &[&[&str]] = &[
        &["-", "-", "-", "-"],
        &["ABCD1234", "-", "-", "-"],
        &["-", "FL9999", "-", "-"],
        &["-", "-", "JFK", "-"],
        &["-", "-", "-", "LAX"],
        &["EFGH5678", "FL9999", "SFO", "SEA"],
        &["ZZZZZZZZ", "-", "-", "-"],
    ];

    for args in args_sets {
        let c_out = Command::new(&c_exe)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
                child.wait_with_output()
            })
            .expect("run C driver");

        let rs_out = Command::new(&rust_exe)
            .args(*args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
                child.wait_with_output()
            })
            .expect("run Rust driver");

        let c_stdout = String::from_utf8_lossy(&c_out.stdout);
        let rs_stdout = String::from_utf8_lossy(&rs_out.stdout);
        assert_eq!(
            c_stdout, rs_stdout,
            "printMatchingDirectives mismatch for args {:?}:\nC:  {:?}\nRust: {:?}",
            args, c_stdout, rs_stdout
        );
    }
}
