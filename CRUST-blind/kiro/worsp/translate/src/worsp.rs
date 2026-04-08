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
fn isop(ch: u8) -> bool {
    matches!(ch, b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>')
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
    let ch = if pos < bytes.len() { bytes[pos] } else { 0 };

    let new_tok = if ch == b'(' {
        state.pos += 1;
        Token { kind: TokenKind::LParen, next: None, val: 0, str: "(".into() }
    } else if ch == b')' {
        state.pos += 1;
        Token { kind: TokenKind::RParen, next: None, val: 0, str: ")".into() }
    } else if ch == b'\'' {
        state.pos += 1;
        Token { kind: TokenKind::Quote, next: None, val: 0, str: "'".into() }
    } else if ch == 0 {
        Token { kind: TokenKind::Eof, next: None, val: 0, str: String::new() }
    } else if ch.is_ascii_alphabetic() || isop(ch) {
        let start = pos;
        while (state.pos as usize) < bytes.len() && (bytes[state.pos as usize].is_ascii_alphanumeric() || isop(bytes[state.pos as usize])) {
            state.pos += 1;
        }
        let s: String = source[start..state.pos as usize].into();
        if s == "true" {
            Token { kind: TokenKind::True, next: None, val: 0, str: s }
        } else if s == "false" {
            Token { kind: TokenKind::False, next: None, val: 0, str: s }
        } else {
            Token { kind: TokenKind::Symbol, next: None, val: 0, str: s }
        }
    } else if ch.is_ascii_digit() {
        let start = pos;
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize].is_ascii_digit() {
            state.pos += 1;
        }
        let s: String = source[start..state.pos as usize].into();
        let val = s.parse::<i32>().unwrap_or(0);
        Token { kind: TokenKind::Digit, next: None, val, str: s }
    } else if ch == b'"' {
        state.pos += 1;
        let start = state.pos as usize;
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize] != b'"' {
            state.pos += 1;
        }
        let s: String = source[start..state.pos as usize].into();
        if (state.pos as usize) < bytes.len() && bytes[state.pos as usize] == b'"' {
            state.pos += 1;
        }
        Token { kind: TokenKind::String, next: None, val: 0, str: s }
    } else if ch == b';' {
        while (state.pos as usize) < bytes.len() && bytes[state.pos as usize] != b'\n' {
            state.pos += 1;
        }
        return next(source, state);
    } else {
        eprintln!("Unexpected token: {}", ch as char);
        std::process::exit(1);
    };
    state.token = Some(Box::new(new_tok));
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
        let s = state.token.as_ref().map(|t| t.str.clone()).unwrap_or_default();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    next(source, state); // eat '('
    let mut exprs: Option<Box<ExpressionList>> = None;
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_to_expr_list(&mut exprs, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode { expressions: exprs }))),
    })
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    next(source, state); // eat quote
    next(source, state); // eat '('
    let mut exprs: Option<Box<ExpressionList>> = None;
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_to_expr_list(&mut exprs, item);
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
    let lit = if tok.kind == TokenKind::Digit {
        let v = tok.val;
        next(source, state);
        LiteralNode { type_: LiteralType::Integer, value: LiteralValue::IntValue(v) }
    } else if tok.kind == TokenKind::String {
        let s = tok.str.clone();
        next(source, state);
        LiteralNode { type_: LiteralType::String, value: LiteralValue::StringValue(s) }
    } else if tok.kind == TokenKind::True {
        next(source, state);
        LiteralNode { type_: LiteralType::Boolean, value: LiteralValue::BooleanValue(true) }
    } else if tok.kind == TokenKind::False {
        next(source, state);
        LiteralNode { type_: LiteralType::Boolean, value: LiteralValue::BooleanValue(false) }
    } else {
        let s = tok.str.clone();
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    };
    Box::new(ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(lit))),
    })
}

fn append_to_expr_list(list: &mut Option<Box<ExpressionList>>, expr: Box<ExpressionNode>) {
    let new_node = Box::new(ExpressionList { expression: Some(expr), next: None });
    match list {
        None => *list = Some(new_node),
        Some(ref mut head) => {
            let mut cur = head;
            while cur.next.is_some() {
                cur = cur.next.as_mut().unwrap();
            }
            cur.next = Some(new_node);
        }
    }
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut exprs: Option<Box<ExpressionList>> = None;
    while match_token(state, TokenKind::Eof) != 1 {
        let expr = parse_expression(source, state);
        append_to_expr_list(&mut exprs, expr);
    }
    result.program = Some(Box::new(ProgramNode { expressions: exprs }));
}
fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => { if let ObjectValue::BoolValue(v) = &obj.value { *v != 0 } else { false } }
        ObjectType::Nil => false,
        _ => true,
    }
}

