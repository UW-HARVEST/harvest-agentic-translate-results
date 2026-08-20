// Differential tests through the shared-library boundary.
//
// BOTH libraries are loaded with `libloading` and only their exported C symbols
// are called — the Rust side is never invoked as a Rust function:
//
//   * `cbuild/libluggage.so` — the pristine `c_src/src/luggage.c`, compiled with
//     `gcc -shared -fPIC -O0 -Dmain=luggage_main` (compiler flags only, `c_src/`
//     is not modified),
//   * `target/<profile>/libdriver.so` — the Rust translation's `cdylib`, which
//     exports the same six symbols and delegates to the same code the `driver`
//     executable runs.
//
// This exercises the lowest-level entry points (`addRoutingDirectiveToList`,
// `supersedes`, `superseded`, `matches`, `printMatchingDirectives`) directly,
// with node lists that the stdin parser could never build (lower-case fields,
// bytes past the NUL terminator, arbitrary timestamps, ...).

mod support;

use std::ffi::c_char;
use std::path::Path;
use support::ffi::*;
use support::*;

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================

#[test]
fn symbol_parity_c_so_vs_rust_so() {
    fn exported(path: &Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("run nm");
        assert!(out.status.success(), "nm failed on {:?}", path);
        let text = String::from_utf8_lossy(&out.stdout);
        let mut syms = Vec::new();
        for line in text.lines() {
            let mut it = line.split_whitespace();
            let (kind, name) = match (it.next(), it.next(), it.next()) {
                (Some(_addr), Some(k), Some(n)) => (k.to_string(), n.to_string()),
                (Some(k), Some(n), None) => (k.to_string(), n.to_string()),
                _ => continue,
            };
            // Only code symbols; drop toolchain / libc / unwinder noise.
            if kind != "T" && kind != "i" {
                continue;
            }
            let base = name.split('@').next().unwrap_or(&name).to_string();
            if base.starts_with("_ITM_")
                || base.starts_with("__cxa")
                || base.starts_with("_Unwind")
                || base.starts_with("__gmon")
                || base == "_init"
                || base == "_fini"
                || base == "rust_eh_personality"
            {
                continue;
            }
            syms.push(base);
        }
        syms.sort();
        syms.dedup();
        syms
    }

    let c = exported(c_so());
    let r = exported(rust_so());
    let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {:?}\n  C:    {:?}\n  Rust: {:?}",
        missing,
        c,
        r
    );
    // The six functions of luggage.c must all be there.
    for want in [
        "addRoutingDirectiveToList",
        "supersedes",
        "superseded",
        "matches",
        "printMatchingDirectives",
        "luggage_main",
    ] {
        assert!(c.contains(&want.to_string()), "C .so missing {}", want);
        assert!(r.contains(&want.to_string()), "Rust .so missing {}", want);
    }
}

#[test]
fn struct_layout_matches_c() {
    assert_eq!(std::mem::size_of::<Node>(), 120);
    assert_eq!(std::mem::align_of::<Node>(), 8);
    let n = Node::zeroed();
    let base = &n as *const Node as usize;
    assert_eq!(&n.time_stamp as *const _ as usize - base, 0);
    assert_eq!(&n.luggage_id as *const _ as usize - base, 4);
    assert_eq!(&n.flight_id as *const _ as usize - base, 13);
    assert_eq!(&n.departure as *const _ as usize - base, 20);
    assert_eq!(&n.arrival as *const _ as usize - base, 24);
    assert_eq!(&n.comments as *const _ as usize - base, 28);
    assert_eq!(&n.next as *const _ as usize - base, 112);
}

// ===========================================================================
// Phase C rows exercised through the .so boundary
// ===========================================================================

/// ERRORS.md row 32: `supersedes(NULL, ...)` returns 0.
#[test]
fn f01_supersedes_null() {
    let (cl, rl) = libs();
    let c: FnSupersedes = sym(&cl, b"supersedes\0");
    let r: FnSupersedes = sym(&rl, b"supersedes\0");
    for (lug, dep) in [
        (&b"ABC"[..], &b"AAA"[..]),
        (b"", b""),
        (b"ZZZZZZZZ", b"ZZZ"),
        (b"a", b"-"),
    ] {
        let l = cs(lug);
        let d = cs(dep);
        let cv = unsafe { c(std::ptr::null_mut(), l.as_ptr() as *const c_char, d.as_ptr() as *const c_char) };
        let rv = unsafe { r(std::ptr::null_mut(), l.as_ptr() as *const c_char, d.as_ptr() as *const c_char) };
        assert_eq!(cv, 0, "C supersedes(NULL) must return 0");
        assert_eq!(cv, rv, "supersedes(NULL, {:?}, {:?})", lug, dep);
    }
}

// ===========================================================================
// Phase B rows exercised through the .so boundary
// ===========================================================================

