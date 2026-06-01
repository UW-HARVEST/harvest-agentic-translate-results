use expr::mapper_expr::{
    mapper_expr_evaluate, mapper_expr_new_from_string, MapperSignalValue,
};

fn approx_eq(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn eval_int(expr_str: &str, x: i32) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 0, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(x));
    match out {
        MapperSignalValue::I32(v) => v,
        MapperSignalValue::F(f) => f as i32,
    }
}

fn eval_float(expr_str: &str, x: f32) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 1, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(x));
    match out {
        MapperSignalValue::F(v) => v,
        MapperSignalValue::I32(i) => i as f32,
    }
}

fn eval_int_to_float(expr_str: &str, x: i32) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 1, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(x));
    match out {
        MapperSignalValue::F(v) => v,
        MapperSignalValue::I32(i) => i as f32,
    }
}

fn eval_float_to_int(expr_str: &str, x: f32) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 0, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(x));
    match out {
        MapperSignalValue::I32(v) => v,
        MapperSignalValue::F(f) => f as i32,
    }
}

#[test]
fn test_mapper_signal_value_as_f32() {
    assert_eq!(MapperSignalValue::F(2.5).as_f32(), Some(2.5));
    assert_eq!(MapperSignalValue::I32(3).as_f32(), None);
}

#[test]
fn test_mapper_signal_value_as_i32() {
    assert_eq!(MapperSignalValue::I32(7).as_i32(), Some(7));
    assert_eq!(MapperSignalValue::F(2.5).as_i32(), None);
}

#[test]
fn test_int_identity() {
    // C: y=x with x=5 -> 5
    assert_eq!(eval_int("y=x", 5), 5);
}

#[test]
fn test_int_plus_one() {
    // C: y=x+1 with x=10 -> 11
    assert_eq!(eval_int("y=x+1", 10), 11);
}

#[test]
fn test_int_mul() {
    // C: y=x*2 with x=7 -> 14
    assert_eq!(eval_int("y=x*2", 7), 14);
}

#[test]
fn test_int_minus() {
    // C: y=x-1 with x=0 -> -1
    assert_eq!(eval_int("y=x-1", 0), -1);
}

#[test]
fn test_int_division() {
    // C: y=10/x with x=3 -> 3 (integer division)
    assert_eq!(eval_int("y=10/x", 3), 3);
}

#[test]
fn test_constant_folding_precedence() {
    // C: y=2+3*4 -> 14 (constant-folded; * binds tighter than +)
    assert_eq!(eval_int("y=2+3*4", 0), 14);
}

#[test]
fn test_constant_folding_parens() {
    // C: y=(2+3)*4 -> 20 (constant-folded)
    assert_eq!(eval_int("y=(2+3)*4", 0), 20);
}

#[test]
fn test_negate_var() {
    // C: y=-x with x=7 -> -7
    assert_eq!(eval_int("y=-x", 7), -7);
}

#[test]
fn test_negate_constant_plus_var() {
    // C: y=-5+x with x=10 -> 5
    assert_eq!(eval_int("y=-5+x", 10), 5);
}

#[test]
fn test_x_squared() {
    // C: y=x*x with x=6 -> 36
    assert_eq!(eval_int("y=x*x", 6), 36);
}

#[test]
fn test_test2_first() {
    // From test.c::test2: y=26*2/2+x*30/(20*1), x=3 -> 30
    assert_eq!(eval_int("y=26*2/2+x*30/(20*1)", 3), 30);
}

#[test]
fn test_test2_second() {
    // From test.c::test2: y=26*2/2+x*30/(20*1), x=321 -> 507
    assert_eq!(eval_int("y=26*2/2+x*30/(20*1)", 321), 507);
}

#[test]
fn test_float_identity() {
    let v = eval_float("y=x", 1.5);
    assert!(approx_eq(v, 1.5, 1e-6), "got {}", v);
}

#[test]
fn test_float_add_constant() {
    // y=x+1.5, x=2.0 -> 3.5
    let v = eval_float("y=x+1.5", 2.0);
    assert!(approx_eq(v, 3.5, 1e-6), "got {}", v);
}

