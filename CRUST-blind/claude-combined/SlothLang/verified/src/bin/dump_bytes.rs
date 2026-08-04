use SlothLang::parser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: dump_bytes <file>");
        return;
    }
    let prog = parser::parse(&args[1]).expect("parse failed");
    let mut i = 0;
    loop {
        if i >= prog.codes.len() {
            break;
        }
        println!("{}: 0x{:02x}", i, prog.codes[i]);
        if prog.codes[i] == 0 {
            break;
        }
        i += 1;
    }
}
