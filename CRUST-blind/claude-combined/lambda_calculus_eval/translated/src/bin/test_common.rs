use lambda_calculus_eval::common::{
    self, append_ast_to_buffer, append_to_buffer, ast_to_string, set_verbose, AstNode, AstNodeType,
    AstNodeUnion, Application, LambdaExpression, Variable,
};

fn make_var(name: &str, type_: &str) -> AstNode {
    AstNode {
        type_: AstNodeType::VAR,
        node: AstNodeUnion::Variable(Variable {
            name: name.to_string(),
            type_: type_.to_string(),
        }),
    }
}

fn make_lambda(parameter: &str, type_: &str, body: AstNode) -> AstNode {
    AstNode {
        type_: AstNodeType::LAMBDA_EXPR,
        node: AstNodeUnion::LambdaExpr(LambdaExpression {
            parameter: parameter.to_string(),
            type_: type_.to_string(),
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
fn test_default_node() {
    let n = AstNode::default();
    assert_eq!(n.type_, AstNodeType::VAR);
    if let AstNodeUnion::Variable(ref v) = n.node {
        assert_eq!(v.name, "");
        assert_eq!(v.type_, "");
    } else {
        panic!("Default should be VAR");
    }
}

#[test]
fn test_set_verbose_does_not_panic() {
    set_verbose(true);
    set_verbose(false);
}

#[test]
fn test_append_to_buffer() {
    let mut buf = String::new();
    append_to_buffer(&mut buf, "hello");
    append_to_buffer(&mut buf, " world");
    assert_eq!(buf, "hello world");
}

#[test]
fn test_ast_to_string_var_with_type() {
    // C: ast_to_string(create_variable("x", "Nat")) -> "(x : Nat) "
    let v = make_var("x", "Nat");
    let s = ast_to_string(&v);
    assert_eq!(s, "(x : Nat) ");
}

#[test]
fn test_ast_to_string_var_without_type() {
    // C: ast_to_string(create_variable("y", NULL)) -> "(y) "
    let v = make_var("y", "");
    let s = ast_to_string(&v);
    assert_eq!(s, "(y) ");
}

#[test]
fn test_ast_to_string_lambda() {
    // C: ast_to_string(create_lambda("z", create_variable("y", NULL), "Bool")) -> "(@z : Bool .(y) ) "
    let body = make_var("y", "");
    let lam = make_lambda("z", "Bool", body);
    let s = ast_to_string(&lam);
    assert_eq!(s, "(@z : Bool .(y) ) ");
}

#[test]
fn test_ast_to_string_application() {
    // C: ast_to_string(create_application(create_variable("x","Nat"), create_variable("y", NULL))) -> "((x : Nat) (y) ) "
    let f = make_var("x", "Nat");
    let a = make_var("y", "");
    let app = make_app(f, a);
    let s = ast_to_string(&app);
    assert_eq!(s, "((x : Nat) (y) ) ");
}

#[test]
fn test_ast_to_string_definition() {
    // DEFINITION node prints just (name) (no type)
    let mut node = make_var("def_name", "");
    node.type_ = AstNodeType::DEFINITION;
    let s = ast_to_string(&node);
    assert_eq!(s, "(def_name) ");
}

#[test]
fn test_append_ast_to_buffer_appends() {
    let mut buf = String::from("PREFIX:");
    let v = make_var("foo", "Nat");
    append_ast_to_buffer(&mut buf, &v);
    assert_eq!(buf, "PREFIX:(foo : Nat) ");
}

#[test]
fn test_format_function_arguments() {
    let s = common::format("ignored", format_args!("hello {}", "world"));
    assert_eq!(s, "hello world");
}

fn main() {}
