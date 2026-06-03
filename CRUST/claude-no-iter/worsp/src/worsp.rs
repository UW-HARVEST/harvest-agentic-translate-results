pub const OBJECT_NUMBER: usize = 50;
pub const BLOCK_SIZE: usize = std::mem::size_of::<Object>();
pub const MEMORY_SIZE: usize = BLOCK_SIZE * OBJECT_NUMBER;
pub const FREE_BITMAP_SIZE: usize = MEMORY_SIZE / BLOCK_SIZE;
pub const MAX_BINDINGS: usize = 10;
pub const MAX_SYMBOL_NAME_LENGTH: usize = 20;
#[derive(Debug, Clone, Copy)]
pub enum ConsCellType {
Cell,
Nil,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Digit,
    LParen,
    RParen,
    Symbol,
    String,
    True,
    False,
    Eof,
    Quote,
}
#[derive(Debug)]
pub struct Token {
pub kind: TokenKind,
pub next: Option<Box<Token>>,
pub val: i32,
pub str: String,
}
#[derive(Debug)]
pub struct ParseState {
pub token: Option<Box<Token>>,
pub pos: i32,
}
#[derive(Debug)]
pub struct ProgramNode {
pub expressions: Option<Box<ExpressionList>>,
}
#[derive(Debug, Clone, Copy)]
pub enum ExpressionType {
Literal,
Symbol,
List,
SymbolicExp,
}
#[derive(Debug)]
pub struct ExpressionList {
pub expression: Option<Box<ExpressionNode>>,
pub next: Option<Box<ExpressionList>>,
}
#[derive(Debug)]
pub struct ExpressionNode {
pub type_: ExpressionType,
pub data: ExpressionData,
}
#[derive(Debug)]
pub enum ExpressionData {
SymbolicExp(Option<Box<SymbolicExpNode>>),
List(Option<Box<ListNode>>),
Literal(Option<Box<LiteralNode>>),
Symbol(Option<Box<SymbolNode>>),
}
#[derive(Debug)]
pub struct SymbolicExpNode {
pub expressions: Option<Box<ExpressionList>>,
}
#[derive(Debug)]
pub struct ListNode {
pub expressions: Option<Box<ExpressionList>>,
}
#[derive(Debug, Clone, Copy)]
pub enum LiteralType {
Integer,
String,
Boolean,
}
#[derive(Debug)]
pub struct LiteralNode {
pub type_: LiteralType,
pub value: LiteralValue,
}
#[derive(Debug)]
pub enum LiteralValue {
IntValue(i32),
BooleanValue(bool),
StringValue(String),
}
#[derive(Debug)]
pub struct SymbolNode {
pub symbol_name: String,
}
#[derive(Debug)]
pub struct ParseResult {
pub program: Option<Box<ProgramNode>>,
}
#[derive(Debug, Clone, Copy)]
pub enum ObjectType {
Integer,
String,
Bool,
List,
Nil,
Function,
}
#[derive(Debug)]
pub struct ConsCell {
pub type_: ConsCellType,
pub car: Option<Box<Object>>,
pub cdr: Option<Box<Object>>,
}
#[derive(Debug)]
pub struct Function {
pub param_symbol_names: Vec<String>,
pub body: Option<Box<ExpressionNode>>,
}
#[derive(Debug)]
pub struct Object {
pub marked: bool,
pub type_: ObjectType,
pub value: ObjectValue,
}
#[derive(Debug)]
pub enum ObjectValue {
IntValue(i32),
StringValue(String),
BoolValue(i32),
ListValue(Option<Box<ConsCell>>),
FunctionValue(Option<Box<Function>>),
}
#[derive(Debug)]
pub struct Binding {
pub symbol_name: String,
pub value: Option<Box<Object>>,
}
#[derive(Debug)]
pub struct Env {
pub bindings: [Binding; MAX_BINDINGS],
pub parent: Option<Box<Env>>,
}
#[derive(Debug)]
pub struct ObjectStack {
pub objects: [Option<Box<Object>>; OBJECT_NUMBER],
pub top: i32,
}
#[derive(Debug)]
pub struct AllocatorContext {
pub gc_less_mode: i32,
pub stack: Option<Box<ObjectStack>>,
pub memory_pool: Option<Box<Object>>,
pub free_bitmap: [u8; FREE_BITMAP_SIZE],
}
pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(ref tok) = state.token {
        if tok.kind == kind { 1 } else { 0 }
    } else {
        0
    }
}

fn matches_token(state: &ParseState, kind: TokenKind) -> bool {
    if let Some(ref tok) = state.token {
        tok.kind == kind
    } else {
        false
    }
}

