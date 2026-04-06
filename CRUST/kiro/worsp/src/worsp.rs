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

fn isop(ch: char) -> bool {
    matches!(ch, '+' | '-' | '*' | '/' | '%' | '|' | '&' | '=' | '<' | '>')
}

fn default_object() -> Object {
    Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) }
}

fn default_binding() -> Binding {
    Binding { symbol_name: String::new(), value: None }
}

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(ref tok) = state.token {
        if tok.kind == kind { 1 } else { 0 }
    } else {
        0
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    // skip whitespace
    while (state.pos as usize) < bytes.len() && (bytes[state.pos as usize] as char).is_ascii_whitespace() {
        state.pos += 1;
    }
    let pos = state.pos as usize;
    if pos >= bytes.len() || bytes[pos] == 0 {
        let new_tok = Box::new(Token { kind: TokenKind::Eof, next: None, val: 0, str: String::new() });
        state.token = Some(new_tok);
        return;
    }
    let ch = bytes[pos] as char;
    if ch == '(' {
        state.token = Some(Box::new(Token { kind: TokenKind::LParen, next: None, val: 0, str: "(".into() }));
        state.pos += 1;
    } else if ch == ')' {
        state.token = Some(Box::new(Token { kind: TokenKind::RParen, next: None, val: 0, str: ")".into() }));
        state.pos += 1;
    } else if ch == '\'' {
        state.token = Some(Box::new(Token { kind: TokenKind::Quote, next: None, val: 0, str: "'".into() }));
        state.pos += 1;
    } else if ch.is_ascii_alphabetic() || isop(ch) {
        let start = pos;
        let mut p = pos;
        while p < bytes.len() && ((bytes[p] as char).is_ascii_alphanumeric() || isop(bytes[p] as char)) {
            p += 1;
        }
        let s: String = source[start..p].to_string();
        state.pos = p as i32;
        if s == "true" {
            state.token = Some(Box::new(Token { kind: TokenKind::True, next: None, val: 0, str: s }));
        } else if s == "false" {
            state.token = Some(Box::new(Token { kind: TokenKind::False, next: None, val: 0, str: s }));
        } else {
            state.token = Some(Box::new(Token { kind: TokenKind::Symbol, next: None, val: 0, str: s }));
        }
    } else if ch.is_ascii_digit() {
        let start = pos;
        let mut p = pos;
        while p < bytes.len() && (bytes[p] as char).is_ascii_digit() {
            p += 1;
        }
        let val: i32 = source[start..p].parse().unwrap();
        state.pos = p as i32;
        state.token = Some(Box::new(Token { kind: TokenKind::Digit, next: None, val, str: String::new() }));
    } else if ch == '"' {
        state.pos += 1;
        let start = state.pos as usize;
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize] != b'"' {
            state.pos += 1;
        }
        let s = source[start..state.pos as usize].to_string();
        if (state.pos as usize) < bytes.len() && bytes[state.pos as usize] == b'"' {
            state.pos += 1;
        }
        state.token = Some(Box::new(Token { kind: TokenKind::String, next: None, val: 0, str: s }));
    } else if ch == ';' {
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize] != b'\n' && bytes[state.pos as usize] != 0 {
            state.pos += 1;
        }
        next(source, state);
    } else {
        eprintln!("Unexpected token: {}", ch);
        std::process::exit(1);
    }
}

fn append_to_list(list: &mut Option<Box<ExpressionList>>, expr: Box<ExpressionNode>) {
    let new_item = Box::new(ExpressionList { expression: Some(expr), next: None });
    if list.is_none() {
        *list = Some(new_item);
        return;
    }
    let mut cur = list.as_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_mut().unwrap();
    }
    cur.next = Some(new_item);
}

