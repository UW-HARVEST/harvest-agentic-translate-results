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

// ============== helpers ==============

fn empty_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: clone_object_value(&obj.value),
    }
}

fn clone_object_value(v: &ObjectValue) -> ObjectValue {
    match v {
        ObjectValue::IntValue(i) => ObjectValue::IntValue(*i),
        ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
        ObjectValue::BoolValue(b) => ObjectValue::BoolValue(*b),
        ObjectValue::ListValue(opt) => match opt {
            Some(cc) => ObjectValue::ListValue(Some(Box::new(clone_conscell(cc)))),
            None => ObjectValue::ListValue(None),
        },
        ObjectValue::FunctionValue(opt) => match opt {
            Some(f) => ObjectValue::FunctionValue(Some(Box::new(clone_function(f)))),
            None => ObjectValue::FunctionValue(None),
        },
    }
}

fn clone_conscell(cc: &ConsCell) -> ConsCell {
    ConsCell {
        type_: cc.type_,
        car: cc.car.as_ref().map(|b| Box::new(clone_object(b))),
        cdr: cc.cdr.as_ref().map(|b| Box::new(clone_object(b))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: f.body.as_ref().map(|b| Box::new(clone_expression(b))),
    }
}

fn clone_expression(e: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: e.type_,
        data: clone_expression_data(&e.data),
    }
}

fn clone_expression_data(d: &ExpressionData) -> ExpressionData {
    match d {
        ExpressionData::SymbolicExp(opt) => ExpressionData::SymbolicExp(
            opt.as_ref().map(|n| Box::new(SymbolicExpNode {
                expressions: n.expressions.as_ref().map(|e| Box::new(clone_expression_list(e))),
            })),
        ),
        ExpressionData::List(opt) => ExpressionData::List(
            opt.as_ref().map(|n| Box::new(ListNode {
                expressions: n.expressions.as_ref().map(|e| Box::new(clone_expression_list(e))),
            })),
        ),
        ExpressionData::Literal(opt) => ExpressionData::Literal(
            opt.as_ref().map(|n| Box::new(LiteralNode {
                type_: n.type_,
                value: clone_literal_value(&n.value),
            })),
        ),
        ExpressionData::Symbol(opt) => ExpressionData::Symbol(
            opt.as_ref().map(|n| Box::new(SymbolNode {
                symbol_name: n.symbol_name.clone(),
            })),
        ),
    }
}

fn clone_literal_value(v: &LiteralValue) -> LiteralValue {
    match v {
        LiteralValue::IntValue(i) => LiteralValue::IntValue(*i),
        LiteralValue::BooleanValue(b) => LiteralValue::BooleanValue(*b),
        LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
    }
}

fn clone_expression_list(l: &ExpressionList) -> ExpressionList {
    ExpressionList {
        expression: l.expression.as_ref().map(|e| Box::new(clone_expression(e))),
        next: l.next.as_ref().map(|n| Box::new(clone_expression_list(n))),
    }
}

fn clone_binding(b: &Binding) -> Binding {
    Binding {
        symbol_name: b.symbol_name.clone(),
        value: b.value.as_ref().map(|v| Box::new(clone_object(v))),
    }
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| clone_binding(&env.bindings[i])),
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

fn empty_binding() -> Binding {
    Binding {
        symbol_name: String::new(),
        value: None,
    }
}

fn binding_is_unset(b: &Binding) -> bool {
    b.symbol_name.is_empty()
}

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => match &obj.value {
            ObjectValue::BoolValue(b) => *b != 0,
            _ => false,
        },
        ObjectType::Nil => false,
        _ => true,
    }
}

fn obj_int(i: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(i),
    }
}

fn obj_str(s: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(s),
    }
}

fn obj_bool(b: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if b { 1 } else { 0 }),
    }
}

fn obj_nil() -> Object {
    nil_object()
}

fn is_op(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-' | '*' | '/' | '%' | '|' | '&' | '=' | '<' | '>'
    )
}

fn assign_object(target: &mut Object, source: Object) {
    target.marked = source.marked;
    target.type_ = source.type_;
    target.value = source.value;
}

