pub fn print_array(matrix: &[Vec<f32>], name: &str, typ: &str) -> i32 {
    println!("-------------{}--------------", name);
    if typ == "float" {
        print_float_array(matrix);
    } else {
        print!("unsupported {}", typ);
    }
    0
}
pub fn print_float_array(matrix: &[Vec<f32>]) -> i32 {
    for row in matrix {
        for val in row {
            print!("{:10.2} ", val);
        }
        println!();
    }
    0
}
