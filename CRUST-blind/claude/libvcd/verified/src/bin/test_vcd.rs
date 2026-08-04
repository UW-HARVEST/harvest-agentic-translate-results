use libvcd::vcd::{
    self, Signal, Timescale, ValueChange, BUFFER_LENGTH, VCD, VCD_DATE_SIZE, VCD_NAME_SIZE,
    VCD_SIGNAL_COUNT, VCD_SIGNAL_SIZE, VCD_TIME_UNIT_SIZE, VCD_VALUE_CHANGE_COUNT,
    VCD_VERSION_SIZE,
};
use std::fs::File;
use std::io::Write;

const RAM_VCD_PATH: &str = "c_src/test/assets/ram.vcd";

/// Convert a fixed-size byte array to a string by trimming at the first NUL.
fn bytes_to_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn write_tmp_vcd(path: &str, content: &str) {
    let mut f = File::create(path).expect("create temp vcd");
    f.write_all(content.as_bytes()).expect("write temp vcd");
}

// ---------- Constants ----------

#[test]
fn test_constants() {
    assert_eq!(VCD_SIGNAL_COUNT, 32);
    assert_eq!(VCD_VALUE_CHANGE_COUNT, 4096);
    assert_eq!(VCD_SIGNAL_SIZE, 64);
    assert_eq!(VCD_NAME_SIZE, 32);
    assert_eq!(VCD_TIME_UNIT_SIZE, 8);
    assert_eq!(VCD_VERSION_SIZE, 64);
    assert_eq!(VCD_DATE_SIZE, 64);
    assert_eq!(BUFFER_LENGTH, 512);
}

// ---------- isexpression ----------

#[test]
fn test_isexpression_true() {
    for c in [
        '-', '0', '1', '5', '9', 'z', 'Z', 'x', 'X', 'b', 'U',
    ] {
        assert!(vcd::isexpression(c), "expected isexpression({:?}) true", c);
    }
}

#[test]
fn test_isexpression_false() {
    for c in ['a', 'A', '$', '#', ' ', '\n', '\t', 'y', '+', '/', '.'] {
        assert!(!vcd::isexpression(c), "expected isexpression({:?}) false", c);
    }
}

// ---------- get_signal_index ----------

#[test]
fn test_get_signal_index_basic() {
    // '!' = 33, baseline -> 0
    assert_eq!(vcd::get_signal_index("!"), Some(0));
    // '"' = 34, -> 1
    assert_eq!(vcd::get_signal_index("\""), Some(1));
    // '#' = 35, -> 2
    assert_eq!(vcd::get_signal_index("#"), Some(2));
    // '@' = 64, -> 31 (max valid in 32-signal range)
    assert_eq!(vcd::get_signal_index("@"), Some(31));
}

#[test]
fn test_get_signal_index_out_of_range() {
    // 'A' = 65, diff = 32, out of range
    assert_eq!(vcd::get_signal_index("A"), None);
    // '~' = 126, diff = 93, out of range
    assert_eq!(vcd::get_signal_index("~"), None);
}

#[test]
fn test_get_signal_index_takes_first_byte() {
    // Behavior: only first byte matters
    assert_eq!(vcd::get_signal_index("!extra"), Some(0));
    assert_eq!(vcd::get_signal_index("@junk"), Some(31));
}

#[test]
fn test_get_signal_index_empty() {
    assert_eq!(vcd::get_signal_index(""), None);
}

// ---------- vcd_read_from_path: missing file ----------

#[test]
fn test_read_from_path_nonexistent() {
    let r = VCD::read_from_path("/tmp/this_file_does_not_exist_xyz_12345.vcd");
    assert!(r.is_err(), "expected error for nonexistent file");
}

// ---------- vcd_read_from_path: ram.vcd ----------

#[test]
fn test_read_from_path_ram_signals_count() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    assert_eq!(vcd.signals.len(), 7);
}

#[test]
fn test_read_from_path_ram_header_fields() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    assert_eq!(bytes_to_string(&vcd.date), "Fri Jul 15 15:17:36 2022");
    assert_eq!(bytes_to_string(&vcd.version), "Icarus Verilog");
    assert_eq!(vcd.timescale.scale, 1);
    assert_eq!(bytes_to_string(&vcd.timescale.unit), "s");
}