// ============== tokenizer ==============

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    match &state.token {
        Some(t) if t.kind == kind => 1,
        _ => 0,
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    // Skip whitespace
    while (state.pos as usize) < bytes.len() {
        let c = bytes[state.pos as usize];
        if (c as char).is_whitespace() || c == b'\n' {
            state.pos += 1;
        } else {
            break;
        }
    }

    let pos = state.pos as usize;
    let new_token: Token;

    if pos >= bytes.len() {
        new_token = Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: String::from("\0"),
        };
    } else {
        let c = bytes[pos] as char;
        if c == '(' {
            state.pos += 1;
            new_token = Token {
                kind: TokenKind::LParen,
                next: None,
                val: 0,
                str: String::from("("),
            };
        } else if c == ')' {
            state.pos += 1;
            new_token = Token {
                kind: TokenKind::RParen,
                next: None,
                val: 0,
                str: String::from(")"),
            };
        } else if c == '\'' {
            state.pos += 1;
            new_token = Token {
                kind: TokenKind::Quote,
                next: None,
                val: 0,
                str: String::from("'"),
            };
        } else if c == '\0' {
            new_token = Token {
                kind: TokenKind::Eof,
                next: None,
                val: 0,
                str: String::from("\0"),
            };
        } else if c.is_ascii_alphabetic() || is_op(c) {
            // tokenize symbol
            let start = pos;
            while (state.pos as usize) < bytes.len() {
                let ch = bytes[state.pos as usize] as char;
                if ch.is_ascii_alphanumeric() || is_op(ch) {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let s = String::from_utf8_lossy(&bytes[start..state.pos as usize]).to_string();
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
            let start = pos;
            while (state.pos as usize) < bytes.len() {
                let ch = bytes[state.pos as usize] as char;
                if ch.is_ascii_digit() {
                    state.pos += 1;
                } else {
                    break;
                }
            }
            let s = String::from_utf8_lossy(&bytes[start..state.pos as usize]).to_string();
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
            while (state.pos as usize) < bytes.len() {
                let ch = bytes[state.pos as usize];
                if ch == b'"' || ch == b'\0' {
                    break;
                }
                state.pos += 1;
            }
            let s = String::from_utf8_lossy(&bytes[start..state.pos as usize]).to_string();
            if (state.pos as usize) < bytes.len() && bytes[state.pos as usize] == b'"' {
                state.pos += 1;
            }
            new_token = Token {
                kind: TokenKind::String,
                next: None,
                val: 0,
                str: s,
            };
        } else if c == ';' {
            // comment - skip until newline or end
            while (state.pos as usize) < bytes.len() {
                let ch = bytes[state.pos as usize];
                if ch == b'\n' || ch == b'\0' {
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

// ============== parser ==============

fn append_expr_to_list(list: &mut Option<Box<ExpressionList>>, expr: ExpressionNode) {
    let new_node = Box::new(ExpressionList {
        expression: Some(Box::new(expr)),
        next: None,
    });
    if list.is_none() {
        *list = Some(new_node);
        return;
    }
    let mut cur = list.as_mut().unwrap();
    while cur.next.is_some() {
        cur = cur.next.as_mut().unwrap();
    }
    cur.next = Some(new_node);
}

fn parse_expression_node(source: &str, state: &mut ParseState) -> ExpressionNode {
    if match_token(state, TokenKind::LParen) == 1 {
        parse_symbolic_expression(source, state)
    } else if match_token(state, TokenKind::Quote) == 1 {
        parse_list_expression(source, state)
    } else if match_token(state, TokenKind::Symbol) == 1 {
        parse_symbol_expression(source, state)
    } else if match_token(state, TokenKind::Digit) == 1
        || match_token(state, TokenKind::String) == 1
        || match_token(state, TokenKind::True) == 1
        || match_token(state, TokenKind::False) == 1
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

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut sym_node = SymbolicExpNode { expressions: None };
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        let expr_item = parse_expression_node(source, state);
        append_expr_to_list(&mut sym_node.expressions, expr_item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(sym_node))),
    }
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut list_node = ListNode { expressions: None };
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        let expr_item = parse_expression_node(source, state);
        append_expr_to_list(&mut list_node.expressions, expr_item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(list_node))),
    }
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let name = state
        .token
        .as_ref()
        .map(|t| t.str.clone())
        .unwrap_or_default();
    next(source, state);
    ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode {
            symbol_name: name,
        }))),
    }
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let (lit_type, value) = if match_token(state, TokenKind::Digit) == 1 {
        let v = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        (LiteralType::Integer, LiteralValue::IntValue(v))
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        (LiteralType::String, LiteralValue::StringValue(s))
    } else if match_token(state, TokenKind::True) == 1 {
        (LiteralType::Boolean, LiteralValue::BooleanValue(true))
    } else if match_token(state, TokenKind::False) == 1 {
        (LiteralType::Boolean, LiteralValue::BooleanValue(false))
    } else {
        panic!("Unexpected token in literal");
    };
    next(source, state);
    ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(LiteralNode {
            type_: lit_type,
            value,
        }))),
    }
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state); // first token
    let mut program = ProgramNode { expressions: None };
    while match_token(state, TokenKind::Eof) == 0 {
        let expr = parse_expression_node(source, state);
        append_expr_to_list(&mut program.expressions, expr);
    }
    result.program = Some(Box::new(program));
}