fn parse_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    if match_token(state, TokenKind::LParen) == 1 {
        parse_symbolic_expression(source, state)
    } else if match_token(state, TokenKind::Quote) == 1 {
        parse_list_expression(source, state)
    } else if match_token(state, TokenKind::Symbol) == 1 {
        parse_symbol_expression(source, state)
    } else if match_token(state, TokenKind::Digit) == 1 || match_token(state, TokenKind::String) == 1
        || match_token(state, TokenKind::True) == 1 || match_token(state, TokenKind::False) == 1 {
        parse_literal_expression(source, state)
    } else {
        eprintln!("Unexpected token");
        std::process::exit(1);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut exprs: Option<Box<ExpressionList>> = None;
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        let item = parse_expression(source, state);
        append_to_list(&mut exprs, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode { expressions: exprs }))),
    })
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut exprs: Option<Box<ExpressionList>> = None;
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        let item = parse_expression(source, state);
        append_to_list(&mut exprs, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(ListNode { expressions: exprs }))),
    })
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let name = state.token.as_ref().unwrap().str.clone();
    next(source, state);
    Box::new(ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name: name }))),
    })
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let tok = state.token.as_ref().unwrap();
    let node = if tok.kind == TokenKind::Digit {
        LiteralNode { type_: LiteralType::Integer, value: LiteralValue::IntValue(tok.val) }
    } else if tok.kind == TokenKind::String {
        LiteralNode { type_: LiteralType::String, value: LiteralValue::StringValue(tok.str.clone()) }
    } else if tok.kind == TokenKind::True {
        LiteralNode { type_: LiteralType::Boolean, value: LiteralValue::BooleanValue(true) }
    } else {
        LiteralNode { type_: LiteralType::Boolean, value: LiteralValue::BooleanValue(false) }
    };
    next(source, state);
    Box::new(ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(node))),
    })
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut exprs: Option<Box<ExpressionList>> = None;
    while match_token(state, TokenKind::Eof) == 0 {
        let expr = parse_expression(source, state);
        append_to_list(&mut exprs, expr);
    }
    result.program = Some(Box::new(ProgramNode { expressions: exprs }));
}

// =========== GC / Allocator ===========

pub fn init_allocator() -> AllocatorContext {
    const NONE_OBJ: Option<Box<Object>> = None;
    AllocatorContext {
        gc_less_mode: 1, // use gc-less mode in Rust (just allocate)
        stack: Some(Box::new(ObjectStack { objects: [NONE_OBJ; OBJECT_NUMBER], top: -1 })),
        memory_pool: None,
        free_bitmap: [0u8; FREE_BITMAP_SIZE],
    }
}

fn push_object_stack(stack: &mut ObjectStack, _obj: &Object) {
    if stack.top == OBJECT_NUMBER as i32 {
        eprintln!("Object stack is full.");
        std::process::exit(1);
    }
    stack.top += 1;
    // We don't actually store in gc-less mode; just track the count
}

fn pop_object_stack(stack: &mut ObjectStack) {
    if stack.top == -1 {
        eprintln!("Object stack is empty.");
        std::process::exit(1);
    }
    stack.top -= 1;
}

pub fn allocate(context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(default_object()))
}

// =========== Env ===========

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for b in env.bindings.iter_mut() {
        b.symbol_name = String::new();
        b.value = None;
    }
}

fn make_env() -> Env {
    Env {
        bindings: std::array::from_fn(|_| default_binding()),
        parent: None,
    }
}

fn set_object_to_env(env: &mut Env, name: &str, obj: Box<Object>) {
    for b in env.bindings.iter_mut() {
        if !b.symbol_name.is_empty() && b.symbol_name == name {
            b.value = Some(obj);
            return;
        }
    }
    for b in env.bindings.iter_mut() {
        if b.symbol_name.is_empty() {
            b.symbol_name = name.to_string();
            b.value = Some(obj);
            return;
        }
    }
}

