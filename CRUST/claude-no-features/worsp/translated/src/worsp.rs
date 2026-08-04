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

// ===========================================================
//   Helpers
// ===========================================================

fn is_op(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-' | '*' | '/' | '%' | '|' | '&' | '=' | '<' | '>'
    )
}

fn new_object_nil() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn new_binding() -> Binding {
    Binding {
        symbol_name: String::new(),
        value: None,
    }
}

fn new_env() -> Env {
    Env {
        bindings: std::array::from_fn(|_| new_binding()),
        parent: None,
    }
}

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: clone_object_value(&obj.value),
    }
}

fn clone_object_value(val: &ObjectValue) -> ObjectValue {
    match val {
        ObjectValue::IntValue(i) => ObjectValue::IntValue(*i),
        ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
        ObjectValue::BoolValue(b) => ObjectValue::BoolValue(*b),
        ObjectValue::ListValue(c) => ObjectValue::ListValue(
            c.as_ref().map(|c| Box::new(clone_cons_cell(c))),
        ),
        ObjectValue::FunctionValue(f) => ObjectValue::FunctionValue(
            f.as_ref().map(|f| Box::new(clone_function(f))),
        ),
    }
}

fn clone_cons_cell(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c.car.as_ref().map(|o| Box::new(clone_object(o))),
        cdr: c.cdr.as_ref().map(|o| Box::new(clone_object(o))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: f.body.as_ref().map(|b| Box::new(clone_expression_node(b))),
    }
}

fn clone_expression_node(e: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: e.type_,
        data: clone_expression_data(&e.data),
    }
}

fn clone_expression_data(d: &ExpressionData) -> ExpressionData {
    match d {
        ExpressionData::SymbolicExp(s) => {
            ExpressionData::SymbolicExp(s.as_ref().map(|n| {
                Box::new(SymbolicExpNode {
                    expressions: clone_expression_list(&n.expressions),
                })
            }))
        }
        ExpressionData::List(l) => ExpressionData::List(l.as_ref().map(|n| {
            Box::new(ListNode {
                expressions: clone_expression_list(&n.expressions),
            })
        })),
        ExpressionData::Literal(l) => ExpressionData::Literal(l.as_ref().map(|n| {
            Box::new(LiteralNode {
                type_: n.type_,
                value: match &n.value {
                    LiteralValue::IntValue(i) => LiteralValue::IntValue(*i),
                    LiteralValue::BooleanValue(b) => LiteralValue::BooleanValue(*b),
                    LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
                },
            })
        })),
        ExpressionData::Symbol(s) => ExpressionData::Symbol(s.as_ref().map(|n| {
            Box::new(SymbolNode {
                symbol_name: n.symbol_name.clone(),
            })
        })),
    }
}

fn clone_expression_list(
    list: &Option<Box<ExpressionList>>,
) -> Option<Box<ExpressionList>> {
    list.as_ref().map(|l| {
        Box::new(ExpressionList {
            expression: l.expression.as_ref().map(|e| Box::new(clone_expression_node(e))),
            next: clone_expression_list(&l.next),
        })
    })
}

