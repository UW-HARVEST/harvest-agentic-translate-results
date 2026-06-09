fn print_line(line: Option<&str>) {
    if let Some(s) = line {
        println!("{}", s);
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

fn driver() {
    print_line(Some("Calling good()..."));
    good();
    print_line(Some("Finished good()"));
    print_line(Some("Calling bad()..."));
    bad();
    print_line(Some("Finished bad()"));
}

fn main() {
    driver();
}