fn is_op_ch(c: char) -> bool {
    matches!(
        c,
        '+' | '-' | '*' | '/' | '%' | '|' | '&' | '=' | '<' | '>'
    )
}
pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    // skip whitespace
    while state.pos < bytes.len() as i32 {
        let c = bytes[state.pos as usize] as char;
        if c.is_whitespace() || c == '\n' {
            state.pos += 1;
        } else {
            break;
        }
    }

    let new_token: Token;
    if state.pos >= bytes.len() as i32 {
        new_token = Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: String::new(),
        };
    } else {
        let c = bytes[state.pos as usize] as char;
        if c == '(' {
            state.pos += 1;
            new_token = Token {
                kind: TokenKind::LParen,
                next: None,
                val: 0,
                str: "(".to_string(),
            };
        } else if c == ')' {
            state.pos += 1;
            new_token = Token {
                kind: TokenKind::RParen,
                next: None,
                val: 0,
                str: ")".to_string(),
            };
        } else if c == '\'' {
            state.pos += 1;
            new_token = Token {
                kind: TokenKind::Quote,
                next: None,
                val: 0,
                str: "'".to_string(),
            };
        } else if c.is_ascii_alphabetic() || is_op_ch(c) {
            let start = state.pos as usize;
            while state.pos < bytes.len() as i32 {
                let cc = bytes[state.pos as usize] as char;
                if cc.is_ascii_alphanumeric() || is_op_ch(cc) {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let s: String = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap()
                .to_string();
            if s == "true" {
                new_token = Token {
                    kind: TokenKind::True,
                    next: None,
                    val: 0,
                    str: String::new(),
                };
            } else if s == "false" {
                new_token = Token {
                    kind: TokenKind::False,
                    next: None,
                    val: 0,
                    str: String::new(),
                };
            } else {
                new_token = Token {
                    kind: TokenKind::Symbol,
                    next: None,
                    val: 0,
                    str: s,
                };
            }
        } else if c.is_ascii_digit() {
            let start = state.pos as usize;
            while state.pos < bytes.len() as i32 {
                let cc = bytes[state.pos as usize] as char;
                if cc.is_ascii_digit() {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize]).unwrap();
            let val: i32 = s.parse().unwrap_or(0);
            new_token = Token {
                kind: TokenKind::Digit,
                next: None,
                val,
                str: String::new(),
            };
        } else if c == '"' {
            state.pos += 1;
            let start = state.pos as usize;
            while state.pos < bytes.len() as i32 {
                let cc = bytes[state.pos as usize] as char;
                if cc == '"' {
                    break;
                }
                state.pos += 1;
            }
            let s: String = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap()
                .to_string();
            if state.pos < bytes.len() as i32 && bytes[state.pos as usize] as char == '"' {
                state.pos += 1;
            }
            new_token = Token {
                kind: TokenKind::String,
                next: None,
                val: 0,
                str: s,
            };
        } else if c == ';' {
            // comment, skip to newline
            while state.pos < bytes.len() as i32 {
                let cc = bytes[state.pos as usize] as char;
                if cc == '\n' {
                    break;
                }
                state.pos += 1;
            }
            next(source, state);
            return;
        } else {
            panic!("Unexpected token: {}", c);
        }
    }

    state.token = Some(Box::new(new_token));
}
fn parse_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    if matches_token(state, TokenKind::LParen) {
        parse_symbolic_expression(source, state)
    } else if matches_token(state, TokenKind::Quote) {
        parse_list_expression(source, state)
    } else if matches_token(state, TokenKind::Symbol) {
        parse_symbol_expression(source, state)
    } else if matches_token(state, TokenKind::Digit)
        || matches_token(state, TokenKind::String)
        || matches_token(state, TokenKind::True)
        || matches_token(state, TokenKind::False)
    {
        parse_literal_expression(source, state)
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        panic!("Unexpected token: {}", s);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut sym = SymbolicExpNode { expressions: None };
    next(source, state); // eat '('
    while !matches_token(state, TokenKind::RParen) {
        let item = parse_expression(source, state);
        append_expression_list(&mut sym.expressions, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(sym))),
    })
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut list = ListNode { expressions: None };
    next(source, state); // eat quote
    next(source, state); // eat '('
    while !matches_token(state, TokenKind::RParen) {
        let item = parse_expression(source, state);
        append_expression_list(&mut list.expressions, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(list))),
    })
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let name = state
        .token
        .as_ref()
        .map(|t| t.str.clone())
        .unwrap_or_default();
    next(source, state);
    Box::new(ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode {
            symbol_name: name,
        }))),
    })
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let lit: LiteralNode = if matches_token(state, TokenKind::Digit) {
        let v = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        LiteralNode {
            type_: LiteralType::Integer,
            value: LiteralValue::IntValue(v),
        }
    } else if matches_token(state, TokenKind::String) {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        LiteralNode {
            type_: LiteralType::String,
            value: LiteralValue::StringValue(s),
        }
    } else if matches_token(state, TokenKind::True) {
        LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(true),
        }
    } else if matches_token(state, TokenKind::False) {
        LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(false),
        }
    } else {
        panic!("Unexpected literal token");
    };
    next(source, state);
    Box::new(ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(lit))),
    })
}

fn append_expression_list(
    head: &mut Option<Box<ExpressionList>>,
    expr: Box<ExpressionNode>,
) {
    let new_node = Box::new(ExpressionList {
        expression: Some(expr),
        next: None,
    });
    if head.is_none() {
        *head = Some(new_node);
        return;
    }
    let mut cur = head.as_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_mut().unwrap();
    }
    cur.next = Some(new_node);
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut program = ProgramNode { expressions: None };
    while !matches_token(state, TokenKind::Eof) {
        let expr = parse_expression(source, state);
        append_expression_list(&mut program.expressions, expr);
    }
    result.program = Some(Box::new(program));
}
fn eval_expr(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    match expression.type_ {
        ExpressionType::List => eval_list_expression(expression, env, context),
        ExpressionType::SymbolicExp => eval_symbolic_expression(expression, env, context),
        ExpressionType::Literal => eval_literal_expression(expression),
        ExpressionType::Symbol => eval_symbol_expression(expression, env, context),
    }
}

