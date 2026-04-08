use expr::mapper_expr::{mapper_expr_new_from_string, mapper_expr_evaluate, MapperSignalValue};

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() < eps
}

// Helper to evaluate a float expression with no variable dependency
fn eval_float_const(expr_str: &str) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 1, 1);
    let inp = MapperSignalValue::F(0.0);
    match mapper_expr_evaluate(&mut e, &inp) {
        MapperSignalValue::F(f) => f,
        MapperSignalValue::I32(i) => i as f32,
    }
}

fn eval_int_const(expr_str: &str) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 0, 1);
    let inp = MapperSignalValue::I32(0);
    match mapper_expr_evaluate(&mut e, &inp) {
        MapperSignalValue::I32(i) => i,
        MapperSignalValue::F(f) => f as i32,
    }
}

// === Integer arithmetic ===

#[test]
fn test_int_addition() {
    assert_eq!(eval_int_const("y=2+3"), 5);
}

#[test]
fn test_int_subtraction() {
    assert_eq!(eval_int_const("y=10-3"), 7);
}

#[test]
fn test_int_multiplication() {
    assert_eq!(eval_int_const("y=6*7"), 42);
}

#[test]
fn test_int_division() {
    assert_eq!(eval_int_const("y=20/4"), 5);
}

#[test]
fn test_int_division_truncation() {
    assert_eq!(eval_int_const("y=7/2"), 3);
}

#[test]
fn test_int_negation() {
    assert_eq!(eval_int_const("y=-5"), -5);
}

#[test]
fn test_int_precedence() {
    // 2 + 3*4 = 14
    assert_eq!(eval_int_const("y=2+3*4"), 14);
}

#[test]
fn test_int_parentheses() {
    // (2+3)*4 = 20
    assert_eq!(eval_int_const("y=(2+3)*4"), 20);
}

#[test]
fn test_int_variable() {
    let mut e = mapper_expr_new_from_string("y=x*2", 0, 0, 1);
    let inp = MapperSignalValue::I32(5);
    let out = mapper_expr_evaluate(&mut e, &inp);
    assert_eq!(out.as_i32(), Some(10));
}

#[test]
fn test_int_complex() {
    // y=26*2/2+x*30/(20*1) with x=3 => 30, x=321 => 507
    let mut e = mapper_expr_new_from_string("y=26*2/2+x*30/(20*1)", 0, 0, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(3));
    assert_eq!(out.as_i32(), Some(30));

    let out2 = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(321));
    assert_eq!(out2.as_i32(), Some(507));
}

// === Float arithmetic ===

#[test]
fn test_float_addition() {
    assert!(approx_eq(eval_float_const("y=2.5+3.5"), 6.0, 1e-5));
}

#[test]
fn test_float_negation() {
    assert!(approx_eq(eval_float_const("y=-3.5"), -3.5, 1e-5));
}

#[test]
fn test_float_variable() {
    let mut e = mapper_expr_new_from_string("y=x*2.0", 1, 1, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(5.0));
    assert!(approx_eq(out.as_f32().unwrap(), 10.0, 1e-5));
}

// === Math functions ===

#[test]
fn test_sin() {
    assert!(approx_eq(eval_float_const("y=sin(0.0)"), 0.0, 1e-5));
}

#[test]
fn test_cos() {
    assert!(approx_eq(eval_float_const("y=cos(0.0)"), 1.0, 1e-5));
}

#[test]
fn test_sqrt() {
    assert!(approx_eq(eval_float_const("y=sqrt(9.0)"), 3.0, 1e-5));
}

#[test]
fn test_abs() {
    assert!(approx_eq(eval_float_const("y=abs(-4.0)"), 4.0, 1e-5));
}

#[test]
fn test_floor() {
    assert!(approx_eq(eval_float_const("y=floor(3.7)"), 3.0, 1e-5));
}

#[test]
fn test_ceil() {
    assert!(approx_eq(eval_float_const("y=ceil(3.2)"), 4.0, 1e-5));
}

#[test]
fn test_round() {
    assert!(approx_eq(eval_float_const("y=round(3.5)"), 4.0, 1e-5));
}

#[test]
fn test_pow() {
    assert!(approx_eq(eval_float_const("y=pow(2.0,10.0)"), 1024.0, 1e-2));
}

#[test]
fn test_min() {
    assert!(approx_eq(eval_float_const("y=min(3.0,5.0)"), 3.0, 1e-5));
}