fn eq_obj(a: &Object, b: &Object) -> bool {
    if std::mem::discriminant(&a.type_) != std::mem::discriminant(&b.type_) {
        // different ObjectType variants
        return false;
    }
    match (&a.type_, &a.value, &b.value) {
        (ObjectType::Integer, ObjectValue::IntValue(x), ObjectValue::IntValue(y)) => x == y,
        (ObjectType::String, ObjectValue::StringValue(x), ObjectValue::StringValue(y)) => x == y,
        (ObjectType::Bool, ObjectValue::BoolValue(x), ObjectValue::BoolValue(y)) => x == y,
        (ObjectType::Nil, _, _) => true,
        _ => false,
    }
}

fn new_nil_object() -> Object {
    Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) }
}

fn new_int_object(v: i32) -> Object {
    Object { marked: false, type_: ObjectType::Integer, value: ObjectValue::IntValue(v) }
}

fn new_bool_object(v: i32) -> Object {
    Object { marked: false, type_: ObjectType::Bool, value: ObjectValue::BoolValue(v) }
}

fn new_string_object(s: String) -> Object {
    Object { marked: false, type_: ObjectType::String, value: ObjectValue::StringValue(s) }
}

pub fn evaluate_expression(
expression: &ExpressionNode,
result: &mut Object,
env: &mut Env,
context: &mut AllocatorContext,
) {
    // push/pop on stack (simplified - we don't have real pool pointers, so skip GC stack ops)
    match expression.type_ {
        ExpressionType::List => evaluate_list_expression(expression, result, env, context),
        ExpressionType::SymbolicExp => evaluate_symbolic_expression(expression, result, env, context),
        ExpressionType::Literal => evaluate_literal_expression(expression, result),
        ExpressionType::Symbol => evaluate_symbol_expression(expression, result, env, context),
    }
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
        let mut env = Env {
            bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
            parent: None,
        };
        init_env(&mut env);
        let mut context = init_allocator();
        let mut exprs = &program.expressions;
        while let Some(ref el) = exprs {
            let mut evaluated = Box::new(new_nil_object());
            if let Some(ref expr) = el.expression {
                evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
            }
            exprs = &el.next;
        }
    }
}
pub fn stringify_object(obj: &Object) -> String {
    match (&obj.type_, &obj.value) {
        (ObjectType::Integer, ObjectValue::IntValue(v)) => format!("{}", v),
        (ObjectType::String, ObjectValue::StringValue(s)) => s.clone(),
        (ObjectType::Bool, ObjectValue::BoolValue(v)) => if *v != 0 { "T".into() } else { "F".into() },
        (ObjectType::List, ObjectValue::ListValue(Some(cell))) => {
            let mut s = "(".to_string();
            let mut cur = cell.as_ref();
            loop {
                s.push_str(&stringify_object(cur.car.as_ref().unwrap()));
                if is_last_cons_cell(cur) {
                    break;
                }
                s.push(' ');
                let cdr = cur.cdr.as_ref().unwrap();
                if let ObjectValue::ListValue(Some(ref next_cell)) = cdr.value {
                    cur = next_cell.as_ref();
                } else {
                    break;
                }
            }
            s.push(')');
            s
        }
        (ObjectType::Nil, _) => "nil".into(),
        (ObjectType::Function, _) => "<function>".into(),
        _ => {
            eprintln!("Unexpected object type");
            std::process::exit(1);
        }
    }
}
pub fn init_env(env: &mut Env) {
    env.parent = None;
    env.bindings = std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None });
}
pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 1, // always gc-less mode in Rust (no raw pointer pool)
        stack: Some(Box::new(ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0; FREE_BITMAP_SIZE],
    }
}

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(new_nil_object()))
}