fn eval_literal_expression(expression: &ExpressionNode) -> Object {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                let v = if let LiteralValue::IntValue(x) = &lit.value {
                    *x
                } else {
                    0
                };
                Object {
                    marked: false,
                    type_: ObjectType::Integer,
                    value: ObjectValue::IntValue(v),
                }
            }
            LiteralType::String => {
                let s = if let LiteralValue::StringValue(x) = &lit.value {
                    x.clone()
                } else {
                    String::new()
                };
                Object {
                    marked: false,
                    type_: ObjectType::String,
                    value: ObjectValue::StringValue(s),
                }
            }
            LiteralType::Boolean => {
                let v = if let LiteralValue::BooleanValue(x) = &lit.value {
                    *x
                } else {
                    false
                };
                Object {
                    marked: false,
                    type_: ObjectType::Bool,
                    value: ObjectValue::BoolValue(if v { 1 } else { 0 }),
                }
            }
        }
    } else {
        nil_object()
    }
}

fn eval_symbol_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    _context: &mut AllocatorContext,
) -> Object {
    if let ExpressionData::Symbol(Some(s)) = &expression.data {
        if s.symbol_name == "nil" {
            return nil_object();
        }
        if let Some(val) = env_lookup(env, &s.symbol_name) {
            return clone_object(val);
        }
        panic!("Undefined symbol: {}", s.symbol_name);
    }
    nil_object()
}

fn eval_list_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let exprs = if let ExpressionData::List(Some(node)) = &expression.data {
        &node.expressions
    } else {
        return nil_object();
    };
    if exprs.is_none() {
        return nil_object();
    }
    let mut items: Vec<Object> = Vec::new();
    let mut current = exprs.as_ref();
    while let Some(node) = current {
        if let Some(e) = &node.expression {
            let evaluated = eval_expr(e, env, context);
            items.push(evaluated);
        }
        current = node.next.as_ref();
    }
    build_object_list_from_objects(items)
}

fn get_expr_list_at<'a>(
    list: &'a Option<Box<ExpressionList>>,
    idx: usize,
) -> Option<&'a ExpressionNode> {
    let mut current = list.as_ref();
    let mut i = 0;
    while let Some(node) = current {
        if i == idx {
            return node.expression.as_deref();
        }
        i += 1;
        current = node.next.as_ref();
    }
    None
}