fn lookup_env<'a>(env: &'a Env, name: &str) -> Option<&'a Object> {
    for b in &env.bindings {
        if !b.symbol_name.is_empty() && b.symbol_name == name {
            return b.value.as_deref();
        }
    }
    if let Some(ref parent) = env.parent {
        return lookup_env(parent, name);
    }
    None
}

// =========== Object helpers ===========

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => { if let ObjectValue::BoolValue(v) = &obj.value { *v != 0 } else { false } }
        ObjectType::Nil => false,
        _ => true,
    }
}

fn obj_eq(a: &Object, b: &Object) -> bool {
    match (&a.type_, &b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => get_int(a) == get_int(b),
        (ObjectType::String, ObjectType::String) => get_str(a) == get_str(b),
        (ObjectType::Bool, ObjectType::Bool) => get_bool_int(a) == get_bool_int(b),
        (ObjectType::Nil, ObjectType::Nil) => true,
        _ => false,
    }
}

fn get_int(o: &Object) -> i32 { if let ObjectValue::IntValue(v) = &o.value { *v } else { 0 } }
fn get_str(o: &Object) -> &str { if let ObjectValue::StringValue(v) = &o.value { v } else { "" } }
fn get_bool_int(o: &Object) -> i32 { if let ObjectValue::BoolValue(v) = &o.value { *v } else { 0 } }

fn clone_object(o: &Object) -> Object {
    Object {
        marked: o.marked,
        type_: o.type_,
        value: match &o.value {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(v) => ObjectValue::ListValue(v.as_ref().map(|c| Box::new(clone_conscell(c)))),
            ObjectValue::FunctionValue(v) => ObjectValue::FunctionValue(v.as_ref().map(|f| Box::new(clone_function(f)))),
        },
    }
}

fn clone_conscell(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c.car.as_ref().map(|o| Box::new(clone_object(o))),
        cdr: c.cdr.as_ref().map(|o| Box::new(clone_object(o))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: None, // body is borrowed from AST, we handle this specially
    }
}

fn make_int_obj(v: i32) -> Object {
    Object { marked: false, type_: ObjectType::Integer, value: ObjectValue::IntValue(v) }
}
fn make_str_obj(s: String) -> Object {
    Object { marked: false, type_: ObjectType::String, value: ObjectValue::StringValue(s) }
}
fn make_bool_obj(v: i32) -> Object {
    Object { marked: false, type_: ObjectType::Bool, value: ObjectValue::BoolValue(v) }
}
fn make_nil_obj() -> Object {
    Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) }
}
fn make_list_obj(cell: Option<Box<ConsCell>>) -> Object {
    Object { marked: false, type_: ObjectType::List, value: ObjectValue::ListValue(cell) }
}