// ===========================================================
//   Tokenizer
// ===========================================================

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(ref token) = state.token {
        if token.kind == kind {
            1
        } else {
            0
        }
    } else {
        0
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    let len = bytes.len();

    // skip whitespaces
    while (state.pos as usize) < len {
        let ch = bytes[state.pos as usize] as char;
        if ch.is_whitespace() || ch == '\n' {
            state.pos += 1;
        } else {
            break;
        }
    }

    if (state.pos as usize) >= len {
        // EOF
        state.token = Some(Box::new(Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: "\0".to_string(),
        }));
        return;
    }

    let ch = bytes[state.pos as usize] as char;
    let mut new_token = Token {
        kind: TokenKind::Eof,
        next: None,
        val: 0,
        str: String::new(),
    };

    if ch == '(' {
        new_token.kind = TokenKind::LParen;
        new_token.str = "(".to_string();
        state.pos += 1;
    } else if ch == ')' {
        new_token.kind = TokenKind::RParen;
        new_token.str = ")".to_string();
        state.pos += 1;
    } else if ch == '\'' {
        new_token.kind = TokenKind::Quote;
        new_token.str = "'".to_string();
        state.pos += 1;
    } else if ch.is_ascii_alphabetic() || is_op(ch) {
        let start = state.pos as usize;
        while (state.pos as usize) < len {
            let c = bytes[state.pos as usize] as char;
            if c.is_ascii_alphanumeric() || is_op(c) {
                state.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&bytes[start..state.pos as usize])
            .unwrap()
            .to_string();
        if s == "true" {
            new_token.kind = TokenKind::True;
        } else if s == "false" {
            new_token.kind = TokenKind::False;
        } else {
            new_token.kind = TokenKind::Symbol;
            new_token.str = s;
        }
    } else if ch.is_ascii_digit() {
        let start = state.pos as usize;
        while (state.pos as usize) < len
            && (bytes[state.pos as usize] as char).is_ascii_digit()
        {
            state.pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..state.pos as usize]).unwrap();
        new_token.kind = TokenKind::Digit;
        new_token.val = s.parse().unwrap_or(0);
    } else if ch == '"' {
        state.pos += 1;
        let start = state.pos as usize;
        while (state.pos as usize) < len {
            let c = bytes[state.pos as usize] as char;
            if c != '"' {
                state.pos += 1;
            } else {
                break;
            }
        }
        let s = std::str::from_utf8(&bytes[start..state.pos as usize])
            .unwrap()
            .to_string();
        new_token.kind = TokenKind::String;
        new_token.str = s;
        if (state.pos as usize) < len && (bytes[state.pos as usize] as char) == '"' {
            state.pos += 1;
        }
    } else if ch == ';' {
        while (state.pos as usize) < len && (bytes[state.pos as usize] as char) != '\n' {
            state.pos += 1;
        }
        next(source, state);
        return;
    } else {
        eprintln!("Unexpected token: {}", ch);
        std::process::exit(1);
    }

    state.token = Some(Box::new(new_token));
}

// ===========================================================
//   Parser
// ===========================================================

fn vec_to_expression_list(mut exprs: Vec<Box<ExpressionNode>>) -> Option<Box<ExpressionList>> {
    let mut head: Option<Box<ExpressionList>> = None;
    while let Some(expr) = exprs.pop() {
        head = Some(Box::new(ExpressionList {
            expression: Some(expr),
            next: head,
        }));
    }
    head
}

fn parse_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    if match_token(state, TokenKind::LParen) != 0 {
        parse_symbolic_expression(source, state)
    } else if match_token(state, TokenKind::Quote) != 0 {
        parse_list_expression(source, state)
    } else if match_token(state, TokenKind::Symbol) != 0 {
        parse_symbol_expression(source, state)
    } else if match_token(state, TokenKind::Digit) != 0
        || match_token(state, TokenKind::String) != 0
        || match_token(state, TokenKind::True) != 0
        || match_token(state, TokenKind::False) != 0
    {
        parse_literal_expression(source, state)
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    next(source, state); // eat '('
    let mut exprs: Vec<Box<ExpressionNode>> = Vec::new();
    while match_token(state, TokenKind::RParen) == 0 {
        let expr = parse_expression(source, state);
        exprs.push(expr);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode {
            expressions: vec_to_expression_list(exprs),
        }))),
    })
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    next(source, state); // eat quote
    next(source, state); // eat '('
    let mut exprs: Vec<Box<ExpressionNode>> = Vec::new();
    while match_token(state, TokenKind::RParen) == 0 {
        let expr = parse_expression(source, state);
        exprs.push(expr);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(ListNode {
            expressions: vec_to_expression_list(exprs),
        }))),
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
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name: name }))),
    })
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let (lit_type, lit_value) = if match_token(state, TokenKind::Digit) != 0 {
        let v = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        (LiteralType::Integer, LiteralValue::IntValue(v))
    } else if match_token(state, TokenKind::String) != 0 {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        (LiteralType::String, LiteralValue::StringValue(s))
    } else if match_token(state, TokenKind::True) != 0 {
        (LiteralType::Boolean, LiteralValue::BooleanValue(true))
    } else if match_token(state, TokenKind::False) != 0 {
        (LiteralType::Boolean, LiteralValue::BooleanValue(false))
    } else {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    };
    next(source, state);
    Box::new(ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: lit_type,
            value: lit_value,
        }))),
    })
}