fn eval_symbolic_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    // Same shape as List in C union
    let exprs_opt: &Option<Box<ExpressionList>> = match &expression.data {
        ExpressionData::SymbolicExp(Some(node)) => &node.expressions,
        ExpressionData::List(Some(node)) => &node.expressions,
        _ => return nil_object(),
    };
    if exprs_opt.is_none() {
        return nil_object();
    }
    let head = exprs_opt.as_ref().unwrap();
    let first_expr = match head.expression.as_ref() {
        Some(e) => e,
        None => return nil_object(),
    };
    let symbol_name = match &first_expr.data {
        ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
        _ => panic!("S-exp must be started with symbol."),
    };

    // Built-in special forms and functions
    let name = symbol_name.as_str();
    match name {
        "if" => {
            let cond = get_expr_list_at(exprs_opt, 1).expect("if must have condition.");
            let then_branch =
                get_expr_list_at(exprs_opt, 2).expect("if must have then clause.");
            let cond_obj = eval_expr(cond, env, context);
            if bool_val(&cond_obj) {
                eval_expr(then_branch, env, context)
            } else if let Some(else_branch) = get_expr_list_at(exprs_opt, 3) {
                eval_expr(else_branch, env, context)
            } else {
                nil_object()
            }
        }
        "while" => {
            let cond = get_expr_list_at(exprs_opt, 1).expect("while must have condition.");
            let body = get_expr_list_at(exprs_opt, 2).expect("while must have body.");
            let cond_clone = clone_expression(cond);
            let body_clone = clone_expression(body);
            loop {
                let cond_obj = eval_expr(&cond_clone, env, context);
                if bool_val(&cond_obj) {
                    let _ = eval_expr(&body_clone, env, context);
                } else {
                    return nil_object();
                }
            }
        }
        "=" => {
            let sym_expr =
                get_expr_list_at(exprs_opt, 1).expect("assignment must have target.");
            let sym_name = match &sym_expr.data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => panic!("Variable name must be symbol."),
            };
            let val_expr =
                get_expr_list_at(exprs_opt, 2).expect("assignment must have expression.");
            let evaluated = eval_expr(val_expr, env, context);
            let cloned = clone_object(&evaluated);
            env_set(env, &sym_name, cloned);
            evaluated
        }
        "defun" => {
            let name_expr =
                get_expr_list_at(exprs_opt, 1).expect("defun must have name.");
            let fn_name = match &name_expr.data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => panic!("Function name must be symbol."),
            };
            let params_expr =
                get_expr_list_at(exprs_opt, 2).expect("defun must have parameters.");
            let params_list = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(n)) => &n.expressions,
                _ => panic!("Function parameter must be list."),
            };
            let mut param_names: Vec<String> = Vec::new();
            let mut cur = params_list.as_ref();
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    if let ExpressionData::Symbol(Some(s)) = &e.data {
                        param_names.push(s.symbol_name.clone());
                    } else {
                        panic!("Function parameter must be symbol.");
                    }
                }
                cur = node.next.as_ref();
            }
            let body_expr = get_expr_list_at(exprs_opt, 3).expect("defun must have body.");
            let body_clone = clone_expression(body_expr);
            let func = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body_clone)),
            };
            let obj = Object {
                marked: false,
                type_: ObjectType::Function,
                value: ObjectValue::FunctionValue(Some(Box::new(func))),
            };
            let cloned = clone_object(&obj);
            env_set(env, &fn_name, cloned);
            obj
        }
        "+" => binary_op(exprs_opt, env, context, |a, b| {
            match (a.type_, b.type_) {
                (ObjectType::Integer, ObjectType::Integer) => {
                    let x = if let ObjectValue::IntValue(v) = a.value { v } else { 0 };
                    let y = if let ObjectValue::IntValue(v) = b.value { v } else { 0 };
                    Object {
                        marked: false,
                        type_: ObjectType::Integer,
                        value: ObjectValue::IntValue(x + y),
                    }
                }
                (ObjectType::String, ObjectType::String) => {
                    let x = if let ObjectValue::StringValue(v) = a.value {
                        v
                    } else {
                        String::new()
                    };
                    let y = if let ObjectValue::StringValue(v) = b.value {
                        v
                    } else {
                        String::new()
                    };
                    Object {
                        marked: false,
                        type_: ObjectType::String,
                        value: ObjectValue::StringValue(format!("{}{}", x, y)),
                    }
                }
                _ => panic!("Type error: operands for + must be integers or strings."),
            }
        }),
        "-" => binary_op(exprs_opt, env, context, |a, b| {
            int_op(a, b, "-", |x, y| x - y)
        }),
        "*" => binary_op(exprs_opt, env, context, |a, b| {
            int_op(a, b, "*", |x, y| x * y)
        }),
        "/" => binary_op(exprs_opt, env, context, |a, b| {
            int_op(a, b, "/", |x, y| x / y)
        }),
        "%" => binary_op(exprs_opt, env, context, |a, b| {
            int_op(a, b, "%", |x, y| x % y)
        }),
        "||" => {
            let mut node = head.next.as_ref();
            while let Some(n) = node {
                if let Some(e) = &n.expression {
                    let v = eval_expr(e, env, context);
                    if bool_val(&v) {
                        return Object {
                            marked: false,
                            type_: ObjectType::Bool,
                            value: ObjectValue::BoolValue(1),
                        };
                    }
                }
                node = n.next.as_ref();
            }
            Object {
                marked: false,
                type_: ObjectType::Bool,
                value: ObjectValue::BoolValue(0),
            }
        }
        "&&" => {
            let mut node = head.next.as_ref();
            while let Some(n) = node {
                if let Some(e) = &n.expression {
                    let v = eval_expr(e, env, context);
                    if !bool_val(&v) {
                        return Object {
                            marked: false,
                            type_: ObjectType::Bool,
                            value: ObjectValue::BoolValue(0),
                        };
                    }
                }
                node = n.next.as_ref();
            }
            Object {
                marked: false,
                type_: ObjectType::Bool,
                value: ObjectValue::BoolValue(1),
            }
        }
        "<" => binary_op(exprs_opt, env, context, |a, b| {
            int_cmp(a, b, "<", |x, y| x < y)
        }),
        ">" => binary_op(exprs_opt, env, context, |a, b| {
            int_cmp(a, b, ">", |x, y| x > y)
        }),
        "eq" => binary_op(exprs_opt, env, context, |a, b| Object {
            marked: false,
            type_: ObjectType::Bool,
            value: ObjectValue::BoolValue(if objects_eq(&a, &b) { 1 } else { 0 }),
        }),
        "not" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("not requires arg"),
                env,
                context,
            );
            if !matches!(v.type_, ObjectType::Bool) {
                panic!("Type error: not operand must be boolean.");
            }
            let bv = if let ObjectValue::BoolValue(x) = v.value { x } else { 0 };
            Object {
                marked: false,
                type_: ObjectType::Bool,
                value: ObjectValue::BoolValue(if bv != 0 { 0 } else { 1 }),
            }
        }
        "print" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("print requires arg"),
                env,
                context,
            );
            println!("{}", stringify_object(&v));
            nil_object()
        }
        "car" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("car requires arg"),
                env,
                context,
            );
            if !matches!(v.type_, ObjectType::List) {
                panic!("Type error: car operand must be list.");
            }
            if let ObjectValue::ListValue(Some(cell)) = v.value {
                if let Some(car) = cell.car {
                    return *car;
                }
            }
            nil_object()
        }
        "cdr" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("cdr requires arg"),
                env,
                context,
            );
            if !matches!(v.type_, ObjectType::List) {
                panic!("Type error: cdr operand must be list.");
            }
            if let ObjectValue::ListValue(Some(cell)) = v.value {
                if let Some(cdr) = cell.cdr {
                    return *cdr;
                }
            }
            nil_object()
        }
        "cons" => {
            let op1 = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("cons arg1"),
                env,
                context,
            );
            let op2 = eval_expr(
                get_expr_list_at(exprs_opt, 2).expect("cons arg2"),
                env,
                context,
            );
            cons_op(op1, op2)
        }
        "split" => {
            let op1 = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("split arg1"),
                env,
                context,
            );
            let op2 = eval_expr(
                get_expr_list_at(exprs_opt, 2).expect("split arg2"),
                env,
                context,
            );
            split_op(op1, op2)
        }
        "list-ref" => {
            let op1 = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("list-ref arg1"),
                env,
                context,
            );
            let op2 = eval_expr(
                get_expr_list_at(exprs_opt, 2).expect("list-ref arg2"),
                env,
                context,
            );
            list_ref_op(op1, op2)
        }
        "progn" => {
            let mut node = head.next.as_ref();
            let mut last = nil_object();
            while let Some(n) = node {
                if let Some(e) = &n.expression {
                    last = eval_expr(e, env, context);
                }
                node = n.next.as_ref();
            }
            last
        }
        "remove-whitespaces" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("remove-whitespaces arg"),
                env,
                context,
            );
            if !matches!(v.type_, ObjectType::String) {
                panic!("Type error: remove-whitespaces operand must be string.");
            }
            let s = if let ObjectValue::StringValue(x) = v.value {
                x
            } else {
                String::new()
            };
            let filtered: String = s.chars().filter(|c| !c.is_whitespace()).collect();
            Object {
                marked: false,
                type_: ObjectType::String,
                value: ObjectValue::StringValue(filtered),
            }
        }
        "pop" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("pop arg"),
                env,
                context,
            );
            pop_op(v)
        }
        "push" => {
            // push takes args (list value); per C: operand2 is at next, operand1 is at next->next
            // C calls: definedFunctionPush(op2, op1, ...) where op1 = expressions->next->next, op2 = expressions->next
            // i.e. push <list-symbol> <value>: appends value to the list.
            // We need to mutate the list bound in env. Get the list symbol name:
            let list_arg_expr =
                get_expr_list_at(exprs_opt, 1).expect("push arg list");
            let val_arg_expr =
                get_expr_list_at(exprs_opt, 2).expect("push arg val");
            let val_obj = eval_expr(val_arg_expr, env, context);
            push_op(list_arg_expr, val_obj, env, context)
        }
        "length" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("length arg"),
                env,
                context,
            );
            length_op(v)
        }
        "is-int-string" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("is-int-string arg"),
                env,
                context,
            );
            match v.type_ {
                ObjectType::String => {
                    let s = if let ObjectValue::StringValue(x) = v.value {
                        x
                    } else {
                        String::new()
                    };
                    let all_digits = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
                    Object {
                        marked: false,
                        type_: ObjectType::Bool,
                        value: ObjectValue::BoolValue(if all_digits { 1 } else { 0 }),
                    }
                }
                _ => Object {
                    marked: false,
                    type_: ObjectType::Bool,
                    value: ObjectValue::BoolValue(0),
                },
            }
        }
        "parse-int" => {
            let v = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("parse-int arg"),
                env,
                context,
            );
            if !matches!(v.type_, ObjectType::String) {
                panic!("Type error: parse-int operand must be string.");
            }
            let s = if let ObjectValue::StringValue(x) = v.value {
                x
            } else {
                String::new()
            };
            let n: i32 = s.parse().expect("parse-int operand must be string of digits.");
            Object {
                marked: false,
                type_: ObjectType::Integer,
                value: ObjectValue::IntValue(n),
            }
        }
        "string-ref" => {
            let op1 = eval_expr(
                get_expr_list_at(exprs_opt, 1).expect("string-ref arg1"),
                env,
                context,
            );
            let op2 = eval_expr(
                get_expr_list_at(exprs_opt, 2).expect("string-ref arg2"),
                env,
                context,
            );
            if !matches!(op1.type_, ObjectType::String) {
                panic!("Type error: string-ref first operand must be string.");
            }
            if !matches!(op2.type_, ObjectType::Integer) {
                panic!("Type error: string-ref second operand must be integer.");
            }
            let s = if let ObjectValue::StringValue(x) = op1.value {
                x
            } else {
                String::new()
            };
            let idx = if let ObjectValue::IntValue(x) = op2.value {
                x
            } else {
                0
            };
            if idx < 0 || idx as usize >= s.len() {
                panic!("Index out of range.");
            }
            let ch = s.as_bytes()[idx as usize] as char;
            Object {
                marked: false,
                type_: ObjectType::String,
                value: ObjectValue::StringValue(ch.to_string()),
            }
        }
        "readline" => {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) => nil_object(),
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    Object {
                        marked: false,
                        type_: ObjectType::String,
                        value: ObjectValue::StringValue(line),
                    }
                }
                Err(_) => nil_object(),
            }
        }
        _ => {
            // user-defined function call
            call_user_function(&symbol_name, exprs_opt, env, context)
        }
    }
}