fn is_last_cons_cell(cell: &ConsCell) -> bool {
    if let Some(ref cdr) = cell.cdr {
        matches!(cdr.type_, ObjectType::Nil)
    } else {
        true
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

fn evaluate_literal_expression(expression: &ExpressionNode, result: &mut Object) {
    if let ExpressionData::Literal(Some(ref lit)) = expression.data {
        match (&lit.type_, &lit.value) {
            (LiteralType::Integer, LiteralValue::IntValue(v)) => {
                result.type_ = ObjectType::Integer;
                result.value = ObjectValue::IntValue(*v);
            }
            (LiteralType::String, LiteralValue::StringValue(s)) => {
                result.type_ = ObjectType::String;
                result.value = ObjectValue::StringValue(s.clone());
            }
            (LiteralType::Boolean, LiteralValue::BooleanValue(b)) => {
                result.type_ = ObjectType::Bool;
                result.value = ObjectValue::BoolValue(if *b { 1 } else { 0 });
            }
            _ => {}
        }
    }
}

fn evaluate_symbol_expression(expression: &ExpressionNode, result: &mut Object, env: &mut Env, context: &mut AllocatorContext) {
    if let ExpressionData::Symbol(Some(ref sym)) = expression.data {
        if sym.symbol_name == "nil" {
            result.type_ = ObjectType::Nil;
            result.value = ObjectValue::IntValue(0);
            return;
        }
        for b in env.bindings.iter() {
            if !b.symbol_name.is_empty() && b.symbol_name == sym.symbol_name {
                if let Some(ref val) = b.value {
                    clone_object(val, result);
                }
                return;
            }
        }
        if let Some(ref mut parent) = env.parent {
            evaluate_symbol_expression(expression, result, parent, context);
        } else {
            eprintln!("Undefined symbol: {}", sym.symbol_name);
            std::process::exit(1);
        }
    }
}

fn clone_object(src: &Object, dst: &mut Object) {
    dst.marked = src.marked;
    dst.type_ = src.type_;
    dst.value = clone_object_value(&src.value);
}

fn clone_object_value(v: &ObjectValue) -> ObjectValue {
    match v {
        ObjectValue::IntValue(i) => ObjectValue::IntValue(*i),
        ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
        ObjectValue::BoolValue(b) => ObjectValue::BoolValue(*b),
        ObjectValue::ListValue(opt) => ObjectValue::ListValue(opt.as_ref().map(|c| Box::new(clone_cons_cell(c)))),
        ObjectValue::FunctionValue(opt) => ObjectValue::FunctionValue(opt.as_ref().map(|f| Box::new(Function {
            param_symbol_names: f.param_symbol_names.clone(),
            body: clone_expr_node_opt(&f.body),
        }))),
    }
}

fn clone_cons_cell(c: &ConsCell) -> ConsCell {
    ConsCell {
        type_: c.type_,
        car: c.car.as_ref().map(|o| Box::new(deep_clone_object(o))),
        cdr: c.cdr.as_ref().map(|o| Box::new(deep_clone_object(o))),
    }
}

fn deep_clone_object(o: &Object) -> Object {
    Object {
        marked: o.marked,
        type_: o.type_,
        value: clone_object_value(&o.value),
    }
}

fn clone_expr_node_opt(e: &Option<Box<ExpressionNode>>) -> Option<Box<ExpressionNode>> {
    e.as_ref().map(|n| Box::new(clone_expr_node(n)))
}

fn clone_expr_node(n: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: n.type_,
        data: match &n.data {
            ExpressionData::SymbolicExp(opt) => ExpressionData::SymbolicExp(opt.as_ref().map(|s| Box::new(SymbolicExpNode {
                expressions: clone_expr_list_opt(&s.expressions),
            }))),
            ExpressionData::List(opt) => ExpressionData::List(opt.as_ref().map(|l| Box::new(ListNode {
                expressions: clone_expr_list_opt(&l.expressions),
            }))),
            ExpressionData::Literal(opt) => ExpressionData::Literal(opt.as_ref().map(|l| Box::new(LiteralNode {
                type_: l.type_,
                value: match &l.value {
                    LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
                    LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
                    LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
                },
            }))),
            ExpressionData::Symbol(opt) => ExpressionData::Symbol(opt.as_ref().map(|s| Box::new(SymbolNode {
                symbol_name: s.symbol_name.clone(),
            }))),
        },
    }
}

fn clone_expr_list_opt(el: &Option<Box<ExpressionList>>) -> Option<Box<ExpressionList>> {
    el.as_ref().map(|e| Box::new(ExpressionList {
        expression: clone_expr_node_opt(&e.expression),
        next: clone_expr_list_opt(&e.next),
    }))
}

fn evaluate_list_expression(expression: &ExpressionNode, result: &mut Object, env: &mut Env, context: &mut AllocatorContext) {
    let exprs = match &expression.data {
        ExpressionData::List(Some(ref l)) => &l.expressions,
        _ => { result.type_ = ObjectType::Nil; return; }
    };
    if exprs.is_none() {
        result.type_ = ObjectType::Nil;
        return;
    }
    result.type_ = ObjectType::List;
    let mut cells: Vec<Box<ConsCell>> = Vec::new();
    let mut cur = exprs;
    while let Some(ref el) = cur {
        let mut cell = Box::new(ConsCell { type_: ConsCellType::Cell, car: None, cdr: None });
        let mut item = Box::new(new_nil_object());
        if let Some(ref expr) = el.expression {
            evaluate_expression(expr, &mut item, env, context);
        }
        cell.car = Some(item);
        cells.push(cell);
        cur = &el.next;
    }
    // link cells: last cell gets nil cdr
    let n = cells.len();
    for i in (0..n).rev() {
        if i == n - 1 {
            let nil_obj = Box::new(new_nil_object());
            cells[i].type_ = ConsCellType::Nil;
            cells[i].cdr = Some(nil_obj);
        } else {
            let next_cell = cells.remove(i + 1);
            let mut cdr_obj = Box::new(new_nil_object());
            cdr_obj.type_ = ObjectType::List;
            cdr_obj.value = ObjectValue::ListValue(Some(next_cell));
            cells[i].type_ = ConsCellType::Cell;
            cells[i].cdr = Some(cdr_obj);
        }
    }
    if !cells.is_empty() {
        result.value = ObjectValue::ListValue(Some(cells.remove(0)));
    }
}

fn get_sym_name(expr: &ExpressionNode) -> Option<&str> {
    if let ExpressionData::Symbol(Some(ref s)) = expr.data {
        Some(&s.symbol_name)
    } else {
        None
    }
}

fn nth_expr(list: &Option<Box<ExpressionList>>, n: usize) -> Option<&ExpressionNode> {
    let mut cur = list;
    let mut i = 0;
    while let Some(ref el) = cur {
        if i == n {
            return el.expression.as_deref();
        }
        i += 1;
        cur = &el.next;
    }
    None
}

fn eval_two_operands(exprs: &Option<Box<ExpressionList>>, env: &mut Env, context: &mut AllocatorContext) -> (Box<Object>, Box<Object>) {
    let mut op1 = Box::new(new_nil_object());
    let mut op2 = Box::new(new_nil_object());
    if let Some(e) = nth_expr(exprs, 1) {
        evaluate_expression(e, &mut op1, env, context);
    }
    if let Some(e) = nth_expr(exprs, 2) {
        evaluate_expression(e, &mut op2, env, context);
    }
    (op1, op2)
}

fn eval_one_operand(exprs: &Option<Box<ExpressionList>>, env: &mut Env, context: &mut AllocatorContext) -> Box<Object> {
    let mut op = Box::new(new_nil_object());
    if let Some(e) = nth_expr(exprs, 1) {
        evaluate_expression(e, &mut op, env, context);
    }
    op
}

fn evaluate_symbolic_expression(expression: &ExpressionNode, result: &mut Object, env: &mut Env, context: &mut AllocatorContext) {
    let exprs_ref = match &expression.data {
        ExpressionData::SymbolicExp(Some(ref s)) => &s.expressions,
        // C code uses expression->data.list->expressions for symbolic too (union)
        ExpressionData::List(Some(ref l)) => &l.expressions,
        _ => {
            result.type_ = ObjectType::Nil;
            return;
        }
    };
    if exprs_ref.is_none() {
        result.type_ = ObjectType::Nil;
        return;
    }
    let first_expr = nth_expr(exprs_ref, 0);
    if first_expr.is_none() {
        result.type_ = ObjectType::Nil;
        return;
    }
    let first = first_expr.unwrap();
    if first.type_ as u8 != ExpressionType::Symbol as u8 {
        eprintln!("S-exp must be started with symbol.");
        std::process::exit(1);
    }
    let sym_name = get_sym_name(first).unwrap().to_string();

    // We need to clone exprs_ref for use in closures that borrow env mutably
    let exprs_cloned = clone_expr_list_opt(exprs_ref);

    match sym_name.as_str() {
        "if" => {
            let cond_expr = nth_expr(&exprs_cloned, 1).expect("if must have condition.");
            let then_expr = nth_expr(&exprs_cloned, 2).expect("if must have then clause.");
            let mut cond_obj = Box::new(new_nil_object());
            evaluate_expression(cond_expr, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(then_expr, result, env, context);
            } else if let Some(els) = nth_expr(&exprs_cloned, 3) {
                evaluate_expression(els, result, env, context);
            } else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            let cond_expr_owned = clone_expr_node(nth_expr(&exprs_cloned, 1).expect("if must have condition."));
            let then_expr_owned = clone_expr_node(nth_expr(&exprs_cloned, 2).expect("if must have then clause."));
            loop {
                let mut cond_obj = Box::new(new_nil_object());
                evaluate_expression(&cond_expr_owned, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(&then_expr_owned, result, env, context);
                } else {
                    result.type_ = ObjectType::Nil;
                    result.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            let sym_expr = nth_expr(&exprs_cloned, 1).expect("assignment must have symbol.");
            let sname = get_sym_name(sym_expr).expect("Variable name must be symbol.").to_string();
            let val_expr = nth_expr(&exprs_cloned, 2).expect("assignment must have expression.");
            let mut evaluated = Box::new(new_nil_object());
            evaluate_expression(val_expr, &mut evaluated, env, context);
            clone_object(&evaluated, result);
            set_object_to_env(env, &sname, evaluated);
        }
        "defun" => {
            let sym_expr = nth_expr(&exprs_cloned, 1).expect("Function name must be symbol.");
            let sname = get_sym_name(sym_expr).expect("Function name must be symbol.").to_string();
            let params_expr = nth_expr(&exprs_cloned, 2).expect("Function must have parameter.");
            let param_exprs = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(ref s)) => &s.expressions,
                _ => { eprintln!("Function parameter must be list."); std::process::exit(1); }
            };
            let mut param_names = Vec::new();
            let mut cur = param_exprs;
            while let Some(ref el) = cur {
                if let Some(ref e) = el.expression {
                    let pn = get_sym_name(e).expect("Function parameter must be symbol.");
                    param_names.push(pn.to_string());
                }
                cur = &el.next;
            }
            let body_expr = nth_expr(&exprs_cloned, 3).expect("Function must have body.");
            let body_owned = clone_expr_node(body_expr);
            result.type_ = ObjectType::Function;
            result.value = ObjectValue::FunctionValue(Some(Box::new(Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body_owned)),
            })));
            let result_clone = Box::new(deep_clone_object(result));
            set_object_to_env(env, &sname, result_clone);
        }
        "+" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_add(&op1, &op2, result);
        }
        "-" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_sub(&op1, &op2, result);
        }
        "*" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_mul(&op1, &op2, result);
        }
        "/" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_div(&op1, &op2, result);
        }
        "%" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_mod(&op1, &op2, result);
        }
        "||" => {
            let mut cur = &exprs_cloned;
            // skip first (the symbol itself)
            if let Some(ref el) = cur { cur = &el.next; }
            while let Some(ref el) = cur {
                if let Some(ref e) = el.expression {
                    let mut operand = Box::new(new_nil_object());
                    evaluate_expression(e, &mut operand, env, context);
                    if bool_val(&operand) {
                        result.type_ = ObjectType::Bool;
                        result.value = ObjectValue::BoolValue(1);
                        return;
                    }
                }
                cur = &el.next;
            }
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(0);
        }
        "&&" => {
            let mut cur = &exprs_cloned;
            if let Some(ref el) = cur { cur = &el.next; }
            while let Some(ref el) = cur {
                if let Some(ref e) = el.expression {
                    let mut operand = Box::new(new_nil_object());
                    evaluate_expression(e, &mut operand, env, context);
                    if !bool_val(&operand) {
                        result.type_ = ObjectType::Bool;
                        result.value = ObjectValue::BoolValue(0);
                        return;
                    }
                }
                cur = &el.next;
            }
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(1);
        }
        "<" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_lt(&op1, &op2, result);
        }
        ">" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_gt(&op1, &op2, result);
        }
        "eq" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(if eq_obj(&op1, &op2) { 1 } else { 0 });
        }
        "not" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_not(&op, result);
        }
        "print" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            let s = stringify_object(&op);
            println!("{}", s);
            result.type_ = ObjectType::Nil;
            result.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_car(&op, result);
        }
        "cdr" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_cdr(&op, result);
        }
        "cons" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_cons(op1, op2, result);
        }
        "readline" => {
            let mut line = String::new();
            if std::io::stdin().read_line(&mut line).unwrap_or(0) > 0 {
                if line.ends_with('\n') { line.pop(); }
                result.type_ = ObjectType::String;
                result.value = ObjectValue::StringValue(line);
            } else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            }
        }
        "split" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_split(&op1, &op2, result);
        }
        "list-ref" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_list_ref(&op1, &op2, result);
        }
        "progn" => {
            let mut cur = &exprs_cloned;
            if let Some(ref el) = cur { cur = &el.next; }
            let mut last: Option<Box<Object>> = None;
            while let Some(ref el) = cur {
                if let Some(ref e) = el.expression {
                    let mut operand = Box::new(new_nil_object());
                    evaluate_expression(e, &mut operand, env, context);
                    last = Some(operand);
                }
                cur = &el.next;
            }
            if let Some(obj) = last {
                clone_object(&obj, result);
            } else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            }
        }
        "remove-whitespaces" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_remove_whitespaces(&op, result);
        }
        "pop" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_pop(&op, result);
        }
        "push" => {
            // C: evaluates next->next first as operand1, next as operand2
            // then calls push(operand2, operand1, ...)
            // i.e. (push list-sym value) => push(list_obj, value_obj)
            let mut value_obj = Box::new(new_nil_object());
            let mut list_obj = Box::new(new_nil_object());
            if let Some(e) = nth_expr(&exprs_cloned, 2) {
                evaluate_expression(e, &mut value_obj, env, context);
            }
            if let Some(e) = nth_expr(&exprs_cloned, 1) {
                evaluate_expression(e, &mut list_obj, env, context);
            }
            defined_function_push(&mut list_obj, value_obj, result, env);
        }
        "length" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_length(&op, result);
        }
        "is-int-string" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_is_int_string(&op, result);
        }
        "parse-int" => {
            let op = eval_one_operand(&exprs_cloned, env, context);
            defined_function_parse_int(&op, result);
        }
        "string-ref" => {
            let (op1, op2) = eval_two_operands(&exprs_cloned, env, context);
            defined_function_string_ref(&op1, &op2, result);
        }
        _ => {
            // user-defined function call
            let func_result = lookup_function(&sym_name, env);
            if let Some((func_params, func_body)) = func_result {
                let mut new_env = Env {
                    bindings: std::array::from_fn(|_| Binding { symbol_name: String::new(), value: None }),
                    parent: Some(Box::new(clone_env(env))),
                };
                let mut param_idx = 0;
                let mut cur = &exprs_cloned;
                if let Some(ref el) = cur { cur = &el.next; } // skip function name
                while param_idx < func_params.len() {
                    if let Some(ref el) = cur {
                        if let Some(ref e) = el.expression {
                            let mut param_val = Box::new(new_nil_object());
                            evaluate_expression(e, &mut param_val, env, context);
                            set_object_to_env(&mut new_env, &func_params[param_idx], param_val);
                        }
                        cur = &el.next;
                    }
                    param_idx += 1;
                }
                evaluate_expression(&func_body, result, &mut new_env, context);
            } else {
                eprintln!("Undefined function: {}", sym_name);
                std::process::exit(1);
            }
        }
    }
}

