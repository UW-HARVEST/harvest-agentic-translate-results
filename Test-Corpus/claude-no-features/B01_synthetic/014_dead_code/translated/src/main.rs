use std::io::Write;

fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = handle.write_all(s.as_bytes());
        let _ = handle.write_all(b"\n");
    }
}

fn helper_good() {
    print_line(Some("helperGood()"));
}

fn good() {
    print_line(Some("good()"));
    helper_good();
}

fn bad() {
    print_line(Some("bad()"));
}

fn main() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
}
