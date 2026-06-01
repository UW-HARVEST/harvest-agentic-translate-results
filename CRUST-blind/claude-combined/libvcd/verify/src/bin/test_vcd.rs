#![allow(unused_imports, dead_code)]

use libvcd::vcd::{
    self, get_signal_index, isexpression, Signal, Timescale, ValueChange, BUFFER_LENGTH,
    VCD_DATE_SIZE, VCD_NAME_SIZE, VCD_SIGNAL_COUNT, VCD_SIGNAL_SIZE, VCD_TIME_UNIT_SIZE,
    VCD_VALUE_CHANGE_COUNT, VCD_VERSION_SIZE,
};

const VCD_PATH: &str = "c_src/test/assets/ram.vcd";

fn nul_str(bytes: &[u8]) -> &str {
    let nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..nul]).unwrap()
}

#[test]
fn test_constants_match_c() {
    assert_eq!(VCD_SIGNAL_COUNT, 32);
    assert_eq!(VCD_VALUE_CHANGE_COUNT, 4096);
    assert_eq!(VCD_SIGNAL_SIZE, 64);
    assert_eq!(VCD_NAME_SIZE, 32);
    assert_eq!(VCD_TIME_UNIT_SIZE, 8);
    assert_eq!(VCD_VERSION_SIZE, 64);
    assert_eq!(VCD_DATE_SIZE, 64);
    assert_eq!(BUFFER_LENGTH, 512);
}

#[test]
fn test_isexpression_true_cases() {
    for c in [
        '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'z', 'Z', 'x', 'X', 'b', 'U',
    ] {
        assert!(isexpression(c), "expected isexpression({}) == true", c);
    }
}

#[test]
fn test_isexpression_false_cases() {
    for c in ['$', '#', ' ', '\t', '\n', 'a', 'c', 'A', 'B', 'C', '!', '+', 'y', 'u'] {
        assert!(!isexpression(c), "expected isexpression({}) == false", c);
    }
}

#[test]
fn test_get_signal_index_basic() {
    // '!' - '!' = 0
    assert_eq!(get_signal_index("!"), Some(0));
    // '"' - '!' = 1
    assert_eq!(get_signal_index("\""), Some(1));
    // '#' - '!' = 2
    assert_eq!(get_signal_index("#"), Some(2));
    // '$' - '!' = 3
    assert_eq!(get_signal_index("$"), Some(3));
    // '%' - '!' = 4
    assert_eq!(get_signal_index("%"), Some(4));
    // '&' - '!' = 5
    assert_eq!(get_signal_index("&"), Some(5));
    // ''' - '!' = 6
    assert_eq!(get_signal_index("'"), Some(6));
}

#[test]
fn test_get_signal_index_at_boundary() {
    // '!' + 31 -> '@' index 31 (valid)
    let c = (b'!' + 31) as char;
    let s = c.to_string();
    assert_eq!(get_signal_index(&s), Some(31));
    // '!' + 32 = 'A' -> id 32, NOT valid (>= VCD_SIGNAL_COUNT)
    let c2 = (b'!' + 32) as char;
    let s2 = c2.to_string();
    assert_eq!(get_signal_index(&s2), None);
}

#[test]
fn test_get_signal_index_uses_first_byte() {
    // Only the first byte of the string is considered (matches C: *string - '!').
    assert_eq!(get_signal_index("!extra"), Some(0));
    assert_eq!(get_signal_index("\"abc"), Some(1));
}

#[test]
fn test_read_from_path_returns_err_for_missing_file() {
    let r = vcd::VCD::read_from_path("/nonexistent/path/does_not_exist.vcd");
    assert!(r.is_err());
}

#[test]
fn test_read_from_path_metadata() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");
    assert_eq!(nul_str(&v.date), "Fri Jul 15 15:17:36 2022");
    assert_eq!(nul_str(&v.version), "Icarus Verilog");
    assert_eq!(nul_str(&v.timescale.unit), "s");
    assert_eq!(v.timescale.scale, 1usize);
}

#[test]
fn test_read_from_path_signals_count_and_names() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");
    assert_eq!(v.signals.len(), 7);
    let names: Vec<&str> = v.signals.iter().map(|s| nul_str(&s.name)).collect();
    assert_eq!(
        names,
        vec!["matched", "address", "clock", "mask", "reset", "word", "write"]
    );
}

