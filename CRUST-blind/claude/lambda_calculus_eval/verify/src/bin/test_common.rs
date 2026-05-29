use lambda_calculus_eval::common::{
    self, append_ast_to_buffer, append_to_buffer, ast_to_string, format, AstNode, AstNodeType,
    AstNodeUnion, Application, LambdaExpression, Variable,
};

fn make_var(name: &str, ty: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: ty.to_string(),
        }),
    }
}

fn make_lambda(param: &str, body: AstNode, ty: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: param.to_string(),
            type_: ty.to_string(),
            body: Some(Box::new(body)),
        }),
    }
}

fn make_app(f: AstNode, a: AstNode) -> AstNode {
    AstNode {
        type_: AstNodeType::APPLICATION,
        node: AstNodeUnion::Application(Application {
            function: Some(Box::new(f)),
            argument: Some(Box::new(a)),
        }),
    }
}

#[test]
fn test_format_basic() {
    // C: format("Hello, %s! You are %d years old.", "John", 30) ==
    //   "Hello, John! You are 30 years old."
    let result = format(
        "Hello, John! You are 30 years old.",
        std::format_args!("{}", "Hello, John! You are 30 years old."),
    );
    assert_eq!(result, "Hello, John! You are 30 years old.");
}

#[test]
fn test_append_to_buffer_simple() {
    let mut buf = String::new();
    append_to_buffer(&mut buf, "hello");
    assert_eq!(buf, "hello");
    append_to_buffer(&mut buf, " world");
    assert_eq!(buf, "hello world");
}

#[test]
fn test_ast_to_string_var_with_type() {
    // C: ast_to_string for variable with name="x", type="Nat" -> "(x : Nat) "
    let var = make_var("x", "Nat");
    let s = ast_to_string(&var);
    assert_eq!(s, "(x : Nat) ");
}

#[test]
fn test_ast_to_string_var_null_type() {
    // C: ast_to_string for variable with name="x", type=NULL -> "(x) "
    let var = make_var("x", "");
    let s = ast_to_string(&var);
    assert_eq!(s, "(x) ");
}

#[test]
fn test_ast_to_string_lambda() {
    // C: lambda("x", body=var("x", NULL), type="Nat") ->
    //   "(@x : Nat .(x) ) "
    let body = make_var("x", "");
    let lambda = make_lambda("x", body, "Nat");
    let s = ast_to_string(&lambda);
    assert_eq!(s, "(@x : Nat .(x) ) ");
}

#[test]
fn test_ast_to_string_application() {
    // C: app(lambda("x", var("x"), "Nat"), var("y", "Nat")) ->
    //   "((@x : Nat .(x) ) (y : Nat) ) "
    let body = make_var("x", "");
    let lambda = make_lambda("x", body, "Nat");
    let yvar = make_var("y", "Nat");
    let app = make_app(lambda, yvar);
    let s = ast_to_string(&app);
    assert_eq!(s, "((@x : Nat .(x) ) (y : Nat) ) ");
}

#[test]
fn test_ast_to_string_definition() {
    // C: var with type=DEFINITION outputs (name)
    let mut def = make_var("foo", "");
    def.type_ = AstNodeType::DEFINITION;
    let s = ast_to_string(&def);
    assert_eq!(s, "(foo) ");
}

#[test]
fn test_ast_to_string_complex_lambda() {
    // C: lambda("x", app(var("x"), var("y", "Nat")), "Nat")
    //  -> "(@x : Nat .((x) (y : Nat) ) ) "
    let inner_body = make_app(make_var("x", ""), make_var("y", "Nat"));
    let lam = make_lambda("x", inner_body, "Nat");
    let s = ast_to_string(&lam);
    assert_eq!(s, "(@x : Nat .((x) (y : Nat) ) ) ");
}

#[test]
fn test_append_ast_to_buffer_var() {
    let var = make_var("a", "Bool");
    let mut buf = String::new();
    append_ast_to_buffer(&mut buf, &var);
    assert_eq!(buf, "(a : Bool) ");
}

#[test]
fn test_set_verbose_does_not_panic() {
    // No assertion needed: just ensure setting verbose doesn't panic.
    common::set_verbose(true);
    common::set_verbose(false);
}

#[test]
fn test_token_enum_variants() {
    // Sanity check: enum compares equally to itself.
    assert!(common::tokens_t::L_PAREN == common::tokens_t::L_PAREN);
    assert!(common::tokens_t::L_PAREN != common::tokens_t::R_PAREN);
}

fn main() {}
