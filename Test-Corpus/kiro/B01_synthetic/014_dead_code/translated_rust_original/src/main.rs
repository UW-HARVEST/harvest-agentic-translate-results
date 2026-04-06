fn print_line(line: &str) {
    println!("{}", line);
}

#[allow(dead_code)]
fn helper_bad() {
    print_line("helperBad()");
}

fn bad() {
    print_line("bad()");
}

fn helper_good() {
    print_line("helperGood()");
}

fn good() {
    print_line("good()");
    helper_good();
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
