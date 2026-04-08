use expr::mapper_expr::{mapper_expr_new_from_string, mapper_expr_evaluate, MapperSignalValue};

fn eval_float(expr_str: &str, input: f32) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 1, 1);
    let inp = MapperSignalValue::F(input);
    match mapper_expr_evaluate(&mut e, &inp) {
        MapperSignalValue::F(f) => f,
        MapperSignalValue::I32(i) => i as f32,
    }
}

fn eval_int(expr_str: &str, input: i32) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 0, 1);
    let inp = MapperSignalValue::I32(input);
    match mapper_expr_evaluate(&mut e, &inp) {
        MapperSignalValue::I32(i) => i,
        MapperSignalValue::F(f) => f as i32,
    }
}

fn eval_float_to_int(expr_str: &str, input: f32) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 0, 1);
    let inp = MapperSignalValue::F(input);
    match mapper_expr_evaluate(&mut e, &inp) {
        MapperSignalValue::I32(i) => i,
        MapperSignalValue::F(f) => f as i32,
    }
}

fn eval_int_to_float(expr_str: &str, input: i32) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 1, 1);
    let inp = MapperSignalValue::I32(input);
    match mapper_expr_evaluate(&mut e, &inp) {
        MapperSignalValue::F(f) => f,
        MapperSignalValue::I32(i) => i as f32,
    }
}

fn assert_float_eq(a: f32, b: f32, tol: f32) {
    assert!((a - b).abs() <= tol, "expected {b}, got {a}, diff={}", (a - b).abs());
}

// ---- Original C test cases ----

#[test]
fn test1_float_complex_expression() {
    let result = eval_float(
        "y=26*2/2+log10(pi)+2.*pow(2,1*(3+7*.1)*1.1+x{-6*2+12})*3*4+cos(2.)",
        3.0,
    );
    assert_float_eq(result, 3250.810791, 0.01);
}

#[test]
fn test2_int_x3() {
    assert_eq!(eval_int("y=26*2/2+x*30/(20*1)", 3), 30);
}

#[test]
fn test2_int_x321() {
    assert_eq!(eval_int("y=26*2/2+x*30/(20*1)", 321), 507);
}

// ---- Simple integer arithmetic ----

#[test]
fn test_add_int() {
    assert_eq!(eval_int("y=x+10", 5), 15);
}

#[test]
fn test_sub_int() {
    assert_eq!(eval_int("y=x-10", 5), -5);
}

#[test]
fn test_mul_int() {
    assert_eq!(eval_int("y=x*3", 7), 21);
}

#[test]
fn test_div_int() {
    assert_eq!(eval_int("y=x/4", 20), 5);
}

// ---- Simple float arithmetic ----

#[test]
fn test_add_float() {
    assert_float_eq(eval_float("y=x+10.0", 5.0), 15.0, 1e-5);
}

#[test]
fn test_sub_float() {
    assert_float_eq(eval_float("y=x-10.0", 5.0), -5.0, 1e-5);
}

#[test]
fn test_mul_float() {
    assert_float_eq(eval_float("y=x*3.0", 7.0), 21.0, 1e-5);
}

#[test]
fn test_div_float() {
    assert_float_eq(eval_float("y=x/4.0", 20.0), 5.0, 1e-5);
}

// ---- Constants ----

#[test]
fn test_const_int() {
    assert_eq!(eval_int("y=42", 0), 42);
}

#[test]
fn test_const_float() {
    assert_float_eq(eval_float("y=3.14", 0.0), 3.14, 1e-4);
}

// ---- Negation ----

#[test]
fn test_neg_int() {
    assert_eq!(eval_int("y=-x", 5), -5);
}

#[test]
fn test_neg_float() {
    assert_float_eq(eval_float("y=-x", 5.0), -5.0, 1e-5);
}

#[test]
fn test_neg_const_int() {
    assert_eq!(eval_int("y=-42", 0), -42);
}

// ---- Math functions (all float) ----

#[test]
fn test_sin() {
    assert_float_eq(eval_float("y=sin(x)", 1.0), 0.8414709568, 1e-5);
}

#[test]
fn test_cos() {
    assert_float_eq(eval_float("y=cos(x)", 1.0), 0.5403022766, 1e-5);
}

#[test]
fn test_tan() {
    assert_float_eq(eval_float("y=tan(x)", 1.0), 1.557407737, 1e-5);
}

#[test]
fn test_abs() {
    assert_float_eq(eval_float("y=abs(x)", -3.5), 3.5, 1e-5);
}

#[test]
fn test_sqrt() {
    assert_float_eq(eval_float("y=sqrt(x)", 16.0), 4.0, 1e-5);
}

#[test]
fn test_log() {
    assert_float_eq(eval_float("y=log(x)", 2.718281828), 1.0, 1e-5);
}

#[test]
fn test_log10() {
    assert_float_eq(eval_float("y=log10(x)", 100.0), 2.0, 1e-5);
}

