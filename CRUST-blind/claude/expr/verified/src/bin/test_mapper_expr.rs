use expr::mapper_expr::{mapper_expr_new_from_string, mapper_expr_evaluate, MapperSignalValue};

fn eval_int(expr_str: &str, input: i32) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 0, 1);
    let r = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(input));
    match r {
        MapperSignalValue::I32(v) => v,
        MapperSignalValue::F(v) => panic!("expected I32, got F({})", v),
    }
}

fn eval_float(expr_str: &str, input: f32) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 1, 1);
    let r = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(input));
    match r {
        MapperSignalValue::F(v) => v,
        MapperSignalValue::I32(v) => panic!("expected F, got I32({})", v),
    }
}

fn eval_int_to_float(expr_str: &str, input: i32) -> f32 {
    let mut e = mapper_expr_new_from_string(expr_str, 0, 1, 1);
    let r = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(input));
    match r {
        MapperSignalValue::F(v) => v,
        MapperSignalValue::I32(v) => panic!("expected F, got I32({})", v),
    }
}

fn eval_float_to_int(expr_str: &str, input: f32) -> i32 {
    let mut e = mapper_expr_new_from_string(expr_str, 1, 0, 1);
    let r = mapper_expr_evaluate(&mut e, &MapperSignalValue::F(input));
    match r {
        MapperSignalValue::I32(v) => v,
        MapperSignalValue::F(v) => panic!("expected I32, got F({})", v),
    }
}

fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

// ----- Integer arithmetic tests -----

#[test]
fn test_identity_int() {
    assert_eq!(eval_int("y=x", 5), 5);
    assert_eq!(eval_int("y=x", 0), 0);
    assert_eq!(eval_int("y=x", -3), -3);
}

#[test]
fn test_int_add() {
    assert_eq!(eval_int("y=x+1", 5), 6);
    assert_eq!(eval_int("y=2+3", 0), 5);
}

#[test]
fn test_int_sub() {
    assert_eq!(eval_int("y=x-1", 5), 4);
    assert_eq!(eval_int("y=2-3", 0), -1);
}

#[test]
fn test_int_mul() {
    assert_eq!(eval_int("y=x*2", 5), 10);
    assert_eq!(eval_int("y=2*3", 0), 6);
}

#[test]
fn test_int_div() {
    assert_eq!(eval_int("y=x/2", 10), 5);
    assert_eq!(eval_int("y=10/3", 0), 3);
}

#[test]
fn test_int_negate() {
    assert_eq!(eval_int("y=-x", 5), -5);
    assert_eq!(eval_int("y=-5+x", 3), -2);
    assert_eq!(eval_int("y=-(2+3)", 0), -5);
}

#[test]
fn test_int_precedence() {
    // 2+3*4 = 14
    assert_eq!(eval_int("y=2+3*4", 0), 14);
    // (2+3)*4 = 20
    assert_eq!(eval_int("y=(2+3)*4", 0), 20);
}

#[test]
fn test_int_test2_inputs() {
    // y=26*2/2+x*30/(20*1)
    assert_eq!(eval_int("y=26*2/2+x*30/(20*1)", 3), 30);
    assert_eq!(eval_int("y=26*2/2+x*30/(20*1)", 321), 507);
}

// ----- Float arithmetic tests -----

#[test]
fn test_float_basic() {
    assert!(approx_eq(eval_float("y=x*2.0", 3.0), 6.0, 1e-6));
    assert!(approx_eq(eval_float("y=2.5+x", 1.5), 4.0, 1e-6));
}

// ----- Math functions -----

#[test]
fn test_func_sin() {
    assert!(approx_eq(eval_float("y=sin(0)", 0.0), 0.0, 1e-6));
}

#[test]
fn test_func_cos() {
    assert!(approx_eq(eval_float("y=cos(0)", 0.0), 1.0, 1e-6));
}

#[test]
fn test_func_tan() {
    assert!(approx_eq(eval_float("y=tan(0)", 0.0), 0.0, 1e-6));
}

#[test]
fn test_func_sqrt() {
    assert!(approx_eq(eval_float("y=sqrt(x)", 16.0), 4.0, 1e-6));
}

