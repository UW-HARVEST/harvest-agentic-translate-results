use std::io::{self, Write};

fn print_line(output: &mut impl Write, line: Option<&str>) {
    if let Some(line) = line {
        let _ = writeln!(output, "{line}");
    }
}

#[allow(dead_code)]
fn helper_bad(output: &mut impl Write) {
    print_line(output, Some("helperBad()"));
}

fn bad(output: &mut impl Write) {
    print_line(output, Some("bad()"));
}

fn helper_good(output: &mut impl Write) {
    print_line(output, Some("helperGood()"));
}

fn good(output: &mut impl Write) {
    print_line(output, Some("good()"));
    helper_good(output);
}

fn main() {
    let stdout = io::stdout();
    let mut output = stdout.lock();

    print_line(&mut output, Some("Calling good()..."));
    good(&mut output);
    print_line(&mut output, Some("Finished good()"));
    print_line(&mut output, Some("Calling bad()..."));
    bad(&mut output);
    print_line(&mut output, Some("Finished bad()"));
}