#[test]
fn test_read_from_path_signal_sizes() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");
    let sizes: Vec<usize> = v.signals.iter().map(|s| s.size).collect();
    assert_eq!(sizes, vec![9, 4, 1, 8, 1, 8, 1]);
}

#[test]
fn test_read_from_path_changes_counts() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");
    let counts: Vec<usize> = v.signals.iter().map(|s| s.value_changes.len()).collect();
    assert_eq!(counts, vec![4, 3, 9, 3, 3, 5, 3]);
}

fn vc_value_str(vc: &ValueChange) -> &str {
    let nul = vc.value.iter().position(|&b| b == 0).unwrap_or(vc.value.len());
    std::str::from_utf8(&vc.value[..nul]).unwrap()
}

#[test]
fn test_read_from_path_value_changes_full() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");

    // matched
    let s = v.get_signal_by_name("matched").unwrap();
    let expected: &[(u32, &str)] = &[(0, "bx"), (1, "b0"), (25, "b10010"), (35, "b10000")];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }

    // address
    let s = v.get_signal_by_name("address").unwrap();
    let expected: &[(u32, &str)] = &[(0, "bx"), (2, "b1"), (12, "b100")];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }

    // clock
    let s = v.get_signal_by_name("clock").unwrap();
    let expected: &[(u32, &str)] = &[
        (0, "0"),
        (5, "1"),
        (10, "0"),
        (15, "1"),
        (20, "0"),
        (25, "1"),
        (30, "0"),
        (35, "1"),
        (40, "0"),
    ];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }

    // mask
    let s = v.get_signal_by_name("mask").unwrap();
    let expected: &[(u32, &str)] = &[(0, "bx"), (22, "b100000"), (32, "b0")];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }

    // reset
    let s = v.get_signal_by_name("reset").unwrap();
    let expected: &[(u32, &str)] = &[(0, "x"), (1, "1"), (2, "0")];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }

    // word
    let s = v.get_signal_by_name("word").unwrap();
    let expected: &[(u32, &str)] = &[
        (0, "bx"),
        (2, "b10010111"),
        (12, "b10110111"),
        (22, "b10010111"),
        (32, "b10110111"),
    ];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }

    // write
    let s = v.get_signal_by_name("write").unwrap();
    let expected: &[(u32, &str)] = &[(0, "0"), (2, "1"), (22, "0")];
    assert_eq!(s.value_changes.len(), expected.len());
    for (vc, (ts, val)) in s.value_changes.iter().zip(expected) {
        assert_eq!(vc.timestamp, *ts);
        assert_eq!(vc_value_str(vc), *val);
    }
}

#[test]
fn test_get_signal_by_name_unknown_returns_none() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");
    assert!(v.get_signal_by_name("does_not_exist").is_none());
    assert!(v.get_signal_by_name("").is_none());
}

#[test]
fn test_get_signal_by_name_returns_correct_struct() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");
    let s = v.get_signal_by_name("clock").unwrap();
    assert_eq!(nul_str(&s.name), "clock");
    assert_eq!(s.size, 1);
    assert_eq!(s.value_changes.len(), 9);
}

fn read_value(v: &vcd::VCD, name: &str, ts: u32) -> Option<String> {
    let signal = v.get_signal_by_name(name).unwrap();
    signal.get_value_at_timestamp(ts).map(|arr| {
        let nul = arr.iter().position(|&b| b == 0).unwrap_or(arr.len());
        std::str::from_utf8(&arr[..nul]).unwrap().to_string()
    })
}

