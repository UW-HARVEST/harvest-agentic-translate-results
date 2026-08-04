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

// =================================================
//   Default implementations
// =================================================

impl Default for Binding {
    fn default() -> Self {
        Binding {
            symbol_name: String::new(),
            value: None,
        }
    }
}

impl Default for Env {
    fn default() -> Self {
        Env {
            bindings: std::array::from_fn(|_| Binding::default()),
            parent: None,
        }
    }
}

impl Default for Object {
    fn default() -> Self {
        Object {
            marked: false,
            type_: ObjectType::Nil,
            value: ObjectValue::IntValue(0),
        }
    }
}

impl Default for Token {
    fn default() -> Self {
        Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: String::new(),
        }
    }
}

impl Default for ParseState {
    fn default() -> Self {
        ParseState {
            token: None,
            pos: 0,
        }
    }
}

impl Default for ParseResult {
    fn default() -> Self {
        ParseResult { program: None }
    }
}

impl Default for ObjectStack {
    fn default() -> Self {
        ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        }
    }
}

// =================================================
//   Clone implementations (manual, not derived)
// =================================================

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
            ExpressionData::SymbolicExp(s) => ExpressionData::SymbolicExp(
                s.as_ref().map(|x| Box::new((**x).clone())),
            ),
            ExpressionData::List(l) => {
                ExpressionData::List(l.as_ref().map(|x| Box::new((**x).clone())))
            }
            ExpressionData::Literal(l) => {
                ExpressionData::Literal(l.as_ref().map(|x| Box::new((**x).clone())))
            }
            ExpressionData::Symbol(s) => {
                ExpressionData::Symbol(s.as_ref().map(|x| Box::new((**x).clone())))
            }
        }
    }
}

impl Clone for SymbolicExpNode {
    fn clone(&self) -> Self {
        SymbolicExpNode {
            expressions: self.expressions.as_ref().map(|x| Box::new((**x).clone())),
        }
    }
}

impl Clone for ListNode {
    fn clone(&self) -> Self {
        ListNode {
            expressions: self.expressions.as_ref().map(|x| Box::new((**x).clone())),
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
            LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
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
            expression: self.expression.as_ref().map(|x| Box::new((**x).clone())),
            next: self.next.as_ref().map(|x| Box::new((**x).clone())),
        }
    }
}

impl Clone for Function {
    fn clone(&self) -> Self {
        Function {
            param_symbol_names: self.param_symbol_names.clone(),
            body: self.body.as_ref().map(|x| Box::new((**x).clone())),
        }
    }
}

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
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(v) => {
                ObjectValue::ListValue(v.as_ref().map(|x| Box::new((**x).clone())))
            }
            ObjectValue::FunctionValue(v) => {
                ObjectValue::FunctionValue(v.as_ref().map(|x| Box::new((**x).clone())))
            }
        }
    }
}

impl Clone for ConsCell {
    fn clone(&self) -> Self {
        ConsCell {
            type_: self.type_,
            car: self.car.as_ref().map(|x| Box::new((**x).clone())),
            cdr: self.cdr.as_ref().map(|x| Box::new((**x).clone())),
        }
    }
}

// =================================================
//   Helper functions
// =================================================

fn is_op(ch: u8) -> bool {
    matches!(
        ch,
        b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>'
    )
}

fn make_nil_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn make_int_object(v: i32) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Integer,
        value: ObjectValue::IntValue(v),
    }
}

fn make_bool_object(v: bool) -> Object {
    Object {
        marked: false,
        type_: ObjectType::Bool,
        value: ObjectValue::BoolValue(if v { 1 } else { 0 }),
    }
}

fn make_string_object(s: String) -> Object {
    Object {
        marked: false,
        type_: ObjectType::String,
        value: ObjectValue::StringValue(s),
    }
}

fn append_expression_to_list_node(list: &mut ListNode, expression: ExpressionNode) {
    let new_node = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });
    if list.expressions.is_none() {
        list.expressions = Some(new_node);
        return;
    }
    let mut current = list.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_node);
}

fn append_expression_to_symbolic_node(
    sym: &mut SymbolicExpNode,
    expression: ExpressionNode,
) {
    let new_node = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });
    if sym.expressions.is_none() {
        sym.expressions = Some(new_node);
        return;
    }
    let mut current = sym.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_node);
}

fn append_expression_to_program(program: &mut ProgramNode, expression: ExpressionNode) {
    let new_node = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });
    if program.expressions.is_none() {
        program.expressions = Some(new_node);
        return;
    }
    let mut current = program.expressions.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_node);
}

