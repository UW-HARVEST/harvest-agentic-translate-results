fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(n: i32) {
    println!("{}", n);
}

fn bad() {
    let int_one: i32 = 1;
    let _int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    // C bug: intOne + intTwo; (result discarded, intSum unchanged)
    let _ = int_one + _int_two;
    print_int_line(int_sum);
}

fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
