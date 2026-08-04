/// Print a matrix with a header banner, dispatching on `typ`.
///
/// Mirrors `int print_array(float **matrix, int x, int y, char* name, char* type)`
/// from c_src/utils.c.
pub fn print_array(matrix: &[Vec<f32>], name: &str, typ: &str) -> i32 {
    println!("-------------{}--------------", name);
    if typ == "float" {
        print_float_array(matrix);
    } else {
        // Match the C `printf("unsupported %s", type)` (no trailing newline).
        print!("unsupported {}", typ);
    }
    0
}

/// Print every cell of a matrix using the C-style "%10.2f " formatting.
pub fn print_float_array(matrix: &[Vec<f32>]) -> i32 {
    for row in matrix.iter() {
        for value in row.iter() {
            print!("{:>10.2} ", value);
        }
        println!();
    }
    0
}
