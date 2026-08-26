// Differential tests for `printMatchingDirectives` through the shared-library
// boundary.
//
// These live in their own test binary because they redirect file descriptor 1 in
// order to capture what the C library's `printf` writes; running them alongside
// other tests in the same process would let unrelated output leak into the
// capture.  Everything is driven from a single `#[test]` so no other test of
// this binary can run concurrently.

mod support;

use std::ffi::c_char;
use support::ffi::*;
use support::*;

#[test]
fn f02_and_f15_print_matching_directives() {
    f02_print_null_list();
    f15_print_random();
}

/// ERRORS.md row 34: `printMatchingDirectives(NULL, ...)` prints nothing.
fn f02_print_null_list() {
    let libc = libc();
    let (cl, rl) = libs();
    let c: FnPrint = sym(&cl, b"printMatchingDirectives\0");
    let r: FnPrint = sym(&rl, b"printMatchingDirectives\0");
    let dash = cs(b"-");
    let p = dash.as_ptr() as *const c_char;
    let cout = capture_stdout(&libc, || unsafe { c(std::ptr::null_mut(), p, p, p, p) });
    let rout = capture_stdout(&libc, || unsafe { r(std::ptr::null_mut(), p, p, p, p) });
    assert!(cout.is_empty(), "C printed {:?} for an empty list", esc(&cout));
    assert_eq!(cout, rout);
}

/// CONFIGS.md row 35: `printMatchingDirectives` with randomized lists, filters
/// and superseded nodes; stdout is captured through file descriptor 1.
fn f15_print_random() {
    let libc = libc();
    let (cl, rl) = libs();
    let c: FnPrint = sym(&cl, b"printMatchingDirectives\0");
    let r: FnPrint = sym(&rl, b"printMatchingDirectives\0");
    let mut rng = Rng::new(0xf015);
    let mut printed_lines = 0usize;
    for iter in 0..400 {
        let lugs = rng.pool(1, 3, b"AB1", 3);
        let deps = rng.pool(1, 3, b"XY", 3);
        let n = rng.below(9);
        let mut nodes_c: Vec<Node> = (0..n)
            .map(|_| gen_node(&mut rng, &lugs, &deps, 10))
            .collect();
        // keep the list sorted by timestamp like the real program does
        nodes_c.sort_by_key(|x| x.time_stamp);
        let mut nodes_r = nodes_c.clone();
        let head_c = link(&mut nodes_c);
        let head_r = link(&mut nodes_r);

        let mut filters: Vec<Vec<u8>> = Vec::new();
        for k in 0..4 {
            let f = match rng.below(6) {
                0..=2 => b"-".to_vec(),
                3 => {
                    if n == 0 {
                        b"-".to_vec()
                    } else {
                        let node = &nodes_c[rng.below(n)];
                        let field: &[u8] = match k {
                            0 => &node.luggage_id,
                            1 => &node.flight_id,
                            2 => &node.departure,
                            _ => &node.arrival,
                        };
                        let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
                        field[..end].to_vec()
                    }
                }
                4 => Vec::new(),
                _ => gen_ffi_field(&mut rng, 4),
            };
            filters.push(f);
        }
        let f: Vec<Vec<u8>> = filters.iter().map(|x| cs(x)).collect();
        let (p0, p1, p2, p3) = (
            f[0].as_ptr() as *const c_char,
            f[1].as_ptr() as *const c_char,
            f[2].as_ptr() as *const c_char,
            f[3].as_ptr() as *const c_char,
        );
        let cout = capture_stdout(&libc, || unsafe { c(head_c, p0, p1, p2, p3) });
        let rout = capture_stdout(&libc, || unsafe { r(head_r, p0, p1, p2, p3) });
        assert_eq!(
            esc(&cout),
            esc(&rout),
            "iter {}: printMatchingDirectives differs for {} nodes, filters [{}, {}, {}, {}]",
            iter,
            n,
            esc(&filters[0]),
            esc(&filters[1]),
            esc(&filters[2]),
            esc(&filters[3])
        );
        printed_lines += cout.iter().filter(|&&b| b == b'\n').count();
    }
    // Guards against a silently broken capture: the randomized lists must have
    // produced a substantial amount of output.
    assert!(
        printed_lines > 100,
        "suspiciously little captured output ({} lines) — is the fd redirection working?",
        printed_lines
    );
}