fn binary_op<F>(
    exprs: &Option<Box<ExpressionList>>,
    env: &mut Env,
    context: &mut AllocatorContext,
    f: F,
) -> Object
where
    F: FnOnce(Object, Object) -> Object,
{
    let a = eval_expr(
        get_expr_list_at(exprs, 1).expect("binary op needs arg1"),
        env,
        context,
    );
    let b = eval_expr(
        get_expr_list_at(exprs, 2).expect("binary op needs arg2"),
        env,
        context,
    );
    f(a, b)
}

fn int_op<F>(a: Object, b: Object, opname: &str, f: F) -> Object
where
    F: FnOnce(i32, i32) -> i32,
{
    match (a.type_, b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let x = if let ObjectValue::IntValue(v) = a.value { v } else { 0 };
            let y = if let ObjectValue::IntValue(v) = b.value { v } else { 0 };
            Object {
                marked: false,
                type_: ObjectType::Integer,
                value: ObjectValue::IntValue(f(x, y)),
            }
        }
        _ => panic!("Type error: operands for {} must be integers.", opname),
    }
}

fn int_cmp<F>(a: Object, b: Object, opname: &str, f: F) -> Object
where
    F: FnOnce(i32, i32) -> bool,
{
    match (a.type_, b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            let x = if let ObjectValue::IntValue(v) = a.value { v } else { 0 };
            let y = if let ObjectValue::IntValue(v) = b.value { v } else { 0 };
            Object {
                marked: false,
                type_: ObjectType::Bool,
                value: ObjectValue::BoolValue(if f(x, y) { 1 } else { 0 }),
            }
        }
        _ => panic!("Type error: operands for {} must be integers.", opname),
    }
}