// ============== environment helpers ==============

fn env_lookup(env: &Env, name: &str) -> Option<Object> {
    for i in 0..MAX_BINDINGS {
        if binding_is_unset(&env.bindings[i]) {
            break;
        }
        if env.bindings[i].symbol_name == name {
            if let Some(v) = &env.bindings[i].value {
                return Some(clone_object(v));
            }
            return None;
        }
    }
    if let Some(parent) = &env.parent {
        return env_lookup(parent, name);
    }
    None
}

fn env_set(env: &mut Env, name: &str, obj: Object) {
    for i in 0..MAX_BINDINGS {
        if binding_is_unset(&env.bindings[i]) {
            env.bindings[i] = Binding {
                symbol_name: name.to_string(),
                value: Some(Box::new(obj)),
            };
            return;
        }
        if env.bindings[i].symbol_name == name {
            env.bindings[i].value = Some(Box::new(obj));
            return;
        }
    }
}

// ============== stringify ==============

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => {
            if let ObjectValue::IntValue(i) = obj.value {
                i.to_string()
            } else {
                String::new()
            }
        }
        ObjectType::String => {
            if let ObjectValue::StringValue(ref s) = obj.value {
                s.clone()
            } else {
                String::new()
            }
        }
        ObjectType::Bool => {
            if let ObjectValue::BoolValue(b) = obj.value {
                if b != 0 {
                    String::from("T")
                } else {
                    String::from("F")
                }
            } else {
                String::new()
            }
        }
        ObjectType::List => {
            if let ObjectValue::ListValue(ref opt) = obj.value {
                let mut s = String::from("(");
                let mut first = true;
                let mut current_opt: Option<&ConsCell> = opt.as_deref();
                while let Some(cc) = current_opt {
                    if !first {
                        s.push(' ');
                    }
                    first = false;
                    if let Some(car) = &cc.car {
                        s.push_str(&stringify_object(car));
                    }
                    // check if last
                    let is_last = match &cc.cdr {
                        Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
                        None => true,
                    };
                    if is_last {
                        break;
                    }
                    // advance into cdr.list_value
                    current_opt = match &cc.cdr {
                        Some(cdr) => match &cdr.value {
                            ObjectValue::ListValue(opt2) => opt2.as_deref(),
                            _ => None,
                        },
                        None => None,
                    };
                }
                s.push(')');
                s
            } else {
                String::from("()")
            }
        }
        ObjectType::Nil => String::from("nil"),
        ObjectType::Function => String::from("<function>"),
    }
}

// ============== object equality ==============

fn obj_eq(a: &Object, b: &Object) -> bool {
    match (a.type_, b.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            matches!((&a.value, &b.value), (ObjectValue::IntValue(x), ObjectValue::IntValue(y)) if x == y)
        }
        (ObjectType::String, ObjectType::String) => {
            matches!((&a.value, &b.value), (ObjectValue::StringValue(x), ObjectValue::StringValue(y)) if x == y)
        }
        (ObjectType::Bool, ObjectType::Bool) => {
            matches!((&a.value, &b.value), (ObjectValue::BoolValue(x), ObjectValue::BoolValue(y)) if x == y)
        }
        (ObjectType::Nil, ObjectType::Nil) => true,
        (ObjectType::List, ObjectType::List) => {
            // compare structurally
            stringify_object(a) == stringify_object(b)
        }
        _ => false,
    }
}

