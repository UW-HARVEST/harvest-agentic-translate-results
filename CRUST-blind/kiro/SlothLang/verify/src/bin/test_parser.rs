use SlothLang::parser;

#[test]
fn test_parse_helloworld() {
    let prog = parser::parse("Examples/HelloWorld.sloth").unwrap();
    assert_eq!(prog.codes[0], 0x01);
    assert_eq!(prog.codes[1], 0x03);
    assert_eq!(prog.codes[2], 0x01);
    assert_eq!(prog.codes[14], 0x08);
    assert_eq!(prog.codes[15], 0x02);
    assert_eq!(prog.codes[17], 0x0a);
    assert_eq!(prog.codes[162], 0x00);
    assert_eq!(prog.codes[163], 0x00);
    assert_eq!(prog.codes.len(), 164);
}

#[test]
fn test_parse_count() {
    let prog = parser::parse("Examples/Count.sloth").unwrap();
    let expected: Vec<u8> = vec![
        0x01, 0x01, 0x0a, 0x08, 0x01, 0x01, 0x0a, 0x08, 0x02,
        0x01, 0x01, 0x02, 0x0a, 0x01, 0x0b, 0x03, 0x01, 0x01,
        0x01, 0x01, 0x03, 0x06, 0x03, 0x09, 0x02, 0x00, 0x00,
    ];
    assert_eq!(prog.codes, expected);
}

#[test]
fn test_parse_gamut_has_all_opcodes() {
    let prog = parser::parse("Examples/Gamut.sloth").unwrap();
    let mut histogram = vec![0i32; 11];
    histogram[7] = 1; // INP not in Gamut but pre-set to 1 per C test
    for &code in &prog.codes {
        if code > 0 && (code as usize) < 11 {
            histogram[code as usize] += 1;
        }
    }
    for i in 1..11 {
        assert!(histogram[i] > 0, "opcode 0x{:02x} missing from Gamut", i);
    }
}

#[test]
fn test_parse_gamut_bytecodes() {
    let prog = parser::parse("Examples/Gamut.sloth").unwrap();
    let expected: Vec<u8> = vec![
        0x01, 0x01, 0x09, 0x02, 0x01, 0x02, 0x01, 0x02, 0x02,
        0x01, 0x02, 0x01, 0x02, 0x03, 0x01, 0x02, 0x01, 0x02,
        0x04, 0x01, 0x02, 0x01, 0x02, 0x05, 0x01, 0x02, 0x01,
        0x02, 0x06, 0x01, 0x01, 0x02, 0x01, 0x02, 0x06, 0x02,
        0x01, 0x02, 0x01, 0x02, 0x06, 0x03, 0x01, 0x02, 0x01,
        0x02, 0x06, 0x04, 0x01, 0x02, 0x01, 0x02, 0x06, 0x05,
        0x01, 0x02, 0x01, 0x02, 0x06, 0x06, 0x08, 0x01, 0x08,
        0x02, 0x0a, 0x00, 0x00,
    ];
    assert_eq!(prog.codes, expected);
}

#[test]
fn test_count_execution_via_parser() {
    let mut prog = parser::parse("Examples/Count.sloth");
    let result = SlothLang::slothvm::execute(&mut prog);
    assert_eq!(result, 11);
}

#[test]
fn test_free_program() {
    let prog = parser::parse("Examples/HelloWorld.sloth");
    parser::free_program(prog);
}

fn main() {}