fn lookup_function(name: &str, env: &Env) -> Option<(Vec<String>, Box<ExpressionNode>)> {
    for b in env.bindings.iter() {
        if !b.symbol_name.is_empty() && b.symbol_name == name {
            if let Some(ref val) = b.value {
                if let ObjectValue::FunctionValue(Some(ref f)) = val.value {
                    let body = clone_expr_node(f.body.as_ref().unwrap());
                    return Some((f.param_symbol_names.clone(), Box::new(body)));
                }
            }
        }
    }
    if let Some(ref parent) = env.parent {
        return lookup_function(name, parent);
    }
    None
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| {
            Binding {
                symbol_name: env.bindings[i].symbol_name.clone(),
                value: env.bindings[i].value.as_ref().map(|o| Box::new(deep_clone_object(o))),
            }
        }),
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

fn defined_function_add(op1: &Object, op2: &Object, result: &mut Object) {
    match (&op1.type_, &op1.value, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(a), ObjectValue::IntValue(b)) => {
            result.type_ = ObjectType::Integer;
            result.value = ObjectValue::IntValue(a + b);
        }
        (ObjectType::String, ObjectValue::StringValue(a), ObjectValue::StringValue(b)) => {
            result.type_ = ObjectType::String;
            result.value = ObjectValue::StringValue(format!("{}{}", a, b));
        }
        _ => { eprintln!("Type error: operands for + must be integers or strings."); std::process::exit(1); }
    }
}

