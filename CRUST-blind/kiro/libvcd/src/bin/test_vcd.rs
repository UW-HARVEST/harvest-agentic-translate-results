use libvcd::vcd::*;

fn fixed_to_string(buf: &[u8]) -> String {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..len]).to_string()
}

fn val_str(val: &[u8; VCD_SIGNAL_SIZE]) -> String {
    fixed_to_string(val)
}

const VCD_PATH: &str = "c_src/test/assets/ram.vcd";

// --- read_from_path ---

#[test]
fn test_read_nonexistent_file() {
    assert!(VCD::read_from_path("nonexistent_file.vcd").is_err());
}

#[test]
fn test_read_valid_file() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    assert_eq!(vcd.signals.len(), 7);
}

// --- date, version, timescale ---

#[test]
fn test_date() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    assert_eq!(fixed_to_string(&vcd.date), "Fri Jul 15 15:17:36 2022");
}

#[test]
fn test_version() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    assert_eq!(fixed_to_string(&vcd.version), "Icarus Verilog");
}

#[test]
fn test_timescale() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    assert_eq!(vcd.timescale.scale, 1);
    assert_eq!(fixed_to_string(&vcd.timescale.unit), "s");
}

// --- signals parsed correctly (top-level only, inner module skipped) ---

#[test]
fn test_signal_names_and_sizes() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let expected: Vec<(&str, usize)> = vec![
        ("matched", 9),
        ("address", 4),
        ("clock", 1),
        ("mask", 8),
        ("reset", 1),
        ("word", 8),
        ("write", 1),
    ];
    assert_eq!(vcd.signals.len(), expected.len());
    for (i, (name, size)) in expected.iter().enumerate() {
        assert_eq!(fixed_to_string(&vcd.signals[i].name), *name);
        assert_eq!(vcd.signals[i].size, *size);
    }
}

// --- get_signal_by_name ---

#[test]
fn test_get_signal_by_name_exists() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("clock");
    assert!(sig.is_some());
    assert_eq!(fixed_to_string(&sig.unwrap().name), "clock");
}

#[test]
fn test_get_signal_by_name_nonexistent() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    assert!(vcd.get_signal_by_name("nonexistent").is_none());
}

#[test]
fn test_get_signal_by_name_all() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    for name in &["matched", "address", "clock", "mask", "reset", "word", "write"] {
        assert!(vcd.get_signal_by_name(name).is_some(), "signal {} not found", name);
    }
}

// --- value changes count ---

#[test]
fn test_value_changes_count() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let expected: Vec<(&str, usize)> = vec![
        ("matched", 4),
        ("address", 3),
        ("clock", 9),
        ("mask", 3),
        ("reset", 3),
        ("word", 5),
        ("write", 3),
    ];
    for (name, count) in expected {
        let sig = vcd.get_signal_by_name(name).unwrap();
        assert_eq!(sig.value_changes.len(), count, "signal {} changes_count", name);
    }
}

// --- value changes content ---

#[test]
fn test_matched_value_changes() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("matched").unwrap();
    let expected: Vec<(u32, &str)> = vec![(0, "bx"), (1, "b0"), (25, "b10010"), (35, "b10000")];
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(sig.value_changes[i].timestamp, *ts);
        assert_eq!(val_str(&sig.value_changes[i].value), *val);
    }
}

#[test]
fn test_clock_value_changes() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("clock").unwrap();
    let expected: Vec<(u32, &str)> = vec![
        (0, "0"), (5, "1"), (10, "0"), (15, "1"), (20, "0"),
        (25, "1"), (30, "0"), (35, "1"), (40, "0"),
    ];
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(sig.value_changes[i].timestamp, *ts);
        assert_eq!(val_str(&sig.value_changes[i].value), *val);
    }
}

#[test]
fn test_reset_value_changes() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("reset").unwrap();
    let expected: Vec<(u32, &str)> = vec![(0, "x"), (1, "1"), (2, "0")];
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(sig.value_changes[i].timestamp, *ts);
        assert_eq!(val_str(&sig.value_changes[i].value), *val);
    }
}