// =================================================
//   Tokenizer
// =================================================

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(ref tok) = state.token {
        if tok.kind == kind {
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
    let mut pos = state.pos as usize;

    // Skip whitespaces
    while pos < bytes.len() && (bytes[pos].is_ascii_whitespace() || bytes[pos] == b'\n') {
        pos += 1;
    }

    // EOF
    if pos >= bytes.len() {
        state.pos = pos as i32;
        state.token = Some(Box::new(Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: "\0".to_string(),
        }));
        return;
    }

    let ch = bytes[pos];

    let new_token = if ch == b'(' {
        pos += 1;
        Token {
            kind: TokenKind::LParen,
            next: None,
            val: 0,
            str: "(".to_string(),
        }
    } else if ch == b')' {
        pos += 1;
        Token {
            kind: TokenKind::RParen,
            next: None,
            val: 0,
            str: ")".to_string(),
        }
    } else if ch == b'\'' {
        pos += 1;
        Token {
            kind: TokenKind::Quote,
            next: None,
            val: 0,
            str: "'".to_string(),
        }
    } else if ch.is_ascii_alphabetic() || is_op(ch) {
        let start = pos;
        while pos < bytes.len()
            && (bytes[pos].is_ascii_alphanumeric() || is_op(bytes[pos]))
        {
            pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..pos])
            .unwrap_or("")
            .to_string();
        if s == "true" {
            Token {
                kind: TokenKind::True,
                next: None,
                val: 0,
                str: String::new(),
            }
        } else if s == "false" {
            Token {
                kind: TokenKind::False,
                next: None,
                val: 0,
                str: String::new(),
            }
        } else {
            Token {
                kind: TokenKind::Symbol,
                next: None,
                val: 0,
                str: s,
            }
        }
    } else if ch.is_ascii_digit() {
        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..pos]).unwrap_or("0");
        let val: i32 = s.parse().unwrap_or(0);
        Token {
            kind: TokenKind::Digit,
            next: None,
            val,
            str: String::new(),
        }
    } else if ch == b'"' {
        pos += 1; // skip opening quote
        let start = pos;
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..pos])
            .unwrap_or("")
            .to_string();
        if pos < bytes.len() && bytes[pos] == b'"' {
            pos += 1; // skip closing quote
        }
        Token {
            kind: TokenKind::String,
            next: None,
            val: 0,
            str: s,
        }
    } else if ch == b';' {
        // Comment, skip until end of line
        while pos < bytes.len() && bytes[pos] != b'\n' {
            pos += 1;
        }
        state.pos = pos as i32;
        next(source, state);
        return;
    } else {
        eprintln!("Unexpected token: {}", ch as char);
        std::process::exit(1);
    };

    state.pos = pos as i32;
    state.token = Some(Box::new(new_token));
}

// =================================================
//   Parser
// =================================================

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut sym_node = Box::new(SymbolicExpNode { expressions: None });
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        if match_token(state, TokenKind::Eof) == 1 {
            break;
        }
        let expr_item = parse_expression(source, state);
        append_expression_to_symbolic_node(&mut sym_node, expr_item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(sym_node)),
    }
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let mut list_node = Box::new(ListNode { expressions: None });
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) == 0 {
        if match_token(state, TokenKind::Eof) == 1 {
            break;
        }
        let expr_item = parse_expression(source, state);
        append_expression_to_list_node(&mut list_node, expr_item);
    }
    next(source, state); // eat ')'
    ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(list_node)),
    }
}

fn parse_symbol_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let symbol_name = state
        .token
        .as_ref()
        .map(|t| t.str.clone())
        .unwrap_or_default();
    let node = Box::new(SymbolNode { symbol_name });
    next(source, state);
    ExpressionNode {
        type_: ExpressionType::Symbol,
        data: ExpressionData::Symbol(Some(node)),
    }
}

fn parse_literal_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    let literal_node = if match_token(state, TokenKind::Digit) == 1 {
        let val = state.token.as_ref().map(|t| t.val).unwrap_or(0);
        Box::new(LiteralNode {
            type_: LiteralType::Integer,
            value: LiteralValue::IntValue(val),
        })
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state
            .token
            .as_ref()
            .map(|t| t.str.clone())
            .unwrap_or_default();
        Box::new(LiteralNode {
            type_: LiteralType::String,
            value: LiteralValue::StringValue(s),
        })
    } else if match_token(state, TokenKind::True) == 1 {
        Box::new(LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(true),
        })
    } else if match_token(state, TokenKind::False) == 1 {
        Box::new(LiteralNode {
            type_: LiteralType::Boolean,
            value: LiteralValue::BooleanValue(false),
        })
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
    ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(literal_node)),
    }
}

fn parse_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
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
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    }
}

fn parse_program(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state); // first token
    let mut program = Box::new(ProgramNode { expressions: None });

    while match_token(state, TokenKind::Eof) == 0 {
        let expression = parse_expression(source, state);
        append_expression_to_program(&mut program, expression);
    }

    result.program = Some(program);
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    parse_program(source, state, result);
}

// =================================================
//   Allocator (simplified for Rust)
// =================================================

pub fn init_allocator() -> AllocatorContext {
    AllocatorContext {
        gc_less_mode: 1,
        stack: Some(Box::new(ObjectStack {
            objects: std::array::from_fn(|_| None),
            top: -1,
        })),
        memory_pool: None,
        free_bitmap: [0u8; FREE_BITMAP_SIZE],
    }
}