#[test]
fn test_func_log() {
    let v = eval_float("y=log(2.71828)", 0.0);
    assert!(approx_eq(v, 0.999999344f32, 1e-5));
}

#[test]
fn test_func_log10() {
    assert!(approx_eq(eval_float("y=log10(100)", 0.0), 2.0, 1e-6));
}

#[test]
fn test_func_exp() {
    let v = eval_float("y=exp(1)", 0.0);
    assert!(approx_eq(v, 2.71828175f32, 1e-5));
}

#[test]
fn test_func_pow() {
    assert!(approx_eq(eval_float("y=pow(2,10)", 0.0), 1024.0, 1e-3));
}

#[test]
fn test_func_floor() {
    assert!(approx_eq(eval_float("y=floor(2.7)", 0.0), 2.0, 1e-6));
}

#[test]
fn test_func_ceil() {
    assert!(approx_eq(eval_float("y=ceil(2.3)", 0.0), 3.0, 1e-6));
}

#[test]
fn test_func_round() {
    assert!(approx_eq(eval_float("y=round(2.5)", 0.0), 3.0, 1e-6));
}

#[test]
fn test_func_abs() {
    assert!(approx_eq(eval_float("y=abs(-5.5)", 0.0), 5.5, 1e-6));
}

#[test]
fn test_func_asin() {
    assert!(approx_eq(eval_float("y=asin(0.5)", 0.0), 0.52359879f32, 1e-5));
}

#[test]
fn test_func_acos() {
    assert!(approx_eq(eval_float("y=acos(0.5)", 0.0), 1.04719758f32, 1e-5));
}

#[test]
fn test_func_atan() {
    assert!(approx_eq(eval_float("y=atan(1)", 0.0), 0.785398185f32, 1e-5));
}

#[test]
fn test_func_atan2() {
    assert!(approx_eq(eval_float("y=atan2(1,1)", 0.0), 0.785398185f32, 1e-5));
}

#[test]
fn test_func_sinh() {
    assert!(approx_eq(eval_float("y=sinh(1)", 0.0), 1.17520118f32, 1e-5));
}

#[test]
fn test_func_cosh() {
    assert!(approx_eq(eval_float("y=cosh(1)", 0.0), 1.54308057f32, 1e-5));
}

#[test]
fn test_func_tanh() {
    assert!(approx_eq(eval_float("y=tanh(1)", 0.0), 0.761594176f32, 1e-5));
}

#[test]
fn test_func_exp2() {
    assert!(approx_eq(eval_float("y=exp2(3)", 0.0), 8.0, 1e-6));
}

#[test]
fn test_func_log2() {
    assert!(approx_eq(eval_float("y=log2(8)", 0.0), 3.0, 1e-6));
}

#[test]
fn test_func_hypot() {
    assert!(approx_eq(eval_float("y=hypot(3,4)", 0.0), 5.0, 1e-6));
}

#[test]
fn test_func_cbrt() {
    assert!(approx_eq(eval_float("y=cbrt(27)", 0.0), 3.0, 1e-6));
}

#[test]
fn test_func_trunc() {
    assert!(approx_eq(eval_float("y=trunc(2.7)", 0.0), 2.0, 1e-6));
}

#[test]
fn test_func_min() {
    assert!(approx_eq(eval_float("y=min(3,4)", 0.0), 3.0, 1e-6));
}

#[test]
fn test_func_max() {
    assert!(approx_eq(eval_float("y=max(3,4)", 0.0), 4.0, 1e-6));
}

#[test]
fn test_func_pi() {
    let v = eval_float("y=pi()", 0.0);
    assert!(approx_eq(v, 3.14159274f32, 1e-5));
}

#[test]
fn test_func_pi_times_two() {
    // The C output indicates pi() with constant folding may collapse incorrectly
    // resulting in 3.14159274 (apparently NOT *2). Match C exactly.
    let v = eval_float("y=pi()*2", 0.0);
    assert!(approx_eq(v, 3.14159274f32, 1e-5));
}