fn cons_op(op1: Object, op2: Object) -> Object {
    match op2.type_ {
        ObjectType::List | ObjectType::Nil => {
            let cell = ConsCell {
                type_: if matches!(op2.type_, ObjectType::Nil) {
                    ConsCellType::Nil
                } else {
                    ConsCellType::Cell
                },
                car: Some(Box::new(op1)),
                cdr: Some(Box::new(op2)),
            };
            Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(cell))),
            }
        }
        _ => {
            // wrap op2 as nil-terminated list
            let inner_cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op2)),
                cdr: Some(Box::new(nil_object())),
            };
            let inner_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cell))),
            };
            let cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1)),
                cdr: Some(Box::new(inner_obj)),
            };
            Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(cell))),
            }
        }
    }
}

fn split_op(op1: Object, op2: Object) -> Object {
    if !matches!(op1.type_, ObjectType::String) {
        panic!("Type error: split first operand must be string.");
    }
    if !matches!(op2.type_, ObjectType::String) {
        panic!("Type error: split second operand must be string.");
    }
    let s1 = if let ObjectValue::StringValue(v) = op1.value {
        v
    } else {
        String::new()
    };
    let s2 = if let ObjectValue::StringValue(v) = op2.value {
        v
    } else {
        String::new()
    };
    let items: Vec<Object> = if s2.is_empty() {
        s1.chars()
            .map(|c| Object {
                marked: false,
                type_: ObjectType::String,
                value: ObjectValue::StringValue(c.to_string()),
            })
            .collect()
    } else {
        s1.split(&s2)
            .filter(|p| !p.is_empty())
            .map(|p| Object {
                marked: false,
                type_: ObjectType::String,
                value: ObjectValue::StringValue(p.to_string()),
            })
            .collect()
    };
    build_object_list_from_objects(items)
}

fn list_ref_op(op1: Object, op2: Object) -> Object {
    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: list-ref first operand must be list.");
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        panic!("Type error: list-ref second operand must be integer.");
    }
    let idx = if let ObjectValue::IntValue(v) = op2.value {
        v
    } else {
        0
    };
    let mut list_obj = op1;
    let mut i = 0;
    loop {
        if !matches!(list_obj.type_, ObjectType::List) {
            panic!("Index out of range.");
        }
        let cell_box = if let ObjectValue::ListValue(Some(c)) = list_obj.value {
            c
        } else {
            panic!("Index out of range.");
        };
        if i == idx {
            return *cell_box.car.expect("missing car");
        }
        let cdr = *cell_box.cdr.expect("missing cdr");
        if matches!(cdr.type_, ObjectType::Nil) {
            panic!("Index out of range.");
        }
        list_obj = cdr;
        i += 1;
    }
}

fn pop_op(op: Object) -> Object {
    if matches!(op.type_, ObjectType::Nil) {
        return nil_object();
    }
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: pop operand must be list.");
    }
    // walk to last cons cell
    let mut current = if let ObjectValue::ListValue(Some(c)) = op.value {
        c
    } else {
        return nil_object();
    };
    loop {
        if is_last_cons_cell(&current) {
            return *current.car.expect("missing car");
        }
        let cdr = current.cdr.expect("missing cdr");
        current = if let ObjectValue::ListValue(Some(c)) = cdr.value {
            c
        } else {
            return nil_object();
        };
    }
}

fn push_op(
    list_expr: &ExpressionNode,
    val_obj: Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    // If list_expr is a Symbol, mutate that env binding
    if let ExpressionData::Symbol(Some(s)) = &list_expr.data {
        let sym_name = s.symbol_name.clone();
        let current = env_lookup(env, &sym_name).map(clone_object);
        let new_list = match current {
            None => {
                // not found, treat as nil
                let cell = ConsCell {
                    type_: ConsCellType::Cell,
                    car: Some(Box::new(clone_object(&val_obj))),
                    cdr: Some(Box::new(nil_object())),
                };
                Object {
                    marked: false,
                    type_: ObjectType::List,
                    value: ObjectValue::ListValue(Some(Box::new(cell))),
                }
            }
            Some(cur) => match cur.type_ {
                ObjectType::Nil => {
                    let cell = ConsCell {
                        type_: ConsCellType::Cell,
                        car: Some(Box::new(clone_object(&val_obj))),
                        cdr: Some(Box::new(nil_object())),
                    };
                    Object {
                        marked: false,
                        type_: ObjectType::List,
                        value: ObjectValue::ListValue(Some(Box::new(cell))),
                    }
                }
                ObjectType::List => {
                    // Append val to end of list
                    let mut items: Vec<Object> = Vec::new();
                    let mut head_cell = if let ObjectValue::ListValue(Some(c)) = cur.value {
                        Some(c)
                    } else {
                        None
                    };
                    while let Some(c) = head_cell {
                        let is_last = is_last_cons_cell(&c);
                        let ConsCell { car, cdr, .. } = *c;
                        if let Some(car) = car {
                            items.push(*car);
                        }
                        if is_last {
                            break;
                        }
                        head_cell = if let Some(cdr_box) = cdr {
                            if let ObjectValue::ListValue(Some(next_cell)) = cdr_box.value {
                                Some(next_cell)
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    }
                    items.push(clone_object(&val_obj));
                    build_object_list_from_objects(items)
                }
                _ => panic!("Type error: push first operand must be list."),
            },
        };
        env_set(env, &sym_name, new_list);
        return val_obj;
    }
    // Otherwise evaluate the expression and return appended list (no env mutation possible)
    let list_obj = eval_expr(list_expr, env, context);
    match list_obj.type_ {
        ObjectType::Nil => {
            let cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(clone_object(&val_obj))),
                cdr: Some(Box::new(nil_object())),
            };
            let _ = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(cell))),
            };
            val_obj
        }
        ObjectType::List => val_obj,
        _ => panic!("Type error: push first operand must be list."),
    }
}