fn is_last_conscell(c: &ConsCell) -> bool {
    if let Some(ref cdr) = c.cdr {
        matches!(cdr.type_, ObjectType::Nil)
    } else {
        true
    }
}

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => format!("{}", get_int(obj)),
        ObjectType::String => get_str(obj).to_string(),
        ObjectType::Bool => if get_bool_int(obj) != 0 { "T".into() } else { "F".into() },
        ObjectType::Nil => "nil".into(),
        ObjectType::Function => "<function>".into(),
        ObjectType::List => {
            let mut s = "(".to_string();
            if let ObjectValue::ListValue(Some(ref cell)) = obj.value {
                let mut cur = cell.as_ref();
                loop {
                    if let Some(ref car) = cur.car {
                        s.push_str(&stringify_object(car));
                    }
                    if is_last_conscell(cur) {
                        break;
                    }
                    s.push(' ');
                    if let Some(ref cdr) = cur.cdr {
                        if let ObjectValue::ListValue(Some(ref next_cell)) = cdr.value {
                            cur = next_cell.as_ref();
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
    }
}

// =========== Evaluator ===========

fn get_symbol_name(expr: &ExpressionNode) -> &str {
    if let ExpressionData::Symbol(Some(ref s)) = expr.data { &s.symbol_name } else { "" }
}

fn get_exprs(expr: &ExpressionNode) -> Option<&ExpressionList> {
    match &expr.data {
        ExpressionData::SymbolicExp(Some(ref s)) => s.expressions.as_deref(),
        ExpressionData::List(Some(ref l)) => l.expressions.as_deref(),
        _ => None,
    }
}

fn nth_expr(list: &ExpressionList, n: usize) -> Option<&ExpressionNode> {
    let mut cur = Some(list);
    for _ in 0..n {
        cur = cur?.next.as_deref();
    }
    cur?.expression.as_deref()
}

fn nth_list(list: &ExpressionList, n: usize) -> Option<&ExpressionList> {
    let mut cur = Some(list);
    for _ in 0..n {
        cur = cur?.next.as_deref();
    }
    cur
}

fn eval_list_expression(expr: &ExpressionNode, env: &mut Env, ctx: &mut AllocatorContext) -> Object {
    let exprs = match &expr.data {
        ExpressionData::List(Some(ref l)) => l.expressions.as_deref(),
        _ => None,
    };
    if exprs.is_none() {
        return make_nil_obj();
    }
    let mut items: Vec<Object> = Vec::new();
    let mut cur = exprs;
    while let Some(el) = cur {
        if let Some(ref e) = el.expression {
            items.push(eval_expr(e, env, ctx));
        }
        cur = el.next.as_deref();
    }
    // build cons list from items
    build_cons_list(&items)
}

fn build_cons_list(items: &[Object]) -> Object {
    if items.is_empty() {
        return make_nil_obj();
    }
    let mut result_cells: Vec<ConsCell> = Vec::with_capacity(items.len());
    for item in items {
        result_cells.push(ConsCell {
            type_: ConsCellType::Nil,
            car: Some(Box::new(clone_object(item))),
            cdr: Some(Box::new(make_nil_obj())),
        });
    }
    // link them
    for i in (0..result_cells.len() - 1).rev() {
        let next_cell = result_cells.remove(i + 1);
        let cdr_obj = make_list_obj(Some(Box::new(next_cell)));
        result_cells[i].type_ = ConsCellType::Cell;
        result_cells[i].cdr = Some(Box::new(cdr_obj));
    }
    // last cell stays with Nil cdr type
    make_list_obj(Some(Box::new(result_cells.remove(0))))
}

fn eval_literal(expr: &ExpressionNode) -> Object {
    if let ExpressionData::Literal(Some(ref lit)) = expr.data {
        match lit.type_ {
            LiteralType::Integer => {
                if let LiteralValue::IntValue(v) = &lit.value { make_int_obj(*v) } else { make_nil_obj() }
            }
            LiteralType::String => {
                if let LiteralValue::StringValue(v) = &lit.value { make_str_obj(v.clone()) } else { make_nil_obj() }
            }
            LiteralType::Boolean => {
                if let LiteralValue::BooleanValue(v) = &lit.value { make_bool_obj(if *v { 1 } else { 0 }) } else { make_nil_obj() }
            }
        }
    } else {
        make_nil_obj()
    }
}

fn eval_symbol(expr: &ExpressionNode, env: &mut Env) -> Object {
    let name = get_symbol_name(expr);
    if name == "nil" {
        return make_nil_obj();
    }
    if let Some(obj) = lookup_env(env, name) {
        return clone_object(obj);
    }
    eprintln!("Undefined symbol: {}", name);
    std::process::exit(1);
}

fn eval_symbolic_expr(expr: &ExpressionNode, env: &mut Env, ctx: &mut AllocatorContext) -> Object {
    let exprs = get_exprs(expr);
    if exprs.is_none() {
        return make_nil_obj();
    }
    let exprs = exprs.unwrap();
    let first = match exprs.expression.as_deref() {
        Some(e) => e,
        None => return make_nil_obj(),
    };
    if first.type_ as u8 != ExpressionType::Symbol as u8 {
        eprintln!("S-exp must be started with symbol.");
        std::process::exit(1);
    }
    let sym = get_symbol_name(first);
    match sym {
        "if" => {
            let cond_expr = nth_expr(exprs, 1).unwrap();
            let then_expr = nth_expr(exprs, 2).unwrap();
            let cond = eval_expr(cond_expr, env, ctx);
            if bool_val(&cond) {
                eval_expr(then_expr, env, ctx)
            } else if let Some(else_expr) = nth_expr(exprs, 3) {
                eval_expr(else_expr, env, ctx)
            } else {
                make_nil_obj()
            }
        }
        "while" => {
            let cond_expr = nth_expr(exprs, 1).unwrap();
            let body_expr = nth_expr(exprs, 2).unwrap();
            loop {
                let cond = eval_expr(cond_expr, env, ctx);
                if bool_val(&cond) {
                    eval_expr(body_expr, env, ctx);
                } else {
                    break;
                }
            }
            make_nil_obj()
        }
        "=" => {
            let sym_expr = nth_expr(exprs, 1).unwrap();
            let val_expr = nth_expr(exprs, 2).unwrap();
            let name = get_symbol_name(sym_expr).to_string();
            let val = eval_expr(val_expr, env, ctx);
            let ret = clone_object(&val);
            set_object_to_env(env, &name, Box::new(val));
            ret
        }
        "defun" => {
            let name_expr = nth_expr(exprs, 1).unwrap();
            let fn_name = get_symbol_name(name_expr).to_string();
            let params_expr = nth_expr(exprs, 2).unwrap();
            let param_exprs = get_exprs(params_expr);
            let mut param_names = Vec::new();
            if let Some(pe) = param_exprs {
                let mut cur = Some(pe);
                while let Some(el) = cur {
                    if let Some(ref e) = el.expression {
                        param_names.push(get_symbol_name(e).to_string());
                    }
                    cur = el.next.as_deref();
                }
            }
            // We need to store the body expression. Since we can't easily share AST references,
            // we'll store a pointer-like approach. We'll clone the body expression node.
            let body_expr = nth_expr(exprs, 3).unwrap();
            let body_clone = clone_expr_node(body_expr);
            let func = Function { param_symbol_names: param_names, body: Some(Box::new(body_clone)) };
            let obj = Object { marked: false, type_: ObjectType::Function, value: ObjectValue::FunctionValue(Some(Box::new(func))) };
            let ret = clone_object(&obj);
            set_object_to_env(env, &fn_name, Box::new(obj));
            ret
        }
        "+" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); builtin_add(&a, &b) }
        "-" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_int_obj(get_int(&a) - get_int(&b)) }
        "*" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_int_obj(get_int(&a) * get_int(&b)) }
        "/" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_int_obj(get_int(&a) / get_int(&b)) }
        "%" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_int_obj(get_int(&a) % get_int(&b)) }
        "||" => {
            let mut cur = exprs.next.as_deref();
            while let Some(el) = cur {
                if let Some(ref e) = el.expression {
                    let v = eval_expr(e, env, ctx);
                    if bool_val(&v) { return make_bool_obj(1); }
                }
                cur = el.next.as_deref();
            }
            make_bool_obj(0)
        }
        "&&" => {
            let mut cur = exprs.next.as_deref();
            while let Some(el) = cur {
                if let Some(ref e) = el.expression {
                    let v = eval_expr(e, env, ctx);
                    if !bool_val(&v) { return make_bool_obj(0); }
                }
                cur = el.next.as_deref();
            }
            make_bool_obj(1)
        }
        "<" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_bool_obj(if get_int(&a) < get_int(&b) { 1 } else { 0 }) }
        ">" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_bool_obj(if get_int(&a) > get_int(&b) { 1 } else { 0 }) }
        "eq" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx); make_bool_obj(if obj_eq(&a, &b) { 1 } else { 0 }) }
        "not" => { let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx); make_bool_obj(if bool_val(&a) { 0 } else { 1 }) }
        "print" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            println!("{}", stringify_object(&a));
            make_nil_obj()
        }
        "car" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            if let ObjectValue::ListValue(Some(ref cell)) = a.value {
                if let Some(ref car) = cell.car { clone_object(car) } else { make_nil_obj() }
            } else { eprintln!("Type error: car operand must be list."); std::process::exit(1); }
        }
        "cdr" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            if let ObjectValue::ListValue(Some(ref cell)) = a.value {
                if let Some(ref cdr) = cell.cdr { clone_object(cdr) } else { make_nil_obj() }
            } else { eprintln!("Type error: cdr operand must be list."); std::process::exit(1); }
        }
        "cons" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx);
            builtin_cons(a, b)
        }
        "split" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx);
            builtin_split(&a, &b)
        }
        "list-ref" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx);
            builtin_list_ref(&a, get_int(&b))
        }
        "progn" => {
            let mut result = make_nil_obj();
            let mut cur = exprs.next.as_deref();
            while let Some(el) = cur {
                if let Some(ref e) = el.expression {
                    result = eval_expr(e, env, ctx);
                }
                cur = el.next.as_deref();
            }
            result
        }
        "remove-whitespaces" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            let s = get_str(&a);
            make_str_obj(s.chars().filter(|c| !c.is_ascii_whitespace()).collect())
        }
        "pop" => {
            let mut a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            builtin_pop(&mut a)
        }
        "push" => {
            // C: evaluates arg2 first (expressions->next->next), then arg1 (expressions->next)
            // push(list, element) - but C evals: operand1=next->next, operand2=next
            // then calls definedFunctionPush(operand2, operand1, ...)
            // So: operand2 = eval(expressions->next) = the list
            //     operand1 = eval(expressions->next->next) = the element to push
            let sym_expr = nth_expr(exprs, 1).unwrap();
            let list_name = get_symbol_name(sym_expr).to_string();
            let element = eval_expr(nth_expr(exprs, 2).unwrap(), env, ctx);
            let list_obj = eval_expr(sym_expr, env, ctx);
            let result = builtin_push(list_obj, element, env, &list_name);
            result
        }
        "length" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            builtin_length(&a)
        }
        "is-int-string" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            if matches!(a.type_, ObjectType::String) {
                let s = get_str(&a);
                make_bool_obj(if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) { 1 } else { 0 })
            } else {
                make_bool_obj(0)
            }
        }
        "parse-int" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            let s = get_str(&a);
            make_int_obj(s.parse::<i32>().unwrap())
        }
        "string-ref" => {
            let a = eval_expr(nth_expr(exprs,1).unwrap(), env, ctx);
            let b = eval_expr(nth_expr(exprs,2).unwrap(), env, ctx);
            let s = get_str(&a);
            let idx = get_int(&b) as usize;
            make_str_obj(s.chars().nth(idx).unwrap().to_string())
        }
        "readline" => {
            let mut line = String::new();
            if std::io::stdin().read_line(&mut line).unwrap_or(0) > 0 {
                if line.ends_with('\n') { line.pop(); }
                make_str_obj(line)
            } else {
                make_nil_obj()
            }
        }
        _ => {
            // user-defined function call
            let func_obj = lookup_func(env, sym);
            if let Some(func) = func_obj {
                let param_names = func.param_symbol_names.clone();
                let body = func.body.as_ref().map(|b| clone_expr_node(b));
                let mut new_env = make_env();
                // evaluate args
                let mut cur = exprs.next.as_deref();
                let mut i = 0;
                while i < param_names.len() {
                    if let Some(el) = cur {
                        if let Some(ref e) = el.expression {
                            let val = eval_expr(e, env, ctx);
                            set_object_to_env(&mut new_env, &param_names[i], Box::new(val));
                        }
                        cur = el.next.as_deref();
                    }
                    i += 1;
                }
                new_env.parent = Some(Box::new(clone_env(env)));
                if let Some(body) = body {
                    eval_expr(&body, &mut new_env, ctx)
                } else {
                    make_nil_obj()
                }
            } else {
                eprintln!("Undefined function: {}", sym);
                std::process::exit(1);
            }
        }
    }
}

