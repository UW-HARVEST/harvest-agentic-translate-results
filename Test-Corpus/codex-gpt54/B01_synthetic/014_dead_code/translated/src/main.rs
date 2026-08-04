fn print_line(line: Option<&str>) {
    if let Some(line) = line {
        println!("{line}");
    }
}

fn helper_bad() {
    print_line(Some("helperBad()"));
}

fn bad() {
    print_line(Some("bad()"));
}

fn helper_good() {
    print_line(Some("helperGood()"));
}

fn good() {
    print_line(Some("good()"));
    helper_good();
}

fn main() {
    let _ = helper_bad as fn();

    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
}