#[test]
fn test_func_logb() {
    assert!(approx_eq(eval_float("y=logb(8.0)", 0.0), 3.0, 1e-6));
    assert!(approx_eq(eval_float("y=logb(0.25)", 0.0), -2.0, 1e-6));
}

// ----- Coercion -----

#[test]
fn test_coerce_int_to_float_simple() {
    assert!(approx_eq(eval_int_to_float("y=x", 5), 5.0, 1e-6));
}

#[test]
fn test_coerce_int_to_float_arith() {
    assert!(approx_eq(eval_int_to_float("y=x+1", 5), 6.0, 1e-6));
}

#[test]
fn test_coerce_float_to_int_simple() {
    assert_eq!(eval_float_to_int("y=x", 5.7f32), 5);
}

#[test]
fn test_coerce_float_to_int_mul() {
    assert_eq!(eval_float_to_int("y=x*2.0", 5.5f32), 11);
}

// ----- Negation with float -----

#[test]
fn test_neg_float_var() {
    assert!(approx_eq(eval_float("y=-x", 3.0), -3.0, 1e-6));
}

#[test]
fn test_neg_float_paren() {
    assert!(approx_eq(eval_float("y=-(x+1)", 3.0), -4.0, 1e-6));
}

// ----- Big test from C test.c (test1) -----

#[test]
fn test_big_expression() {
    // The C version output: 3250.81079
    let v = eval_float(
        "y=26*2/2+log10(pi)+2.*pow(2,1*(3+7*.1)*1.1+x{-6*2+12})*3*4+cos(2.)",
        3.0,
    );
    assert!(approx_eq(v, 3250.81079f32, 0.5),
        "expected ~3250.810791, got {}", v);
}

// ----- MapperSignalValue tests -----

#[test]
fn test_mapper_signal_value_as_f32() {
    assert_eq!(MapperSignalValue::F(3.5).as_f32(), Some(3.5));
    assert_eq!(MapperSignalValue::I32(7).as_f32(), Some(7.0));
}

#[test]
fn test_mapper_signal_value_as_i32() {
    assert_eq!(MapperSignalValue::F(3.7).as_i32(), Some(3));
    assert_eq!(MapperSignalValue::I32(7).as_i32(), Some(7));
}

// ----- Multi-call evaluation (history wiring) -----

#[test]
fn test_int_multi_call() {
    // Build once, evaluate multiple times.
    let mut e = mapper_expr_new_from_string("y=26*2/2+x*30/(20*1)", 0, 0, 1);
    let r1 = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(3));
    match r1 {
        MapperSignalValue::I32(v) => assert_eq!(v, 30),
        _ => panic!(),
    }
    let r2 = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(321));
    match r2 {
        MapperSignalValue::I32(v) => assert_eq!(v, 507),
        _ => panic!(),
    }
}

// ----- Verify MapperExpr fields -----

#[test]
fn test_mapper_expr_struct_fields_simple() {
    let e = mapper_expr_new_from_string("y=x", 0, 0, 1);
    assert_eq!(e.vector_size, 1);
    // Without history reference, history_size should be 1 (oldest_samps=0 -> ceil(0)+1=1)
    assert_eq!(e.history_size, 1);
    assert_eq!(e.history_pos, -1);
    assert_eq!(e.input_history.len(), 1);
    assert_eq!(e.output_history.len(), 1);
}

#[test]
fn test_mapper_expr_history_reference_size() {
    // x{-6*2+12} = x{0}; oldest_samps may be 0 since constant folded to 0.
    // y=x{-3} -> oldest_samps=-3 -> history_size = 4
    let e = mapper_expr_new_from_string("y=x{-3}", 0, 0, 1);
    assert_eq!(e.history_size, 4);
}

#[test]
fn test_history_pos_after_eval() {
    let mut e = mapper_expr_new_from_string("y=x", 0, 0, 1);
    let _ = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(5));
    assert_eq!(e.history_pos, 0);
    let _ = mapper_expr_evaluate(&mut e, &MapperSignalValue::I32(7));
    // history_size is 1, so wraps to 0
    assert_eq!(e.history_pos, 0);
}

fn main() {}
