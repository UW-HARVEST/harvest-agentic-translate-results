//! Harness self-check: proves the `.so` loading + stdout capture mechanism
//! actually observes output, so that a later "both empty, therefore equal"
//! result can never be a false pass.

mod common;
use common::*;

#[test]
fn harness_observes_output() {
    let p = pair();

    let (c, r) = p.run_step(0);
    assert!(
        !c.is_empty(),
        "capture returned nothing for the C .so — the capture mechanism is broken"
    );
    assert!(
        is_four_house_lines(&c),
        "unexpected C output shape: {:?}",
        String::from_utf8_lossy(&c)
    );
    same("smoke run(0)", &c, &r);

    let (c, r) = p.driver_step_raw(b"not-a-number");
    assert_eq!(c, ERROR_LINE, "C error output shape changed");
    same("smoke driver(invalid)", &c, &r);

    let (c, r) = p.driver_step_raw(b"3");
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        8,
        "driver on a valid input must print 2 x 4 house lines"
    );
    same("smoke driver(valid)", &c, &r);

    assert!(parse_last_state(&c).is_some(), "state parser must work");
}
