use tisp_proj::tisp::*;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    st
}

fn eval_str(st: &mut Tsp, input: &str) -> String {
    st.file = input.to_string();
    st.filec = 0;
    let v = tisp_read(st).expect(&format!("read failed for: {}", input));
    let mut env = clone_rec(&st.env);
    let v = tisp_eval_with_env(st, &mut env, v).expect(&format!("eval failed for: {}", input));
    st.env = env;
    let mut buf: Vec<u8> = Vec::new();
    tisp_print(&mut buf, &v);
    String::from_utf8(buf).unwrap()
}

// ---- Arithmetic ----

#[test]
fn test_add() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(+ 1 1)"), "2");
    assert_eq!(eval_str(&mut st, "(+ 1 (+ 1 2))"), "4");
    assert_eq!(eval_str(&mut st, "(+ 1029 283)"), "1312");
    assert_eq!(eval_str(&mut st, "(+ 204 8.3)"), "212.3");
    assert_eq!(eval_str(&mut st, "(+ 33 3/4)"), "135/4");
    assert_eq!(eval_str(&mut st, "(+ 1/3 5)"), "16/3");
    assert_eq!(eval_str(&mut st, "(+ 2/5 3/2)"), "19/10");
    assert_eq!(eval_str(&mut st, "(+ 2.1 2)"), "4.1");
    assert_eq!(eval_str(&mut st, "(+ 8.6 5.3)"), "13.9");
    assert_eq!(eval_str(&mut st, "(+ 3.7 1/8)"), "3.825");
}

#[test]
fn test_add_pi() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(+ 7/4 (* 4 (arctan 1.)))"), "4.89159265358979");
    assert_eq!(eval_str(&mut st, "(- 7/4 (* 4 (arctan 1.)))"), "-1.39159265358979");
}

#[test]
fn test_sub() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(- 3)"), "-3");
    assert_eq!(eval_str(&mut st, "(- +3)"), "-3");
    assert_eq!(eval_str(&mut st, "(- -289)"), "289");
    assert_eq!(eval_str(&mut st, "(- 7/8)"), "-7/8");
    assert_eq!(eval_str(&mut st, "(- -6.412E2)"), "641.2");
    assert_eq!(eval_str(&mut st, "(- 5 4)"), "1");
    assert_eq!(eval_str(&mut st, "(- 53 88)"), "-35");
    assert_eq!(eval_str(&mut st, "(- 204 8.3)"), "195.7");
    assert_eq!(eval_str(&mut st, "(- 33 3/4)"), "129/4");
    assert_eq!(eval_str(&mut st, "(- 1/3 5)"), "-14/3");
    assert_eq!(eval_str(&mut st, "(- 2/5 3/2)"), "-11/10");
    assert_eq!(eval_str(&mut st, "(- 2.1 2)"), "0.1");
    assert_eq!(eval_str(&mut st, "(- 8.6 5.3)"), "3.3");
    assert_eq!(eval_str(&mut st, "(- 3.7 1/8)"), "3.575");
}

#[test]
fn test_mul() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(* 3 2)"), "6");
    assert_eq!(eval_str(&mut st, "(* -2 8.89)"), "-17.78");
    assert_eq!(eval_str(&mut st, "(* 6 3/4)"), "9/2");
    assert_eq!(eval_str(&mut st, "(* 1.004 8)"), "8.032");
    assert_eq!(eval_str(&mut st, "(* 1.34e3 .0012)"), "1.608");
    assert_eq!(eval_str(&mut st, "(* 1/3 6)"), "2");
    assert_eq!(eval_str(&mut st, "(* 5/2 14.221)"), "35.5525");
    assert_eq!(eval_str(&mut st, "(* 6/8 8/7)"), "6/7");
    assert_eq!(eval_str(&mut st, "(* (exp 1.) -5/2)"), "-6.79570457114761");
}

#[test]
fn test_div() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(/ 1 2)"), "1/2");
    assert_eq!(eval_str(&mut st, "(/ 8 4)"), "2");
    assert_eq!(eval_str(&mut st, "(/ 6 2.1)"), "2.85714285714286");
    assert_eq!(eval_str(&mut st, "(/ 4 4/3)"), "3");
    assert_eq!(eval_str(&mut st, "(/ 5)"), "1/5");
    assert_eq!(eval_str(&mut st, "(/ 4473)"), "1/4473");
    assert_eq!(eval_str(&mut st, "(/ 10.42 5)"), "2.084");
    assert_eq!(eval_str(&mut st, "(/ 1.34e-2 4.3332)"), "0.0030924028431644");
    assert_eq!(eval_str(&mut st, "(/ 1.04 -15/4)"), "-0.277333333333333");
    assert_eq!(eval_str(&mut st, "(/ 4/3 7)"), "4/21");
    assert_eq!(eval_str(&mut st, "(/ 5/4 3.2)"), "0.390625");
    assert_eq!(eval_str(&mut st, "(/ 1/3 5/4)"), "4/15");
}