#[test]
fn test_read_from_path_ram_signal_names_and_sizes() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let expected: &[(&str, usize)] = &[
        ("matched", 9),
        ("address", 4),
        ("clock", 1),
        ("mask", 8),
        ("reset", 1),
        ("word", 8),
        ("write", 1),
    ];
    for (i, (name, size)) in expected.iter().enumerate() {
        let actual_name = bytes_to_string(&vcd.signals[i].name);
        assert_eq!(&actual_name, name, "signal[{}] name mismatch", i);
        assert_eq!(vcd.signals[i].size, *size, "signal[{}] size mismatch", i);
    }
}

#[test]
fn test_read_from_path_ram_changes_count() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let expected: &[(&str, usize)] = &[
        ("matched", 4),
        ("address", 3),
        ("clock", 9),
        ("mask", 3),
        ("reset", 3),
        ("word", 5),
        ("write", 3),
    ];
    for (i, (name, count)) in expected.iter().enumerate() {
        let actual_name = bytes_to_string(&vcd.signals[i].name);
        assert_eq!(&actual_name, name);
        assert_eq!(
            vcd.signals[i].value_changes.len(),
            *count,
            "signal[{}] {} changes count",
            i,
            name
        );
    }
}

#[test]
fn test_read_from_path_ram_matched_value_changes() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("matched").expect("find matched");
    let expected: &[(u32, &str)] = &[
        (0, "bx"),
        (1, "b0"),
        (25, "b10010"),
        (35, "b10000"),
    ];
    assert_eq!(s.value_changes.len(), expected.len());
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(s.value_changes[i].timestamp, *ts);
        assert_eq!(bytes_to_string(&s.value_changes[i].value), *val);
    }
}

#[test]
fn test_read_from_path_ram_clock_value_changes() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("clock").expect("find clock");
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
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(s.value_changes[i].timestamp, *ts);
        assert_eq!(bytes_to_string(&s.value_changes[i].value), *val);
    }
}

#[test]
fn test_read_from_path_ram_reset_value_changes() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("reset").expect("find reset");
    let expected: &[(u32, &str)] = &[(0, "x"), (1, "1"), (2, "0")];
    assert_eq!(s.value_changes.len(), expected.len());
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(s.value_changes[i].timestamp, *ts);
        assert_eq!(bytes_to_string(&s.value_changes[i].value), *val);
    }
}

#[test]
fn test_read_from_path_ram_word_value_changes() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("word").expect("find word");
    let expected: &[(u32, &str)] = &[
        (0, "bx"),
        (2, "b10010111"),
        (12, "b10110111"),
        (22, "b10010111"),
        (32, "b10110111"),
    ];
    assert_eq!(s.value_changes.len(), expected.len());
    for (i, (ts, val)) in expected.iter().enumerate() {
        assert_eq!(s.value_changes[i].timestamp, *ts);
        assert_eq!(bytes_to_string(&s.value_changes[i].value), *val);
    }
}

// ---------- get_signal_by_name ----------

#[test]
fn test_get_signal_by_name_found() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    for name in &["matched", "address", "clock", "mask", "reset", "word", "write"] {
        let s = vcd.get_signal_by_name(name);
        assert!(s.is_some(), "expected signal {} to be found", name);
    }
}

#[test]
fn test_get_signal_by_name_not_found() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    assert!(vcd.get_signal_by_name("nonexistent").is_none());
    assert!(vcd.get_signal_by_name("").is_none());
    assert!(vcd.get_signal_by_name("matche").is_none()); // partial
    assert!(vcd.get_signal_by_name("matched_").is_none());
}

// ---------- get_value_at_timestamp ----------

#[test]
fn test_get_value_at_timestamp_matched() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("matched").expect("matched");
    let cases: &[(u32, &str)] = &[
        (0, "bx"),
        (1, "b0"),
        (24, "b0"),
        (25, "b10010"),
        (35, "b10000"),
        (100, "b10000"),
    ];
    for (ts, expected) in cases {
        let v = s.get_value_at_timestamp(*ts).expect("expected Some");
        assert_eq!(
            bytes_to_string(v),
            *expected,
            "matched@{} expected {}",
            ts,
            expected
        );
    }
}

