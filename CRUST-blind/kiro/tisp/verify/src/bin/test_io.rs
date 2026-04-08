use tisp_proj::tisp::*;
use tisp_proj::io::count_parens;

fn setup() -> Tsp {
    let mut st = tisp_env_init(1024);
    tib_env_core(&mut st);
    tib_env_math(&mut st);
    tib_env_string(&mut st);
    tib_env_io(&mut st);
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

#[test]
fn test_count_parens_balanced() {
    assert_eq!(count_parens("(+ 1 2)", 7), 0);
    assert_eq!(count_parens("[1 2 3]", 7), 0);
    assert_eq!(count_parens("{a: 1}", 6), 0);
}

#[test]
fn test_count_parens_unbalanced() {
    assert_eq!(count_parens("(+ 1", 4), 1);
    assert_eq!(count_parens("((", 2), 2);
    assert_eq!(count_parens(")", 1), -1);
}

#[test]
fn test_parse() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(eval (parse \"(+ 1 2)\"))"), "3");
    assert_eq!(eval_str(&mut st, "(eval (parse \"42\"))"), "42");
}

#[test]
fn test_parse_nil_returns_quit() {
    let mut st = setup();
    assert_eq!(eval_str(&mut st, "(parse Nil)"), "quit");
}

#[test]
fn test_write_to_file_and_read() {
    let mut st = setup();
    let tmpfile = "/tmp/tisp_test_io.txt";
    let write_expr = format!("(write \"{}\" Nil \"hello\")", tmpfile);
    assert_eq!(eval_str(&mut st, &write_expr), "Void");
    let read_expr = format!("(read \"{}\")", tmpfile);
    assert_eq!(eval_str(&mut st, &read_expr), "hello");
    let _ = std::fs::remove_file(tmpfile);
}

fn main() {}