fn length_op(op: Object) -> Object {
    match op.type_ {
        ObjectType::Nil => Object {
            marked: false,
            type_: ObjectType::Integer,
            value: ObjectValue::IntValue(0),
        },
        ObjectType::List => {
            let mut len = 1i32;
            let mut current = if let ObjectValue::ListValue(Some(c)) = op.value {
                c
            } else {
                return Object {
                    marked: false,
                    type_: ObjectType::Integer,
                    value: ObjectValue::IntValue(0),
                };
            };
            loop {
                if is_last_cons_cell(&current) {
                    break;
                }
                len += 1;
                let cdr = current.cdr.expect("cdr");
                current = if let ObjectValue::ListValue(Some(c)) = cdr.value {
                    c
                } else {
                    break;
                };
            }
            Object {
                marked: false,
                type_: ObjectType::Integer,
                value: ObjectValue::IntValue(len),
            }
        }
        ObjectType::String => {
            let s = if let ObjectValue::StringValue(v) = op.value {
                v
            } else {
                String::new()
            };
            Object {
                marked: false,
                type_: ObjectType::Integer,
                value: ObjectValue::IntValue(s.len() as i32),
            }
        }
        _ => panic!("Type error: length operand must be list or string."),
    }
}

fn call_user_function(
    name: &str,
    exprs: &Option<Box<ExpressionList>>,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    // look up function
    let func_obj = match env_lookup(env, name) {
        Some(o) => clone_object(o),
        None => panic!("Undefined function: {}", name),
    };
    let func = match func_obj.value {
        ObjectValue::FunctionValue(Some(f)) => f,
        _ => panic!("Undefined function: {}", name),
    };

    // Build new env with parent set to a clone of current env for scope semantics
    let mut new_env = Env {
        bindings: std::array::from_fn(|_| Binding {
            symbol_name: String::new(),
            value: None,
        }),
        parent: Some(Box::new(clone_env(env))),
    };

    // bind params
    let mut param_node = if let Some(head) = exprs.as_ref() {
        head.next.as_ref()
    } else {
        None
    };
    for pname in &func.param_symbol_names {
        let pn = match param_node {
            Some(n) => n,
            None => break,
        };
        if let Some(pe) = &pn.expression {
            let pv = eval_expr(pe, env, context);
            env_set(&mut new_env, pname, pv);
        }
        param_node = pn.next.as_ref();
    }

    let body = func.body.expect("function must have body");
    let result = eval_expr(&body, &mut new_env, context);

    // Propagate any binding changes from new_env's parent back to env
    // The parent of new_env carries any updates from inside the function body
    if let Some(parent) = new_env.parent {
        copy_env_bindings(&parent, env);
    }
    result
}

fn clone_env(env: &Env) -> Env {
    let mut bindings: [Binding; MAX_BINDINGS] = std::array::from_fn(|_| Binding {
        symbol_name: String::new(),
        value: None,
    });
    for i in 0..MAX_BINDINGS {
        bindings[i].symbol_name = env.bindings[i].symbol_name.clone();
        bindings[i].value = env.bindings[i].value.as_ref().map(|o| Box::new(clone_object(o)));
    }
    Env {
        bindings,
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

fn copy_env_bindings(src: &Env, dst: &mut Env) {
    for i in 0..MAX_BINDINGS {
        dst.bindings[i].symbol_name = src.bindings[i].symbol_name.clone();
        dst.bindings[i].value = src.bindings[i].value.as_ref().map(|o| Box::new(clone_object(o)));
    }
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let evaluated = eval_expr(expression, env, context);
    *result = evaluated;
}
pub fn evaluate_expression_with_context(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
) {
    let mut context = init_allocator();
    evaluate_expression(expression, result, env, &mut context);
}
pub fn evaluate(result: &mut ParseResult) {
    let program = match &result.program {
        Some(p) => p,
        None => return,
    };
    let mut env = Env {
        bindings: std::array::from_fn(|_| Binding {
            symbol_name: String::new(),
            value: None,
        }),
        parent: None,
    };
    init_env(&mut env);
    let mut context = init_allocator();

    let mut current = program.expressions.as_ref();
    while let Some(node) = current {
        if let Some(e) = &node.expression {
            let mut out = nil_object();
            evaluate_expression(e, &mut out, &mut env, &mut context);
        }
        current = node.next.as_ref();
    }
}
pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => match &obj.value {
            ObjectValue::IntValue(v) => v.to_string(),
            _ => String::new(),
        },
        ObjectType::String => match &obj.value {
            ObjectValue::StringValue(v) => v.clone(),
            _ => String::new(),
        },
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(v) => {
                if *v != 0 {
                    "T".to_string()
                } else {
                    "F".to_string()
                }
            }
            _ => String::new(),
        },
        ObjectType::List => {
            let mut out = String::from("(");
            if let ObjectValue::ListValue(Some(cell)) = &obj.value {
                let mut current: &ConsCell = cell.as_ref();
                loop {
                    if let Some(car) = &current.car {
                        out.push_str(&stringify_object(car));
                    }
                    if is_last_cons_cell(current) {
                        break;
                    }
                    out.push(' ');
                    if let Some(cdr) = &current.cdr {
                        if let ObjectValue::ListValue(Some(next_cell)) = &cdr.value {
                            current = next_cell.as_ref();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            out.push(')');
            out
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}

// =================================================
//   Env helpers
// =================================================

fn env_lookup<'a>(env: &'a Env, name: &str) -> Option<&'a Object> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == name {
            return env.bindings[i].value.as_deref();
        }
    }
    if let Some(parent) = &env.parent {
        env_lookup(parent, name)
    } else {
        None
    }
}

fn env_set(env: &mut Env, name: &str, value: Object) {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name == name && !env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].value = Some(Box::new(value));
            return;
        }
        if env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].symbol_name = name.to_string();
            env.bindings[i].value = Some(Box::new(value));
            return;
        }
    }
}

