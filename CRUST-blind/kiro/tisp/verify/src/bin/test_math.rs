use tisp_proj::tisp::*;
use tisp_proj::core;
use tisp_proj::math;
use tisp_proj::string;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    core::tib_env_core(&mut st);
    math::tib_env_math(&mut st);
    string::tib_env_string(&mut st);
    st
}

fn eval_str(st: &mut Tsp, input: &str) -> String {
    st.file = input.to_string();
    st.filec = 0;
    let v = match tisp_read(st) {
        Some(v) => v,
        None => return "READ_ERROR".to_string(),
    };
    let v = match tisp_eval(st, v) {
        Some(v) => v,
        None => return "EVAL_ERROR".to_string(),
    };
    let mut buf = Vec::new();
    tisp_print(&mut buf, &v);
    String::from_utf8(buf).unwrap_or_default()
}

// Numbers (Dec/Int/numerator/denominator)
#[test] fn test_dec_rat()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Dec 1/2)"), "0.5"); }
#[test] fn test_dec_neg()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Dec 3/-2)"), "-1.5"); }
#[test] fn test_dec_int()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Dec 1)"), "1.0"); }
#[test] fn test_dec_dec()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Dec 3.14)"), "3.14"); }
#[test] fn test_int_rat()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Int 1/2)"), "0"); }
#[test] fn test_int_neg()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Int 3/-2)"), "-1"); }
#[test] fn test_int_int()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Int 1)"), "1"); }
#[test] fn test_int_dec()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(Int 3.14)"), "3"); }
#[test] fn test_num1()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(numerator 3)"), "3"); }
#[test] fn test_num2()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(numerator 9/2)"), "9"); }
#[test] fn test_num3()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(numerator 9/15)"), "3"); }
#[test] fn test_den1()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(denominator 83)"), "1"); }
#[test] fn test_den2()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(denominator 3/2)"), "2"); }
#[test] fn test_den3()     { let mut st = setup(); assert_eq!(eval_str(&mut st, "(denominator 10/15)"), "3"); }

// Round
#[test] fn test_round1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round 7/3)"), "2"); }
#[test] fn test_round2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round -3/4)"), "-1"); }
#[test] fn test_round3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round 6.3)"), "6.0"); }
#[test] fn test_round4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round -8.1)"), "-8.0"); }
#[test] fn test_round5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round 3)"), "3"); }
#[test] fn test_round6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round -81)"), "-81"); }
#[test] fn test_round7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(round 0)"), "0"); }
#[test] fn test_floor1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor 5/3)"), "1"); }
#[test] fn test_floor2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor -9/4)"), "-3"); }
#[test] fn test_floor3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor 6.3)"), "6.0"); }
#[test] fn test_floor4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor -8.1)"), "-9.0"); }
#[test] fn test_floor5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor 3)"), "3"); }
#[test] fn test_floor6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor -81)"), "-81"); }
#[test] fn test_floor7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(floor 0)"), "0"); }
#[test] fn test_ceil1()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(ceil 1/2)"), "1"); }
#[test] fn test_ceil2()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(ceil -8/5)"), "-1"); }
#[test] fn test_ceil5()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(ceil 128)"), "128"); }
#[test] fn test_ceil6()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(ceil -2)"), "-2"); }
#[test] fn test_ceil7()   { let mut st = setup(); assert_eq!(eval_str(&mut st, "(ceil 0)"), "0"); }