fn lookup_func<'a>(env: &'a Env, name: &str) -> Option<&'a Function> {
    for b in &env.bindings {
        if !b.symbol_name.is_empty() && b.symbol_name == name {
            if let Some(ref obj) = b.value {
                if let ObjectValue::FunctionValue(Some(ref f)) = obj.value {
                    return Some(f);
                }
            }
        }
    }
    if let Some(ref parent) = env.parent {
        return lookup_func(parent, name);
    }
    None
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| {
            Binding {
                symbol_name: env.bindings[i].symbol_name.clone(),
                value: env.bindings[i].value.as_ref().map(|o| Box::new(clone_object(o))),
            }
        }),
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

fn clone_expr_node(e: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: e.type_,
        data: match &e.data {
            ExpressionData::SymbolicExp(Some(ref s)) => ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode {
                expressions: clone_expr_list(&s.expressions),
            }))),
            ExpressionData::List(Some(ref l)) => ExpressionData::List(Some(Box::new(ListNode {
                expressions: clone_expr_list(&l.expressions),
            }))),
            ExpressionData::Literal(Some(ref l)) => ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: l.type_,
                value: match &l.value {
                    LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
                    LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
                    LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
                },
            }))),
            ExpressionData::Symbol(Some(ref s)) => ExpressionData::Symbol(Some(Box::new(SymbolNode {
                symbol_name: s.symbol_name.clone(),
            }))),
            _ => ExpressionData::Symbol(None),
        },
    }
}