fn parse_program(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut exprs: Vec<Box<ExpressionNode>> = Vec::new();
    while match_token(state, TokenKind::Eof) == 0 {
        let expr = parse_expression(source, state);
        exprs.push(expr);
    }
    let program = Box::new(ProgramNode {
        expressions: vec_to_expression_list(exprs),
    });
    result.program = Some(program);
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    parse_program(source, state, result);
}

// ===========================================================
//   Evaluator
// ===========================================================

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => match obj.value {
            ObjectValue::BoolValue(v) => v != 0,
            _ => false,
        },
        ObjectType::Nil => false,
        _ => true,
    }
}

fn eq_objects(op1: &Object, op2: &Object) -> bool {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => match (&op1.value, &op2.value) {
            (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => a == b,
            _ => false,
        },
        (ObjectType::String, ObjectType::String) => match (&op1.value, &op2.value) {
            (ObjectValue::StringValue(a), ObjectValue::StringValue(b)) => a == b,
            _ => false,
        },
        (ObjectType::Bool, ObjectType::Bool) => match (&op1.value, &op2.value) {
            (ObjectValue::BoolValue(a), ObjectValue::BoolValue(b)) => a == b,
            _ => false,
        },
        (ObjectType::Nil, ObjectType::Nil) => true,
        (ObjectType::List, ObjectType::List) => false,
        _ => false,
    }
}

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => match &obj.value {
            ObjectValue::IntValue(v) => v.to_string(),
            _ => String::from("0"),
        },
        ObjectType::String => match &obj.value {
            ObjectValue::StringValue(s) => s.clone(),
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
            _ => "F".to_string(),
        },
        ObjectType::List => {
            let mut s = String::from("(");
            if let ObjectValue::ListValue(ref maybe_cell) = obj.value {
                let mut current = maybe_cell.as_ref().map(|b| b.as_ref());
                let mut first = true;
                while let Some(c) = current {
                    if !first {
                        s.push(' ');
                    }
                    first = false;
                    if let Some(ref car) = c.car {
                        s.push_str(&stringify_object(car));
                    }
                    if let Some(ref cdr) = c.cdr {
                        if matches!(cdr.type_, ObjectType::Nil) {
                            break;
                        }
                        if let ObjectValue::ListValue(ref cdr_cell) = cdr.value {
                            current = cdr_cell.as_ref().map(|b| b.as_ref());
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
            s.push(')');
            s
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for binding in env.bindings.iter_mut() {
        binding.symbol_name = String::new();
        binding.value = None;
    }
}

pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 0,
        stack: Some(Box::new(ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0; FREE_BITMAP_SIZE],
    }
}

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(new_object_nil()))
}

// Look up symbol in env chain. Returns a clone of the object if found.
fn env_lookup(env: &Env, name: &str) -> Option<Object> {
    for binding in env.bindings.iter() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == name {
            return binding.value.as_ref().map(|v| clone_object(v));
        }
    }
    if let Some(ref parent) = env.parent {
        return env_lookup(parent, name);
    }
    None
}

fn env_set(env: &mut Env, name: &str, value: Object) {
    for binding in env.bindings.iter_mut() {
        if binding.symbol_name == name {
            binding.value = Some(Box::new(value));
            return;
        }
        if binding.symbol_name.is_empty() {
            binding.symbol_name = name.to_string();
            binding.value = Some(Box::new(value));
            return;
        }
    }
}

fn make_int(v: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(v),
    }
}

fn make_bool(v: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if v { 1 } else { 0 }),
    }
}

fn make_string(s: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(s),
    }
}

fn make_nil() -> Object {
    new_object_nil()
}

fn evaluate_expression_inner(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    match expression.type_ {
        ExpressionType::Literal => evaluate_literal(expression),
        ExpressionType::Symbol => evaluate_symbol(expression, env),
        ExpressionType::List => evaluate_list(expression, env, context),
        ExpressionType::SymbolicExp => evaluate_symbolic(expression, env, context),
    }
}