#[test]
fn test_exp() {
    assert_float_eq(eval_float("y=exp(x)", 1.0), 2.718281746, 1e-5);
}

#[test]
fn test_floor() {
    assert_float_eq(eval_float("y=floor(x)", 3.7), 3.0, 1e-5);
}

#[test]
fn test_round() {
    assert_float_eq(eval_float("y=round(x)", 3.5), 4.0, 1e-5);
}

#[test]
fn test_ceil() {
    assert_float_eq(eval_float("y=ceil(x)", 3.2), 4.0, 1e-5);
}

#[test]
fn test_asin() {
    assert_float_eq(eval_float("y=asin(x)", 0.5), 0.5235987902, 1e-5);
}

#[test]
fn test_acos() {
    assert_float_eq(eval_float("y=acos(x)", 0.5), 1.04719758, 1e-5);
}

#[test]
fn test_atan() {
    assert_float_eq(eval_float("y=atan(x)", 1.0), 0.7853981853, 1e-5);
}

#[test]
fn test_atan2() {
    assert_float_eq(eval_float("y=atan2(x,2.0)", 1.0), 0.463647604, 1e-5);
}

#[test]
fn test_sinh() {
    assert_float_eq(eval_float("y=sinh(x)", 1.0), 1.175201178, 1e-5);
}

#[test]
fn test_cosh() {
    assert_float_eq(eval_float("y=cosh(x)", 1.0), 1.543080568, 1e-5);
}

#[test]
fn test_tanh() {
    assert_float_eq(eval_float("y=tanh(x)", 1.0), 0.7615941763, 1e-5);
}

#[test]
fn test_exp2() {
    assert_float_eq(eval_float("y=exp2(x)", 3.0), 8.0, 1e-5);
}

#[test]
fn test_log2() {
    assert_float_eq(eval_float("y=log2(x)", 8.0), 3.0, 1e-5);
}

#[test]
fn test_hypot() {
    assert_float_eq(eval_float("y=hypot(x,4.0)", 3.0), 5.0, 1e-5);
}

#[test]
fn test_cbrt() {
    assert_float_eq(eval_float("y=cbrt(x)", 27.0), 3.0, 1e-5);
}

#[test]
fn test_trunc() {
    assert_float_eq(eval_float("y=trunc(x)", 3.7), 3.0, 1e-5);
}

#[test]
fn test_min() {
    assert_float_eq(eval_float("y=min(x,2.0)", 3.0), 2.0, 1e-5);
}

#[test]
fn test_max() {
    assert_float_eq(eval_float("y=max(x,2.0)", 3.0), 3.0, 1e-5);
}

#[test]
fn test_pi() {
    assert_float_eq(eval_float("y=pi", 0.0), std::f32::consts::PI, 1e-5);
}

#[test]
fn test_pow() {
    assert_float_eq(eval_float("y=pow(x,2.0)", 3.0), 9.0, 1e-5);
}

#[test]
fn test_logb() {
    assert_float_eq(eval_float("y=logb(x)", 8.0), 3.0, 1e-5);
}

// ---- Type coercion ----

#[test]
fn test_float_to_int_coercion() {
    assert_eq!(eval_float_to_int("y=x*2.5", 3.0), 7);
}

#[test]
fn test_int_to_float_coercion() {
    assert_float_eq(eval_int_to_float("y=x+1", 5), 6.0, 1e-5);
}

// ---- Nested / parenthesized expressions ----

#[test]
fn test_nested_sincos() {
    assert_float_eq(eval_float("y=sin(cos(x))", 0.0), 0.8414709568, 1e-5);
}

#[test]
fn test_paren_expr() {
    assert_float_eq(eval_float("y=(x+1.0)*(x-1.0)", 3.0), 8.0, 1e-5);
}

// ---- Integer division truncation ----

#[test]
fn test_int_div_truncation() {
    assert_eq!(eval_int("y=x/3", 10), 3);
}

#[test]
fn test_int_div_negative() {
    assert_eq!(eval_int("y=x/3", -10), -3);
}

// ---- Quadratic expression ----

#[test]
fn test_quadratic() {
    assert_float_eq(eval_float("y=x*x+2.0*x+1.0", 3.0), 16.0, 1e-5);
}

// ---- History index ----

#[test]
fn test_history_zero_index() {
    assert_float_eq(eval_float("y=x{0}*2.0", 5.0), 10.0, 1e-5);
}

// ---- MapperSignalValue accessors ----

#[test]
fn test_signal_value_f32() {
    let v = MapperSignalValue::F(1.5);
    assert_eq!(v.as_f32(), Some(1.5));
    assert_eq!(v.as_i32(), None);
}

#[test]
fn test_signal_value_i32() {
    let v = MapperSignalValue::I32(42);
    assert_eq!(v.as_i32(), Some(42));
    assert_eq!(v.as_f32(), None);
}

fn main() {}