#[test]
fn test_get_value_at_timestamp_address() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("address").expect("address");
    let cases: &[(u32, &str)] = &[
        (0, "bx"),
        (2, "b1"),
        (11, "b1"),
        (12, "b100"),
        (22, "b100"),
    ];
    for (ts, expected) in cases {
        let v = s.get_value_at_timestamp(*ts).expect("expected Some");
        assert_eq!(bytes_to_string(v), *expected);
    }
}

#[test]
fn test_get_value_at_timestamp_clock_at_5() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("clock").expect("clock");
    let v = s.get_value_at_timestamp(5).expect("expected Some");
    assert_eq!(bytes_to_string(v), "1");
}

#[test]
fn test_get_value_at_timestamp_word_misc() {
    let vcd = VCD::read_from_path(RAM_VCD_PATH).expect("parse ram.vcd");
    let s = vcd.get_signal_by_name("word").expect("word");
    let cases: &[(u32, &str)] = &[
        (1, "bx"),
        (25, "b10010111"),
        (32, "b10110111"),
        (100, "b10110111"),
    ];
    for (ts, expected) in cases {
        let v = s.get_value_at_timestamp(*ts).expect("expected Some");
        assert_eq!(bytes_to_string(v), *expected);
    }
}

#[test]
fn test_get_value_at_timestamp_before_first_change() {
    // If the first value change is at ts=10, then ts<10 should return None
    let path = "/tmp/libvcd_test_before.vcd";
    let content = "$timescale 1ns $end\n\
                   $var wire 1 ! a $end\n\
                   $enddefinitions $end\n\
                   #10\n1!\n#20\n0!\n";
    write_tmp_vcd(path, content);

    let vcd = VCD::read_from_path(path).expect("parse");
    let s = vcd.get_signal_by_name("a").expect("find a");
    assert_eq!(s.value_changes.len(), 2);
    assert!(
        s.get_value_at_timestamp(5).is_none(),
        "before first change should be None"
    );
    let v = s.get_value_at_timestamp(10).expect("Some at exact change");
    assert_eq!(bytes_to_string(v), "1");
    let v = s.get_value_at_timestamp(15).expect("Some between changes");
    assert_eq!(bytes_to_string(v), "1");
    let v = s.get_value_at_timestamp(20).expect("Some at second change");
    assert_eq!(bytes_to_string(v), "0");
    let v = s.get_value_at_timestamp(1000).expect("Some after last change");
    assert_eq!(bytes_to_string(v), "0");
}

// ---------- Custom VCD parsing ----------

#[test]
fn test_parse_minimal_vcd() {
    let path = "/tmp/libvcd_test_minimal.vcd";
    let content = "$timescale\n\t1ns $end\n\
                   $var wire 1 ! a $end\n\
                   $enddefinitions $end\n\
                   #0\n0!\n#5\n1!\n";
    write_tmp_vcd(path, content);

    let vcd = VCD::read_from_path(path).expect("parse minimal");
    assert_eq!(vcd.signals.len(), 1);
    assert_eq!(vcd.timescale.scale, 1);
    // C parses unit as "ns " (with trailing space)
    assert_eq!(bytes_to_string(&vcd.timescale.unit), "ns ");
    assert_eq!(bytes_to_string(&vcd.signals[0].name), "a");
    assert_eq!(vcd.signals[0].size, 1);
    assert_eq!(vcd.signals[0].value_changes.len(), 2);
    assert_eq!(vcd.signals[0].value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_string(&vcd.signals[0].value_changes[0].value), "0");
    assert_eq!(vcd.signals[0].value_changes[1].timestamp, 5);
    assert_eq!(bytes_to_string(&vcd.signals[0].value_changes[1].value), "1");
}

#[test]
fn test_parse_with_date_version_timescale() {
    let path = "/tmp/libvcd_test_full.vcd";
    let content = "$date\n\tToday\n$end\n\
                   $version\n\tv1.0\n$end\n\
                   $timescale\n\t100ps $end\n\
                   $var wire 1 ! sig $end\n\
                   $enddefinitions $end\n\
                   #0\n1!\n";
    write_tmp_vcd(path, content);

    let vcd = VCD::read_from_path(path).expect("parse full");
    assert_eq!(bytes_to_string(&vcd.date), "Today");
    assert_eq!(bytes_to_string(&vcd.version), "v1.0");
    assert_eq!(vcd.timescale.scale, 100);
    assert_eq!(bytes_to_string(&vcd.timescale.unit), "ps ");
}