fn evaluate_literal(expression: &ExpressionNode) -> Object {
    if let ExpressionData::Literal(Some(ref lit)) = expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                if let LiteralValue::IntValue(v) = lit.value {
                    return make_int(v);
                }
            }
            LiteralType::String => {
                if let LiteralValue::StringValue(ref s) = lit.value {
                    return make_string(s.clone());
                }
            }
            LiteralType::Boolean => {
                if let LiteralValue::BooleanValue(b) = lit.value {
                    return make_bool(b);
                }
            }
        }
    }
    make_nil()
}

fn evaluate_symbol(expression: &ExpressionNode, env: &mut Env) -> Object {
    if let ExpressionData::Symbol(Some(ref sym)) = expression.data {
        if sym.symbol_name == "nil" {
            return make_nil();
        }
        if let Some(obj) = env_lookup(env, &sym.symbol_name) {
            return obj;
        }
        eprintln!("Undefined symbol: {}", sym.symbol_name);
        std::process::exit(1);
    }
    make_nil()
}

fn collect_expr_list(list: &Option<Box<ExpressionList>>) -> Vec<&ExpressionNode> {
    let mut v = Vec::new();
    let mut current = list.as_ref();
    while let Some(node) = current {
        if let Some(ref e) = node.expression {
            v.push(e.as_ref());
        }
        current = node.next.as_ref();
    }
    v
}

fn evaluate_list(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let exprs = if let ExpressionData::List(Some(ref list)) = expression.data {
        collect_expr_list(&list.expressions)
    } else {
        return make_nil();
    };

    if exprs.is_empty() {
        return make_nil();
    }

    // Evaluate all expressions
    let mut values: Vec<Object> = Vec::with_capacity(exprs.len());
    for e in &exprs {
        let v = evaluate_expression_inner(e, env, context);
        values.push(v);
    }

    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(build_cons_list(values)),
    }
}

fn build_cons_list(items: Vec<Object>) -> Option<Box<ConsCell>> {
    if items.is_empty() {
        return None;
    }
    // Build from the back
    let mut iter = items.into_iter().rev();
    let last = iter.next().unwrap();
    let nil_obj = Box::new(make_nil());
    let mut cell = Box::new(ConsCell {
        type_: ConsCellType::Nil,
        car: Some(Box::new(last)),
        cdr: Some(nil_obj),
    });
    for item in iter {
        let cdr_obj = Box::new(Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(cell)),
        });
        cell = Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(item)),
            cdr: Some(cdr_obj),
        });
    }
    Some(cell)
}

