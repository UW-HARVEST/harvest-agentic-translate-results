use SlothLang::throw;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        std::process::exit(0);
    }
    match args[1].as_str() {
        "math_err" => throw::math_err("division by zero"),
        "op_err" => throw::op_err("operation", 0x0a),
        "math_err_empty" => throw::math_err(""),
        "op_err_empty" => throw::op_err("", 0x01),
        "op_err_99" => throw::op_err("input type", 0x99),
        _ => std::process::exit(0),
    }
}
