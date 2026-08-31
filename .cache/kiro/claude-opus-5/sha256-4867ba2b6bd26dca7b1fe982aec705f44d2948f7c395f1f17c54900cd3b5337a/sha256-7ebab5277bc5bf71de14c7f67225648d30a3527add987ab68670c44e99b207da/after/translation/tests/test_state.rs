//! Interleaves `run` and `driver` to confirm the mutable `the_house` global
//! evolves identically in both implementations, including across the
//! `driver` -> `run` boundary and around signed-overflow wrap-around.

mod harness;
use harness::*;

#[test]
fn interleaved_state_matches_c() {
    let (c, rust) = load_pair();

    #[derive(Copy, Clone)]
    enum Call {
        Run(i32),
        Driver(i32),
    }
    use Call::*;

    let script = [
        Run(3),
        Driver(3),
        Run(-4),
        Run(0),
        Driver(i32::MAX),
        Run(1), // pushes `bedrooms` past INT_MAX -> wrap
        Run(i32::MIN),
        Driver(-1), // and back the other way
        Run(0),
        Driver(0),
        Run(i32::MAX),
        Driver(i32::MIN),
        Run(17),
        Driver(-17),
        Run(0),
    ];

    for (i, call) in script.iter().enumerate() {
        let (label, c_out, r_out) = match *call {
            Run(a) => (format!("run({a})"), c.run(a), rust.run(a)),
            Driver(a) => (format!("driver({a})"), c.driver(a), rust.driver(a)),
        };
        assert_same(&format!("{label} [step {i}]"), &c_out, &r_out);
    }
}