fn evaluate_symbolic(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let exprs = if let ExpressionData::SymbolicExp(Some(ref s)) = expression.data {
        collect_expr_list(&s.expressions)
    } else {
        return make_nil();
    };

    if exprs.is_empty() {
        return make_nil();
    }

    // First expr should be a symbol (function/special form name)
    let first = exprs[0];
    let sym_name: String = if let ExpressionData::Symbol(Some(ref s)) = first.data {
        s.symbol_name.clone()
    } else {
        eprintln!("S-exp must be started with symbol.");
        std::process::exit(1);
    };

    let args: Vec<&ExpressionNode> = exprs[1..].to_vec();

    match sym_name.as_str() {
        "if" => {
            if args.is_empty() {
                eprintln!("if must have condition.");
                std::process::exit(1);
            }
            if args.len() < 2 {
                eprintln!("if must have then clause.");
                std::process::exit(1);
            }
            let cond = evaluate_expression_inner(args[0], env, context);
            if bool_val(&cond) {
                evaluate_expression_inner(args[1], env, context)
            } else if args.len() >= 3 {
                evaluate_expression_inner(args[2], env, context)
            } else {
                make_nil()
            }
        }
        "while" => {
            if args.len() < 2 {
                eprintln!("while must have condition and body.");
                std::process::exit(1);
            }
            loop {
                let cond = evaluate_expression_inner(args[0], env, context);
                if bool_val(&cond) {
                    let _ = evaluate_expression_inner(args[1], env, context);
                } else {
                    return make_nil();
                }
            }
        }
        "=" => {
            if args.len() < 2 {
                eprintln!("assignment must have name and expression.");
                std::process::exit(1);
            }
            let sym_name = if let ExpressionData::Symbol(Some(ref s)) = args[0].data {
                s.symbol_name.clone()
            } else {
                eprintln!("Variable name must be symbol.");
                std::process::exit(1);
            };
            let value = evaluate_expression_inner(args[1], env, context);
            let cloned = clone_object(&value);
            env_set(env, &sym_name, cloned);
            value
        }
        "defun" => {
            if args.len() < 3 {
                eprintln!("defun needs name, params, body.");
                std::process::exit(1);
            }
            let fn_name = if let ExpressionData::Symbol(Some(ref s)) = args[0].data {
                s.symbol_name.clone()
            } else {
                eprintln!("Function name must be symbol.");
                std::process::exit(1);
            };
            let param_names: Vec<String> =
                if let ExpressionData::SymbolicExp(Some(ref se)) = args[1].data {
                    let plist = collect_expr_list(&se.expressions);
                    plist
                        .iter()
                        .map(|p| {
                            if let ExpressionData::Symbol(Some(ref s)) = p.data {
                                s.symbol_name.clone()
                            } else {
                                eprintln!("Function parameter must be symbol.");
                                std::process::exit(1);
                            }
                        })
                        .collect()
                } else {
                    eprintln!("Function parameter must be list.");
                    std::process::exit(1);
                };
            let body = clone_expression_node(args[2]);
            let function = Object {
                marked: false,
                type_: ObjectType::Function,
                value: ObjectValue::FunctionValue(Some(Box::new(Function {
                    param_symbol_names: param_names,
                    body: Some(Box::new(body)),
                }))),
            };
            let cloned = clone_object(&function);
            env_set(env, &fn_name, cloned);
            function
        }
        "+" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            match (a.type_, b.type_) {
                (ObjectType::Integer, ObjectType::Integer) => {
                    if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) =
                        (a.value, b.value)
                    {
                        make_int(x + y)
                    } else {
                        make_nil()
                    }
                }
                (ObjectType::String, ObjectType::String) => {
                    if let (ObjectValue::StringValue(x), ObjectValue::StringValue(y)) =
                        (a.value, b.value)
                    {
                        make_string(format!("{}{}", x, y))
                    } else {
                        make_nil()
                    }
                }
                _ => {
                    eprintln!("Type error: operands for + must be integers or strings.");
                    std::process::exit(1);
                }
            }
        }
        "-" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) = (a.value, b.value) {
                make_int(x - y)
            } else {
                eprintln!("Type error: operands for - must be integers.");
                std::process::exit(1);
            }
        }
        "*" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) = (a.value, b.value) {
                make_int(x * y)
            } else {
                eprintln!("Type error: operands for * must be integers.");
                std::process::exit(1);
            }
        }
        "/" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) = (a.value, b.value) {
                make_int(x / y)
            } else {
                eprintln!("Type error: operands for / must be integers.");
                std::process::exit(1);
            }
        }
        "%" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) = (a.value, b.value) {
                make_int(x % y)
            } else {
                eprintln!("Type error: operands for % must be integers.");
                std::process::exit(1);
            }
        }
        "||" => {
            for arg in &args {
                let v = evaluate_expression_inner(arg, env, context);
                if bool_val(&v) {
                    return make_bool(true);
                }
            }
            make_bool(false)
        }
        "&&" => {
            for arg in &args {
                let v = evaluate_expression_inner(arg, env, context);
                if !bool_val(&v) {
                    return make_bool(false);
                }
            }
            make_bool(true)
        }
        "<" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) = (a.value, b.value) {
                make_bool(x < y)
            } else {
                eprintln!("Type error: operands for < must be integers.");
                std::process::exit(1);
            }
        }
        ">" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            if let (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) = (a.value, b.value) {
                make_bool(x > y)
            } else {
                eprintln!("Type error: operands for > must be integers.");
                std::process::exit(1);
            }
        }
        "eq" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            make_bool(eq_objects(&a, &b))
        }
        "not" => {
            let a = evaluate_expression_inner(args[0], env, context);
            if !matches!(a.type_, ObjectType::Bool) {
                eprintln!("Type error: not operand must be boolean.");
                std::process::exit(1);
            }
            make_bool(!bool_val(&a))
        }
        "print" => {
            let a = evaluate_expression_inner(args[0], env, context);
            println!("{}", stringify_object(&a));
            make_nil()
        }
        "car" => {
            let a = evaluate_expression_inner(args[0], env, context);
            if !matches!(a.type_, ObjectType::List) {
                eprintln!("Type error: car operand must be list.");
                std::process::exit(1);
            }
            if let ObjectValue::ListValue(Some(ref cell)) = a.value {
                if let Some(ref car) = cell.car {
                    return clone_object(car);
                }
            }
            make_nil()
        }
        "cdr" => {
            let a = evaluate_expression_inner(args[0], env, context);
            if !matches!(a.type_, ObjectType::List) {
                eprintln!("Type error: cdr operand must be list.");
                std::process::exit(1);
            }
            if let ObjectValue::ListValue(Some(ref cell)) = a.value {
                if let Some(ref cdr) = cell.cdr {
                    return clone_object(cdr);
                }
            }
            make_nil()
        }
        "cons" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            cons_objects(a, b)
        }
        "split" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            split_strings(&a, &b)
        }
        "list-ref" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            list_ref(&a, &b)
        }
        "progn" => {
            let mut last = make_nil();
            for arg in &args {
                last = evaluate_expression_inner(arg, env, context);
            }
            last
        }
        "remove-whitespaces" => {
            let a = evaluate_expression_inner(args[0], env, context);
            if let ObjectValue::StringValue(s) = a.value {
                let new_s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
                make_string(new_s)
            } else {
                eprintln!("Type error: remove-whitespaces operand must be string.");
                std::process::exit(1);
            }
        }
        "length" => {
            let a = evaluate_expression_inner(args[0], env, context);
            length_op(&a)
        }
        "is-int-string" => {
            let a = evaluate_expression_inner(args[0], env, context);
            if let ObjectValue::StringValue(ref s) = a.value {
                let all_digits = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
                make_bool(all_digits)
            } else {
                make_bool(false)
            }
        }
        "parse-int" => {
            let a = evaluate_expression_inner(args[0], env, context);
            if let ObjectValue::StringValue(ref s) = a.value {
                if !s.chars().all(|c| c.is_ascii_digit()) {
                    eprintln!("Type error: parse-int operand must be string of digits.");
                    std::process::exit(1);
                }
                make_int(s.parse().unwrap_or(0))
            } else {
                eprintln!("Type error: parse-int operand must be string.");
                std::process::exit(1);
            }
        }
        "string-ref" => {
            let a = evaluate_expression_inner(args[0], env, context);
            let b = evaluate_expression_inner(args[1], env, context);
            string_ref(&a, &b)
        }
        _ => {
            // function call
            let fn_obj = match env_lookup(env, &sym_name) {
                Some(o) => o,
                None => {
                    eprintln!("Undefined function: {}", sym_name);
                    std::process::exit(1);
                }
            };
            if !matches!(fn_obj.type_, ObjectType::Function) {
                eprintln!("Not a function: {}", sym_name);
                std::process::exit(1);
            }
            let function = if let ObjectValue::FunctionValue(Some(f)) = fn_obj.value {
                f
            } else {
                eprintln!("Not a function: {}", sym_name);
                std::process::exit(1);
            };
            // Evaluate args
            let arg_values: Vec<Object> = args
                .iter()
                .map(|a| evaluate_expression_inner(a, env, context))
                .collect();

            // Move env into new_env.parent
            let parent = std::mem::replace(env, new_env());
            env.parent = Some(Box::new(parent));
            // Bind params
            for (name, val) in function.param_symbol_names.iter().zip(arg_values.into_iter()) {
                env_set(env, name, val);
            }
            // Evaluate body
            let result = if let Some(ref body) = function.body {
                evaluate_expression_inner(body, env, context)
            } else {
                make_nil()
            };
            // Restore env
            let parent = env.parent.take().map(|b| *b).unwrap_or_else(new_env);
            *env = parent;
            result
        }
    }
}

