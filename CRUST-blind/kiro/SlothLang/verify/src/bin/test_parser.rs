use SlothLang::parser;
use SlothLang::slothvm;

#[test]
fn test_parse_helloworld_bytecodes() {
    let prog = parser::parse("Examples/HelloWorld.sloth").unwrap();
    // 163 meaningful codes + 1 trailing 0x00 from hadToken on nap line
    assert_eq!(prog.codes.len(), 164);
    assert_eq!(prog.codes[0], 0x01);  // PUSH
    assert_eq!(prog.codes[1], 0x03);
    assert_eq!(prog.codes[2], 0x01);  // PUSH
    assert_eq!(prog.codes[14], 0x08); // OUT
    assert_eq!(prog.codes[17], 0x0a);
    assert_eq!(prog.codes[162], 0x00); // EXIT (nap)
    assert_eq!(prog.codes[163], 0x00); // trailing from hadToken
}

#[test]
fn test_parse_count_bytecodes() {
    let prog = parser::parse("Examples/Count.sloth").unwrap();
    assert_eq!(prog.codes.len(), 27);
    assert_eq!(prog.codes[0], 0x01);  // PUSH
    assert_eq!(prog.codes[1], 0x01);  // value 1
    assert_eq!(prog.codes[2], 0x0a);  // DUP
    assert_eq!(prog.codes[25], 0x00); // EXIT
}

#[test]
fn test_parse_gamut_all_opcodes_present() {
    let prog = parser::parse("Examples/Gamut.sloth").unwrap();
    assert_eq!(prog.codes.len(), 67);

    let mut histogram = vec![0u32; 11];
    // Gamut has INP commented out, so pre-seed opcode 7
    histogram[7] = 1;
    for &code in &prog.codes {
        if (code as usize) < 11 {
            histogram[code as usize] += 1;
        }
    }
    for i in 1..11 {
        assert!(histogram[i] > 0, "opcode {} not found in Gamut.sloth", i);
    }
}

#[test]
fn test_parse_gamut_specific_codes() {
    let prog = parser::parse("Examples/Gamut.sloth").unwrap();
    assert_eq!(prog.codes[0], 0x01);  // PUSH
    assert_eq!(prog.codes[1], 0x01);
    assert_eq!(prog.codes[2], 0x09);  // GOTO
    assert_eq!(prog.codes[65], 0x00); // EXIT
}

// Integration: parse + execute Count.sloth returns 11
#[test]
fn test_parse_and_execute_count() {
    let mut prog = parser::parse("Examples/Count.sloth");
    assert_eq!(slothvm::execute(&mut prog), 11);
}

#[test]
fn test_free_program() {
    let prog = parser::parse("Examples/HelloWorld.sloth");
    parser::free_program(prog);
}

#[test]
fn test_free_program_none() {
    parser::free_program(None);
}

fn main() {}