pub fn allocate(_context: &mut AllocatorContext, _env: &mut Env) -> Option<Box<Object>> {
    Some(Box::new(make_nil_object()))
}

// =================================================
//   Stringify
// =================================================

fn stringify_object_inner(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => {
            if let ObjectValue::IntValue(v) = obj.value {
                v.to_string()
            } else {
                String::from("0")
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
            if let ObjectValue::BoolValue(v) = obj.value {
                if v != 0 {
                    String::from("T")
                } else {
                    String::from("F")
                }
            } else {
                String::from("F")
            }
        }
        ObjectType::List => {
            let mut result = String::from("(");
            if let ObjectValue::ListValue(Some(ref cell)) = obj.value {
                let items = cons_cell_to_vec(cell);
                let parts: Vec<String> =
                    items.iter().map(stringify_object_inner).collect();
                result.push_str(&parts.join(" "));
            }
            result.push(')');
            result
        }
        ObjectType::Nil => String::from("nil"),
        ObjectType::Function => String::from("<function>"),
    }
}

pub fn stringify_object(obj: &Object) -> String {
    stringify_object_inner(obj)
}

// =================================================
//   List <-> Vec helpers
// =================================================

fn cons_cell_to_vec(cell: &ConsCell) -> Vec<Object> {
    let mut result = vec![];
    if let Some(car) = cell.car.as_ref() {
        result.push((**car).clone());
    }
    let mut current_cdr = cell.cdr.as_deref();
    loop {
        match current_cdr {
            None => break,
            Some(cdr_obj) => {
                if matches!(cdr_obj.type_, ObjectType::Nil) {
                    break;
                }
                if let ObjectValue::ListValue(Some(ref next_cell)) = cdr_obj.value {
                    if let Some(car) = next_cell.car.as_ref() {
                        result.push((**car).clone());
                    }
                    current_cdr = next_cell.cdr.as_deref();
                } else {
                    break;
                }
            }
        }
    }
    result
}

fn vec_to_cons_cell(items: Vec<Object>) -> Option<Box<ConsCell>> {
    let mut iter = items.into_iter().rev();
    let last = iter.next()?;

    let mut current_cell = Box::new(ConsCell {
        type_: ConsCellType::Nil,
        car: Some(Box::new(last)),
        cdr: Some(Box::new(make_nil_object())),
    });

    for item in iter {
        let list_obj = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(current_cell)),
        };
        current_cell = Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(Box::new(item)),
            cdr: Some(Box::new(list_obj)),
        });
    }

    Some(current_cell)
}

fn vec_to_list_object(items: Vec<Object>) -> Object {
    if items.is_empty() {
        return make_nil_object();
    }
    let cell = vec_to_cons_cell(items);
    Object {
        marked: false,
        type_: ObjectType::List,
        value: ObjectValue::ListValue(cell),
    }
}

// =================================================
//   Boolean / equality helpers
// =================================================

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => {
            if let ObjectValue::BoolValue(v) = obj.value {
                v != 0
            } else {
                false
            }
        }
        ObjectType::Nil => false,
        _ => true,
    }
}

fn objects_eq(op1: &Object, op2: &Object) -> bool {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
                (&op1.value, &op2.value)
            {
                a == b
            } else {
                false
            }
        }
        (ObjectType::String, ObjectType::String) => {
            if let (ObjectValue::StringValue(a), ObjectValue::StringValue(b)) =
                (&op1.value, &op2.value)
            {
                a == b
            } else {
                false
            }
        }
        (ObjectType::Bool, ObjectType::Bool) => {
            if let (ObjectValue::BoolValue(a), ObjectValue::BoolValue(b)) =
                (&op1.value, &op2.value)
            {
                (*a != 0) == (*b != 0)
            } else {
                false
            }
        }
        (ObjectType::Nil, ObjectType::Nil) => true,
        (ObjectType::List, ObjectType::List) => false,
        _ => false,
    }
}

// =================================================
//   Env helpers
// =================================================

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for binding in env.bindings.iter_mut() {
        binding.symbol_name = String::new();
        binding.value = None;
    }
}

fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: Object) {
    // Try to find existing binding
    for binding in env.bindings.iter_mut() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == symbol_name {
            binding.value = Some(Box::new(obj));
            return;
        }
    }
    // Add new binding at the first empty slot
    for binding in env.bindings.iter_mut() {
        if binding.symbol_name.is_empty() {
            binding.symbol_name = symbol_name.to_string();
            binding.value = Some(Box::new(obj));
            return;
        }
    }
}

fn lookup_in_env(env: &Env, symbol_name: &str) -> Option<Object> {
    for binding in env.bindings.iter() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == symbol_name {
            return binding.value.as_ref().map(|v| (**v).clone());
        }
    }
    if let Some(ref parent) = env.parent {
        return lookup_in_env(parent, symbol_name);
    }
    None
}

