//! Interleaved `run` / `driver` calls, to confirm the shared file-static
//! `the_house` state is threaded through both entry points identically.

mod common;

use common::{assert_same, call_driver_raw, check_driver, check_run};

#[derive(Clone, Copy, Debug)]
enum Call {
    Run(i32),
    Driver(&'static [u8]),
}

#[test]
fn interleaved_calls_match_c() {
    let script = [
        Call::Run(0),
        Call::Driver(b"1"),
        Call::Run(-1),
        Call::Driver(b"oops"),
        Call::Run(3),
        Call::Driver(b"  -17junk"),
        Call::Run(0),
        Call::Driver(b"2147483648"),
        Call::Run(1),
        Call::Driver(b"-0"),
        Call::Run(-4),
        Call::Driver(b""),
        Call::Run(11),
        Call::Driver(b"+000000009"),
        Call::Run(0),
        Call::Driver(b"-2147483648"),
        Call::Run(2147483647),
        Call::Driver(b"2147483647"),
        Call::Run(-2147483648),
        Call::Driver(b"5"),
    ];

    for (i, call) in script.iter().enumerate() {
        let case = format!("{call:?} [step #{i}]");
        match *call {
            Call::Run(v) => check_run(v, &case),
            Call::Driver(s) => check_driver(s, &case),
        }
    }
}

/// `driver` on a buffer whose NUL terminator is followed by more bytes: the
/// terminator must stop the scan in both implementations.
#[test]
fn driver_stops_at_nul() {
    let inputs: [&[u8]; 6] = [
        b"12\0abc",
        b"\0999",
        b"-\0 7",
        b"  \0",
        b"9223372036854775808\0" as &[u8],
        b"7\0\0\0",
    ];
    for raw in inputs {
        let (c_out, rust_out) = call_driver_raw(raw);
        assert_same(&format!("driver_raw({raw:?})"), &c_out, &rust_out);
    }
}
