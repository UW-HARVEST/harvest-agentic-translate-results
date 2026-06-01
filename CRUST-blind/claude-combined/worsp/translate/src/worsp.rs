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

// ============================================================
//   Helper traits and constructors
// ============================================================

impl Clone for Object {
    fn clone(&self) -> Self {
        Object {
            marked: self.marked,
            type_: self.type_,
            value: self.value.clone(),
        }
    }
}

impl Clone for ObjectValue {
    fn clone(&self) -> Self {
        match self {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(s) => ObjectValue::StringValue(s.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(c) => ObjectValue::ListValue(c.as_ref().map(|cc| Box::new((**cc).clone()))),
            ObjectValue::FunctionValue(f) => ObjectValue::FunctionValue(f.as_ref().map(|ff| Box::new((**ff).clone()))),
        }
    }
}

impl Clone for ConsCell {
    fn clone(&self) -> Self {
        ConsCell {
            type_: self.type_,
            car: self.car.as_ref().map(|c| Box::new((**c).clone())),
            cdr: self.cdr.as_ref().map(|c| Box::new((**c).clone())),
        }
    }
}

impl Clone for Function {
    fn clone(&self) -> Self {
        Function {
            param_symbol_names: self.param_symbol_names.clone(),
            body: self.body.as_ref().map(|b| Box::new((**b).clone())),
        }
    }
}

impl Clone for ExpressionNode {
    fn clone(&self) -> Self {
        ExpressionNode {
            type_: self.type_,
            data: self.data.clone(),
        }
    }
}

impl Clone for ExpressionData {
    fn clone(&self) -> Self {
        match self {
            ExpressionData::SymbolicExp(s) => ExpressionData::SymbolicExp(s.as_ref().map(|x| Box::new((**x).clone()))),
            ExpressionData::List(s) => ExpressionData::List(s.as_ref().map(|x| Box::new((**x).clone()))),
            ExpressionData::Literal(s) => ExpressionData::Literal(s.as_ref().map(|x| Box::new((**x).clone()))),
            ExpressionData::Symbol(s) => ExpressionData::Symbol(s.as_ref().map(|x| Box::new((**x).clone()))),
        }
    }
}

impl Clone for SymbolicExpNode {
    fn clone(&self) -> Self {
        SymbolicExpNode {
            expressions: self.expressions.as_ref().map(|e| Box::new((**e).clone())),
        }
    }
}

impl Clone for ListNode {
    fn clone(&self) -> Self {
        ListNode {
            expressions: self.expressions.as_ref().map(|e| Box::new((**e).clone())),
        }
    }
}

impl Clone for LiteralNode {
    fn clone(&self) -> Self {
        LiteralNode {
            type_: self.type_,
            value: self.value.clone(),
        }
    }
}

impl Clone for LiteralValue {
    fn clone(&self) -> Self {
        match self {
            LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
            LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
            LiteralValue::StringValue(s) => LiteralValue::StringValue(s.clone()),
        }
    }
}

impl Clone for SymbolNode {
    fn clone(&self) -> Self {
        SymbolNode {
            symbol_name: self.symbol_name.clone(),
        }
    }
}

impl Clone for ExpressionList {
    fn clone(&self) -> Self {
        ExpressionList {
            expression: self.expression.as_ref().map(|e| Box::new((**e).clone())),
            next: self.next.as_ref().map(|e| Box::new((**e).clone())),
        }
    }
}

fn new_object() -> Object {
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

fn empty_bindings() -> [Binding; MAX_BINDINGS] {
    [
        new_binding(), new_binding(), new_binding(), new_binding(), new_binding(),
        new_binding(), new_binding(), new_binding(), new_binding(), new_binding(),
    ]
}

fn empty_object_stack() -> [Option<Box<Object>>; OBJECT_NUMBER] {
    // OBJECT_NUMBER = 50
    [
        None, None, None, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, None,
        None, None, None, None, None, None, None, None, None, None,
    ]
}

// ============================================================
//   Tokenizer
// ============================================================

fn isop(ch: u8) -> bool {
    matches!(ch, b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>')
}

fn is_space_byte(ch: u8) -> bool {
    matches!(ch, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    match &state.token {
        Some(tok) if tok.kind == kind => 1,
        _ => 0,
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    // Skip whitespace
    while (state.pos as usize) < bytes.len() && is_space_byte(bytes[state.pos as usize]) {
        state.pos += 1;
    }

    let mut new_token = Token {
        kind: TokenKind::Eof,
        next: None,
        val: 0,
        str: String::new(),
    };

    let pos = state.pos as usize;
    if pos >= bytes.len() {
        new_token.kind = TokenKind::Eof;
        new_token.str = String::from("\0");
    } else {
        let ch = bytes[pos];
        if ch == b'(' {
            new_token.kind = TokenKind::LParen;
            new_token.str = String::from("(");
            state.pos += 1;
        } else if ch == b')' {
            new_token.kind = TokenKind::RParen;
            new_token.str = String::from(")");
            state.pos += 1;
        } else if ch == b'\'' {
            new_token.kind = TokenKind::Quote;
            new_token.str = String::from("'");
            state.pos += 1;
        } else if ch == 0 {
            new_token.kind = TokenKind::Eof;
            new_token.str = String::from("\0");
        } else if ch.is_ascii_alphabetic() || isop(ch) {
            // tokenize symbol
            let start = pos;
            while (state.pos as usize) < bytes.len()
                && (bytes[state.pos as usize].is_ascii_alphanumeric() || isop(bytes[state.pos as usize]))
            {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap_or("")
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
            let start = pos;
            while (state.pos as usize) < bytes.len() && bytes[state.pos as usize].is_ascii_digit() {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap_or("0");
            let val: i32 = s.parse().unwrap_or(0);
            new_token.kind = TokenKind::Digit;
            new_token.val = val;
        } else if ch == b'"' {
            state.pos += 1; // skip "
            let start = state.pos as usize;
            while (state.pos as usize) < bytes.len()
                && bytes[state.pos as usize] != b'"'
                && bytes[state.pos as usize] != 0
            {
                state.pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..state.pos as usize])
                .unwrap_or("")
                .to_string();
            new_token.kind = TokenKind::String;
            new_token.str = s;
            if (state.pos as usize) < bytes.len() && bytes[state.pos as usize] == b'"' {
                state.pos += 1;
            }
        } else if ch == b';' {
            // comment
            while (state.pos as usize) < bytes.len()
                && bytes[state.pos as usize] != b'\n'
                && bytes[state.pos as usize] != 0
            {
                state.pos += 1;
            }
            next(source, state);
            return;
        } else {
            panic!("Unexpected token: {}", ch as char);
        }
    }

    state.token = Some(Box::new(new_token));
}

// ============================================================
//   Parser
// ============================================================

fn append_expression_to_list(list: &mut Option<Box<ExpressionList>>, expression: Box<ExpressionNode>) {
    let new_node = Box::new(ExpressionList {
        expression: Some(expression),
        next: None,
    });
    if list.is_none() {
        *list = Some(new_node);
        return;
    }
    let mut current = list.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_node);
}

fn parse_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
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
        let s = state.token.as_ref().map(|t| t.str.clone()).unwrap_or_default();
        panic!("Unexpected token: {}", s);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut sexp = SymbolicExpNode { expressions: None };
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_expression_to_list(&mut sexp.expressions, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(sexp))),
    })
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut list = ListNode { expressions: None };
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_expression_to_list(&mut list.expressions, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(list))),
    })
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let symbol_name = state.token.as_ref().map(|t| t.str.clone()).unwrap_or_default();
    next(source, state);
    Box::new(ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name }))),
    })
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let lit = if match_token(state, TokenKind::Digit) == 1 {
        let val = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        next(source, state);
        LiteralNode {
            type_: LiteralType::Integer,
            value: LiteralValue::IntValue(val),
        }
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state.token.as_ref().map(|t| t.str.clone()).unwrap_or_default();
        next(source, state);
        LiteralNode {
            type_: LiteralType::String,
            value: LiteralValue::StringValue(s),
        }
    } else if match_token(state, TokenKind::True) == 1 {
        next(source, state);
        LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(true),
        }
    } else if match_token(state, TokenKind::False) == 1 {
        next(source, state);
        LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(false),
        }
    } else {
        let s = state.token.as_ref().map(|t| t.str.clone()).unwrap_or_default();
        panic!("Unexpected token: {}", s);
    };
    Box::new(ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(lit))),
    })
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut program = ProgramNode { expressions: None };
    while match_token(state, TokenKind::Eof) != 1 {
        let expr = parse_expression(source, state);
        append_expression_to_list(&mut program.expressions, expr);
    }
    result.program = Some(Box::new(program));
}