fn cons_objects(op1: Object, op2: Object) -> Object {
    let cdr_obj: Box<Object> = match op2.type_ {
        ObjectType::List => Box::new(op2),
        ObjectType::Nil => Box::new(op2),
        _ => {
            // Wrap op2 in a list
            let nil = Box::new(make_nil());
            let cell = Box::new(ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op2)),
                cdr: Some(nil),
            });
            Box::new(Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(cell)),
            })
        }
    };

    let cell = Box::new(ConsCell {
        type_: match cdr_obj.type_ {
            ObjectType::Nil => ConsCellType::Nil,
            _ => ConsCellType::Cell,
        },
        car: Some(Box::new(op1)),
        cdr: Some(cdr_obj),
    });

    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(Some(cell)),
    }
}

fn split_strings(op1: &Object, op2: &Object) -> Object {
    let s1 = if let ObjectValue::StringValue(ref s) = op1.value {
        s.clone()
    } else {
        eprintln!("Type error: split first operand must be string.");
        std::process::exit(1);
    };
    let s2 = if let ObjectValue::StringValue(ref s) = op2.value {
        s.clone()
    } else {
        eprintln!("Type error: split second operand must be string.");
        std::process::exit(1);
    };

    let parts: Vec<String> = if s2.is_empty() {
        s1.chars().map(|c| c.to_string()).collect()
    } else {
        s1.split(&s2 as &str)
            .filter(|p| !p.is_empty())
            .map(|s| s.to_string())
            .collect()
    };

    let items: Vec<Object> = parts.into_iter().map(make_string).collect();
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(build_cons_list(items)),
    }
}