// ============== allocator ==============

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

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(empty_object()))
}

// ============== evaluator ==============

fn evaluate_list_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let exprs_opt: Option<&ExpressionList> = match &expression.data {
        ExpressionData::List(Some(node)) => node.expressions.as_deref(),
        _ => None,
    };

    if exprs_opt.is_none() {
        return obj_nil();
    }

    // Evaluate each item, collect into Vec
    let mut items: Vec<Object> = Vec::new();
    let mut current = exprs_opt;
    while let Some(el) = current {
        if let Some(expr) = &el.expression {
            let mut tmp = empty_object();
            evaluate_expression(expr, &mut tmp, env, context);
            items.push(tmp);
        }
        current = el.next.as_deref();
    }

    if items.is_empty() {
        return obj_nil();
    }

    // Build cons-list from items
    build_cons_list(items)
}

fn build_cons_list(items: Vec<Object>) -> Object {
    // Build from back
    let mut cell_obj_chain: Option<Box<Object>> = None;

    let mut iter = items.into_iter().rev();
    for (idx, item) in iter.by_ref().enumerate() {
        // construct cons cell with car=item
        let cdr_obj: Option<Box<Object>> = if cell_obj_chain.is_none() {
            // last cell -> cdr = nil object
            Some(Box::new(obj_nil()))
        } else {
            cell_obj_chain.take()
        };
        let cell_type = if idx == 0 {
            ConsCellType::Nil
        } else {
            ConsCellType::Cell
        };
        let new_cell = ConsCell {
            type_: cell_type,
            car: Some(Box::new(item)),
            cdr: cdr_obj,
        };
        let new_obj = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(new_cell))),
        };
        cell_obj_chain = Some(Box::new(new_obj));
    }
    match cell_obj_chain {
        Some(b) => *b,
        None => obj_nil(),
    }
}

fn defined_function_add(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_int(a.wrapping_add(*b))
        }
        (ObjectValue::StringValue(a), ObjectValue::StringValue(b), ObjectType::String, ObjectType::String) => {
            let mut s = a.clone();
            s.push_str(b);
            obj_str(s)
        }
        _ => panic!("Type error: operands for + must be integers or strings."),
    }
}

fn defined_function_sub(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_int(a.wrapping_sub(*b))
        }
        _ => panic!("Type error: operands for - must be integers."),
    }
}

fn defined_function_mul(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_int(a.wrapping_mul(*b))
        }
        _ => panic!("Type error: operands for * must be integers."),
    }
}

fn defined_function_div(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_int(a / b)
        }
        _ => panic!("Type error: operands for / must be integers."),
    }
}

fn defined_function_mod(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_int(a % b)
        }
        _ => panic!("Type error: operands for % must be integers."),
    }
}

fn defined_function_lt(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_bool(a < b)
        }
        _ => panic!("Type error: operands for < must be integers."),
    }
}

fn defined_function_gt(op1: &Object, op2: &Object) -> Object {
    match (&op1.value, &op2.value, op1.type_, op2.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer, ObjectType::Integer) => {
            obj_bool(a > b)
        }
        _ => panic!("Type error: operands for > must be integers."),
    }
}

fn defined_function_not(op: &Object) -> Object {
    match op.type_ {
        ObjectType::Bool => {
            if let ObjectValue::BoolValue(b) = op.value {
                obj_bool(b == 0)
            } else {
                obj_bool(true)
            }
        }
        _ => panic!("Type error: not operand must be boolean."),
    }
}

fn defined_function_car(op: &Object) -> Object {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: car operand must be list.");
    }
    if let ObjectValue::ListValue(Some(cc)) = &op.value {
        if let Some(car) = &cc.car {
            return clone_object(car);
        }
    }
    obj_nil()
}

fn defined_function_cdr(op: &Object) -> Object {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: cdr operand must be list.");
    }
    if let ObjectValue::ListValue(Some(cc)) = &op.value {
        if let Some(cdr) = &cc.cdr {
            return clone_object(cdr);
        }
    }
    obj_nil()
}