// ============================================================
//   Allocator (simplified: just creates new Objects)
// ============================================================

pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 0,
        stack: Some(Box::new(ObjectStack {
            objects: empty_object_stack(),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0u8; FREE_BITMAP_SIZE],
    }
}

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(new_object()))
}

// ============================================================
//   Env helpers
// ============================================================

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
    }
}

fn env_lookup(env: &Env, symbol_name: &str) -> Option<Object> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == symbol_name {
            return env.bindings[i].value.as_ref().map(|v| (**v).clone());
        }
    }
    if let Some(parent) = &env.parent {
        return env_lookup(parent, symbol_name);
    }
    None
}

fn env_set(env: &mut Env, symbol_name: &str, value: Object) {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name == symbol_name && !env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].value = Some(Box::new(value));
            return;
        }
        if env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].symbol_name = symbol_name.to_string();
            env.bindings[i].value = Some(Box::new(value));
            return;
        }
    }
}

// ============================================================
//   Helpers for Objects
// ============================================================

fn make_int(v: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(v),
    }
}

fn make_string(s: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(s),
    }
}

fn make_bool(b: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if b { 1 } else { 0 }),
    }
}

fn make_nil() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn obj_int_value(obj: &Object) -> i32 {
    match &obj.value {
        ObjectValue::IntValue(v) => *v,
        _ => 0,
    }
}