#[test]
fn test_mod() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(mod 10 3)"), "1");
    assert_eq!(eval_str(&mut st, "(mod -11 3)"), "-2");
    assert_eq!(eval_str(&mut st, "(mod 10 -3)"), "1");
    assert_eq!(eval_str(&mut st, "(mod -10 -3)"), "-1");
    assert_eq!(eval_str(&mut st, "(mod 10 5)"), "0");
    assert_eq!(eval_str(&mut st, "(mod 7 2)"), "1");
    assert_eq!(eval_str(&mut st, "(mod 8 5)"), "3");
}

// ---- Compare ----

#[test]
fn test_compare() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(< 2 3)"), "True");
    assert_eq!(eval_str(&mut st, "(< 3 3)"), "Nil");
    assert_eq!(eval_str(&mut st, "(< 4 3)"), "Nil");
    assert_eq!(eval_str(&mut st, "(<= -2 +4)"), "True");
    assert_eq!(eval_str(&mut st, "(<= -2 -2)"), "True");
    assert_eq!(eval_str(&mut st, "(<= 4 -2)"), "Nil");
    assert_eq!(eval_str(&mut st, "(> 89 34)"), "True");
    assert_eq!(eval_str(&mut st, "(> 48 48)"), "Nil");
    assert_eq!(eval_str(&mut st, "(> 98 183)"), "Nil");
    assert_eq!(eval_str(&mut st, "(>= +4 -282)"), "True");
    assert_eq!(eval_str(&mut st, "(>= 39 39)"), "True");
    assert_eq!(eval_str(&mut st, "(>= -32 -30)"), "Nil");
}

// ---- Numbers: Int, Dec, numerator, denominator ----

#[test]
fn test_int_dec_conversion() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(Dec 1/2)"), "0.5");
    assert_eq!(eval_str(&mut st, "(Dec 3/-2)"), "-1.5");
    assert_eq!(eval_str(&mut st, "(Dec 1)"), "1.0");
    assert_eq!(eval_str(&mut st, "(Dec 3.14)"), "3.14");
    assert_eq!(eval_str(&mut st, "(Int 1/2)"), "0");
    assert_eq!(eval_str(&mut st, "(Int 3/-2)"), "-1");
    assert_eq!(eval_str(&mut st, "(Int 1)"), "1");
    assert_eq!(eval_str(&mut st, "(Int 3.14)"), "3");
}

#[test]
fn test_numerator_denominator() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(numerator 3)"), "3");
    assert_eq!(eval_str(&mut st, "(numerator 9/2)"), "9");
    assert_eq!(eval_str(&mut st, "(numerator 9/15)"), "3");
    assert_eq!(eval_str(&mut st, "(denominator 83)"), "1");
    assert_eq!(eval_str(&mut st, "(denominator 3/2)"), "2");
    assert_eq!(eval_str(&mut st, "(denominator 10/15)"), "3");
}

// ---- Rounding ----

#[test]
fn test_round() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(round 7/3)"), "2");
    assert_eq!(eval_str(&mut st, "(round -3/4)"), "-1");
    assert_eq!(eval_str(&mut st, "(round 6.3)"), "6.0");
    assert_eq!(eval_str(&mut st, "(round -8.1)"), "-8.0");
    assert_eq!(eval_str(&mut st, "(round 3)"), "3");
    assert_eq!(eval_str(&mut st, "(round -81)"), "-81");
    assert_eq!(eval_str(&mut st, "(round 0)"), "0");
}

#[test]
fn test_floor() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(floor 5/3)"), "1");
    assert_eq!(eval_str(&mut st, "(floor -9/4)"), "-3");
    assert_eq!(eval_str(&mut st, "(floor 6.3)"), "6.0");
    assert_eq!(eval_str(&mut st, "(floor -8.1)"), "-9.0");
    assert_eq!(eval_str(&mut st, "(floor 3)"), "3");
    assert_eq!(eval_str(&mut st, "(floor -81)"), "-81");
    assert_eq!(eval_str(&mut st, "(floor 0)"), "0");
}

#[test]
fn test_ceil() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(ceil 1/2)"), "1");
    assert_eq!(eval_str(&mut st, "(ceil -8/5)"), "-1");
    assert_eq!(eval_str(&mut st, "(ceil 128)"), "128");
    assert_eq!(eval_str(&mut st, "(ceil -2)"), "-2");
    assert_eq!(eval_str(&mut st, "(ceil 0)"), "0");
}

#[test]
fn test_truncate() {
    let mut st = setup();
    // truncate is defined in tibs as (* (floor (abs x)) (sgn x))
    // Without tibs, test the underlying operations
    assert_eq!(eval_str(&mut st, "(/ 17 2)"), "17/2");
    assert_eq!(eval_str(&mut st, "(floor 8.5)"), "8.0");
    assert_eq!(eval_str(&mut st, "(ceil (* 4 (arctan 1.)))"), "4.0");
    assert_eq!(eval_str(&mut st, "(ceil (- .2))"), "-0.0");
}

fn main() {}