fn defined_function_cons(op1: &Object, op2: &Object) -> Object {
    let car = Some(Box::new(clone_object(op1)));
    let cdr: Option<Box<Object>>;
    let cc_type;
    match op2.type_ {
        ObjectType::List => {
            cc_type = ConsCellType::Cell;
            cdr = Some(Box::new(clone_object(op2)));
        }
        ObjectType::Nil => {
            cc_type = ConsCellType::Nil;
            cdr = Some(Box::new(clone_object(op2)));
        }
        _ => {
            // wrap op2 as a list with single element followed by nil
            let inner_cell = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(clone_object(op2))),
                cdr: Some(Box::new(obj_nil())),
            };
            let cdr_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cell))),
            };
            cc_type = ConsCellType::Cell;
            cdr = Some(Box::new(cdr_obj));
        }
    }
    let cell = ConsCell {
        type_: cc_type,
        car,
        cdr,
    };
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(Some(Box::new(cell))),
    }
}

fn defined_function_split(op1: &Object, op2: &Object) -> Object {
    let s1 = match (&op1.value, op1.type_) {
        (ObjectValue::StringValue(s), ObjectType::String) => s.clone(),
        _ => panic!("Type error: split first operand must be string."),
    };
    let s2 = match (&op2.value, op2.type_) {
        (ObjectValue::StringValue(s), ObjectType::String) => s.clone(),
        _ => panic!("Type error: split second operand must be string."),
    };
    let items: Vec<Object> = if s2.is_empty() {
        s1.chars().map(|c| obj_str(c.to_string())).collect()
    } else {
        s1.split(&s2)
            .filter(|p| !p.is_empty())
            .map(|p| obj_str(p.to_string()))
            .collect()
    };
    if items.is_empty() {
        obj_nil()
    } else {
        build_cons_list(items)
    }
}

fn defined_function_list_ref(op1: &Object, op2: &Object) -> Object {
    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: list-ref first operand must be list.");
    }
    let index = match (&op2.value, op2.type_) {
        (ObjectValue::IntValue(i), ObjectType::Integer) => *i,
        _ => panic!("Type error: list-ref second operand must be integer."),
    };
    let mut current_opt: Option<&ConsCell> = match &op1.value {
        ObjectValue::ListValue(opt) => opt.as_deref(),
        _ => None,
    };
    let mut i: i32 = 0;
    while let Some(cc) = current_opt {
        if i == index {
            if let Some(car) = &cc.car {
                return clone_object(car);
            } else {
                return obj_nil();
            }
        }
        // advance
        let cdr_is_nil = match &cc.cdr {
            Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
            None => true,
        };
        if cdr_is_nil {
            panic!("Index out of range.");
        }
        current_opt = match &cc.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(opt) => opt.as_deref(),
                _ => None,
            },
            None => None,
        };
        i += 1;
    }
    panic!("Index out of range.");
}

fn defined_function_remove_whitespaces(op: &Object) -> Object {
    let s = match (&op.value, op.type_) {
        (ObjectValue::StringValue(s), ObjectType::String) => s.clone(),
        _ => panic!("Type error: remove-whitespaces operand must be string."),
    };
    let new_s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    obj_str(new_s)
}

fn defined_function_pop(op: &Object) -> Object {
    if matches!(op.type_, ObjectType::Nil) {
        return obj_nil();
    }
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: pop operand must be list.");
    }
    // walk to last car
    let mut current_opt: Option<&ConsCell> = match &op.value {
        ObjectValue::ListValue(opt) => opt.as_deref(),
        _ => None,
    };
    let mut last: Option<Object> = None;
    while let Some(cc) = current_opt {
        let is_last = match &cc.cdr {
            Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
            None => true,
        };
        if is_last {
            if let Some(car) = &cc.car {
                last = Some(clone_object(car));
            }
            break;
        }
        current_opt = match &cc.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(opt) => opt.as_deref(),
                _ => None,
            },
            None => None,
        };
    }
    last.unwrap_or_else(obj_nil)
}