fn obj_bool_value(obj: &Object) -> i32 {
    match &obj.value {
        ObjectValue::BoolValue(v) => *v,
        _ => 0,
    }
}

fn obj_string_value(obj: &Object) -> String {
    match &obj.value {
        ObjectValue::StringValue(s) => s.clone(),
        _ => String::new(),
    }
}

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => obj_bool_value(obj) != 0,
        ObjectType::Nil => false,
        _ => true,
    }
}

fn obj_eq(op1: &Object, op2: &Object) -> bool {
    if !same_type(op1.type_, op2.type_) {
        return false;
    }
    match op1.type_ {
        ObjectType::Integer => obj_int_value(op1) == obj_int_value(op2),
        ObjectType::String => obj_string_value(op1) == obj_string_value(op2),
        ObjectType::Bool => obj_bool_value(op1) == obj_bool_value(op2),
        ObjectType::List => {
            // C compares pointers - but we compare by structure
            match (&op1.value, &op2.value) {
                (ObjectValue::ListValue(a), ObjectValue::ListValue(b)) => {
                    list_structurally_equal(a.as_deref(), b.as_deref())
                }
                _ => false,
            }
        }
        ObjectType::Nil => true,
        _ => false,
    }
}

fn list_structurally_equal(a: Option<&ConsCell>, b: Option<&ConsCell>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            let car_eq = match (&x.car, &y.car) {
                (Some(c1), Some(c2)) => obj_eq(c1, c2),
                (None, None) => true,
                _ => false,
            };
            if !car_eq {
                return false;
            }
            match (&x.cdr, &y.cdr) {
                (Some(c1), Some(c2)) => obj_eq(c1, c2),
                (None, None) => true,
                _ => false,
            }
        }
        _ => false,
    }
}

fn same_type(a: ObjectType, b: ObjectType) -> bool {
    matches!((a, b),
        (ObjectType::Integer, ObjectType::Integer) |
        (ObjectType::String, ObjectType::String) |
        (ObjectType::Bool, ObjectType::Bool) |
        (ObjectType::List, ObjectType::List) |
        (ObjectType::Nil, ObjectType::Nil) |
        (ObjectType::Function, ObjectType::Function))
}

fn is_last_cons_cell(cc: &ConsCell) -> bool {
    match &cc.cdr {
        Some(cdr) => matches!(cdr.type_, ObjectType::Nil),
        None => true,
    }
}

// ============================================================
//   stringifyObject
// ============================================================

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => format!("{}", obj_int_value(obj)),
        ObjectType::String => obj_string_value(obj),
        ObjectType::Bool => {
            if obj_bool_value(obj) != 0 {
                "T".to_string()
            } else {
                "F".to_string()
            }
        }
        ObjectType::List => {
            let mut s = String::from("(");
            if let ObjectValue::ListValue(Some(cc)) = &obj.value {
                let mut current: &ConsCell = cc;
                loop {
                    let car_str = match &current.car {
                        Some(c) => stringify_object(c),
                        None => String::new(),
                    };
                    s.push_str(&car_str);
                    if is_last_cons_cell(current) {
                        break;
                    }
                    s.push(' ');
                    match &current.cdr {
                        Some(cdr) => match &cdr.value {
                            ObjectValue::ListValue(Some(next_cc)) => current = next_cc,
                            _ => break,
                        },
                        None => break,
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

// ============================================================
//   Defined functions
// ============================================================

fn defined_add(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_int(obj_int_value(op1) + obj_int_value(op2));
        }
        (ObjectType::String, ObjectType::String) => {
            let s = format!("{}{}", obj_string_value(op1), obj_string_value(op2));
            *evaluated = make_string(s);
        }
        _ => panic!("Type error: operands for + must be integers or strings."),
    }
}

fn defined_sub(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_int(obj_int_value(op1) - obj_int_value(op2));
        }
        _ => panic!("Type error: operands for - must be integers."),
    }
}

fn defined_mul(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_int(obj_int_value(op1) * obj_int_value(op2));
        }
        _ => panic!("Type error: operands for * must be integers."),
    }
}

fn defined_div(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_int(obj_int_value(op1) / obj_int_value(op2));
        }
        _ => panic!("Type error: operands for / must be integers."),
    }
}

fn defined_mod(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_int(obj_int_value(op1) % obj_int_value(op2));
        }
        _ => panic!("Type error: operands for % must be integers."),
    }
}