fn take_from_env(env: &mut Env, symbol_name: &str) -> Option<Object> {
    for binding in env.bindings.iter_mut() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == symbol_name {
            return binding.value.take().map(|b| *b);
        }
    }
    if let Some(parent) = env.parent.as_mut() {
        return take_from_env(parent, symbol_name);
    }
    None
}

fn store_to_existing_binding(env: &mut Env, symbol_name: &str, obj: Object) -> bool {
    for binding in env.bindings.iter_mut() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == symbol_name {
            binding.value = Some(Box::new(obj));
            return true;
        }
    }
    if let Some(parent) = env.parent.as_mut() {
        return store_to_existing_binding(parent, symbol_name, obj);
    }
    false
}

// =================================================
//   Defined functions
// =================================================

fn defined_function_add(op1: &Object, op2: &Object, evaluated: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
                (&op1.value, &op2.value)
            {
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(a.wrapping_add(*b));
            }
        }
        (ObjectType::String, ObjectType::String) => {
            if let (ObjectValue::StringValue(a), ObjectValue::StringValue(b)) =
                (&op1.value, &op2.value)
            {
                evaluated.type_ = ObjectType::String;
                evaluated.value = ObjectValue::StringValue(format!("{}{}", a, b));
            }
        }
        _ => {
            eprintln!("Type error: operands for + must be integers or strings.");
            std::process::exit(1);
        }
    }
}

fn defined_function_sub(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
        (&op1.value, &op2.value)
    {
        if matches!(op1.type_, ObjectType::Integer)
            && matches!(op2.type_, ObjectType::Integer)
        {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_sub(*b));
            return;
        }
    }
    eprintln!("Type error: operands for - must be integers.");
    std::process::exit(1);
}

fn defined_function_mul(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
        (&op1.value, &op2.value)
    {
        if matches!(op1.type_, ObjectType::Integer)
            && matches!(op2.type_, ObjectType::Integer)
        {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a.wrapping_mul(*b));
            return;
        }
    }
    eprintln!("Type error: operands for * must be integers.");
    std::process::exit(1);
}

fn defined_function_div(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
        (&op1.value, &op2.value)
    {
        if matches!(op1.type_, ObjectType::Integer)
            && matches!(op2.type_, ObjectType::Integer)
        {
            if *b == 0 {
                eprintln!("Division by zero.");
                std::process::exit(1);
            }
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a / b);
            return;
        }
    }
    eprintln!("Type error: operands for / must be integers.");
    std::process::exit(1);
}

fn defined_function_mod(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
        (&op1.value, &op2.value)
    {
        if matches!(op1.type_, ObjectType::Integer)
            && matches!(op2.type_, ObjectType::Integer)
        {
            if *b == 0 {
                eprintln!("Modulo by zero.");
                std::process::exit(1);
            }
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(a % b);
            return;
        }
    }
    eprintln!("Type error: operands for % must be integers.");
    std::process::exit(1);
}

fn defined_function_lt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
        (&op1.value, &op2.value)
    {
        if matches!(op1.type_, ObjectType::Integer)
            && matches!(op2.type_, ObjectType::Integer)
        {
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a < b { 1 } else { 0 });
            return;
        }
    }
    eprintln!("Type error: operands for < must be integers.");
    std::process::exit(1);
}

fn defined_function_gt(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if let (ObjectValue::IntValue(a), ObjectValue::IntValue(b)) =
        (&op1.value, &op2.value)
    {
        if matches!(op1.type_, ObjectType::Integer)
            && matches!(op2.type_, ObjectType::Integer)
        {
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(if a > b { 1 } else { 0 });
            return;
        }
    }
    eprintln!("Type error: operands for > must be integers.");
    std::process::exit(1);
}

fn defined_function_eq(op1: &Object, op2: &Object, evaluated: &mut Object) {
    let eq = objects_eq(op1, op2);
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if eq { 1 } else { 0 });
}

fn defined_function_not(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::Bool) {
        eprintln!("Type error: not operand must be boolean.");
        std::process::exit(1);
    }
    let v = if let ObjectValue::BoolValue(v) = op.value {
        v != 0
    } else {
        false
    };
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if v { 0 } else { 1 });
}

fn defined_function_car(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: car operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(ref cell)) = op.value {
        if let Some(car) = cell.car.as_ref() {
            *evaluated = (**car).clone();
        }
    }
}

fn defined_function_cdr(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: cdr operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(ref cell)) = op.value {
        if let Some(cdr) = cell.cdr.as_ref() {
            *evaluated = (**cdr).clone();
        }
    }
}