fn defined_function_push(op1: &Object, op2: &Object) -> Object {
    // op1 is target list, op2 is value to push
    if matches!(op1.type_, ObjectType::Nil) {
        return clone_object(op2);
    }
    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: push second operand must be list.");
    }
    // collect current items
    let mut items: Vec<Object> = Vec::new();
    let mut current_opt: Option<&ConsCell> = match &op1.value {
        ObjectValue::ListValue(opt) => opt.as_deref(),
        _ => None,
    };
    while let Some(cc) = current_opt {
        if let Some(car) = &cc.car {
            items.push(clone_object(car));
        }
        let is_last = match &cc.cdr {
            Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
            None => true,
        };
        if is_last {
            break;
        }
        current_opt = match &cc.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(opt) => opt.as_deref(),
                _ => None,
            },
            None => None,
        };
    }
    items.push(clone_object(op2));
    build_cons_list(items)
}

fn defined_function_length(op: &Object) -> Object {
    match op.type_ {
        ObjectType::Nil => obj_int(0),
        ObjectType::List => {
            let mut len = 0i32;
            let mut current_opt: Option<&ConsCell> = match &op.value {
                ObjectValue::ListValue(opt) => opt.as_deref(),
                _ => None,
            };
            while let Some(cc) = current_opt {
                len += 1;
                let is_last = match &cc.cdr {
                    Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
                    None => true,
                };
                if is_last {
                    break;
                }
                current_opt = match &cc.cdr {
                    Some(cdr) => match &cdr.value {
                        ObjectValue::ListValue(opt) => opt.as_deref(),
                        _ => None,
                    },
                    None => None,
                };
            }
            obj_int(len)
        }
        ObjectType::String => {
            if let ObjectValue::StringValue(s) = &op.value {
                obj_int(s.len() as i32)
            } else {
                obj_int(0)
            }
        }
        _ => panic!("Type error: length operand must be list or string."),
    }
}

fn defined_function_is_int_string(op: &Object) -> Object {
    if let (ObjectType::String, ObjectValue::StringValue(s)) = (op.type_, &op.value) {
        if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
            return obj_bool(true);
        }
        return obj_bool(false);
    }
    obj_bool(false)
}

fn defined_function_parse_int(op: &Object) -> Object {
    match (op.type_, &op.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => {
            if !s.chars().all(|c| c.is_ascii_digit()) {
                panic!("Type error: parse-int operand must be string of digits.");
            }
            let v: i32 = s.parse().unwrap_or(0);
            obj_int(v)
        }
        _ => panic!("Type error: parse-int operand must be string."),
    }
}

fn defined_function_string_ref(op1: &Object, op2: &Object) -> Object {
    let s = match (op1.type_, &op1.value) {
        (ObjectType::String, ObjectValue::StringValue(s)) => s,
        _ => panic!("Type error: string-ref first operand must be string."),
    };
    let idx = match (op2.type_, &op2.value) {
        (ObjectType::Integer, ObjectValue::IntValue(i)) => *i,
        _ => panic!("Type error: string-ref second operand must be integer."),
    };
    if idx < 0 || (idx as usize) >= s.len() {
        panic!("Index out of range.");
    }
    let bytes = s.as_bytes();
    let c = bytes[idx as usize] as char;
    obj_str(c.to_string())
}

// ============== symbolic expression evaluation ==============

fn get_symbol_name(expr: &ExpressionNode) -> Option<String> {
    if let ExpressionData::Symbol(Some(s)) = &expr.data {
        return Some(s.symbol_name.clone());
    }
    None
}

fn list_get(list: &Option<Box<ExpressionList>>, idx: usize) -> Option<&ExpressionNode> {
    let mut cur = list.as_deref();
    let mut i = 0;
    while let Some(el) = cur {
        if i == idx {
            return el.expression.as_deref();
        }
        cur = el.next.as_deref();
        i += 1;
    }
    None
}

fn list_len(list: &Option<Box<ExpressionList>>) -> usize {
    let mut cur = list.as_deref();
    let mut n = 0;
    while let Some(el) = cur {
        n += 1;
        cur = el.next.as_deref();
    }
    n
}