fn defined_or(op1: &Object, op2: &Object, evaluated: &mut Object) {
    *evaluated = make_bool(bool_val(op1) || bool_val(op2));
}

fn defined_and(op1: &Object, op2: &Object, evaluated: &mut Object) {
    *evaluated = make_bool(bool_val(op1) && bool_val(op2));
}

fn defined_lt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_bool(obj_int_value(op1) < obj_int_value(op2));
        }
        _ => panic!("Type error: operands for < must be integers."),
    }
}

fn defined_gt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            *evaluated = make_bool(obj_int_value(op1) > obj_int_value(op2));
        }
        _ => panic!("Type error: operands for < must be integers."),
    }
}

fn defined_eq(op1: &Object, op2: &Object, evaluated: &mut Object) {
    *evaluated = make_bool(obj_eq(op1, op2));
}

fn defined_car(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: car operand must be list.");
    }
    if let ObjectValue::ListValue(Some(cc)) = &op.value {
        if let Some(car) = &cc.car {
            *evaluated = (**car).clone();
        }
    }
}

fn defined_cdr(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: cdr operand must be list.");
    }
    if let ObjectValue::ListValue(Some(cc)) = &op.value {
        if let Some(cdr) = &cc.cdr {
            *evaluated = (**cdr).clone();
        }
    }
}

fn defined_cons(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let op1_clone = op1.clone();
    match op2.type_ {
        ObjectType::List => {
            let cc = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1_clone)),
                cdr: Some(Box::new(op2.clone())),
            };
            evaluated.type_ = ObjectType::List;
            evaluated.value = ObjectValue::ListValue(Some(Box::new(cc)));
        }
        ObjectType::Nil => {
            let cc = ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(op1_clone)),
                cdr: Some(Box::new(op2.clone())),
            };
            evaluated.type_ = ObjectType::List;
            evaluated.value = ObjectValue::ListValue(Some(Box::new(cc)));
        }
        _ => {
            // wrap op2 in a list cell
            let inner_cc = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op2.clone())),
                cdr: Some(Box::new(make_nil())),
            };
            let inner_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(inner_cc))),
            };
            let cc = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(op1_clone)),
                cdr: Some(Box::new(inner_obj)),
            };
            evaluated.type_ = ObjectType::List;
            evaluated.value = ObjectValue::ListValue(Some(Box::new(cc)));
        }
    }
}

fn defined_not(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::Bool) {
        panic!("Type error: not operand must be boolean.");
    }
    *evaluated = make_bool(obj_bool_value(op) == 0);
}

fn defined_split(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        panic!("Type error: split first operand must be string.");
    }
    if !matches!(op2.type_, ObjectType::String) {
        panic!("Type error: split second operand must be string.");
    }
    let s1 = obj_string_value(op1);
    let s2 = obj_string_value(op2);

    let parts: Vec<String> = if s2.is_empty() {
        s1.chars().map(|c| c.to_string()).collect()
    } else {
        s1.split(&s2).map(|p| p.to_string()).collect()
    };

    if parts.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Build list from parts: each is a string
    let mut head: Option<Box<ConsCell>> = None;
    for part in parts.into_iter().rev() {
        let car = make_string(part);
        let cdr = match head {
            Some(prev_cc) => Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(prev_cc)),
            },
            None => make_nil(),
        };
        let cell_type = match &cdr.type_ {
            ObjectType::Nil => ConsCellType::Nil,
            _ => ConsCellType::Cell,
        };
        let new_cc = ConsCell {
            type_: cell_type,
            car: Some(Box::new(car)),
            cdr: Some(Box::new(cdr)),
        };
        head = Some(Box::new(new_cc));
    }
    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(head);
}

fn defined_list_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: list-ref first operand must be list.");
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        panic!("Type error: list-ref second operand must be integer.");
    }
    let index = obj_int_value(op2);
    let mut current = match &op1.value {
        ObjectValue::ListValue(Some(cc)) => &**cc,
        _ => panic!("Empty list"),
    };
    for _ in 0..index {
        match &current.cdr {
            Some(cdr) => match cdr.type_ {
                ObjectType::Nil => panic!("Index out of range."),
                ObjectType::List => match &cdr.value {
                    ObjectValue::ListValue(Some(next_cc)) => current = &**next_cc,
                    _ => panic!("Index out of range."),
                },
                _ => panic!("Index out of range."),
            },
            None => panic!("Index out of range."),
        }
    }
    if let Some(car) = &current.car {
        *evaluated = (**car).clone();
    }
}

fn defined_remove_whitespaces(op1: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        panic!("Type error: remove-whitespaces operand must be string.");
    }
    let s = obj_string_value(op1);
    let new_s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    *evaluated = make_string(new_s);
}

