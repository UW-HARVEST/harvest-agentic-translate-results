use SlothLang::parser;

#[test]
fn test_parse_helloworld_first_codes() {
    let prog = parser::parse("c_src/Examples/HelloWorld.sloth").expect("parse should succeed");
    // From C parser dump (verified by running the C parser on HelloWorld.sloth):
    // 0: 0x01 (PUSH), 1: 0x03, 2: 0x01, 14: 0x08 (OUT), 17: 0x0a (DUP)
    assert_eq!(prog.codes[0], 0x01);
    assert_eq!(prog.codes[1], 0x03);
    assert_eq!(prog.codes[2], 0x01);
    assert_eq!(prog.codes[14], 0x08);
    assert_eq!(prog.codes[17], 0x0a);
    assert_eq!(prog.pc, 0);
}

#[test]
fn test_parse_count_full_bytes() {
    // Full bytecode dump of Count.sloth from running the C parser:
    // 0:01 1:01 2:0a 3:08 4:01 5:01 6:0a 7:08 8:02 9:01 10:01 11:02 12:0a
    // 13:01 14:0b 15:03 16:01 17:01 18:01 19:01 20:03 21:06 22:03 23:09 24:02 25:00
    let expected: [u8; 26] = [
        0x01, 0x01, 0x0a, 0x08, 0x01, 0x01, 0x0a, 0x08, 0x02, 0x01, 0x01, 0x02, 0x0a,
        0x01, 0x0b, 0x03, 0x01, 0x01, 0x01, 0x01, 0x03, 0x06, 0x03, 0x09, 0x02, 0x00,
    ];
    let prog = parser::parse("c_src/Examples/Count.sloth").expect("parse should succeed");
    for (i, &expected_byte) in expected.iter().enumerate() {
        assert_eq!(
            prog.codes[i], expected_byte,
            "byte {} mismatched: got 0x{:02x}, expected 0x{:02x}",
            i, prog.codes[i], expected_byte
        );
    }
}

#[test]
fn test_parse_gamut_full_bytes() {
    // Full bytecode dump of Gamut.sloth from running the C parser:
    // 0..65 ending with 0x00 (EXIT).
    let expected: [u8; 66] = [
        0x01, 0x01, 0x09, 0x02, 0x01, 0x02, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01, 0x02,
        0x03, 0x01, 0x02, 0x01, 0x02, 0x04, 0x01, 0x02, 0x01, 0x02, 0x05, 0x01, 0x02,
        0x01, 0x02, 0x06, 0x01, 0x01, 0x02, 0x01, 0x02, 0x06, 0x02, 0x01, 0x02, 0x01,
        0x02, 0x06, 0x03, 0x01, 0x02, 0x01, 0x02, 0x06, 0x04, 0x01, 0x02, 0x01, 0x02,
        0x06, 0x05, 0x01, 0x02, 0x01, 0x02, 0x06, 0x06, 0x08, 0x01, 0x08, 0x02, 0x0a,
        0x00,
    ];
    let prog = parser::parse("c_src/Examples/Gamut.sloth").expect("parse should succeed");
    for (i, &expected_byte) in expected.iter().enumerate() {
        assert_eq!(
            prog.codes[i], expected_byte,
            "Gamut byte {} mismatched: got 0x{:02x}, expected 0x{:02x}",
            i, prog.codes[i], expected_byte
        );
    }
}

#[test]
fn test_parse_includes_histogram_for_gamut() {
    // Mirrors tests.c histogram check: histogram[7] starts at 1 (INP unused in Gamut).
    let prog = parser::parse("c_src/Examples/Gamut.sloth").expect("parse should succeed");
    let mut histogram = [0u32, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0];
    let mut i = 0;
    loop {
        let pc = prog.codes[i];
        if pc == 0 {
            break;
        }
        if (pc as usize) < 11 {
            histogram[pc as usize] += 1;
        }
        i += 1;
    }
    for j in 1..11 {
        assert!(histogram[j] > 0, "missing opcode {} in Gamut", j);
    }
}

#[test]
fn test_parse_returns_some() {
    let prog = parser::parse("c_src/Examples/Count.sloth");
    assert!(prog.is_some());
}

#[test]
fn test_parse_helloworld_length_terminated() {
    let prog = parser::parse("c_src/Examples/HelloWorld.sloth").expect("parse should succeed");
    // The C tests assert specific bytes; opcode 0x00 ends the program.
    // After running the C parser HelloWorld ends with EXIT at index 162.
    assert_eq!(prog.codes[162], 0x00);
}

#[test]
fn test_free_program_does_not_panic() {
    let prog = parser::parse("c_src/Examples/Count.sloth");
    parser::free_program(prog);
}

#[test]
fn test_prog_len_count() {
    // Count.sloth has 16 lines, prog_len should be 16 * 3 = 48
    let f = std::fs::File::open("c_src/Examples/Count.sloth").expect("open");
    let len = SlothLang::parser::prog_len(&f);
    assert_eq!(len, 16 * 3);
}

#[test]
fn test_prog_len_gamut() {
    // Gamut.sloth has 54 lines.
    let f = std::fs::File::open("c_src/Examples/Gamut.sloth").expect("open");
    let len = SlothLang::parser::prog_len(&f);
    assert_eq!(len, 54 * 3);
}

fn main() {}