fn list_iter(list: &Option<Box<ExpressionList>>) -> Vec<&ExpressionNode> {
    let mut v: Vec<&ExpressionNode> = Vec::new();
    let mut cur = list.as_deref();
    while let Some(el) = cur {
        if let Some(e) = &el.expression {
            v.push(e);
        }
        cur = el.next.as_deref();
    }
    v
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let exprs_opt: &Option<Box<ExpressionList>> = match &expression.data {
        ExpressionData::SymbolicExp(Some(node)) => &node.expressions,
        ExpressionData::List(Some(node)) => &node.expressions,
        _ => return obj_nil(),
    };

    if exprs_opt.is_none() {
        return obj_nil();
    }

    let items = list_iter(exprs_opt);
    if items.is_empty() {
        return obj_nil();
    }
    let head = items[0];
    let head_name = match get_symbol_name(head) {
        Some(n) => n,
        None => panic!("S-exp must be started with symbol."),
    };

    match head_name.as_str() {
        "if" => {
            if items.len() < 3 {
                panic!("if must have condition and then clause.");
            }
            let cond = items[1];
            let then_expr = items[2];
            let mut cond_obj = empty_object();
            evaluate_expression(cond, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                let mut r = empty_object();
                evaluate_expression(then_expr, &mut r, env, context);
                r
            } else if items.len() >= 4 {
                let els = items[3];
                let mut r = empty_object();
                evaluate_expression(els, &mut r, env, context);
                r
            } else {
                obj_nil()
            }
        }
        "while" => {
            if items.len() < 3 {
                panic!("while must have condition and body.");
            }
            let cond = items[1];
            let body = items[2];
            loop {
                let mut cond_obj = empty_object();
                evaluate_expression(cond, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    let mut r = empty_object();
                    evaluate_expression(body, &mut r, env, context);
                    let _ = r;
                } else {
                    break;
                }
            }
            obj_nil()
        }
        "=" => {
            if items.len() < 3 {
                panic!("assignment must have name and expression.");
            }
            let name = match get_symbol_name(items[1]) {
                Some(n) => n,
                None => panic!("Variable name must be symbol."),
            };
            let mut r = empty_object();
            evaluate_expression(items[2], &mut r, env, context);
            let cloned = clone_object(&r);
            env_set(env, &name, cloned);
            r
        }
        "defun" => {
            if items.len() < 4 {
                panic!("defun must have name, params, body.");
            }
            let name = match get_symbol_name(items[1]) {
                Some(n) => n,
                None => panic!("Function name must be symbol."),
            };
            let params_expr = items[2];
            let params_list_opt: &Option<Box<ExpressionList>> = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(n)) => &n.expressions,
                _ => panic!("Function parameter must be list."),
            };
            let mut param_names: Vec<String> = Vec::new();
            let p_items = list_iter(params_list_opt);
            for p in p_items {
                if let Some(n) = get_symbol_name(p) {
                    param_names.push(n);
                } else {
                    panic!("Function parameter must be symbol.");
                }
            }
            let body = items[3];
            let func = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(clone_expression(body))),
            };
            let func_obj = Object {
                marked: false,
                type_: ObjectType::Function,
                value: ObjectValue::FunctionValue(Some(Box::new(func))),
            };
            let cloned = clone_object(&func_obj);
            env_set(env, &name, cloned);
            func_obj
        }
        "+" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_add(&a, &b)
        }
        "-" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_sub(&a, &b)
        }
        "*" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_mul(&a, &b)
        }
        "/" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_div(&a, &b)
        }
        "%" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_mod(&a, &b)
        }
        "||" => {
            for i in 1..items.len() {
                let v = eval_arg(items.get(i), env, context);
                if bool_val(&v) {
                    return obj_bool(true);
                }
            }
            obj_bool(false)
        }
        "&&" => {
            for i in 1..items.len() {
                let v = eval_arg(items.get(i), env, context);
                if !bool_val(&v) {
                    return obj_bool(false);
                }
            }
            obj_bool(true)
        }
        "<" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_lt(&a, &b)
        }
        ">" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_gt(&a, &b)
        }
        "eq" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            obj_bool(obj_eq(&a, &b))
        }
        "not" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_not(&a)
        }
        "print" => {
            let a = eval_arg(items.get(1), env, context);
            let s = stringify_object(&a);
            println!("{}", s);
            obj_nil()
        }
        "car" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_car(&a)
        }
        "cdr" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_cdr(&a)
        }
        "cons" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_cons(&a, &b)
        }
        "readline" => {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) => obj_nil(),
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    obj_str(line)
                }
                Err(_) => obj_nil(),
            }
        }
        "split" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_split(&a, &b)
        }
        "list-ref" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_list_ref(&a, &b)
        }
        "progn" => {
            let mut last = obj_nil();
            let mut had = false;
            for i in 1..items.len() {
                last = eval_arg(items.get(i), env, context);
                had = true;
            }
            if had {
                last
            } else {
                obj_nil()
            }
        }
        "remove-whitespaces" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_remove_whitespaces(&a)
        }
        "pop" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_pop(&a)
        }
        "push" => {
            // C version: definedFunctionPush(operand2, operand1, ...)
            // Where operand1 = items[2] (value) evaluated, operand2 = items[1] (target list) evaluated
            let target = eval_arg(items.get(1), env, context);
            let value = eval_arg(items.get(2), env, context);
            defined_function_push(&target, &value)
        }
        "length" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_length(&a)
        }
        "is-int-string" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_is_int_string(&a)
        }
        "parse-int" => {
            let a = eval_arg(items.get(1), env, context);
            defined_function_parse_int(&a)
        }
        "string-ref" => {
            let a = eval_arg(items.get(1), env, context);
            let b = eval_arg(items.get(2), env, context);
            defined_function_string_ref(&a, &b)
        }
        _ => {
            // user-defined function call
            let func_obj = match env_lookup(env, &head_name) {
                Some(o) => o,
                None => panic!("Undefined function: {}", head_name),
            };
            let function = match func_obj.value {
                ObjectValue::FunctionValue(Some(f)) => *f,
                _ => panic!("Undefined function: {}", head_name),
            };
            // build new env with current env as parent
            let parent_clone = clone_env(env);
            let mut new_env = Env {
                bindings: std::array::from_fn(|_| empty_binding()),
                parent: Some(Box::new(parent_clone)),
            };
            // evaluate args in current env, bind to params
            for (j, pname) in function.param_symbol_names.iter().enumerate() {
                let arg_expr = items.get(j + 1);
                let val = eval_arg(arg_expr, env, context);
                env_set(&mut new_env, pname, val);
            }
            let body = match function.body {
                Some(b) => b,
                None => panic!("Function has no body."),
            };
            let mut result = empty_object();
            evaluate_expression(&body, &mut result, &mut new_env, context);
            result
        }
    }
}

