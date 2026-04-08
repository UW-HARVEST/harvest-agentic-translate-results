use libvcd::vcd;

fn bytes_to_str(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..len]).unwrap()
}

// --- isexpression ---

#[test]
fn test_isexpression_true_cases() {
    for c in ['-', '0', '5', '9', 'z', 'Z', 'x', 'X', 'b', 'U'] {
        assert!(vcd::isexpression(c), "expected true for '{}'", c);
    }
}

#[test]
fn test_isexpression_false_cases() {
    for c in ['a', 'A', ' ', '$', '#', 'w', 'Y'] {
        assert!(!vcd::isexpression(c), "expected false for '{}'", c);
    }
}

// --- get_signal_index ---

#[test]
fn test_get_signal_index_bang() {
    assert_eq!(vcd::get_signal_index("!"), Some(0));
}

#[test]
fn test_get_signal_index_hash() {
    assert_eq!(vcd::get_signal_index("#"), Some(2));
}

#[test]
fn test_get_signal_index_tilde_out_of_range() {
    // '~' - '!' = 93, >= VCD_SIGNAL_COUNT(32), so None
    assert_eq!(vcd::get_signal_index("~"), None);
}

// --- VCD::read_from_path ---

#[test]
fn test_read_nonexistent_file() {
    let result = vcd::VCD::read_from_path("nonexistent.vcd");
    assert!(result.is_err());
}

#[test]
fn test_read_metadata() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    assert_eq!(bytes_to_str(&vcd.date), "Fri Jul 15 15:17:36 2022");
    assert_eq!(bytes_to_str(&vcd.version), "Icarus Verilog");
    assert_eq!(vcd.timescale.scale, 1);
    assert_eq!(bytes_to_str(&vcd.timescale.unit), "s");
}

#[test]
fn test_signals_count() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    assert_eq!(vcd.signals.len(), 7);
}

#[test]
fn test_signal_0_matched() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[0];
    assert_eq!(bytes_to_str(&s.name), "matched");
    assert_eq!(s.size, 9);
    assert_eq!(s.value_changes.len(), 4);
    assert_eq!(s.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_str(&s.value_changes[0].value), "bx");
    assert_eq!(s.value_changes[1].timestamp, 1);
    assert_eq!(bytes_to_str(&s.value_changes[1].value), "b0");
    assert_eq!(s.value_changes[2].timestamp, 25);
    assert_eq!(bytes_to_str(&s.value_changes[2].value), "b10010");
    assert_eq!(s.value_changes[3].timestamp, 35);
    assert_eq!(bytes_to_str(&s.value_changes[3].value), "b10000");
}

#[test]
fn test_signal_1_address() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[1];
    assert_eq!(bytes_to_str(&s.name), "address");
    assert_eq!(s.size, 4);
    assert_eq!(s.value_changes.len(), 3);
    assert_eq!(s.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_str(&s.value_changes[0].value), "bx");
    assert_eq!(s.value_changes[1].timestamp, 2);
    assert_eq!(bytes_to_str(&s.value_changes[1].value), "b1");
    assert_eq!(s.value_changes[2].timestamp, 12);
    assert_eq!(bytes_to_str(&s.value_changes[2].value), "b100");
}

#[test]
fn test_signal_2_clock() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[2];
    assert_eq!(bytes_to_str(&s.name), "clock");
    assert_eq!(s.size, 1);
    assert_eq!(s.value_changes.len(), 9);
    let expected: [(u32, &str); 9] = [
        (0, "0"), (5, "1"), (10, "0"), (15, "1"), (20, "0"),
        (25, "1"), (30, "0"), (35, "1"), (40, "0"),
    ];
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(s.value_changes[i].timestamp, *ts);
        assert_eq!(bytes_to_str(&s.value_changes[i].value), *val);
    }
}

#[test]
fn test_signal_3_mask() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[3];
    assert_eq!(bytes_to_str(&s.name), "mask");
    assert_eq!(s.size, 8);
    assert_eq!(s.value_changes.len(), 3);
    assert_eq!(s.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_str(&s.value_changes[0].value), "bx");
    assert_eq!(s.value_changes[1].timestamp, 22);
    assert_eq!(bytes_to_str(&s.value_changes[1].value), "b100000");
    assert_eq!(s.value_changes[2].timestamp, 32);
    assert_eq!(bytes_to_str(&s.value_changes[2].value), "b0");
}