fn defined_pop(op: &Object, evaluated: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(op.type_, ObjectType::List) {
        panic!("Type error: pop operand must be list.");
    }
    // walk to last cons cell and return its car
    let mut current = match &op.value {
        ObjectValue::ListValue(Some(cc)) => &**cc,
        _ => return,
    };
    loop {
        if is_last_cons_cell(current) {
            if let Some(car) = &current.car {
                *evaluated = (**car).clone();
            }
            return;
        }
        match &current.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(Some(next_cc)) => current = &**next_cc,
                _ => return,
            },
            None => return,
        }
    }
}

fn defined_push(op1: &Object, op2: &Object, evaluated: &mut Object, env: &mut Env) {
    // op1 is the value, op2 is the list (but the C code passes them reversed, the call site knows)
    // Actually, looking at C code:
    //   definedFunctionPush(operand2, operand1, evaluated, env, context);
    // where operand1 is value to push and operand2 is the list.
    // The C signature is push(op1, op2, ...): op1 is the list, op2 is the value.
    // In our implementation here: op1 = list, op2 = value.

    if matches!(op1.type_, ObjectType::Nil) {
        // create new list with op2 as the only element, store back to env binding
        // If op1 was bound, replace its binding
        let new_cc = ConsCell {
            type_: ConsCellType::Nil,
            car: Some(Box::new(op2.clone())),
            cdr: Some(Box::new(make_nil())),
        };
        let new_list = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(new_cc))),
        };
        // Find binding whose value matches op1 (nil) -- since we don't have pointer identity, just find any nil
        for i in 0..MAX_BINDINGS {
            if env.bindings[i].symbol_name.is_empty() {
                break;
            }
            if let Some(v) = &env.bindings[i].value {
                if matches!(v.type_, ObjectType::Nil) {
                    env.bindings[i].value = Some(Box::new(new_list.clone()));
                    break;
                }
            }
        }
        *evaluated = op2.clone();
        return;
    }

    if !matches!(op1.type_, ObjectType::List) {
        panic!("Type error: push second operand must be list.");
    }

    // walk to last cons cell, append a new one
    fn append(cc: &mut ConsCell, value: Object) {
        if is_last_cons_cell(cc) {
            let new_cc = ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(value)),
                cdr: Some(Box::new(make_nil())),
            };
            let new_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(Box::new(new_cc))),
            };
            // Convert current to non-last
            cc.type_ = ConsCellType::Cell;
            cc.cdr = Some(Box::new(new_obj));
            return;
        }
        if let Some(cdr) = cc.cdr.as_mut() {
            if let ObjectValue::ListValue(Some(next_cc)) = &mut cdr.value {
                append(next_cc, value);
            }
        }
    }

    // We need to mutate the list. Since op1 is &Object (immutable), find it in the env and mutate.
    // Find the binding whose value equals op1 structurally and mutate that.
    let value_to_push = op2.clone();
    let mut mutated = false;
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        let matches_op1 = if let Some(v) = &env.bindings[i].value {
            matches!(v.type_, ObjectType::List) && obj_eq(v, op1)
        } else {
            false
        };
        if matches_op1 {
            if let Some(v) = env.bindings[i].value.as_mut() {
                if let ObjectValue::ListValue(Some(cc)) = &mut v.value {
                    append(cc, value_to_push.clone());
                    mutated = true;
                    break;
                }
            }
        }
    }
    if !mutated {
        // also check parent envs
        let mut current_env_opt = env.parent.as_mut();
        while let Some(parent_env) = current_env_opt {
            for i in 0..MAX_BINDINGS {
                if parent_env.bindings[i].symbol_name.is_empty() {
                    break;
                }
                let matches_op1 = if let Some(v) = &parent_env.bindings[i].value {
                    matches!(v.type_, ObjectType::List) && obj_eq(v, op1)
                } else {
                    false
                };
                if matches_op1 {
                    if let Some(v) = parent_env.bindings[i].value.as_mut() {
                        if let ObjectValue::ListValue(Some(cc)) = &mut v.value {
                            append(cc, value_to_push.clone());
                            mutated = true;
                            break;
                        }
                    }
                }
            }
            if mutated {
                break;
            }
            current_env_opt = parent_env.parent.as_mut();
        }
    }

    *evaluated = op2.clone();
}

fn defined_length(op: &Object, evaluated: &mut Object) {
    match op.type_ {
        ObjectType::Nil => {
            *evaluated = make_int(0);
        }
        ObjectType::List => {
            let mut length = 1i32;
            let mut current = match &op.value {
                ObjectValue::ListValue(Some(cc)) => &**cc,
                _ => {
                    *evaluated = make_int(0);
                    return;
                }
            };
            loop {
                if is_last_cons_cell(current) {
                    break;
                }
                length += 1;
                match &current.cdr {
                    Some(cdr) => match &cdr.value {
                        ObjectValue::ListValue(Some(next_cc)) => current = &**next_cc,
                        _ => break,
                    },
                    None => break,
                }
            }
            *evaluated = make_int(length);
        }
        ObjectType::String => {
            *evaluated = make_int(obj_string_value(op).len() as i32);
        }
        _ => panic!("Type error: length operand must be list or string."),
    }
}