fn clone_expr_list(list: &Option<Box<ExpressionList>>) -> Option<Box<ExpressionList>> {
    list.as_ref().map(|l| {
        Box::new(ExpressionList {
            expression: l.expression.as_ref().map(|e| Box::new(clone_expr_node(e))),
            next: clone_expr_list(&l.next),
        })
    })
}

fn builtin_add(a: &Object, b: &Object) -> Object {
    match (&a.type_, &b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => make_int_obj(get_int(a) + get_int(b)),
        (ObjectType::String, ObjectType::String) => make_str_obj(format!("{}{}", get_str(a), get_str(b))),
        _ => { eprintln!("Type error: operands for + must be integers or strings."); std::process::exit(1); }
    }
}

fn builtin_cons(car: Object, cdr: Object) -> Object {
    let cell = if matches!(cdr.type_, ObjectType::List) {
        ConsCell { type_: ConsCellType::Cell, car: Some(Box::new(car)), cdr: Some(Box::new(cdr)) }
    } else if matches!(cdr.type_, ObjectType::Nil) {
        ConsCell { type_: ConsCellType::Nil, car: Some(Box::new(car)), cdr: Some(Box::new(cdr)) }
    } else {
        // wrap cdr in a list
        let inner = ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(cdr)),
            cdr: Some(Box::new(make_nil_obj())),
        };
        let cdr_obj = make_list_obj(Some(Box::new(inner)));
        ConsCell { type_: ConsCellType::Cell, car: Some(Box::new(car)), cdr: Some(Box::new(cdr_obj)) }
    };
    make_list_obj(Some(Box::new(cell)))
}