#[test]
fn test_parse_long_signal_id_ignored() {
    // Per C: signal_ids longer than 1 char are ignored on assignments
    let path = "/tmp/libvcd_test_longid.vcd";
    let content = "$timescale 1ns $end\n\
                   $var wire 1 ! a $end\n\
                   $enddefinitions $end\n\
                   #10\n1!!\n";
    write_tmp_vcd(path, content);

    let vcd = VCD::read_from_path(path).expect("parse");
    let s = vcd.get_signal_by_name("a").expect("find a");
    // The "1!!" assignment uses a 2-char id, so it should be ignored
    assert_eq!(s.value_changes.len(), 0);
}

#[test]
fn test_parse_two_signals() {
    let path = "/tmp/libvcd_test_two.vcd";
    let content = "$timescale 1ns $end\n\
                   $var wire 1 ! a $end\n\
                   $var wire 1 \" b $end\n\
                   $enddefinitions $end\n\
                   #0\n0!\n1\"\n#5\n1!\n0\"\n";
    write_tmp_vcd(path, content);

    let vcd = VCD::read_from_path(path).expect("parse");
    assert_eq!(vcd.signals.len(), 2);
    let a = vcd.get_signal_by_name("a").expect("a");
    let b = vcd.get_signal_by_name("b").expect("b");
    assert_eq!(a.value_changes.len(), 2);
    assert_eq!(b.value_changes.len(), 2);
    assert_eq!(a.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_string(&a.value_changes[0].value), "0");
    assert_eq!(a.value_changes[1].timestamp, 5);
    assert_eq!(bytes_to_string(&a.value_changes[1].value), "1");
    assert_eq!(b.value_changes[0].timestamp, 0);
    assert_eq!(bytes_to_string(&b.value_changes[0].value), "1");
    assert_eq!(b.value_changes[1].timestamp, 5);
    assert_eq!(bytes_to_string(&b.value_changes[1].value), "0");
}

#[test]
fn test_parse_vector_assignment() {
    let path = "/tmp/libvcd_test_vector.vcd";
    let content = "$var reg 8 ! my_signal_name [7:0] $end\n\
                   $enddefinitions $end\n\
                   #0\nb1010 !\n";
    write_tmp_vcd(path, content);

    let vcd = VCD::read_from_path(path).expect("parse");
    assert_eq!(vcd.signals.len(), 1);
    assert_eq!(bytes_to_string(&vcd.signals[0].name), "my_signal_name");
    assert_eq!(vcd.signals[0].size, 8);
    assert_eq!(vcd.signals[0].value_changes.len(), 1);
    assert_eq!(vcd.signals[0].value_changes[0].timestamp, 0);
    assert_eq!(
        bytes_to_string(&vcd.signals[0].value_changes[0].value),
        "b1010"
    );
}

// ---------- Struct construction smoke tests ----------

#[test]
fn test_value_change_struct_fields() {
    let vc = ValueChange {
        timestamp: 42,
        value: [b'b'; VCD_SIGNAL_SIZE],
    };
    assert_eq!(vc.timestamp, 42);
    assert_eq!(vc.value[0], b'b');
    assert_eq!(vc.value.len(), VCD_SIGNAL_SIZE);
}

#[test]
fn test_signal_struct_fields() {
    let s = Signal {
        name: [0u8; VCD_NAME_SIZE],
        size: 7,
        value_changes: Vec::new(),
    };
    assert_eq!(s.size, 7);
    assert_eq!(s.value_changes.len(), 0);
    assert_eq!(s.name.len(), VCD_NAME_SIZE);
}

#[test]
fn test_timescale_struct_fields() {
    let t = Timescale {
        unit: [0u8; VCD_TIME_UNIT_SIZE],
        scale: 100,
    };
    assert_eq!(t.scale, 100);
    assert_eq!(t.unit.len(), VCD_TIME_UNIT_SIZE);
}

fn main() {}