#[test]
fn test_write_value_changes() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("write").unwrap();
    let expected: Vec<(u32, &str)> = vec![(0, "0"), (2, "1"), (22, "0")];
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(sig.value_changes[i].timestamp, *ts);
        assert_eq!(val_str(&sig.value_changes[i].value), *val);
    }
}

// --- get_value_at_timestamp ---

#[test]
fn test_value_at_exact_timestamp() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("clock").unwrap();
    assert_eq!(val_str(sig.get_value_at_timestamp(0).unwrap()), "0");
    assert_eq!(val_str(sig.get_value_at_timestamp(5).unwrap()), "1");
    assert_eq!(val_str(sig.get_value_at_timestamp(10).unwrap()), "0");
}

#[test]
fn test_value_between_timestamps() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("clock").unwrap();
    // Between 5 and 10, value should be "1" (last value at or before 7)
    assert_eq!(val_str(sig.get_value_at_timestamp(7).unwrap()), "1");
}

#[test]
fn test_value_after_last_change() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("clock").unwrap();
    // Last change is at 40 with value "0"
    assert_eq!(val_str(sig.get_value_at_timestamp(100).unwrap()), "0");
}

#[test]
fn test_value_at_timestamp_matched() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("matched").unwrap();
    assert_eq!(val_str(sig.get_value_at_timestamp(0).unwrap()), "bx");
    assert_eq!(val_str(sig.get_value_at_timestamp(1).unwrap()), "b0");
    assert_eq!(val_str(sig.get_value_at_timestamp(25).unwrap()), "b10010");
}

#[test]
fn test_value_at_timestamp_mask() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("mask").unwrap();
    assert_eq!(val_str(sig.get_value_at_timestamp(22).unwrap()), "b100000");
}

#[test]
fn test_value_at_timestamp_reset() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("reset").unwrap();
    assert_eq!(val_str(sig.get_value_at_timestamp(1).unwrap()), "1");
    assert_eq!(val_str(sig.get_value_at_timestamp(2).unwrap()), "0");
}

#[test]
fn test_value_at_timestamp_word() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("word").unwrap();
    assert_eq!(val_str(sig.get_value_at_timestamp(2).unwrap()), "b10010111");
}

#[test]
fn test_value_at_timestamp_write() {
    let vcd = VCD::read_from_path(VCD_PATH).unwrap();
    let sig = vcd.get_signal_by_name("write").unwrap();
    assert_eq!(val_str(sig.get_value_at_timestamp(0).unwrap()), "0");
    assert_eq!(val_str(sig.get_value_at_timestamp(2).unwrap()), "1");
}

// --- get_signal_index ---

#[test]
fn test_get_signal_index() {
    assert_eq!(get_signal_index("!"), Some(0));
    assert_eq!(get_signal_index("\""), Some(1));
    assert_eq!(get_signal_index("#"), Some(2));
    assert_eq!(get_signal_index("@"), Some(31));
    assert_eq!(get_signal_index("A"), None); // 32 >= VCD_SIGNAL_COUNT
}

// --- isexpression ---

#[test]
fn test_isexpression() {
    for c in ['-', '0', '1', '5', '9', 'z', 'Z', 'x', 'X', 'b', 'U'] {
        assert!(isexpression(c), "expected true for '{}'", c);
    }
    for c in ['a', 'c', '$', '#', ' ', '\n'] {
        assert!(!isexpression(c), "expected false for '{}'", c);
    }
}

// --- constants ---

#[test]
fn test_constants() {
    assert_eq!(VCD_SIGNAL_COUNT, 32);
    assert_eq!(VCD_VALUE_CHANGE_COUNT, 4096);
    assert_eq!(VCD_SIGNAL_SIZE, 64);
    assert_eq!(VCD_NAME_SIZE, 32);
    assert_eq!(VCD_TIME_UNIT_SIZE, 8);
    assert_eq!(VCD_VERSION_SIZE, 64);
    assert_eq!(VCD_DATE_SIZE, 64);
}

fn main() {}