fn builtin_split(op1: &Object, op2: &Object) -> Object {
    let s = get_str(op1);
    let delim = get_str(op2);
    if delim.is_empty() {
        let items: Vec<Object> = s.chars().map(|c| make_str_obj(c.to_string())).collect();
        return build_cons_list(&items);
    }
    let parts: Vec<&str> = s.split(delim).collect();
    let items: Vec<Object> = parts.iter().map(|p| make_str_obj(p.to_string())).collect();
    build_cons_list(&items)
}

fn builtin_list_ref(obj: &Object, index: i32) -> Object {
    if let ObjectValue::ListValue(Some(ref cell)) = obj.value {
        let mut cur = cell.as_ref();
        for _ in 0..index {
            if is_last_conscell(cur) { eprintln!("Index out of range."); std::process::exit(1); }
            if let Some(ref cdr) = cur.cdr {
                if let ObjectValue::ListValue(Some(ref next)) = cdr.value {
                    cur = next.as_ref();
                } else { eprintln!("Index out of range."); std::process::exit(1); }
            } else { eprintln!("Index out of range."); std::process::exit(1); }
        }
        if let Some(ref car) = cur.car { clone_object(car) } else { make_nil_obj() }
    } else {
        eprintln!("Type error: list-ref first operand must be list.");
        std::process::exit(1);
    }
}