fn defined_is_int_string(op: &Object, evaluated: &mut Object) {
    match op.type_ {
        ObjectType::String => {
            let s = obj_string_value(op);
            let result = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
            // C: returns true if all are digit, even if empty (loop body never runs if empty -> returns true)
            let result = s.chars().all(|c| c.is_ascii_digit());
            *evaluated = make_bool(result);
        }
        _ => {
            *evaluated = make_bool(false);
        }
    }
}

fn defined_parse_int(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        panic!("Type error: parse-int operand must be string.");
    }
    let s = obj_string_value(op);
    if !s.chars().all(|c| c.is_ascii_digit()) {
        panic!("Type error: parse-int operand must be string of digits.");
    }
    let v: i32 = s.parse().unwrap_or(0);
    *evaluated = make_int(v);
}

fn defined_string_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        panic!("Type error: string-ref first operand must be string.");
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        panic!("Type error: string-ref second operand must be integer.");
    }
    let s = obj_string_value(op1);
    let index = obj_int_value(op2);
    if index < 0 || (index as usize) >= s.len() {
        panic!("Index out of range.");
    }
    let ch = s.as_bytes()[index as usize] as char;
    *evaluated = make_string(ch.to_string());
}

// ============================================================
//   Evaluator
// ============================================================

fn evaluate_list_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions = match &expression.data {
        ExpressionData::List(Some(list_node)) => list_node.expressions.as_deref(),
        _ => None,
    };

    if expressions.is_none() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Build list of evaluated objects
    let mut items: Vec<Object> = Vec::new();
    let mut cur = expressions;
    while let Some(node) = cur {
        if let Some(expr) = &node.expression {
            let mut item = new_object();
            evaluate_expression(expr, &mut item, env, context);
            items.push(item);
        }
        cur = node.next.as_deref();
    }

    if items.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    // Build cons cell list
    let mut head: Option<Box<ConsCell>> = None;
    for item in items.into_iter().rev() {
        let cdr_obj = match head {
            Some(prev_cc) => Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(prev_cc)),
            },
            None => make_nil(),
        };
        let cell_type = match &cdr_obj.type_ {
            ObjectType::Nil => ConsCellType::Nil,
            _ => ConsCellType::Cell,
        };
        let new_cc = ConsCell {
            type_: cell_type,
            car: Some(Box::new(item)),
            cdr: Some(Box::new(cdr_obj)),
        };
        head = Some(Box::new(new_cc));
    }

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(head);
}

fn get_symbol_name(expr: &ExpressionNode) -> Option<String> {
    match &expr.data {
        ExpressionData::Symbol(Some(s)) => Some(s.symbol_name.clone()),
        _ => None,
    }
}

fn get_nth_expression(list: &ExpressionList, n: usize) -> Option<&ExpressionNode> {
    let mut current = Some(list);
    for _ in 0..n {
        current = current.and_then(|l| l.next.as_deref());
    }
    current.and_then(|l| l.expression.as_deref())
}