fn defined_function_sub(op1: &Object, op2: &Object, result: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) = (&op1.value, &op2.value) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(a - b);
    } else { eprintln!("Type error: operands for - must be integers."); std::process::exit(1); }
}

fn defined_function_mul(op1: &Object, op2: &Object, result: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) = (&op1.value, &op2.value) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(a * b);
    } else { eprintln!("Type error: operands for * must be integers."); std::process::exit(1); }
}

fn defined_function_div(op1: &Object, op2: &Object, result: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) = (&op1.value, &op2.value) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(a / b);
    } else { eprintln!("Type error: operands for / must be integers."); std::process::exit(1); }
}

fn defined_function_mod(op1: &Object, op2: &Object, result: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) = (&op1.value, &op2.value) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(a % b);
    } else { eprintln!("Type error: operands for % must be integers."); std::process::exit(1); }
}

fn defined_function_lt(op1: &Object, op2: &Object, result: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) = (&op1.value, &op2.value) {
        result.type_ = ObjectType::Bool;
        result.value = ObjectValue::BoolValue(if a < b { 1 } else { 0 });
    } else { eprintln!("Type error: operands for < must be integers."); std::process::exit(1); }
}

fn defined_function_gt(op1: &Object, op2: &Object, result: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) = (&op1.value, &op2.value) {
        result.type_ = ObjectType::Bool;
        result.value = ObjectValue::BoolValue(if a > b { 1 } else { 0 });
    } else { eprintln!("Type error: operands for < must be integers."); std::process::exit(1); }
}

