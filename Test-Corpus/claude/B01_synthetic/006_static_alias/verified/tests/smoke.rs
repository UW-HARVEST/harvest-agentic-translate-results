// Infrastructure smoke test: both shared objects load, both exported symbols
// resolve, and a trivial call through each of them agrees.

mod common;

use common::*;

#[test]
fn ffi_infrastructure_works() {
    let p = load_pair("smoke");

    // static_alias: fresh image, inner == 1, *outer = 5 -> then branch.
    let mut cv: i32 = 5;
    let mut rv: i32 = 5;
    let cr = unsafe { (p.c.static_alias)(&mut cv) };
    let rr = unsafe { (p.rust.static_alias)(&mut rv) };
    assert_eq!(unsafe { *cr }, unsafe { *rr }, "returned value");
    assert_eq!(cr == &mut cv as *mut i32, rr == &mut rv as *mut i32, "aliasing");
    assert_eq!(cv, rv, "caller cell");
    assert_eq!(unsafe { *cr }, 6);

    // main: argc != 3 error path.
    let mut argv = Argv::new(&[b"driver".as_slice()]);
    let (crc, cout) = call_main(&p.c, 1, &mut argv);
    let (rrc, rout) = call_main(&p.rust, 1, &mut argv);
    assert_eq!(crc, rrc, "rc");
    assert_eq!(cout, rout, "stdout: {:?} vs {:?}", show(&cout), show(&rout));
    assert_eq!(
        cout,
        b"Error: should only be two (integer) arguments!\n".to_vec()
    );

    // main: happy path (inner is 6 by now in both images).
    let mut argv = Argv::new(&[b"driver".as_slice(), b"3".as_slice(), b"4".as_slice()]);
    let (crc, cout) = call_main(&p.c, 3, &mut argv);
    let (rrc, rout) = call_main(&p.rust, 3, &mut argv);
    assert_eq!(crc, rrc);
    assert_eq!(cout, rout, "stdout: {:?} vs {:?}", show(&cout), show(&rout));
    assert!(!cout.is_empty(), "expected output");
}