#[test]
fn test_float_mul() {
    // y=2.5*x, x=4.0 -> 10.0
    let v = eval_float("y=2.5*x", 4.0);
    assert!(approx_eq(v, 10.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_sin_zero() {
    let v = eval_float("y=sin(x)", 0.0);
    assert!(approx_eq(v, 0.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_cos_zero() {
    let v = eval_float("y=cos(0.0)", 0.0);
    assert!(approx_eq(v, 1.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_pow() {
    // pow(2,3) -> 8
    let v = eval_float("y=pow(2,3)", 0.0);
    assert!(approx_eq(v, 8.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_sqrt() {
    let v = eval_float("y=sqrt(16.0)", 0.0);
    assert!(approx_eq(v, 4.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_abs() {
    let v = eval_float("y=abs(-3.5)", 0.0);
    assert!(approx_eq(v, 3.5, 1e-6), "got {}", v);
}

#[test]
fn test_func_pi() {
    let v = eval_float("y=pi", 0.0);
    assert!(approx_eq(v, std::f32::consts::PI, 1e-5), "got {}", v);
}

#[test]
fn test_func_min() {
    let v = eval_float("y=min(1.0,2.0)", 0.0);
    assert!(approx_eq(v, 1.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_max() {
    let v = eval_float("y=max(1.0,2.0)", 0.0);
    assert!(approx_eq(v, 2.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_log10() {
    let v = eval_float("y=log10(100.0)", 0.0);
    assert!(approx_eq(v, 2.0, 1e-5), "got {}", v);
}

#[test]
fn test_func_log() {
    let v = eval_float("y=log(2.718281828)", 0.0);
    assert!(approx_eq(v, 1.0, 1e-5), "got {}", v);
}

#[test]
fn test_func_exp() {
    let v = eval_float("y=exp(1.0)", 0.0);
    assert!(approx_eq(v, std::f32::consts::E, 1e-5), "got {}", v);
}

#[test]
fn test_func_floor() {
    let v = eval_float("y=floor(3.7)", 0.0);
    assert!(approx_eq(v, 3.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_ceil() {
    let v = eval_float("y=ceil(3.2)", 0.0);
    assert!(approx_eq(v, 4.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_round() {
    let v = eval_float("y=round(3.5)", 0.0);
    assert!(approx_eq(v, 4.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_tan_zero() {
    let v = eval_float("y=tan(0.0)", 0.0);
    assert!(approx_eq(v, 0.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_asin_one() {
    let v = eval_float("y=asin(1.0)", 0.0);
    assert!(approx_eq(v, std::f32::consts::FRAC_PI_2, 1e-5), "got {}", v);
}

#[test]
fn test_func_acos_zero() {
    let v = eval_float("y=acos(0.0)", 0.0);
    assert!(approx_eq(v, std::f32::consts::FRAC_PI_2, 1e-5), "got {}", v);
}

#[test]
fn test_func_atan_one() {
    let v = eval_float("y=atan(1.0)", 0.0);
    assert!(approx_eq(v, std::f32::consts::FRAC_PI_4, 1e-5), "got {}", v);
}

#[test]
fn test_func_atan2() {
    let v = eval_float("y=atan2(1.0,1.0)", 0.0);
    assert!(approx_eq(v, std::f32::consts::FRAC_PI_4, 1e-5), "got {}", v);
}

#[test]
fn test_func_hypot() {
    let v = eval_float("y=hypot(3.0,4.0)", 0.0);
    assert!(approx_eq(v, 5.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_cbrt() {
    let v = eval_float("y=cbrt(27.0)", 0.0);
    assert!(approx_eq(v, 3.0, 1e-5), "got {}", v);
}

#[test]
fn test_func_trunc() {
    let v = eval_float("y=trunc(3.7)", 0.0);
    assert!(approx_eq(v, 3.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_exp2() {
    let v = eval_float("y=exp2(3.0)", 0.0);
    assert!(approx_eq(v, 8.0, 1e-6), "got {}", v);
}

#[test]
fn test_func_log2() {
    let v = eval_float("y=log2(8.0)", 0.0);
    assert!(approx_eq(v, 3.0, 1e-6), "got {}", v);
}

#[test]
fn test_input_int_output_float_identity() {
    let v = eval_int_to_float("y=x", 5);
    assert!(approx_eq(v, 5.0, 1e-6), "got {}", v);
}

#[test]
fn test_input_int_output_float_plus_one() {
    let v = eval_int_to_float("y=x+1", 5);
    assert!(approx_eq(v, 6.0, 1e-6), "got {}", v);
}

#[test]
fn test_input_float_output_int_identity() {
    let v = eval_float_to_int("y=x", 3.7);
    assert_eq!(v, 3);
}

#[test]
fn test_input_float_output_int_mul() {
    let v = eval_float_to_int("y=x*2.0", 1.5);
    assert_eq!(v, 3);
}

#[test]
fn test_test1_complex() {
    // From test.c::test1: full expression with x=3.0
    // C output: 3250.810791
    let s = "y=26*2/2+log10(pi)+2.*pow(2,1*(3+7*.1)*1.1+x{-6*2+12})*3*4+cos(2.)";
    let v = eval_float(s, 3.0);
    let expected: f32 = 26.0 * 2.0 / 2.0
        + (std::f32::consts::PI as f32).log10()
        + 2.0_f32 * (2.0_f32).powf(1.0 * (3.0 + 7.0 * 0.1_f32) * 1.1_f32 + 3.0) * 3.0 * 4.0
        + (2.0_f32).cos();
    assert!(approx_eq(v, expected, 1e-2), "got {}, expected {}", v, expected);
    // Also exact value from C run
    assert!(approx_eq(v, 3250.810791, 1e-2), "got {}", v);
}

#[test]
fn test_struct_fields() {
    // Verify MapperExpr struct fields are computed correctly for a simple expr.
    let e = mapper_expr_new_from_string("y=x", 0, 0, 1);
    assert_eq!(e.vector_size, 1);
    assert_eq!(e.history_size, 1);
    assert_eq!(e.history_pos, -1);
    // input_history sized for vector_size * history_size = 1 entry
    assert_eq!(e.input_history.len(), 1);
    assert_eq!(e.output_history.len(), 1);
}

#[test]
fn test_history_size_with_history_index() {
    // Reference x{-3} should require history_size = ceil(3) + 1 = 4
    let e = mapper_expr_new_from_string("y=x{-3}", 0, 0, 1);
    assert_eq!(e.history_size, 4);
    assert_eq!(e.input_history.len(), 4);
    assert_eq!(e.output_history.len(), 4);
}

#[test]
fn test_repeated_evaluation_history() {
    // Evaluate y=x+1 multiple times and ensure history_pos advances
    let mut e = mapper_expr_new_from_string("y=x+1", 0, 0, 1);
    assert_eq!(e.history_pos, -1);
    let r1 = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(2));
    assert_eq!(r1.as_i32(), Some(3));
    assert_eq!(e.history_pos, 0);
    let r2 = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(5));
    assert_eq!(r2.as_i32(), Some(6));
}

fn main() {}