fn defined_function_not(op: &Object, result: &mut Object) {
    if let ObjectValue::BoolValue(v) = &op.value {
        result.type_ = ObjectType::Bool;
        result.value = ObjectValue::BoolValue(if *v != 0 { 0 } else { 1 });
    } else { eprintln!("Type error: not operand must be boolean."); std::process::exit(1); }
}

fn defined_function_car(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: car operand must be list."); std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(ref cell)) = op.value {
        if let Some(ref car) = cell.car {
            clone_object(car, result);
        }
    }
}

fn defined_function_cdr(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: cdr operand must be list."); std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(ref cell)) = op.value {
        if let Some(ref cdr) = cell.cdr {
            clone_object(cdr, result);
        }
    }
}

fn defined_function_cons(op1: Box<Object>, op2: Box<Object>, result: &mut Object) {
    result.type_ = ObjectType::List;
    let cdr = if matches!(op2.type_, ObjectType::List) || matches!(op2.type_, ObjectType::Nil) {
        op2
    } else {
        let inner_cell = Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(op2),
            cdr: Some(Box::new(new_nil_object())),
        });
        let mut cdr_obj = Box::new(new_nil_object());
        cdr_obj.type_ = ObjectType::List;
        cdr_obj.value = ObjectValue::ListValue(Some(inner_cell));
        cdr_obj
    };
    result.value = ObjectValue::ListValue(Some(Box::new(ConsCell {
        type_: ConsCellType::Cell,
        car: Some(op1),
        cdr: Some(cdr),
    })));
}