fn build_object_list_from_objects(items: Vec<Object>) -> Object {
    if items.is_empty() {
        return nil_object();
    }
    // Build from the end backwards
    let mut result_obj = Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    };
    let mut first = true;
    for item in items.into_iter().rev() {
        if first {
            // last cell: cdr -> nil
            let cell = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(item)),
                cdr: Some(Box::new(nil_object())),
            };
            result_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(cell))),
            };
            first = false;
        } else {
            let cell = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(item)),
                cdr: Some(Box::new(result_obj)),
            };
            result_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(cell))),
            };
        }
    }
    result_obj
}
pub fn init_env(env: &mut Env) {
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
    }
    env.parent = None;
}
pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }))
}

pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 0,
        stack: Some(Box::new(ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0u8; FREE_BITMAP_SIZE],
    }
}

// =================================================
//   Cloning helpers (since structs don't derive Clone)
// =================================================

fn clone_expression(node: &ExpressionNode) -> ExpressionNode {
    let data = match &node.data {
        ExpressionData::SymbolicExp(opt) => {
            ExpressionData::SymbolicExp(opt.as_ref().map(|n| {
                Box::new(SymbolicExpNode {
                    expressions: clone_expression_list(&n.expressions),
                })
            }))
        }
        ExpressionData::List(opt) => ExpressionData::List(opt.as_ref().map(|n| {
            Box::new(ListNode {
                expressions: clone_expression_list(&n.expressions),
            })
        })),
        ExpressionData::Literal(opt) => ExpressionData::Literal(opt.as_ref().map(|n| {
            Box::new(LiteralNode {
                type_: n.type_,
                value: match &n.value {
                    LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
                    LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
                    LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
                },
            })
        })),
        ExpressionData::Symbol(opt) => ExpressionData::Symbol(opt.as_ref().map(|n| {
            Box::new(SymbolNode {
                symbol_name: n.symbol_name.clone(),
            })
        })),
    };
    ExpressionNode {
        type_: node.type_,
        data,
    }
}

fn clone_expression_list(
    list: &Option<Box<ExpressionList>>,
) -> Option<Box<ExpressionList>> {
    list.as_ref().map(|n| {
        Box::new(ExpressionList {
            expression: n.expression.as_ref().map(|e| Box::new(clone_expression(e))),
            next: clone_expression_list(&n.next),
        })
    })
}

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: match &obj.value {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(opt) => {
                ObjectValue::ListValue(opt.as_ref().map(|c| Box::new(clone_cons(c))))
            }
            ObjectValue::FunctionValue(opt) => ObjectValue::FunctionValue(
                opt.as_ref().map(|f| {
                    Box::new(Function {
                        param_symbol_names: f.param_symbol_names.clone(),
                        body: f.body.as_ref().map(|b| Box::new(clone_expression(b))),
                    })
                }),
            ),
        },
    }
}

fn clone_cons(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c.car.as_ref().map(|o| Box::new(clone_object(o))),
        cdr: c.cdr.as_ref().map(|o| Box::new(clone_object(o))),
    }
}

fn nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn is_last_cons_cell(c: &ConsCell) -> bool {
    match &c.cdr {
        Some(o) => matches!(o.type_, ObjectType::Nil),
        None => true,
    }
}

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(v) => *v != 0,
            _ => false,
        },
        ObjectType::Nil => false,
        _ => true,
    }
}

fn objects_eq(a: &Object, b: &Object) -> bool {
    match (a.type_, b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            match (&a.value, &b.value) {
                (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) => x == y,
                _ => false,
            }
        }
        (ObjectType::String, ObjectType::String) => match (&a.value, &b.value) {
            (ObjectValue::StringValue(x), ObjectValue::StringValue(y)) => x == y,
            _ => false,
        },
        (ObjectType::Bool, ObjectType::Bool) => match (&a.value, &b.value) {
            (ObjectValue::BoolValue(x), ObjectValue::BoolValue(y)) => x == y,
            _ => false,
        },
        (ObjectType::Nil, ObjectType::Nil) => true,
        (ObjectType::List, ObjectType::List) => false, // list identity not preserved here
        _ => false,
    }
}