#[test]
fn test_get_value_at_timestamp_matches_c() {
    let v = vcd::VCD::read_from_path(VCD_PATH).expect("should parse");

    // Values produced by running the C reference test_prog binary.
    let cases: &[(&str, u32, &str)] = &[
        ("matched", 25, "b10010"),
        ("matched", 35, "b10000"),
        ("word", 22, "b10010111"),
        ("mask", 22, "b100000"),
        ("address", 12, "b100"),
        ("clock", 5, "1"),
        ("clock", 9, "1"),
        ("reset", 1, "1"),
        ("reset", 2, "0"),
        ("write", 1, "0"),
        ("write", 2, "1"),
        ("write", 22, "0"),
        ("write", 99, "0"),
        ("mask", 0, "bx"),
        ("mask", 1, "bx"),
        ("mask", 21, "bx"),
        ("mask", 30, "b100000"),
        ("mask", 35, "b0"),
        ("address", 0, "bx"),
        ("address", 1, "bx"),
        ("address", 2, "b1"),
        ("address", 11, "b1"),
        ("address", 13, "b100"),
        ("address", 100, "b100"),
        ("reset", 0, "x"),
        ("reset", 99, "0"),
        ("clock", 0, "0"),
        ("clock", 4, "0"),
        ("clock", 10, "0"),
        ("clock", 14, "0"),
        ("clock", 15, "1"),
        ("clock", 19, "1"),
        ("clock", 20, "0"),
        ("clock", 24, "0"),
        ("clock", 25, "1"),
        ("clock", 29, "1"),
        ("clock", 30, "0"),
        ("word", 0, "bx"),
        ("word", 21, "b10110111"),
        ("word", 31, "b10010111"),
        ("word", 32, "b10110111"),
        ("word", 33, "b10110111"),
        ("matched", 0, "bx"),
        ("matched", 1, "b0"),
        ("matched", 24, "b0"),
        ("matched", 34, "b10010"),
        ("matched", 36, "b10000"),
        ("matched", 100, "b10000"),
    ];

    for (name, ts, expected) in cases {
        let got = read_value(&v, name, *ts).unwrap_or_else(|| {
            panic!("{} at {} returned None (expected {})", name, ts, expected)
        });
        assert_eq!(
            got, *expected,
            "signal {} at timestamp {} mismatch",
            name, ts
        );
    }
}

#[test]
fn test_get_value_at_timestamp_before_first_change_returns_none_when_empty() {
    // Construct an empty signal manually to verify the None branch.
    let s = Signal {
        name: [0u8; VCD_NAME_SIZE],
        size: 0,
        value_changes: Vec::new(),
    };
    assert!(s.get_value_at_timestamp(0).is_none());
    assert!(s.get_value_at_timestamp(100).is_none());
}

#[test]
fn test_get_value_at_timestamp_picks_latest_not_after() {
    // Manually build a signal with known changes to verify the
    // "latest <= timestamp" semantics directly.
    let mk_val = |s: &str| {
        let mut b = [0u8; VCD_SIGNAL_SIZE];
        b[..s.len()].copy_from_slice(s.as_bytes());
        b
    };

    let s = Signal {
        name: [0u8; VCD_NAME_SIZE],
        size: 1,
        value_changes: vec![
            ValueChange {
                timestamp: 5,
                value: mk_val("a"),
            },
            ValueChange {
                timestamp: 10,
                value: mk_val("b"),
            },
            ValueChange {
                timestamp: 20,
                value: mk_val("c"),
            },
        ],
    };

    // Before any change -> None.
    assert!(s.get_value_at_timestamp(0).is_none());
    assert!(s.get_value_at_timestamp(4).is_none());

    // Exactly at first change.
    let v = s.get_value_at_timestamp(5).unwrap();
    let nul = v.iter().position(|&b| b == 0).unwrap_or(v.len());
    assert_eq!(std::str::from_utf8(&v[..nul]).unwrap(), "a");

    // Between changes.
    let v = s.get_value_at_timestamp(7).unwrap();
    let nul = v.iter().position(|&b| b == 0).unwrap_or(v.len());
    assert_eq!(std::str::from_utf8(&v[..nul]).unwrap(), "a");

    // Exactly at second change.
    let v = s.get_value_at_timestamp(10).unwrap();
    let nul = v.iter().position(|&b| b == 0).unwrap_or(v.len());
    assert_eq!(std::str::from_utf8(&v[..nul]).unwrap(), "b");

    // After all changes.
    let v = s.get_value_at_timestamp(99).unwrap();
    let nul = v.iter().position(|&b| b == 0).unwrap_or(v.len());
    assert_eq!(std::str::from_utf8(&v[..nul]).unwrap(), "c");
}

#[test]
fn test_timescale_struct_can_be_constructed() {
    let mut unit = [0u8; VCD_TIME_UNIT_SIZE];
    unit[..2].copy_from_slice(b"ns");
    let t = Timescale { unit, scale: 10 };
    assert_eq!(t.scale, 10);
    assert_eq!(&t.unit[..2], b"ns");
}

#[test]
fn test_value_change_struct_can_be_constructed() {
    let mut value = [0u8; VCD_SIGNAL_SIZE];
    value[..3].copy_from_slice(b"b01");
    let vc = ValueChange {
        timestamp: 42,
        value,
    };
    assert_eq!(vc.timestamp, 42);
    assert_eq!(&vc.value[..3], b"b01");
}

fn main() {}
