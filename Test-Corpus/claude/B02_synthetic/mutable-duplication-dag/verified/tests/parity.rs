// Parity tests: verify C and Rust .so produce identical results across the FFI
// boundary. The Rust .so MUST mirror the C .so byte-for-byte.

mod common;

use common::*;
use std::os::raw::c_int;

/// Run a closure against both C and Rust libraries and assert their results match.
fn for_each_lib<F>(mut f: F)
where
    F: FnMut(&str, &DagLib),
{
    let (c_path, rust_path) = lib_paths();
    let c = DagLib::load(&c_path);
    f("C", &c);
    let r = DagLib::load(&rust_path);
    f("Rust", &r);
}

#[test]
fn test_create_graph_initial_state() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            assert!(!g.is_null(), "{}: create_graph returned NULL", name);
            assert_eq!(
                (*g).node_count, 0,
                "{}: new graph should have node_count=0",
                name
            );
            for i in 0..MAX_NODES {
                assert!(
                    (*g).nodes[i].is_null(),
                    "{}: nodes[{}] should be NULL",
                    name,
                    i
                );
            }
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_node_basic() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let nyc = cstr("New York");
            let n = (lib.add_node)(g, nyc.as_ptr() as *const _);
            assert!(!n.is_null(), "{}: add_node returned NULL", name);
            assert_eq!((*g).node_count, 1, "{}", name);
            assert_eq!((*n).ref_count, 1, "{}", name);
            assert_eq!((*n).edge_count, 0, "{}", name);
            assert_eq!(city_name_bytes(&(*n).city_name), b"New York", "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_node_duplicate() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let nyc = cstr("New York");
            let n1 = (lib.add_node)(g, nyc.as_ptr() as *const _);
            assert!(!n1.is_null(), "{}", name);
            let n2 = (lib.add_node)(g, nyc.as_ptr() as *const _);
            assert!(n2.is_null(), "{}: duplicate should return NULL", name);
            assert_eq!((*g).node_count, 1, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_node_truncation() {
    // Long name should be truncated to MAX_CITY_NAME-1 = 63 bytes.
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let long: String = "A".repeat(80);
            let cs = cstr(&long);
            let n = (lib.add_node)(g, cs.as_ptr() as *const _);
            assert!(!n.is_null(), "{}", name);
            let bytes = city_name_bytes(&(*n).city_name);
            assert_eq!(bytes.len(), 63, "{}: truncated to 63 bytes", name);
            assert!(bytes.iter().all(|&b| b == b'A'), "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_edge_basic() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            let r = (lib.add_edge)(a, b, 5);
            assert_eq!(r, 0, "{}", name);
            assert_eq!((*a).edge_count, 1, "{}", name);
            assert_eq!((*a).edges[0].destination, b, "{}", name);
            assert_eq!((*a).edges[0].distance, 5, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_edge_negative_distance() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            let r = (lib.add_edge)(a, b, -1);
            assert_eq!(r, -1, "{}", name);
            assert_eq!((*a).edge_count, 0, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_edge_null() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let r1 = (lib.add_edge)(std::ptr::null_mut(), a, 5);
            let r2 = (lib.add_edge)(a, std::ptr::null_mut(), 5);
            assert_eq!(r1, -1, "{}", name);
            assert_eq!(r2, -1, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_edge_duplicate() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            assert_eq!((lib.add_edge)(a, b, 5), 0);
            let r = (lib.add_edge)(a, b, 7);
            assert_eq!(r, -1, "{}: duplicate edge", name);
            assert_eq!((*a).edge_count, 1, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_add_edge_max() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let mut targets = vec![];
            for i in 0..(MAX_EDGES + 2) {
                let n = (lib.add_node)(g, cstr(&format!("N{}", i)).as_ptr() as *const _);
                targets.push(n);
            }
            for i in 0..MAX_EDGES {
                let r = (lib.add_edge)(a, targets[i], i as c_int + 1);
                assert_eq!(r, 0, "{}: edge {}", name, i);
            }
            // adding one more should fail
            let r = (lib.add_edge)(a, targets[MAX_EDGES], 100);
            assert_eq!(r, -1, "{}", name);
            assert_eq!((*a).edge_count, MAX_EDGES as c_int, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_get_node_by_name_found_and_not() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let _ = (lib.add_node)(g, cstr("Boston").as_ptr() as *const _);
            let _ = (lib.add_node)(g, cstr("Atlanta").as_ptr() as *const _);
            let f = (lib.get_node_by_name)(g, cstr("Boston").as_ptr() as *const _);
            assert!(!f.is_null(), "{}", name);
            assert_eq!(city_name_bytes(&(*f).city_name), b"Boston", "{}", name);
            let nf = (lib.get_node_by_name)(g, cstr("Nowhere").as_ptr() as *const _);
            assert!(nf.is_null(), "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_delete_node_decrements_ref() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            // Bump ref count via shallow_copy so we can observe decrement
            // without triggering free.
            (lib.shallow_copy)(a);
            assert_eq!((*a).ref_count, 2, "{}", name);
            (lib.delete_node)(a);
            assert_eq!((*a).ref_count, 1, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_shallow_copy_increments_reachable() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            let c = (lib.add_node)(g, cstr("C").as_ptr() as *const _);
            (lib.add_edge)(a, b, 1);
            (lib.add_edge)(b, c, 2);
            assert_eq!((*a).ref_count, 1, "{}", name);
            assert_eq!((*b).ref_count, 1, "{}", name);
            assert_eq!((*c).ref_count, 1, "{}", name);
            let r = (lib.shallow_copy)(a);
            assert_eq!(r, a, "{}", name);
            assert_eq!((*a).ref_count, 2, "{}", name);
            assert_eq!((*b).ref_count, 2, "{}", name);
            assert_eq!((*c).ref_count, 2, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_shallow_copy_with_cycle() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            (lib.add_edge)(a, b, 1);
            (lib.add_edge)(b, a, 1);
            (lib.shallow_copy)(a);
            assert_eq!((*a).ref_count, 2, "{}", name);
            assert_eq!((*b).ref_count, 2, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_shallow_copy_null() {
    for_each_lib(|name, lib| {
        unsafe {
            let r = (lib.shallow_copy)(std::ptr::null_mut());
            assert!(r.is_null(), "{}", name);
        }
    });
}

#[test]
fn test_find_shortest_path_simple() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            let c = (lib.add_node)(g, cstr("C").as_ptr() as *const _);
            (lib.add_edge)(a, b, 5);
            (lib.add_edge)(b, c, 3);
            (lib.add_edge)(a, c, 100);
            let mut len: c_int = 0;
            let p = (lib.find_shortest_path)(a, c, &mut len);
            assert!(!p.is_null(), "{}", name);
            assert_eq!(len, 3, "{}", name);
            assert_eq!(*p.offset(0), a, "{}", name);
            assert_eq!(*p.offset(1), b, "{}", name);
            assert_eq!(*p.offset(2), c, "{}", name);
            libc_free(p as *mut _);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_find_shortest_path_self() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let mut len: c_int = 0;
            let p = (lib.find_shortest_path)(a, a, &mut len);
            assert!(!p.is_null(), "{}", name);
            assert_eq!(len, 1, "{}", name);
            assert_eq!(*p.offset(0), a, "{}", name);
            libc_free(p as *mut _);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_find_shortest_path_no_path() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            let mut len: c_int = 12345;
            let p = (lib.find_shortest_path)(a, b, &mut len);
            assert!(p.is_null(), "{}", name);
            assert_eq!(len, 0, "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_find_shortest_path_null_inputs() {
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let mut len: c_int = 0;
            let r1 = (lib.find_shortest_path)(std::ptr::null_mut(), a, &mut len);
            assert!(r1.is_null(), "{}", name);
            let r2 = (lib.find_shortest_path)(a, std::ptr::null_mut(), &mut len);
            assert!(r2.is_null(), "{}", name);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_find_shortest_path_complex() {
    // Diamond shape with multiple equal/competing paths.
    for_each_lib(|name, lib| {
        unsafe {
            let g = (lib.create_graph)();
            let a = (lib.add_node)(g, cstr("A").as_ptr() as *const _);
            let b = (lib.add_node)(g, cstr("B").as_ptr() as *const _);
            let c = (lib.add_node)(g, cstr("C").as_ptr() as *const _);
            let d = (lib.add_node)(g, cstr("D").as_ptr() as *const _);
            let e = (lib.add_node)(g, cstr("E").as_ptr() as *const _);
            (lib.add_edge)(a, b, 1);
            (lib.add_edge)(a, c, 4);
            (lib.add_edge)(b, c, 2);
            (lib.add_edge)(b, d, 6);
            (lib.add_edge)(c, d, 3);
            (lib.add_edge)(d, e, 1);
            (lib.add_edge)(c, e, 8);
            let mut len: c_int = 0;
            let p = (lib.find_shortest_path)(a, e, &mut len);
            assert!(!p.is_null(), "{}", name);
            // Expected path: A -> B -> C -> D -> E (1+2+3+1=7)
            assert_eq!(len, 5, "{}", name);
            let names: Vec<Vec<u8>> = (0..len)
                .map(|i| city_name_bytes(&(**p.offset(i as isize)).city_name))
                .collect();
            assert_eq!(
                names,
                vec![
                    b"A".to_vec(),
                    b"B".to_vec(),
                    b"C".to_vec(),
                    b"D".to_vec(),
                    b"E".to_vec()
                ],
                "{}",
                name
            );
            libc_free(p as *mut _);
            (lib.free_graph)(g);
        }
    });
}

#[test]
fn test_print_outputs_match() {
    // All print_* parity checks live in one test so a single stdout-redirect
    // critical section runs end-to-end, avoiding races with the test
    // framework's own writes to fd 1.
    let (c_path, r_path) = lib_paths();
    let c_lib = DagLib::load(&c_path);
    let r_lib = DagLib::load(&r_path);

    let print_node_scenario = |lib: &DagLib| unsafe {
        let g = (lib.create_graph)();
        let a = (lib.add_node)(g, cstr("Boston").as_ptr() as *const _);
        let b = (lib.add_node)(g, cstr("NYC").as_ptr() as *const _);
        (lib.add_edge)(a, b, 200);
        (lib.print_node)(a);
        (lib.free_graph)(g);
    };

    let print_graph_scenario = |lib: &DagLib| unsafe {
        let g = (lib.create_graph)();
        let a = (lib.add_node)(g, cstr("Alpha").as_ptr() as *const _);
        let b = (lib.add_node)(g, cstr("Beta").as_ptr() as *const _);
        let c = (lib.add_node)(g, cstr("Gamma").as_ptr() as *const _);
        (lib.add_edge)(a, b, 1);
        (lib.add_edge)(b, c, 2);
        (lib.add_edge)(a, c, 100);
        (lib.print_graph)(g);
        (lib.free_graph)(g);
    };

    let null_node_scenario = |lib: &DagLib| unsafe {
        (lib.print_node)(std::ptr::null_mut());
    };
    let null_graph_scenario = |lib: &DagLib| unsafe {
        (lib.print_graph)(std::ptr::null_mut());
    };

    let pairs: Vec<(&str, Box<dyn Fn(&DagLib)>)> = vec![
        ("print_node", Box::new(print_node_scenario)),
        ("print_graph", Box::new(print_graph_scenario)),
        ("print_node(NULL)", Box::new(null_node_scenario)),
        ("print_graph(NULL)", Box::new(null_graph_scenario)),
    ];

    for (label, scenario) in pairs.iter() {
        let c_out = capture_stdout(|| scenario(&c_lib));
        let r_out = capture_stdout(|| scenario(&r_lib));
        assert_eq!(
            c_out,
            r_out,
            "{} output mismatch:\n C: {:?}\n R: {:?}",
            label,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out),
        );
    }
}

// ----- helpers -----

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

unsafe fn libc_free(p: *mut std::ffi::c_void) {
    free(p);
}

use std::sync::Mutex;
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Capture bytes written to stdout by `f`. Both C printf and Rust println write
/// to fd 1, so we redirect fd 1 to a pipe, run f, fflush, then read the pipe.
/// Uses a global mutex to serialize captures across parallel tests, since
/// redirecting fd 1 is a process-wide operation.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let mut fds: [i32; 2] = [0; 2];
        if pipe(fds.as_mut_ptr()) != 0 {
            panic!("pipe failed");
        }
        let saved = dup(1);
        if saved < 0 {
            panic!("dup failed");
        }
        // Flush whatever is buffered for the original stdout first so we don't
        // capture leftover bytes.
        fflush(std::ptr::null_mut());
        if dup2(fds[1], 1) < 0 {
            panic!("dup2 failed");
        }
        close(fds[1]);
        f();
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(fds[0], buf.as_mut_ptr(), buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
            if (n as usize) < buf.len() {
                break;
            }
        }
        close(fds[0]);
        out
    }
}