// Arithmetic
#[test] fn test_add1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 1 1)"), "2"); }
#[test] fn test_add2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 1 (+ 1 2))"), "4"); }
#[test] fn test_add3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 1029 283)"), "1312"); }
#[test] fn test_add4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 204  8.3)"), "212.3"); }
#[test] fn test_add5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 33   3/4)"), "135/4"); }
#[test] fn test_add6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 1/3 5)"), "16/3"); }
#[test] fn test_add8()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 2/5 3/2)"), "19/10"); }
#[test] fn test_add9()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 2.1 2)"), "4.1"); }
#[test] fn test_add10() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 8.6 5.3)"), "13.9"); }
#[test] fn test_add11() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(+ 3.7 1/8)"), "3.825"); }
#[test] fn test_neg1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 3)"), "-3"); }
#[test] fn test_neg2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- +3)"), "-3"); }
#[test] fn test_neg3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- -289)"), "289"); }
#[test] fn test_neg4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 7/8)"), "-7/8"); }
#[test] fn test_sub1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 5 4)"), "1"); }
#[test] fn test_sub2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 53 88)"), "-35"); }
#[test] fn test_sub3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 204  8.3)"), "195.7"); }
#[test] fn test_sub4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 33   3/4)"), "129/4"); }
#[test] fn test_sub5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 1/3 5)"), "-14/3"); }
#[test] fn test_sub7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 2/5 3/2)"), "-11/10"); }
#[test] fn test_sub8()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 2.1 2)"), "0.1"); }
#[test] fn test_sub9()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 8.6 5.3)"), "3.3"); }
#[test] fn test_sub10() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(- 3.7 1/8)"), "3.575"); }
#[test] fn test_mul1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 3 2)"), "6"); }
#[test] fn test_mul2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* -2 8.89)"), "-17.78"); }
#[test] fn test_mul3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 6 3/4)"), "9/2"); }
#[test] fn test_mul4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 1.004 8)"), "8.032"); }
#[test] fn test_mul5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 1.34e3 .0012)"), "1.608"); }
#[test] fn test_mul7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 1/3 6)"), "2"); }
#[test] fn test_mul8()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 5/2 14.221)"), "35.5525"); }
#[test] fn test_mul9()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(* 6/8 8/7)"), "6/7"); }
#[test] fn test_div1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 1 2)"), "1/2"); }
#[test] fn test_div2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 8 4)"), "2"); }
#[test] fn test_div3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 6 2.1)"), "2.85714285714286"); }
#[test] fn test_div4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 4 4/3)"), "3"); }
#[test] fn test_div5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 5)"), "1/5"); }
#[test] fn test_div6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 4473)"), "1/4473"); }
#[test] fn test_div7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 10.42 5)"), "2.084"); }
#[test] fn test_div10() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 4/3 7)"), "4/21"); }
#[test] fn test_div11() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 5/4 3.2)"), "0.390625"); }
#[test] fn test_div12() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(/ 1/3 5/4)"), "4/15"); }
#[test] fn test_mod1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod 10 3)"), "1"); }
#[test] fn test_mod2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod -11 3)"), "-2"); }
#[test] fn test_mod3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod 10 -3)"), "1"); }
#[test] fn test_mod4()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod -10 -3)"), "-1"); }
#[test] fn test_mod5()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod 10 5)"), "0"); }
#[test] fn test_mod6()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod 7 2)"), "1"); }
#[test] fn test_mod7()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(mod 8 5)"), "3"); }

// Compare
#[test] fn test_lt1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(< 2 3)"), "True"); }
#[test] fn test_lt2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(< 3 3)"), "Nil"); }
#[test] fn test_lt3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(< 4 3)"), "Nil"); }
#[test] fn test_lte1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(<= -2 +4)"), "True"); }
#[test] fn test_lte2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(<= -2 -2)"), "True"); }
#[test] fn test_lte3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(<= 4 -2)"), "Nil"); }
#[test] fn test_gt1()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(> 89 34)"), "True"); }
#[test] fn test_gt2()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(> 48 48)"), "Nil"); }
#[test] fn test_gt3()  { let mut st = setup(); assert_eq!(eval_str(&mut st, "(> 98 183)"), "Nil"); }
#[test] fn test_gte1() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(>= +4 -282)"), "True"); }
#[test] fn test_gte2() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(>= 39 39)"), "True"); }
#[test] fn test_gte3() { let mut st = setup(); assert_eq!(eval_str(&mut st, "(>= -32 -30)"), "Nil"); }

fn main() {}