/// CONFIGS.md row 30: `matches`.
#[test]
fn f10_matches_random() {
    let (cl, rl) = libs();
    let c: FnMatches = sym(&cl, b"matches\0");
    let r: FnMatches = sym(&rl, b"matches\0");

    let fixed: [(&[u8], &[u8]); 16] = [
        (b"-", b"ABC"),
        (b"-", b""),
        (b"-x", b"ABC"),
        (b"--", b"--"),
        (b"", b""),
        (b"", b"A"),
        (b"A", b""),
        (b"ABC", b"ABC"),
        (b"ABC", b"ABD"),
        (b"abc", b"ABC"),
        (b"ABCDEFGH", b"ABCDEFGH"),
        (b"ABCDEFGH", b"ABCDEFG"),
        (b"\xff", b"\xff"),
        (b"\xff", b"\xfe"),
        (b"A-", b"A-"),
        (b"-", b"-"),
    ];
    for (e, a) in fixed.iter() {
        let ec = cs(e);
        let ac = cs(a);
        let cv = unsafe { c(ec.as_ptr() as *const c_char, ac.as_ptr() as *const c_char) };
        let rv = unsafe { r(ec.as_ptr() as *const c_char, ac.as_ptr() as *const c_char) };
        assert_eq!(cv, rv, "matches({:?}, {:?}): C={} Rust={}", e, a, cv, rv);
    }

    let mut rng = Rng::new(0xf010);
    for _ in 0..4000 {
        let a = gen_ffi_field(&mut rng, 8);
        let e = match rng.below(5) {
            0 => b"-".to_vec(),
            1 => a.clone(),
            2 => {
                let mut v = b"-".to_vec();
                v.extend(gen_ffi_field(&mut rng, 4));
                v
            }
            3 => Vec::new(),
            _ => gen_ffi_field(&mut rng, 8),
        };
        let ec = cs(&e);
        let ac = cs(&a);
        let cv = unsafe { c(ec.as_ptr() as *const c_char, ac.as_ptr() as *const c_char) };
        let rv = unsafe { r(ec.as_ptr() as *const c_char, ac.as_ptr() as *const c_char) };
        assert_eq!(
            cv,
            rv,
            "matches({}, {}): C={} Rust={}",
            esc(&e),
            esc(&a),
            cv,
            rv
        );
    }
}

/// CONFIGS.md row 31: `supersedes` over randomized chains.
#[test]
fn f11_supersedes_random() {
    let (cl, rl) = libs();
    let c: FnSupersedes = sym(&cl, b"supersedes\0");
    let r: FnSupersedes = sym(&rl, b"supersedes\0");
    let mut rng = Rng::new(0xf011);
    for _ in 0..1500 {
        let lugs = rng.pool(1, 3, b"AB1", 3);
        let deps = rng.pool(1, 3, b"XY", 3);
        let n = rng.below(9);
        let mut nodes_c: Vec<Node> = (0..n)
            .map(|_| gen_node(&mut rng, &lugs, &deps, 8))
            .collect();
        let mut nodes_r = nodes_c.clone();
        let head_c = link(&mut nodes_c);
        let head_r = link(&mut nodes_r);

        let lug = if rng.flip() {
            rng.pick(&lugs).clone()
        } else {
            gen_ffi_field(&mut rng, 8)
        };
        let dep = if rng.flip() {
            rng.pick(&deps).clone()
        } else {
            gen_ffi_field(&mut rng, 3)
        };
        let l = cs(&lug);
        let d = cs(&dep);
        let cv = unsafe { c(head_c, l.as_ptr() as *const c_char, d.as_ptr() as *const c_char) };
        let rv = unsafe { r(head_r, l.as_ptr() as *const c_char, d.as_ptr() as *const c_char) };
        assert_eq!(
            cv,
            rv,
            "supersedes(chain of {}, {}, {}): C={} Rust={}",
            n,
            esc(&lug),
            esc(&dep),
            cv,
            rv
        );
    }
}

/// CONFIGS.md row 32: `superseded`.
#[test]
fn f12_superseded_random() {
    let (cl, rl) = libs();
    let c: FnSuperseded = sym(&cl, b"superseded\0");
    let r: FnSuperseded = sym(&rl, b"superseded\0");
    let mut rng = Rng::new(0xf012);
    for _ in 0..1500 {
        let lugs = rng.pool(1, 2, b"AB", 2);
        let deps = rng.pool(1, 2, b"XY", 2);
        let n = rng.range(1, 8);
        let mut nodes_c: Vec<Node> = (0..n)
            .map(|_| gen_node(&mut rng, &lugs, &deps, 8))
            .collect();
        let mut nodes_r = nodes_c.clone();
        let head_c = link(&mut nodes_c);
        let head_r = link(&mut nodes_r);
        let cv = unsafe { c(head_c) };
        let rv = unsafe { r(head_r) };
        assert_eq!(cv, rv, "superseded(chain of {}): C={} Rust={}", n, cv, rv);
    }
}