fn defined_function_split(op1: &Object, op2: &Object, result: &mut Object) {
    let s1 = if let ObjectValue::StringValue(ref s) = op1.value { s.clone() }
    else { eprintln!("Type error: split first operand must be string."); std::process::exit(1); };
    let s2 = if let ObjectValue::StringValue(ref s) = op2.value { s.clone() }
    else { eprintln!("Type error: split second operand must be string."); std::process::exit(1); };

    let parts: Vec<String> = if s2.is_empty() {
        s1.chars().map(|c| c.to_string()).collect()
    } else {
        s1.split(&s2).map(|s| s.to_string()).collect()
    };

    if parts.is_empty() {
        result.type_ = ObjectType::Nil;
        return;
    }
    result.type_ = ObjectType::List;
    let mut cells: Vec<Box<ConsCell>> = parts.iter().map(|p| {
        let mut obj = Box::new(new_nil_object());
        obj.type_ = ObjectType::String;
        obj.value = ObjectValue::StringValue(p.clone());
        Box::new(ConsCell { type_: ConsCellType::Cell, car: Some(obj), cdr: None })
    }).collect();
    let n = cells.len();
    for i in (0..n).rev() {
        if i == n - 1 {
            cells[i].cdr = Some(Box::new(new_nil_object()));
        } else {
            let next = cells.remove(i + 1);
            let mut cdr_obj = Box::new(new_nil_object());
            cdr_obj.type_ = ObjectType::List;
            cdr_obj.value = ObjectValue::ListValue(Some(next));
            cells[i].cdr = Some(cdr_obj);
        }
    }
    result.value = ObjectValue::ListValue(Some(cells.remove(0)));
}

fn defined_function_list_ref(op1: &Object, op2: &Object, result: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        eprintln!("Type error: list-ref first operand must be list."); std::process::exit(1);
    }
    let idx = if let ObjectValue::IntValue(v) = &op2.value { *v }
    else { eprintln!("Type error: list-ref second operand must be integer."); std::process::exit(1); };
    let mut cur = if let ObjectValue::ListValue(Some(ref c)) = op1.value { c.as_ref() }
    else { eprintln!("Index out of range."); std::process::exit(1); };
    for _ in 0..idx {
        if is_last_cons_cell(cur) { eprintln!("Index out of range."); std::process::exit(1); }
        if let Some(ref cdr) = cur.cdr {
            if let ObjectValue::ListValue(Some(ref next)) = cdr.value { cur = next.as_ref(); }
            else { eprintln!("Index out of range."); std::process::exit(1); }
        } else { eprintln!("Index out of range."); std::process::exit(1); }
    }
    if let Some(ref car) = cur.car { clone_object(car, result); }
}

fn defined_function_remove_whitespaces(op: &Object, result: &mut Object) {
    if let ObjectValue::StringValue(ref s) = op.value {
        result.type_ = ObjectType::String;
        result.value = ObjectValue::StringValue(s.chars().filter(|c| !c.is_ascii_whitespace()).collect());
    } else { eprintln!("Type error: remove-whitespaces operand must be string."); std::process::exit(1); }
}

fn defined_function_pop(op: &Object, result: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: pop operand must be list."); std::process::exit(1);
    }
    // Find last element - since we use value types, just traverse and return last car
    let cell = if let ObjectValue::ListValue(Some(ref c)) = op.value { c.as_ref() }
    else { result.type_ = ObjectType::Nil; return; };
    let mut cur = cell;
    loop {
        if is_last_cons_cell(cur) {
            if let Some(ref car) = cur.car { clone_object(car, result); }
            return;
        }
        if let Some(ref cdr) = cur.cdr {
            if let ObjectValue::ListValue(Some(ref next)) = cdr.value { cur = next.as_ref(); }
            else { if let Some(ref car) = cur.car { clone_object(car, result); } return; }
        } else { if let Some(ref car) = cur.car { clone_object(car, result); } return; }
    }
}