fn defined_function_cons(op1: &Object, op2: &Object, evaluated: &mut Object) {
    // Build the cons cell with car=op1
    let car_box = Box::new(op1.clone());

    let cell = match op2.type_ {
        ObjectType::List => Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(car_box),
            cdr: Some(Box::new(op2.clone())),
        }),
        ObjectType::Nil => Box::new(ConsCell {
            type_: ConsCellType::Nil,
            car: Some(car_box),
            cdr: Some(Box::new(op2.clone())),
        }),
        _ => {
            // Build list of two elements
            let inner_cell = Box::new(ConsCell {
                type_: ConsCellType::Nil,
                car: Some(Box::new(op2.clone())),
                cdr: Some(Box::new(make_nil_object())),
            });
            let cdr_obj = Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(inner_cell)),
            };
            Box::new(ConsCell {
                type_: ConsCellType::Cell,
                car: Some(car_box),
                cdr: Some(Box::new(cdr_obj)),
            })
        }
    };

    evaluated.type_ = ObjectType::List;
    evaluated.value = ObjectValue::ListValue(Some(cell));
}

fn defined_function_split(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: split first operand must be string.");
        std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::String) {
        eprintln!("Type error: split second operand must be string.");
        std::process::exit(1);
    }
    let s = if let ObjectValue::StringValue(ref v) = op1.value {
        v.clone()
    } else {
        String::new()
    };
    let sep = if let ObjectValue::StringValue(ref v) = op2.value {
        v.clone()
    } else {
        String::new()
    };

    let parts: Vec<Object> = if sep.is_empty() {
        s.chars()
            .map(|c| make_string_object(c.to_string()))
            .collect()
    } else {
        s.split(sep.as_str())
            .filter(|s| !s.is_empty())
            .map(|p| make_string_object(p.to_string()))
            .collect()
    };

    *evaluated = vec_to_list_object(parts);
}

fn defined_function_list_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        eprintln!("Type error: list-ref first operand must be list.");
        std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        eprintln!("Type error: list-ref second operand must be integer.");
        std::process::exit(1);
    }
    let index = if let ObjectValue::IntValue(v) = op2.value {
        v
    } else {
        0
    };
    if let ObjectValue::ListValue(Some(ref cell)) = op1.value {
        let items = cons_cell_to_vec(cell);
        if index < 0 || (index as usize) >= items.len() {
            eprintln!("Index out of range.");
            std::process::exit(1);
        }
        *evaluated = items[index as usize].clone();
    }
}

fn defined_function_remove_whitespaces(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        eprintln!("Type error: remove-whitespaces operand must be string.");
        std::process::exit(1);
    }
    if let ObjectValue::StringValue(ref s) = op.value {
        let new_s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        evaluated.type_ = ObjectType::String;
        evaluated.value = ObjectValue::StringValue(new_s);
    }
}

fn defined_function_length(op: &Object, evaluated: &mut Object) {
    match op.type_ {
        ObjectType::Nil => {
            evaluated.type_ = ObjectType::Integer;
            evaluated.value = ObjectValue::IntValue(0);
        }
        ObjectType::List => {
            if let ObjectValue::ListValue(Some(ref cell)) = op.value {
                let items = cons_cell_to_vec(cell);
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(items.len() as i32);
            } else {
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        ObjectType::String => {
            if let ObjectValue::StringValue(ref s) = op.value {
                evaluated.type_ = ObjectType::Integer;
                evaluated.value = ObjectValue::IntValue(s.len() as i32);
            }
        }
        _ => {
            eprintln!("Type error: length operand must be list or string.");
            std::process::exit(1);
        }
    }
}

fn defined_function_is_int_string(op: &Object, evaluated: &mut Object) {
    let result = if let ObjectType::String = op.type_ {
        if let ObjectValue::StringValue(ref s) = op.value {
            !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
        } else {
            false
        }
    } else {
        false
    };
    evaluated.type_ = ObjectType::Bool;
    evaluated.value = ObjectValue::BoolValue(if result { 1 } else { 0 });
}

fn defined_function_parse_int(op: &Object, evaluated: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        eprintln!("Type error: parse-int operand must be string.");
        std::process::exit(1);
    }
    if let ObjectValue::StringValue(ref s) = op.value {
        if !s.chars().all(|c| c.is_ascii_digit()) {
            eprintln!("Type error: parse-int operand must be string of digits.");
            std::process::exit(1);
        }
        let v: i32 = s.parse().unwrap_or(0);
        evaluated.type_ = ObjectType::Integer;
        evaluated.value = ObjectValue::IntValue(v);
    }
}

fn defined_function_string_ref(op1: &Object, op2: &Object, evaluated: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: string-ref first operand must be string.");
        std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        eprintln!("Type error: string-ref second operand must be integer.");
        std::process::exit(1);
    }
    let index = if let ObjectValue::IntValue(v) = op2.value {
        v
    } else {
        0
    };
    if let ObjectValue::StringValue(ref s) = op1.value {
        if index < 0 || (index as usize) >= s.len() {
            eprintln!("Index out of range.");
            std::process::exit(1);
        }
        let bytes = s.as_bytes();
        let ch = bytes[index as usize] as char;
        evaluated.type_ = ObjectType::String;
        evaluated.value = ObjectValue::StringValue(ch.to_string());
    }
}