/// CONFIGS.md row 33: `addRoutingDirectiveToList` — single insertion into a
/// chain (empty / head / middle / tail / among ties).
#[test]
fn f13_add_random() {
    let (cl, rl) = libs();
    let c: FnAdd = sym(&cl, b"addRoutingDirectiveToList\0");
    let r: FnAdd = sym(&rl, b"addRoutingDirectiveToList\0");
    let mut rng = Rng::new(0xf013);
    for iter in 0..2000 {
        let lugs = rng.pool(1, 2, b"AB", 2);
        let deps = rng.pool(1, 2, b"XY", 2);
        // nodes[0] is the list head the C `main` keeps on the stack
        let n = rng.below(9);
        let mut nodes: Vec<Node> = Vec::new();
        let mut head = Node::zeroed();
        head.time_stamp = 0;
        nodes.push(head);
        let mut stamps: Vec<u32> = (0..n).map(|_| rng.below(20) as u32).collect();
        stamps.sort_unstable(); // the C code keeps the list sorted
        for s in stamps {
            let mut node = gen_node(&mut rng, &lugs, &deps, 20);
            node.time_stamp = s;
            nodes.push(node);
        }
        let mut new_node_c = gen_node(&mut rng, &lugs, &deps, 20);
        new_node_c.time_stamp = match rng.below(4) {
            0 => 0,
            1 => rng.below(20) as u32,
            2 => 19,
            _ => rng.below(25) as u32,
        };
        let mut nodes_c = nodes.clone();
        let mut nodes_r = nodes.clone();
        let mut new_node_r = new_node_c;
        let base_c = link(&mut nodes_c);
        let base_r = link(&mut nodes_r);
        unsafe {
            c(base_c, &mut new_node_c as *mut Node);
            r(base_r, &mut new_node_r as *mut Node);
        }
        let order_c = unsafe {
            chain_indices(
                base_c,
                nodes_c.as_ptr(),
                nodes_c.len(),
                &new_node_c as *const Node,
            )
        };
        let order_r = unsafe {
            chain_indices(
                base_r,
                nodes_r.as_ptr(),
                nodes_r.len(),
                &new_node_r as *const Node,
            )
        };
        assert_eq!(
            order_c, order_r,
            "iter {}: insertion order differs (existing timestamps {:?}, new {})",
            iter,
            nodes_c.iter().map(|x| x.time_stamp).collect::<Vec<_>>(),
            new_node_c.time_stamp
        );
        // the payload of every node must be untouched
        for (a, b) in nodes_c.iter().zip(nodes_r.iter()) {
            assert_eq!(a.time_stamp, b.time_stamp);
            assert_eq!(a.luggage_id, b.luggage_id);
            assert_eq!(a.comments, b.comments);
        }
    }
}

/// CONFIGS.md row 34: build a whole list by repeated insertion (what `main`
/// does) and compare the resulting chain.
#[test]
fn f14_add_sequence() {
    let (cl, rl) = libs();
    let c: FnAdd = sym(&cl, b"addRoutingDirectiveToList\0");
    let r: FnAdd = sym(&rl, b"addRoutingDirectiveToList\0");
    let mut rng = Rng::new(0xf014);
    for iter in 0..300 {
        let lugs = rng.pool(1, 3, b"AB1", 3);
        let deps = rng.pool(1, 3, b"XY", 3);
        let n = rng.range(1, 30);
        // index 0 = head, 1..=n = the nodes inserted one after the other
        let mut proto: Vec<Node> = vec![Node::zeroed()];
        for _ in 0..n {
            let mut node = gen_node(&mut rng, &lugs, &deps, 12);
            node.time_stamp = match rng.below(3) {
                0 => rng.below(6) as u32,
                1 => rng.below(1000) as u32,
                _ => rng.next_u64() as u32,
            };
            proto.push(node);
        }
        let mut nodes_c = proto.clone();
        let mut nodes_r = proto.clone();
        // unlink everything: insertion must build the chain itself
        for v in nodes_c.iter_mut().chain(nodes_r.iter_mut()) {
            v.next = std::ptr::null_mut();
        }
        let base_c = nodes_c.as_mut_ptr();
        let base_r = nodes_r.as_mut_ptr();
        unsafe {
            for i in 1..=n {
                c(base_c, base_c.add(i));
                r(base_r, base_r.add(i));
            }
        }
        let order_c =
            unsafe { chain_indices(base_c, nodes_c.as_ptr(), nodes_c.len(), std::ptr::null()) };
        let order_r =
            unsafe { chain_indices(base_r, nodes_r.as_ptr(), nodes_r.len(), std::ptr::null()) };
        assert_eq!(
            order_c, order_r,
            "iter {}: chain built from {} nodes differs (timestamps {:?})",
            iter,
            n,
            proto.iter().map(|x| x.time_stamp).collect::<Vec<_>>()
        );
    }
}