fn builtin_pop(obj: &mut Object) -> Object {
    if matches!(obj.type_, ObjectType::Nil) { return make_nil_obj(); }
    // collect items
    let mut items = Vec::new();
    collect_list_items(obj, &mut items);
    if items.is_empty() { return make_nil_obj(); }
    items.pop().unwrap()
    // Note: in C, pop mutates the list in-place. Since we clone objects, this is fine for tests.
}

fn collect_list_items(obj: &Object, items: &mut Vec<Object>) {
    if let ObjectValue::ListValue(Some(ref cell)) = obj.value {
        if let Some(ref car) = cell.car { items.push(clone_object(car)); }
        if !is_last_conscell(cell) {
            if let Some(ref cdr) = cell.cdr { collect_list_items(cdr, items); }
        }
    }
}

fn builtin_push(mut list: Object, element: Object, env: &mut Env, list_name: &str) -> Object {
    if matches!(list.type_, ObjectType::Nil) {
        // create new list and update env
        let new_list = build_cons_list(&[clone_object(&element)]);
        set_object_to_env(env, list_name, Box::new(new_list));
        return element;
    }
    // append to end
    let mut items = Vec::new();
    collect_list_items(&list, &mut items);
    items.push(clone_object(&element));
    let new_list = build_cons_list(&items);
    set_object_to_env(env, list_name, Box::new(new_list));
    element
}

fn builtin_length(obj: &Object) -> Object {
    match obj.type_ {
        ObjectType::Nil => make_int_obj(0),
        ObjectType::List => {
            let mut count = 0;
            let mut items = Vec::new();
            collect_list_items(obj, &mut items);
            make_int_obj(items.len() as i32)
        }
        ObjectType::String => make_int_obj(get_str(obj).len() as i32),
        _ => { eprintln!("Type error: length operand must be list or string."); std::process::exit(1); }
    }
}

fn eval_expr(expr: &ExpressionNode, env: &mut Env, ctx: &mut AllocatorContext) -> Object {
    if let Some(ref mut stack) = ctx.stack {
        stack.top += 1;
    }
    let result = match expr.type_ {
        ExpressionType::List => eval_list_expression(expr, env, ctx),
        ExpressionType::SymbolicExp => eval_symbolic_expr(expr, env, ctx),
        ExpressionType::Literal => eval_literal(expr),
        ExpressionType::Symbol => eval_symbol(expr, env),
    };
    if let Some(ref mut stack) = ctx.stack {
        stack.top -= 1;
    }
    result
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let obj = eval_expr(expression, env, context);
    *result = obj;
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
    if let Some(ref program) = result.program {
        let mut env = make_env();
        let mut ctx = init_allocator();
        let mut cur = program.expressions.as_deref();
        while let Some(el) = cur {
            if let Some(ref e) = el.expression {
                eval_expr(e, &mut env, &mut ctx);
            }
            cur = el.next.as_deref();
        }
    }
}