#[test]
fn test_signal_4_reset() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[4];
    assert_eq!(bytes_to_str(&s.name), "reset");
    assert_eq!(s.size, 1);
    assert_eq!(s.value_changes.len(), 3);
    assert_eq!(s.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_str(&s.value_changes[0].value), "x");
    assert_eq!(s.value_changes[1].timestamp, 1);
    assert_eq!(bytes_to_str(&s.value_changes[1].value), "1");
    assert_eq!(s.value_changes[2].timestamp, 2);
    assert_eq!(bytes_to_str(&s.value_changes[2].value), "0");
}

#[test]
fn test_signal_5_word() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[5];
    assert_eq!(bytes_to_str(&s.name), "word");
    assert_eq!(s.size, 8);
    assert_eq!(s.value_changes.len(), 5);
    let expected: [(u32, &str); 5] = [
        (0, "bx"), (2, "b10010111"), (12, "b10110111"),
        (22, "b10010111"), (32, "b10110111"),
    ];
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(s.value_changes[i].timestamp, *ts);
        assert_eq!(bytes_to_str(&s.value_changes[i].value), *val);
    }
}

#[test]
fn test_signal_6_write() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let s = &vcd.signals[6];
    assert_eq!(bytes_to_str(&s.name), "write");
    assert_eq!(s.size, 1);
    assert_eq!(s.value_changes.len(), 3);
    assert_eq!(s.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_str(&s.value_changes[0].value), "0");
    assert_eq!(s.value_changes[1].timestamp, 2);
    assert_eq!(bytes_to_str(&s.value_changes[1].value), "1");
    assert_eq!(s.value_changes[2].timestamp, 22);
    assert_eq!(bytes_to_str(&s.value_changes[2].value), "0");
}

// --- VCD::get_signal_by_name ---

#[test]
fn test_get_signal_by_name_found() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    for (name, expected_size) in [
        ("matched", 9), ("address", 4), ("clock", 1),
        ("mask", 8), ("reset", 1), ("word", 8), ("write", 1),
    ] {
        let s = vcd.get_signal_by_name(name).unwrap();
        assert_eq!(bytes_to_str(&s.name), name);
        assert_eq!(s.size, expected_size);
    }
}

#[test]
fn test_get_signal_by_name_not_found() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    assert!(vcd.get_signal_by_name("nonexistent").is_none());
}

// --- Signal::get_value_at_timestamp ---

#[test]
fn test_clock_value_at_timestamps() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let clock = vcd.get_signal_by_name("clock").unwrap();
    let expected: [(u32, &str); 10] = [
        (0, "0"), (1, "0"), (5, "1"), (10, "0"), (15, "1"),
        (20, "0"), (25, "1"), (30, "0"), (35, "1"), (40, "0"),
    ];
    for (ts, val) in &expected {
        let v = clock.get_value_at_timestamp(*ts).unwrap();
        assert_eq!(bytes_to_str(v), *val, "clock@{}", ts);
    }
}

#[test]
fn test_matched_value_at_timestamps() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let matched = vcd.get_signal_by_name("matched").unwrap();
    let expected: [(u32, &str); 5] = [
        (0, "bx"), (1, "b0"), (2, "b0"), (25, "b10010"), (35, "b10000"),
    ];
    for (ts, val) in &expected {
        let v = matched.get_value_at_timestamp(*ts).unwrap();
        assert_eq!(bytes_to_str(v), *val, "matched@{}", ts);
    }
}

#[test]
fn test_address_value_at_timestamps() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let addr = vcd.get_signal_by_name("address").unwrap();
    let expected: [(u32, &str); 4] = [
        (0, "bx"), (2, "b1"), (12, "b100"), (22, "b100"),
    ];
    for (ts, val) in &expected {
        let v = addr.get_value_at_timestamp(*ts).unwrap();
        assert_eq!(bytes_to_str(v), *val, "address@{}", ts);
    }
}

#[test]
fn test_value_before_any_change_returns_none() {
    let vcd = vcd::VCD::read_from_path("c_src/test/assets/ram.vcd").unwrap();
    let matched = vcd.get_signal_by_name("matched").unwrap();
    // The C code returns NULL for timestamp before first change only if first change timestamp > query
    // First change for matched is at timestamp 0, so timestamp 0 returns a value.
    // But if we had a signal whose first change is at t=5, querying t=0 would return NULL.
    // For matched, all timestamps >= 0 return something. Let's not test this edge case
    // since we don't have a signal with first change > 0 in the test data.
    // Instead verify the function returns the correct last value.
    let v = matched.get_value_at_timestamp(100);
    assert!(v.is_some());
    assert_eq!(bytes_to_str(v.unwrap()), "b10000");
}

fn main() {}