fn get_nth_list(list: &ExpressionList, n: usize) -> Option<&ExpressionList> {
    let mut current = Some(list);
    for _ in 0..n {
        current = current.and_then(|l| l.next.as_deref());
    }
    current
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    // C uses expression->data.list->expressions but for SYMBOLIC_EXP it should be symbolic_exp->expressions
    // The C code is buggy here - it accesses ->data.list->expressions but ListNode and SymbolicExpNode
    // are structurally identical (single 'expressions' field) so it works.
    let expressions_opt = match &expression.data {
        ExpressionData::SymbolicExp(Some(s)) => s.expressions.as_deref(),
        ExpressionData::List(Some(s)) => s.expressions.as_deref(),
        _ => None,
    };

    let expressions = match expressions_opt {
        Some(e) => e,
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let head_expr_opt = expressions.expression.as_deref();
    let head_expr = match head_expr_opt {
        Some(e) => e,
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
    };

    if !matches!(head_expr.type_, ExpressionType::Symbol) {
        panic!("S-exp must be started with symbol.");
    }

    let symbol_name = match get_symbol_name(head_expr) {
        Some(s) => s,
        None => panic!("S-exp must be started with symbol."),
    };

    match symbol_name.as_str() {
        "if" => {
            let cond = get_nth_expression(expressions, 1).expect("if must have condition.");
            let then_expr = get_nth_expression(expressions, 2).expect("if must have then clause.");
            let mut cond_obj = new_object();
            evaluate_expression(cond, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(then_expr, evaluated, env, context);
            } else if let Some(else_expr) = get_nth_expression(expressions, 3) {
                evaluate_expression(else_expr, evaluated, env, context);
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            let cond_expr = get_nth_expression(expressions, 1).expect("while condition");
            let body_expr = get_nth_expression(expressions, 2).expect("while body");
            // Need owned copies to avoid borrow issues
            let cond_clone = cond_expr.clone();
            let body_clone = body_expr.clone();
            loop {
                let mut cond_obj = new_object();
                evaluate_expression(&cond_clone, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(&body_clone, evaluated, env, context);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            let sym_expr = get_nth_expression(expressions, 1).expect("variable");
            if !matches!(sym_expr.type_, ExpressionType::Symbol) {
                panic!("Variable name must be symbol.");
            }
            let var_name = get_symbol_name(sym_expr).unwrap();
            let value_expr = get_nth_expression(expressions, 2).expect("assignment value");
            let mut value_obj = new_object();
            evaluate_expression(value_expr, &mut value_obj, env, context);
            *evaluated = value_obj.clone();
            env_set(env, &var_name, value_obj);
        }
        "defun" => {
            let name_expr = get_nth_expression(expressions, 1).expect("function name");
            if !matches!(name_expr.type_, ExpressionType::Symbol) {
                panic!("Function name must be symbol.");
            }
            let func_name = get_symbol_name(name_expr).unwrap();
            let params_expr = get_nth_expression(expressions, 2).expect("function params");
            if !matches!(params_expr.type_, ExpressionType::SymbolicExp) {
                panic!("Function parameter must be list.");
            }
            let params_list_opt = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(s)) => s.expressions.as_deref(),
                _ => None,
            };
            let mut param_names: Vec<String> = Vec::new();
            let mut cur = params_list_opt;
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    if !matches!(e.type_, ExpressionType::Symbol) {
                        panic!("Function parameter must be symbol.");
                    }
                    param_names.push(get_symbol_name(e).unwrap());
                }
                cur = node.next.as_deref();
            }
            let body_expr = get_nth_expression(expressions, 3).expect("function body");
            let function = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body_expr.clone())),
            };
            evaluated.type_ = ObjectType::Function;
            evaluated.value = ObjectValue::FunctionValue(Some(Box::new(function)));
            let evaluated_clone = evaluated.clone();
            env_set(env, &func_name, evaluated_clone);
        }
        "+" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_add(&op1, &op2, evaluated);
        }
        "-" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_sub(&op1, &op2, evaluated);
        }
        "*" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_mul(&op1, &op2, evaluated);
        }
        "/" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_div(&op1, &op2, evaluated);
        }
        "%" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_mod(&op1, &op2, evaluated);
        }
        "||" => {
            let mut cur = get_nth_list(expressions, 1);
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let mut op = new_object();
                    evaluate_expression(e, &mut op, env, context);
                    if bool_val(&op) {
                        *evaluated = make_bool(true);
                        return;
                    }
                }
                cur = node.next.as_deref();
            }
            *evaluated = make_bool(false);
        }
        "&&" => {
            let mut cur = get_nth_list(expressions, 1);
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let mut op = new_object();
                    evaluate_expression(e, &mut op, env, context);
                    if !bool_val(&op) {
                        *evaluated = make_bool(false);
                        return;
                    }
                }
                cur = node.next.as_deref();
            }
            *evaluated = make_bool(true);
        }
        "<" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_lt(&op1, &op2, evaluated);
        }
        ">" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_gt(&op1, &op2, evaluated);
        }
        "eq" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_eq(&op1, &op2, evaluated);
        }
        "not" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_not(&op, evaluated);
        }
        "print" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            let s = stringify_object(&op);
            println!("{}", s);
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_car(&op, evaluated);
        }
        "cdr" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_cdr(&op, evaluated);
        }
        "cons" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_cons(&op1, &op2, evaluated);
        }
        "readline" => {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                        if line.ends_with('\r') {
                            line.pop();
                        }
                    }
                    *evaluated = make_string(line);
                }
                Err(_) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            }
        }
        "split" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_split(&op1, &op2, evaluated);
        }
        "list-ref" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_list_ref(&op1, &op2, evaluated);
        }
        "progn" => {
            let mut cur = get_nth_list(expressions, 1);
            let mut last: Option<Object> = None;
            while let Some(node) = cur {
                if let Some(e) = &node.expression {
                    let mut op = new_object();
                    evaluate_expression(e, &mut op, env, context);
                    last = Some(op);
                }
                cur = node.next.as_deref();
            }
            match last {
                Some(o) => *evaluated = o,
                None => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            }
        }
        "remove-whitespaces" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_remove_whitespaces(&op, evaluated);
        }
        "pop" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_pop(&op, evaluated);
        }
        "push" => {
            // C: definedFunctionPush(operand2, operand1, evaluated, env, context)
            // where operand1 = expressions->next->next (second arg)
            //       operand2 = expressions->next (first arg)
            // And signature is push(op1, op2, ...): op1 = list (first arg in source), op2 = value (second arg)
            // Looking at C call:
            //   evaluateExpression(expressions->next->next->expression, operand1, ...);  -> operand1 is value
            //   evaluateExpression(expressions->next->expression, operand2, ...);        -> operand2 is list
            //   definedFunctionPush(operand2, operand1, ...)                              -> push(list, value, ...)
            // So our defined_push takes (list, value).
            let mut value_obj = new_object();
            let mut list_obj = new_object();
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut value_obj, env, context);
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut list_obj, env, context);
            defined_push(&list_obj, &value_obj, evaluated, env);
        }
        "length" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_length(&op, evaluated);
        }
        "is-int-string" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_is_int_string(&op, evaluated);
        }
        "parse-int" => {
            let mut op = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op, env, context);
            defined_parse_int(&op, evaluated);
        }
        "string-ref" => {
            let mut op1 = new_object();
            let mut op2 = new_object();
            evaluate_expression(get_nth_expression(expressions, 1).unwrap(), &mut op1, env, context);
            evaluate_expression(get_nth_expression(expressions, 2).unwrap(), &mut op2, env, context);
            defined_string_ref(&op1, &op2, evaluated);
        }
        _ => {
            // user-defined function call
            let func_obj = match env_lookup(env, &symbol_name) {
                Some(o) => o,
                None => panic!("Undefined function: {}", symbol_name),
            };
            if !matches!(func_obj.type_, ObjectType::Function) {
                panic!("Undefined function: {}", symbol_name);
            }
            let function = match &func_obj.value {
                ObjectValue::FunctionValue(Some(f)) => (**f).clone(),
                _ => panic!("Undefined function: {}", symbol_name),
            };
            // Evaluate args in current env
            let mut new_env = Env {
                bindings: empty_bindings(),
                parent: None,
            };
            init_env(&mut new_env);
            // Use parent = current env (cloned) for variable lookup
            // Note: functions may need access to parent env's bindings
            // Build new_env with bindings for params
            let mut param_expr_list = get_nth_list(expressions, 1);
            for (j, param_name) in function.param_symbol_names.iter().enumerate() {
                let _ = j;
                let arg_expr = match param_expr_list {
                    Some(node) => node.expression.as_deref().expect("param expr"),
                    None => panic!("Not enough arguments for function {}", symbol_name),
                };
                let mut param = new_object();
                evaluate_expression(arg_expr, &mut param, env, context);
                env_set(&mut new_env, param_name, param);
                param_expr_list = param_expr_list.and_then(|l| l.next.as_deref());
            }
            // Set parent to a clone of the current env so functions can see outer bindings
            new_env.parent = Some(Box::new(clone_env(env)));
            let body = function.body.as_deref().expect("function body");
            evaluate_expression(body, evaluated, &mut new_env, context);
        }
    }
}