// =================================================
//   Evaluator helpers
// =================================================

fn collect_expression_list(list: &ExpressionList) -> Vec<&ExpressionNode> {
    let mut result = vec![];
    let mut current: Option<&ExpressionList> = Some(list);
    while let Some(node) = current {
        if let Some(expr) = node.expression.as_deref() {
            result.push(expr);
        }
        current = node.next.as_deref();
    }
    result
}

fn evaluate_list_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let exprs = if let ExpressionData::List(Some(ref list_node)) = expression.data {
        list_node.expressions.as_deref()
    } else {
        None
    };

    let exprs = match exprs {
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
        Some(e) => e,
    };

    let expr_vec = collect_expression_list(exprs);
    let mut items: Vec<Object> = vec![];
    for expr in expr_vec {
        let mut item = make_nil_object();
        evaluate_expression(expr, &mut item, env, context);
        items.push(item);
    }

    if items.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
    } else {
        *evaluated = vec_to_list_object(items);
    }
}

fn evaluate_literal_expression(expression: &ExpressionNode, evaluated: &mut Object) {
    if let ExpressionData::Literal(Some(ref lit)) = expression.data {
        match lit.type_ {
            LiteralType::Integer => {
                if let LiteralValue::IntValue(v) = lit.value {
                    evaluated.type_ = ObjectType::Integer;
                    evaluated.value = ObjectValue::IntValue(v);
                }
            }
            LiteralType::String => {
                if let LiteralValue::StringValue(ref s) = lit.value {
                    evaluated.type_ = ObjectType::String;
                    evaluated.value = ObjectValue::StringValue(s.clone());
                }
            }
            LiteralType::Boolean => {
                if let LiteralValue::BooleanValue(v) = lit.value {
                    evaluated.type_ = ObjectType::Bool;
                    evaluated.value = ObjectValue::BoolValue(if v { 1 } else { 0 });
                }
            }
        }
    }
}

fn evaluate_symbol_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
) {
    let name = if let ExpressionData::Symbol(Some(ref sym)) = expression.data {
        sym.symbol_name.clone()
    } else {
        return;
    };
    if name == "nil" {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }
    match lookup_in_env(env, &name) {
        Some(obj) => {
            *evaluated = obj;
        }
        None => {
            eprintln!("Undefined symbol: {}", name);
            std::process::exit(1);
        }
    }
}

fn extract_symbol_name(expr: &ExpressionNode) -> Option<String> {
    if let ExpressionData::Symbol(Some(ref sym)) = expr.data {
        Some(sym.symbol_name.clone())
    } else {
        None
    }
}