#[test]
fn test_max() {
    assert!(approx_eq(eval_float_const("y=max(3.0,5.0)"), 5.0, 1e-5));
}

#[test]
fn test_pi() {
    assert!(approx_eq(eval_float_const("y=pi"), std::f32::consts::PI, 1e-5));
}

#[test]
fn test_log() {
    assert!(approx_eq(eval_float_const("y=log(1.0)"), 0.0, 1e-5));
}

#[test]
fn test_log10() {
    assert!(approx_eq(eval_float_const("y=log10(100.0)"), 2.0, 1e-5));
}

#[test]
fn test_exp() {
    assert!(approx_eq(eval_float_const("y=exp(0.0)"), 1.0, 1e-5));
}

#[test]
fn test_atan2() {
    // atan2(1.0, 1.0) = pi/4 ≈ 0.785398
    assert!(approx_eq(eval_float_const("y=atan2(1.0,1.0)"), std::f32::consts::FRAC_PI_4, 1e-4));
}

#[test]
fn test_hypot() {
    assert!(approx_eq(eval_float_const("y=hypot(3.0,4.0)"), 5.0, 1e-5));
}

#[test]
fn test_exp2() {
    assert!(approx_eq(eval_float_const("y=exp2(3.0)"), 8.0, 1e-5));
}

#[test]
fn test_log2() {
    assert!(approx_eq(eval_float_const("y=log2(8.0)"), 3.0, 1e-5));
}

#[test]
fn test_cbrt() {
    assert!(approx_eq(eval_float_const("y=cbrt(27.0)"), 3.0, 1e-5));
}

#[test]
fn test_trunc() {
    assert!(approx_eq(eval_float_const("y=trunc(3.9)"), 3.0, 1e-5));
}

#[test]
fn test_asin() {
    // asin(1.0) = pi/2 ≈ 1.570796
    assert!(approx_eq(eval_float_const("y=asin(1.0)"), std::f32::consts::FRAC_PI_2, 1e-4));
}

#[test]
fn test_acos() {
    assert!(approx_eq(eval_float_const("y=acos(1.0)"), 0.0, 1e-5));
}

#[test]
fn test_atan() {
    // atan(1.0) = pi/4 ≈ 0.785398
    assert!(approx_eq(eval_float_const("y=atan(1.0)"), std::f32::consts::FRAC_PI_4, 1e-4));
}

#[test]
fn test_sinh() {
    // sinh(1.0) ≈ 1.175201
    assert!(approx_eq(eval_float_const("y=sinh(1.0)"), 1.0_f32.sinh(), 1e-4));
}

#[test]
fn test_cosh() {
    // cosh(1.0) ≈ 1.543081
    assert!(approx_eq(eval_float_const("y=cosh(1.0)"), 1.0_f32.cosh(), 1e-4));
}

#[test]
fn test_tanh() {
    // tanh(1.0) ≈ 0.761594
    assert!(approx_eq(eval_float_const("y=tanh(1.0)"), 1.0_f32.tanh(), 1e-4));
}

#[test]
fn test_logb() {
    // logb(8.0) = 3.0
    assert!(approx_eq(eval_float_const("y=logb(8.0)"), 3.0, 1e-5));
}

// === Type coercion ===

#[test]
fn test_int_to_float_coercion() {
    // int input, float output: y=x+1 with x=10 => 11.0
    let mut e = mapper_expr_new_from_string("y=x+1", 0, 1, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(10));
    assert!(approx_eq(out.as_f32().unwrap(), 11.0, 1e-5));
}

#[test]
fn test_float_to_int_coercion() {
    // float input, int output: y=x+1.5 with x=10.0 => 11
    let mut e = mapper_expr_new_from_string("y=x+1.5", 1, 0, 1);
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(10.0));
    assert_eq!(out.as_i32(), Some(11));
}

// === Complex expression from C test1 ===

#[test]
fn test_complex_expression() {
    // y=26*2/2+log10(pi)+2.*pow(2,1*(3+7*.1)*1.1+x{-6*2+12})*3*4+cos(2.)
    // with x=3.0 => 3250.810791
    let mut e = mapper_expr_new_from_string(
        "y=26*2/2+log10(pi)+2.*pow(2,1*(3+7*.1)*1.1+x{-6*2+12})*3*4+cos(2.)",
        1, 1, 1,
    );
    let out = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(3.0));
    assert!(approx_eq(out.as_f32().unwrap(), 3250.810791, 0.01));
}

fn main() {}