fn list_ref(op1: &Object, op2: &Object) -> Object {
    if !matches!(op1.type_, ObjectType::List) {
        eprintln!("Type error: list-ref first operand must be list.");
        std::process::exit(1);
    }
    let idx = if let ObjectValue::IntValue(v) = op2.value {
        v
    } else {
        eprintln!("Type error: list-ref second operand must be integer.");
        std::process::exit(1);
    };

    let mut current_cell: Option<&ConsCell> =
        if let ObjectValue::ListValue(Some(ref c)) = op1.value {
            Some(c.as_ref())
        } else {
            None
        };

    for _ in 0..idx {
        let next_cell = if let Some(c) = current_cell {
            if let Some(ref cdr) = c.cdr {
                if matches!(cdr.type_, ObjectType::Nil) {
                    eprintln!("Index out of range.");
                    std::process::exit(1);
                }
                if let ObjectValue::ListValue(Some(ref next)) = cdr.value {
                    Some(next.as_ref())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        current_cell = next_cell;
    }

    if let Some(c) = current_cell {
        if let Some(ref car) = c.car {
            return clone_object(car);
        }
    }
    make_nil()
}

fn length_op(op: &Object) -> Object {
    match op.type_ {
        ObjectType::Nil => make_int(0),
        ObjectType::List => {
            let mut count = 0;
            let mut current: Option<&ConsCell> =
                if let ObjectValue::ListValue(Some(ref c)) = op.value {
                    Some(c.as_ref())
                } else {
                    None
                };
            while let Some(c) = current {
                count += 1;
                if let Some(ref cdr) = c.cdr {
                    if matches!(cdr.type_, ObjectType::Nil) {
                        break;
                    }
                    if let ObjectValue::ListValue(Some(ref next)) = cdr.value {
                        current = Some(next.as_ref());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            make_int(count)
        }
        ObjectType::String => {
            if let ObjectValue::StringValue(ref s) = op.value {
                make_int(s.len() as i32)
            } else {
                make_int(0)
            }
        }
        _ => {
            eprintln!("Type error: length operand must be list or string.");
            std::process::exit(1);
        }
    }
}

fn string_ref(op1: &Object, op2: &Object) -> Object {
    let s = if let ObjectValue::StringValue(ref s) = op1.value {
        s.clone()
    } else {
        eprintln!("Type error: string-ref first operand must be string.");
        std::process::exit(1);
    };
    let idx = if let ObjectValue::IntValue(v) = op2.value {
        v
    } else {
        eprintln!("Type error: string-ref second operand must be integer.");
        std::process::exit(1);
    };
    if idx < 0 || (idx as usize) >= s.len() {
        eprintln!("Index out of range.");
        std::process::exit(1);
    }
    let ch = s.as_bytes()[idx as usize] as char;
    make_string(ch.to_string())
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let val = evaluate_expression_inner(expression, env, context);
    *result = val;
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
    let mut env = new_env();
    let mut context = init_allocator();
    if let Some(ref program) = result.program {
        let mut current = program.expressions.as_ref();
        while let Some(node) = current {
            if let Some(ref expr) = node.expression {
                let mut obj = make_nil();
                evaluate_expression(expr, &mut obj, &mut env, &mut context);
            }
            current = node.next.as_ref();
        }
    }
}