fn lookup_function(env: &Env, name: &str) -> Option<Function> {
    for binding in env.bindings.iter() {
        if binding.symbol_name.is_empty() {
            break;
        }
        if binding.symbol_name == name {
            if let Some(ref obj) = binding.value {
                if let ObjectValue::FunctionValue(Some(ref f)) = obj.value {
                    return Some((**f).clone());
                }
            }
        }
    }
    if let Some(ref parent) = env.parent {
        return lookup_function(parent, name);
    }
    None
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    evaluated: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let exprs_opt = if let ExpressionData::SymbolicExp(Some(ref sym_node)) = expression.data
    {
        sym_node.expressions.as_deref()
    } else {
        None
    };

    let exprs = match exprs_opt {
        None => {
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
            return;
        }
        Some(e) => e,
    };

    let exp_vec = collect_expression_list(exprs);
    if exp_vec.is_empty() {
        evaluated.type_ = ObjectType::Nil;
        evaluated.value = ObjectValue::IntValue(0);
        return;
    }

    let head = exp_vec[0];
    if !matches!(head.type_, ExpressionType::Symbol) {
        eprintln!("S-exp must be started with symbol.");
        std::process::exit(1);
    }
    let symbol_name = match extract_symbol_name(head) {
        Some(s) => s,
        None => {
            eprintln!("S-exp must be started with symbol.");
            std::process::exit(1);
        }
    };

    let args: &[&ExpressionNode] = &exp_vec[1..];

    match symbol_name.as_str() {
        "if" => {
            if args.is_empty() {
                eprintln!("if must have condition.");
                std::process::exit(1);
            }
            let cond = args[0];
            if args.len() < 2 {
                eprintln!("if must have then clause.");
                std::process::exit(1);
            }
            let then_e = args[1];
            let mut cond_obj = make_nil_object();
            evaluate_expression(cond, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(then_e, evaluated, env, context);
            } else if args.len() > 2 {
                let else_e = args[2];
                evaluate_expression(else_e, evaluated, env, context);
            } else {
                evaluated.type_ = ObjectType::Nil;
                evaluated.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            if args.is_empty() {
                eprintln!("while must have condition.");
                std::process::exit(1);
            }
            let cond = args[0];
            if args.len() < 2 {
                eprintln!("while must have then clause.");
                std::process::exit(1);
            }
            let then_e = args[1];
            loop {
                let mut cond_obj = make_nil_object();
                evaluate_expression(cond, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(then_e, evaluated, env, context);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            if args.len() < 2 {
                eprintln!("assignment must have variable and expression.");
                std::process::exit(1);
            }
            let sym_expr = args[0];
            if !matches!(sym_expr.type_, ExpressionType::Symbol) {
                eprintln!("Variable name must be symbol.");
                std::process::exit(1);
            }
            let sym_name = extract_symbol_name(sym_expr).unwrap_or_default();
            let value_expr = args[1];
            let mut val = make_nil_object();
            evaluate_expression(value_expr, &mut val, env, context);
            *evaluated = val.clone();
            set_object_to_env(env, &sym_name, val);
        }
        "defun" => {
            if args.len() < 3 {
                eprintln!("defun must have name, params, and body.");
                std::process::exit(1);
            }
            let name_expr = args[0];
            if !matches!(name_expr.type_, ExpressionType::Symbol) {
                eprintln!("Function name must be symbol.");
                std::process::exit(1);
            }
            let func_name = extract_symbol_name(name_expr).unwrap_or_default();
            let params_expr = args[1];
            if !matches!(params_expr.type_, ExpressionType::SymbolicExp) {
                eprintln!("Function parameter must be list.");
                std::process::exit(1);
            }
            let mut param_names: Vec<String> = vec![];
            if let ExpressionData::SymbolicExp(Some(ref sym_node)) = params_expr.data {
                if let Some(ref params) = sym_node.expressions {
                    let param_vec = collect_expression_list(params);
                    for p in param_vec {
                        if !matches!(p.type_, ExpressionType::Symbol) {
                            eprintln!("Function parameter must be symbol.");
                            std::process::exit(1);
                        }
                        if let Some(name) = extract_symbol_name(p) {
                            param_names.push(name);
                        }
                    }
                }
            }
            let body_expr = args[2];
            let function = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(body_expr.clone())),
            };
            evaluated.type_ = ObjectType::Function;
            evaluated.value = ObjectValue::FunctionValue(Some(Box::new(function)));
            set_object_to_env(env, &func_name, evaluated.clone());
        }
        "+" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_add(&op1, &op2, evaluated);
        }
        "-" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_sub(&op1, &op2, evaluated);
        }
        "*" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_mul(&op1, &op2, evaluated);
        }
        "/" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_div(&op1, &op2, evaluated);
        }
        "%" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_mod(&op1, &op2, evaluated);
        }
        "||" => {
            for arg in args {
                let mut op = make_nil_object();
                evaluate_expression(arg, &mut op, env, context);
                if bool_val(&op) {
                    evaluated.type_ = ObjectType::Bool;
                    evaluated.value = ObjectValue::BoolValue(1);
                    return;
                }
            }
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(0);
        }
        "&&" => {
            for arg in args {
                let mut op = make_nil_object();
                evaluate_expression(arg, &mut op, env, context);
                if !bool_val(&op) {
                    evaluated.type_ = ObjectType::Bool;
                    evaluated.value = ObjectValue::BoolValue(0);
                    return;
                }
            }
            evaluated.type_ = ObjectType::Bool;
            evaluated.value = ObjectValue::BoolValue(1);
        }
        "<" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_lt(&op1, &op2, evaluated);
        }
        ">" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_gt(&op1, &op2, evaluated);
        }
        "eq" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_eq(&op1, &op2, evaluated);
        }
        "not" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_not(&op, evaluated);
        }
        "print" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            let s = stringify_object(&op);
            println!("{}", s);
            evaluated.type_ = ObjectType::Nil;
            evaluated.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_car(&op, evaluated);
        }
        "cdr" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_cdr(&op, evaluated);
        }
        "cons" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_cons(&op1, &op2, evaluated);
        }
        "readline" => {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(0) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
                Ok(_) => {
                    if line.ends_with('\n') {
                        line.pop();
                    }
                    evaluated.type_ = ObjectType::String;
                    evaluated.value = ObjectValue::StringValue(line);
                }
                Err(_) => {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            }
        }
        "split" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_split(&op1, &op2, evaluated);
        }
        "list-ref" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_list_ref(&op1, &op2, evaluated);
        }
        "progn" => {
            let mut last: Option<Object> = None;
            for arg in args {
                let mut op = make_nil_object();
                evaluate_expression(arg, &mut op, env, context);
                last = Some(op);
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
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_remove_whitespaces(&op, evaluated);
        }
        "pop" => {
            // pop from a symbol or literal
            let arg = args[0];
            if matches!(arg.type_, ExpressionType::Symbol) {
                let sym_name = extract_symbol_name(arg).unwrap_or_default();
                // Take the value, mutate, put back
                if let Some(mut val) = take_from_env(env, &sym_name) {
                    pop_in_place(&mut val, evaluated);
                    store_to_existing_binding(env, &sym_name, val);
                } else {
                    evaluated.type_ = ObjectType::Nil;
                    evaluated.value = ObjectValue::IntValue(0);
                }
            } else {
                let mut op = make_nil_object();
                evaluate_expression(arg, &mut op, env, context);
                pop_in_place(&mut op, evaluated);
            }
        }
        "push" => {
            // push: (push var value)
            let var_arg = args[0];
            let val_arg = args[1];
            // First evaluate value
            let mut value = make_nil_object();
            evaluate_expression(val_arg, &mut value, env, context);
            *evaluated = value.clone();

            if matches!(var_arg.type_, ExpressionType::Symbol) {
                let sym_name = extract_symbol_name(var_arg).unwrap_or_default();
                if let Some(mut val) = take_from_env(env, &sym_name) {
                    push_in_place(&mut val, value);
                    store_to_existing_binding(env, &sym_name, val);
                }
            }
        }
        "length" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_length(&op, evaluated);
        }
        "is-int-string" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_is_int_string(&op, evaluated);
        }
        "parse-int" => {
            let mut op = make_nil_object();
            evaluate_expression(args[0], &mut op, env, context);
            defined_function_parse_int(&op, evaluated);
        }
        "string-ref" => {
            let mut op1 = make_nil_object();
            let mut op2 = make_nil_object();
            evaluate_expression(args[0], &mut op1, env, context);
            evaluate_expression(args[1], &mut op2, env, context);
            defined_function_string_ref(&op1, &op2, evaluated);
        }
        _ => {
            // user-defined function call
            let function = match lookup_function(env, &symbol_name) {
                Some(f) => f,
                None => {
                    eprintln!("Undefined function: {}", symbol_name);
                    std::process::exit(1);
                }
            };

            // Evaluate args in current env
            let mut evaluated_args: Vec<Object> = vec![];
            for (i, arg) in args.iter().enumerate() {
                if i >= function.param_symbol_names.len() {
                    break;
                }
                let mut param = make_nil_object();
                evaluate_expression(arg, &mut param, env, context);
                evaluated_args.push(param);
            }

            // Move env into new_env.parent
            let parent = std::mem::take(env);
            let mut new_env = Env::default();
            new_env.parent = Some(Box::new(parent));

            // Bind params
            for (i, name) in function.param_symbol_names.iter().enumerate() {
                if i < evaluated_args.len() {
                    set_object_to_env(&mut new_env, name, evaluated_args[i].clone());
                }
            }

            // Evaluate body
            if let Some(ref body) = function.body {
                evaluate_expression(body, evaluated, &mut new_env, context);
            }

            // Restore parent
            if let Some(parent_box) = new_env.parent.take() {
                *env = *parent_box;
            }
        }
    }
}