fn eval_arg(
    expr: Option<&&ExpressionNode>,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    match expr {
        Some(e) => {
            let mut r = empty_object();
            evaluate_expression(e, &mut r, env, context);
            r
        }
        None => obj_nil(),
    }
}

fn evaluate_literal_expression(expression: &ExpressionNode) -> Object {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                if let LiteralValue::IntValue(i) = lit.value {
                    return obj_int(i);
                }
            }
            LiteralType::String => {
                if let LiteralValue::StringValue(s) = &lit.value {
                    return obj_str(s.clone());
                }
            }
            LiteralType::Boolean => {
                if let LiteralValue::BooleanValue(b) = lit.value {
                    return obj_bool(b);
                }
            }
        }
    }
    obj_nil()
}

fn evaluate_symbol_expression(
    expression: &ExpressionNode,
    env: &mut Env,
    _context: &mut AllocatorContext,
) -> Object {
    let name = match &expression.data {
        ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
        _ => return obj_nil(),
    };
    if name == "nil" {
        return obj_nil();
    }
    match env_lookup(env, &name) {
        Some(v) => v,
        None => panic!("Undefined symbol: {}", name),
    }
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let val = match expression.type_ {
        ExpressionType::List => evaluate_list_expression(expression, env, context),
        ExpressionType::SymbolicExp => evaluate_symbolic_expression(expression, env, context),
        ExpressionType::Literal => evaluate_literal_expression(expression),
        ExpressionType::Symbol => evaluate_symbol_expression(expression, env, context),
    };
    assign_object(result, val);
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
        bindings: std::array::from_fn(|_| empty_binding()),
        parent: None,
    };
    let mut context = init_allocator();
    let mut cur = program.expressions.as_deref();
    while let Some(el) = cur {
        if let Some(expr) = &el.expression {
            let mut r = empty_object();
            evaluate_expression(expr, &mut r, &mut env, &mut context);
        }
        cur = el.next.as_deref();
    }
}

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for i in 0..MAX_BINDINGS {
        env.bindings[i] = empty_binding();
    }
}