fn clone_env(env: &Env) -> Env {
    let mut new_bindings = empty_bindings();
    for i in 0..MAX_BINDINGS {
        new_bindings[i] = Binding {
            symbol_name: env.bindings[i].symbol_name.clone(),
            value: env.bindings[i].value.as_ref().map(|v| Box::new((**v).clone())),
        };
    }
    Env {
        bindings: new_bindings,
        parent: env.parent.as_ref().map(|p| Box::new(clone_env(p))),
    }
}

fn evaluate_literal_expression(expression: &ExpressionNode, evaluated: &mut Object) {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                if let LiteralValue::IntValue(v) = &lit.value {
                    *evaluated = make_int(*v);
                }
            }
            LiteralType::String => {
                if let LiteralValue::StringValue(s) = &lit.value {
                    *evaluated = make_string(s.clone());
                }
            }
            LiteralType::Boolean => {
                if let LiteralValue::BooleanValue(b) = &lit.value {
                    *evaluated = make_bool(*b);
                }
            }
        }
    }
}

fn evaluate_symbol_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    _context: &mut AllocatorContext,
) {
    let symbol_name = match &expression.data {
        ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
        _ => return,
    };
    if symbol_name == "nil" {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    match env_lookup(env, &symbol_name) {
        Some(obj) => {
            *evaluated = obj;
        }
        None => panic!("Undefined symbol: {}", symbol_name),
    }
}

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
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
    let program = match &result.program {
        Some(p) => p,
        None => return,
    };
    let mut env = Env {
        bindings: empty_bindings(),
        parent: None,
    };
    init_env(&mut env);
    let mut context = init_allocator();

    let mut cur = program.expressions.as_deref();
    let exprs: Vec<ExpressionNode> = {
        let mut v = Vec::new();
        while let Some(node) = cur {
            if let Some(e) = &node.expression {
                v.push((**e).clone());
            }
            cur = node.next.as_deref();
        }
        v
    };

    for expr in exprs {
        let mut evaluated = new_object();
        evaluate_expression(&expr, &mut evaluated, &mut env, &mut context);
    }
}