fn pop_in_place(list_obj: &mut Object, result: &mut Object) {
    if matches!(list_obj.type_, ObjectType::Nil) {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(list_obj.type_, ObjectType::List) {
        eprintln!("Type error: pop operand must be list.");
        std::process::exit(1);
    }
    // Convert to vec, pop, rebuild
    let mut items = if let ObjectValue::ListValue(Some(ref cell)) = list_obj.value {
        cons_cell_to_vec(cell)
    } else {
        vec![]
    };

    if items.is_empty() {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }

    if items.len() == 1 {
        // Only one element: return it but DON'T modify the list
        // (Mirror C behavior: when prev is NULL, just return car)
        *result = items[items.len() - 1].clone();
        return;
    }

    let last = items.pop().unwrap();
    *result = last;
    *list_obj = vec_to_list_object(items);
}

fn push_in_place(list_obj: &mut Object, value: Object) {
    if matches!(list_obj.type_, ObjectType::Nil) {
        // Replace with a single-element list
        *list_obj = vec_to_list_object(vec![value]);
        return;
    }
    if !matches!(list_obj.type_, ObjectType::List) {
        eprintln!("Type error: push first operand must be list.");
        std::process::exit(1);
    }
    let mut items = if let ObjectValue::ListValue(Some(ref cell)) = list_obj.value {
        cons_cell_to_vec(cell)
    } else {
        vec![]
    };
    items.push(value);
    *list_obj = vec_to_list_object(items);
}

// =================================================
//   Public evaluation entry points
// =================================================

pub fn evaluate_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    match expression.type_ {
        ExpressionType::List => {
            evaluate_list_expression(expression, result, env, context);
        }
        ExpressionType::SymbolicExp => {
            evaluate_symbolic_expression(expression, result, env, context);
        }
        ExpressionType::Literal => {
            evaluate_literal_expression(expression, result);
        }
        ExpressionType::Symbol => {
            evaluate_symbol_expression(expression, result, env);
        }
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
    let mut env = Env::default();
    init_env(&mut env);
    let mut context = init_allocator();

    if let Some(ref program) = result.program {
        let mut current = program.expressions.as_deref();
        while let Some(node) = current {
            if let Some(expr) = node.expression.as_deref() {
                let mut evaluated = make_nil_object();
                evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
            }
            current = node.next.as_deref();
        }
    }
}