fn defined_function_push(list_obj: &mut Object, value_obj: Box<Object>, result: &mut Object, env: &mut Env) {
    if matches!(list_obj.type_, ObjectType::Nil) {
        clone_object(&value_obj, result);
        // Find the binding in env that points to this nil and replace it
        for b in env.bindings.iter_mut() {
            if !b.symbol_name.is_empty() {
                if let Some(ref val) = b.value {
                    if matches!(val.type_, ObjectType::Nil) {
                        let cell = Box::new(ConsCell {
                            type_: ConsCellType::Cell,
                            car: Some(value_obj),
                            cdr: Some(Box::new(new_nil_object())),
                        });
                        let mut new_list = Box::new(new_nil_object());
                        new_list.type_ = ObjectType::List;
                        new_list.value = ObjectValue::ListValue(Some(cell));
                        b.value = Some(new_list);
                        return;
                    }
                }
            }
        }
        return;
    }
    if !matches!(list_obj.type_, ObjectType::List) {
        eprintln!("Type error: push second operand must be list."); std::process::exit(1);
    }
    // Append value to end of list
    append_to_list_obj(list_obj, value_obj);
    // result = the pushed value (already set above for nil case)
    // For list case, result is the value that was pushed
    // Actually in C: *evaluated = *op2 where op2 is the value
    // We need to get the last car which is the value we just pushed
    // Let's just set result from the last element
    // Actually the C code does *evaluated = *op2 before the push logic for nil,
    // and at the end for list case. Let's just traverse to get last.
    result.type_ = ObjectType::Nil; // will be overwritten
    if let ObjectValue::ListValue(Some(ref cell)) = list_obj.value {
        get_last_car(cell, result);
    }
}

fn append_to_list_obj(obj: &mut Object, value: Box<Object>) {
    if let ObjectValue::ListValue(Some(ref mut cell)) = obj.value {
        append_to_cons_cell(cell, value);
    }
}

fn append_to_cons_cell(cell: &mut ConsCell, value: Box<Object>) {
    if is_last_cons_cell_mut(cell) {
        let new_cell = Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(value),
            cdr: Some(Box::new(new_nil_object())),
        });
        if let Some(ref mut cdr) = cell.cdr {
            cdr.type_ = ObjectType::List;
            cdr.value = ObjectValue::ListValue(Some(new_cell));
        }
    } else if let Some(ref mut cdr) = cell.cdr {
        if let ObjectValue::ListValue(Some(ref mut next)) = cdr.value {
            append_to_cons_cell(next, value);
        }
    }
}

fn is_last_cons_cell_mut(cell: &ConsCell) -> bool {
    if let Some(ref cdr) = cell.cdr {
        matches!(cdr.type_, ObjectType::Nil)
    } else {
        true
    }
}

fn get_last_car(cell: &ConsCell, result: &mut Object) {
    if is_last_cons_cell(cell) {
        if let Some(ref car) = cell.car { clone_object(car, result); }
    } else if let Some(ref cdr) = cell.cdr {
        if let ObjectValue::ListValue(Some(ref next)) = cdr.value {
            get_last_car(next, result);
        }
    }
}

fn defined_function_length(op: &Object, result: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(0);
        return;
    }
    if matches!(op.type_, ObjectType::List) {
        let mut len = 1i32;
        if let ObjectValue::ListValue(Some(ref cell)) = op.value {
            let mut cur = cell.as_ref();
            while !is_last_cons_cell(cur) {
                len += 1;
                if let Some(ref cdr) = cur.cdr {
                    if let ObjectValue::ListValue(Some(ref next)) = cdr.value { cur = next.as_ref(); }
                    else { break; }
                } else { break; }
            }
        }
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(len);
    } else if matches!(op.type_, ObjectType::String) {
        if let ObjectValue::StringValue(ref s) = op.value {
            result.type_ = ObjectType::Integer;
            result.value = ObjectValue::IntValue(s.len() as i32);
        }
    } else { eprintln!("Type error: length operand must be list or string."); std::process::exit(1); }
}

fn defined_function_is_int_string(op: &Object, result: &mut Object) {
    result.type_ = ObjectType::Bool;
    if let (ObjectType::String, ObjectValue::StringValue(ref s)) = (&op.type_, &op.value) {
        result.value = ObjectValue::BoolValue(if s.chars().all(|c| c.is_ascii_digit()) { 1 } else { 0 });
    } else {
        result.value = ObjectValue::BoolValue(0);
    }
}

fn defined_function_parse_int(op: &Object, result: &mut Object) {
    if let ObjectValue::StringValue(ref s) = op.value {
        if !s.chars().all(|c| c.is_ascii_digit()) {
            eprintln!("Type error: parse-int operand must be string of digits."); std::process::exit(1);
        }
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(s.parse::<i32>().unwrap_or(0));
    } else { eprintln!("Type error: parse-int operand must be string."); std::process::exit(1); }
}

fn defined_function_string_ref(op1: &Object, op2: &Object, result: &mut Object) {
    let s = if let ObjectValue::StringValue(ref s) = op1.value { s.clone() }
    else { eprintln!("Type error: string-ref first operand must be string."); std::process::exit(1); };
    let idx = if let ObjectValue::IntValue(v) = &op2.value { *v }
    else { eprintln!("Type error: string-ref second operand must be integer."); std::process::exit(1); };
    if idx < 0 || idx as usize >= s.len() {
        eprintln!("Index out of range."); std::process::exit(1);
    }
    result.type_ = ObjectType::String;
    result.value = ObjectValue::StringValue(s.as_bytes()[idx as usize..idx as usize + 1].iter().map(|&b| b as char).collect());
}
