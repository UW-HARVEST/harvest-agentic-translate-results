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

// =========== Helper functions ===========

fn isop(ch: u8) -> bool {
    matches!(ch, b'+' | b'-' | b'*' | b'/' | b'%' | b'|' | b'&' | b'=' | b'<' | b'>')
}

fn new_object_nil() -> Box<Object> {
    Box::new(Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    })
}

fn new_object_default() -> Box<Object> {
    new_object_nil()
}

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: match &obj.value {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(v) => ObjectValue::ListValue(v.as_ref().map(|c| Box::new(clone_conscell(c)))),
            ObjectValue::FunctionValue(v) => ObjectValue::FunctionValue(v.as_ref().map(|f| Box::new(clone_function(f)))),
        },
    }
}

fn clone_conscell(cc: &ConsCell) -> ConsCell {
    ConsCell {
        type_: cc.type_,
        car: cc.car.as_ref().map(|o| Box::new(clone_object(o))),
        cdr: cc.cdr.as_ref().map(|o| Box::new(clone_object(o))),
    }
}

fn clone_function(f: &Function) -> Function {
    Function {
        param_symbol_names: f.param_symbol_names.clone(),
        body: f.body.as_ref().map(|b| Box::new(clone_expression_node(b))),
    }
}

fn clone_expression_node(node: &ExpressionNode) -> ExpressionNode {
    ExpressionNode {
        type_: node.type_,
        data: match &node.data {
            ExpressionData::SymbolicExp(s) => ExpressionData::SymbolicExp(
                s.as_ref().map(|se| Box::new(SymbolicExpNode {
                    expressions: se.expressions.as_ref().map(|e| Box::new(clone_expression_list(e))),
                })),
            ),
            ExpressionData::List(l) => ExpressionData::List(
                l.as_ref().map(|ln| Box::new(ListNode {
                    expressions: ln.expressions.as_ref().map(|e| Box::new(clone_expression_list(e))),
                })),
            ),
            ExpressionData::Literal(l) => ExpressionData::Literal(
                l.as_ref().map(|ln| Box::new(LiteralNode {
                    type_: ln.type_,
                    value: match &ln.value {
                        LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
                        LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
                        LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
                    },
                })),
            ),
            ExpressionData::Symbol(s) => ExpressionData::Symbol(
                s.as_ref().map(|sn| Box::new(SymbolNode {
                    symbol_name: sn.symbol_name.clone(),
                })),
            ),
        },
    }
}

fn clone_expression_list(list: &ExpressionList) -> ExpressionList {
    ExpressionList {
        expression: list.expression.as_ref().map(|e| Box::new(clone_expression_node(e))),
        next: list.next.as_ref().map(|n| Box::new(clone_expression_list(n))),
    }
}

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => {
            if let ObjectValue::BoolValue(v) = &obj.value { *v != 0 } else { true }
        }
        ObjectType::Nil => false,
        _ => true,
    }
}

fn obj_eq(op1: &Object, op2: &Object) -> bool {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            get_int(&op1.value) == get_int(&op2.value)
        }
        (ObjectType::String, ObjectType::String) => {
            get_string(&op1.value) == get_string(&op2.value)
        }
        (ObjectType::Bool, ObjectType::Bool) => {
            get_bool_val(&op1.value) == get_bool_val(&op2.value)
        }
        (ObjectType::Nil, ObjectType::Nil) => true,
        _ => false,
    }
}

fn get_int(v: &ObjectValue) -> i32 {
    if let ObjectValue::IntValue(i) = v { *i } else { 0 }
}

fn get_string(v: &ObjectValue) -> &str {
    if let ObjectValue::StringValue(s) = v { s } else { "" }
}

fn get_bool_val(v: &ObjectValue) -> i32 {
    if let ObjectValue::BoolValue(b) = v { *b } else { 0 }
}

fn is_last_cons_cell(cc: &ConsCell) -> bool {
    if let Some(cdr) = &cc.cdr {
        matches!(cdr.type_, ObjectType::Nil)
    } else {
        true
    }
}

fn _get_symbol_name(data: &ExpressionData) -> &str {
    match data {
        ExpressionData::Symbol(Some(s)) => &s.symbol_name,
        ExpressionData::SymbolicExp(Some(se)) => {
            if let Some(el) = &se.expressions {
                if let Some(e) = &el.expression {
                    if let ExpressionData::Symbol(Some(s)) = &e.data {
                        return &s.symbol_name;
                    }
                }
            }
            ""
        }
        _ => "",
    }
}

fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: Box<Object>) {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            break;
        }
        if env.bindings[i].symbol_name == symbol_name {
            env.bindings[i].value = Some(obj);
            return;
        }
    }
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() {
            env.bindings[i].symbol_name = symbol_name.to_string();
            env.bindings[i].value = Some(obj);
            return;
        }
    }
}

fn copy_obj_into(dest: &mut Object, src: &Object) {
    dest.type_ = src.type_;
    dest.marked = src.marked;
    dest.value = match &src.value {
        ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
        ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
        ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
        ObjectValue::ListValue(v) => ObjectValue::ListValue(v.as_ref().map(|c| Box::new(clone_conscell(c)))),
        ObjectValue::FunctionValue(v) => ObjectValue::FunctionValue(v.as_ref().map(|f| Box::new(clone_function(f)))),
    };
}

fn new_empty_binding() -> Binding {
    Binding { symbol_name: String::new(), value: None }
}

fn new_env() -> Env {
    Env {
        bindings: std::array::from_fn(|_| new_empty_binding()),
        parent: None,
    }
}

// =========== Public API ===========

pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    if let Some(token) = &state.token {
        if token.kind == kind { 1 } else { 0 }
    } else {
        0
    }
}

pub fn next(source: &str, state: &mut ParseState) {
    let bytes = source.as_bytes();
    let mut pos = state.pos as usize;

    // Skip whitespace
    while pos < bytes.len() && (bytes[pos].is_ascii_whitespace()) {
        pos += 1;
    }

    let new_token = if pos >= bytes.len() || bytes[pos] == 0 {
        Box::new(Token { kind: TokenKind::Eof, next: None, val: 0, str: String::new() })
    } else if bytes[pos] == b'(' {
        pos += 1;
        Box::new(Token { kind: TokenKind::LParen, next: None, val: 0, str: "(".to_string() })
    } else if bytes[pos] == b')' {
        pos += 1;
        Box::new(Token { kind: TokenKind::RParen, next: None, val: 0, str: ")".to_string() })
    } else if bytes[pos] == b'\'' {
        pos += 1;
        Box::new(Token { kind: TokenKind::Quote, next: None, val: 0, str: "'".to_string() })
    } else if bytes[pos].is_ascii_alphabetic() || isop(bytes[pos]) {
        let start = pos;
        while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || isop(bytes[pos])) {
            pos += 1;
        }
        let s: String = source[start..pos].to_string();
        if s == "true" {
            Box::new(Token { kind: TokenKind::True, next: None, val: 0, str: String::new() })
        } else if s == "false" {
            Box::new(Token { kind: TokenKind::False, next: None, val: 0, str: String::new() })
        } else {
            Box::new(Token { kind: TokenKind::Symbol, next: None, val: 0, str: s })
        }
    } else if bytes[pos].is_ascii_digit() {
        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let s: &str = &source[start..pos];
        let val: i32 = s.parse().unwrap_or(0);
        Box::new(Token { kind: TokenKind::Digit, next: None, val, str: String::new() })
    } else if bytes[pos] == b'"' {
        pos += 1; // skip opening quote
        let start = pos;
        while pos < bytes.len() && bytes[pos] != b'"' {
            pos += 1;
        }
        let s = source[start..pos].to_string();
        if pos < bytes.len() && bytes[pos] == b'"' {
            pos += 1;
        }
        Box::new(Token { kind: TokenKind::String, next: None, val: 0, str: s })
    } else if bytes[pos] == b';' {
        while pos < bytes.len() && bytes[pos] != b'\n' && bytes[pos] != 0 {
            pos += 1;
        }
        state.pos = pos as i32;
        next(source, state);
        return;
    } else {
        eprintln!("Unexpected token: {}", bytes[pos] as char);
        std::process::exit(1);
    };

    state.pos = pos as i32;
    state.token = Some(new_token);
}

fn append_to_list(list: &mut Option<Box<ExpressionList>>, expr: Box<ExpressionNode>) {
    let new_el = Box::new(ExpressionList { expression: Some(expr), next: None });
    if list.is_none() {
        *list = Some(new_el);
        return;
    }
    let mut current = list.as_mut().unwrap();
    while current.next.is_some() {
        current = current.next.as_mut().unwrap();
    }
    current.next = Some(new_el);
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
        let s = state.token.as_ref().map(|t| t.str.as_str()).unwrap_or("");
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    }
}

fn parse_symbolic_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut expressions: Option<Box<ExpressionList>> = None;
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_to_list(&mut expressions, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::SymbolicExp,
        data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode { expressions }))),
    })
}

fn parse_list_expression(source: &str, state: &mut ParseState) -> Box<ExpressionNode> {
    let mut expressions: Option<Box<ExpressionList>> = None;
    next(source, state); // eat quote
    next(source, state); // eat '('
    while match_token(state, TokenKind::RParen) != 1 {
        let item = parse_expression(source, state);
        append_to_list(&mut expressions, item);
    }
    next(source, state); // eat ')'
    Box::new(ExpressionNode {
        type_: ExpressionType::List,
        data: ExpressionData::List(Some(Box::new(ListNode { expressions }))),
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
    let node = if match_token(state, TokenKind::Digit) == 1 {
        let val = state.token.as_ref().unwrap().val;
        next(source, state);
        LiteralNode { type_: LiteralType::Integer, value: LiteralValue::IntValue(val) }
    } else if match_token(state, TokenKind::String) == 1 {
        let s = state.token.as_ref().unwrap().str.clone();
        next(source, state);
        LiteralNode { type_: LiteralType::String, value: LiteralValue::StringValue(s) }
    } else if match_token(state, TokenKind::True) == 1 {
        next(source, state);
        LiteralNode { type_: LiteralType::Boolean, value: LiteralValue::BooleanValue(true) }
    } else if match_token(state, TokenKind::False) == 1 {
        next(source, state);
        LiteralNode { type_: LiteralType::Boolean, value: LiteralValue::BooleanValue(false) }
    } else {
        let s = state.token.as_ref().map(|t| t.str.as_str()).unwrap_or("");
        eprintln!("Unexpected token: {}", s);
        std::process::exit(1);
    };
    Box::new(ExpressionNode {
        type_: ExpressionType::Literal,
        data: ExpressionData::Literal(Some(Box::new(node))),
    })
}

pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut expressions: Option<Box<ExpressionList>> = None;
    while match_token(state, TokenKind::Eof) != 1 {
        let expr = parse_expression(source, state);
        append_to_list(&mut expressions, expr);
    }
    result.program = Some(Box::new(ProgramNode { expressions }));
}

// =========== Evaluator ===========

fn evaluate_list_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions = match &expression.data {
        ExpressionData::List(Some(l)) => &l.expressions,
        _ => { result.type_ = ObjectType::Nil; result.value = ObjectValue::IntValue(0); return; }
    };

    if expressions.is_none() {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }

    result.type_ = ObjectType::List;

    let mut cells: Vec<Box<Object>> = Vec::new();
    let mut cur = expressions.as_ref();
    while let Some(el) = cur {
        if let Some(expr) = &el.expression {
            let mut item = new_object_default();
            evaluate_expression(expr, &mut item, env, context);
            cells.push(item);
        }
        cur = el.next.as_ref();
    }

    // Build cons cell chain from the evaluated items
    let mut root_cc: Option<Box<ConsCell>> = None;
    let mut last_cc: *mut ConsCell = std::ptr::null_mut();

    for (i, item) in cells.into_iter().enumerate() {
        let nil_obj = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
        let mut cc = Box::new(ConsCell { type_: ConsCellType::Nil, car: Some(item), cdr: Some(nil_obj) });
        if i == 0 {
            let ptr: *mut ConsCell = &mut *cc;
            root_cc = Some(cc);
            last_cc = ptr;
        } else {
            let ptr: *mut ConsCell = &mut *cc;
            let wrapper = Box::new(Object {
                marked: false,
                type_: ObjectType::List,
                value: ObjectValue::ListValue(Some(cc)),
            });
            unsafe { (*last_cc).cdr = Some(wrapper); (*last_cc).type_ = ConsCellType::Cell; }
            last_cc = ptr;
        }
    }

    result.value = ObjectValue::ListValue(root_cc);
}

fn evaluate_literal_expression(expression: &ExpressionNode, result: &mut Object) {
    if let ExpressionData::Literal(Some(lit)) = &expression.data {
        match &lit.value {
            LiteralValue::IntValue(v) => {
                result.type_ = ObjectType::Integer;
                result.value = ObjectValue::IntValue(*v);
            }
            LiteralValue::StringValue(v) => {
                result.type_ = ObjectType::String;
                result.value = ObjectValue::StringValue(v.clone());
            }
            LiteralValue::BooleanValue(v) => {
                result.type_ = ObjectType::Bool;
                result.value = ObjectValue::BoolValue(if *v { 1 } else { 0 });
            }
        }
    }
}

fn evaluate_symbol_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    _context: &mut AllocatorContext,
) {
    let name = match &expression.data {
        ExpressionData::Symbol(Some(s)) => &s.symbol_name,
        _ => { result.type_ = ObjectType::Nil; return; }
    };

    if name == "nil" {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }

    // Search current env
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() { break; }
        if env.bindings[i].symbol_name == *name {
            if let Some(val) = &env.bindings[i].value {
                copy_obj_into(result, val);
            }
            return;
        }
    }

    // Search parent
    if let Some(parent) = &mut env.parent {
        evaluate_symbol_expression(expression, result, parent, _context);
    } else {
        eprintln!("Undefined symbol: {}", name);
        std::process::exit(1);
    }
}

fn get_expressions_from_symbolic(expression: &ExpressionNode) -> Option<&ExpressionList> {
    match &expression.data {
        ExpressionData::SymbolicExp(Some(se)) => se.expressions.as_ref().map(|b| b.as_ref()),
        _ => None,
    }
}

fn eval_two_operands(
    expressions: &ExpressionList,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> (Box<Object>, Box<Object>) {
    let next1 = expressions.next.as_ref().unwrap();
    let next2 = next1.next.as_ref().unwrap();
    let mut op1 = new_object_default();
    let mut op2 = new_object_default();
    evaluate_expression(next1.expression.as_ref().unwrap(), &mut op1, env, context);
    evaluate_expression(next2.expression.as_ref().unwrap(), &mut op2, env, context);
    (op1, op2)
}

fn eval_one_operand(
    expressions: &ExpressionList,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Box<Object> {
    let next1 = expressions.next.as_ref().unwrap();
    let mut op = new_object_default();
    evaluate_expression(next1.expression.as_ref().unwrap(), &mut op, env, context);
    op
}

fn evaluate_symbolic_expression(
    expression: &ExpressionNode,
    result: &mut Object,
    env: &mut Env,
    context: &mut AllocatorContext,
) {
    let expressions = match get_expressions_from_symbolic(expression) {
        Some(e) => e,
        None => {
            result.type_ = ObjectType::Nil;
            result.value = ObjectValue::IntValue(0);
            return;
        }
    };

    let first_expr = match &expressions.expression {
        Some(e) => e,
        None => { result.type_ = ObjectType::Nil; return; }
    };

    if !matches!(first_expr.type_, ExpressionType::Symbol) {
        eprintln!("S-exp must be started with symbol.");
        std::process::exit(1);
    }

    let sym_name = match &first_expr.data {
        ExpressionData::Symbol(Some(s)) => s.symbol_name.as_str(),
        _ => { eprintln!("S-exp must be started with symbol."); std::process::exit(1); }
    };

    // We need to clone the expression list to avoid borrow issues
    let expr_clone = clone_expression_list(expressions);

    match sym_name {
        "if" => {
            let cond_expr = expr_clone.next.as_ref().unwrap().expression.as_ref().unwrap();
            let then_expr = expr_clone.next.as_ref().unwrap().next.as_ref().unwrap().expression.as_ref().unwrap();
            let mut cond_obj = new_object_default();
            evaluate_expression(cond_expr, &mut cond_obj, env, context);
            if bool_val(&cond_obj) {
                evaluate_expression(then_expr, result, env, context);
            } else if let Some(else_el) = &expr_clone.next.as_ref().unwrap().next.as_ref().unwrap().next {
                if let Some(else_expr) = &else_el.expression {
                    evaluate_expression(else_expr, result, env, context);
                } else {
                    result.type_ = ObjectType::Nil;
                    result.value = ObjectValue::IntValue(0);
                }
            } else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            }
        }
        "while" => {
            let cond_expr_ref = expr_clone.next.as_ref().unwrap().expression.as_ref().unwrap();
            let then_expr_ref = expr_clone.next.as_ref().unwrap().next.as_ref().unwrap().expression.as_ref().unwrap();
            loop {
                let mut cond_obj = new_object_default();
                evaluate_expression(cond_expr_ref, &mut cond_obj, env, context);
                if bool_val(&cond_obj) {
                    evaluate_expression(then_expr_ref, result, env, context);
                } else {
                    result.type_ = ObjectType::Nil;
                    result.value = ObjectValue::IntValue(0);
                    break;
                }
            }
        }
        "=" => {
            let sym_expr = expr_clone.next.as_ref().unwrap().expression.as_ref().unwrap();
            let val_name = match &sym_expr.data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => { eprintln!("Variable name must be symbol."); std::process::exit(1); }
            };
            let val_expr = expr_clone.next.as_ref().unwrap().next.as_ref().unwrap().expression.as_ref().unwrap();
            let mut evaluated = new_object_default();
            evaluate_expression(val_expr, &mut evaluated, env, context);
            copy_obj_into(result, &evaluated);
            set_object_to_env(env, &val_name, evaluated);
        }
        "defun" => {
            let sym_expr = expr_clone.next.as_ref().unwrap().expression.as_ref().unwrap();
            let fn_name = match &sym_expr.data {
                ExpressionData::Symbol(Some(s)) => s.symbol_name.clone(),
                _ => { eprintln!("Function name must be symbol."); std::process::exit(1); }
            };
            let params_expr = expr_clone.next.as_ref().unwrap().next.as_ref().unwrap().expression.as_ref().unwrap();
            let params_list = match &params_expr.data {
                ExpressionData::SymbolicExp(Some(se)) => &se.expressions,
                _ => { eprintln!("Function parameter must be list."); std::process::exit(1); }
            };
            let mut param_names = Vec::new();
            let mut cur = params_list.as_ref();
            while let Some(el) = cur {
                if let Some(e) = &el.expression {
                    if let ExpressionData::Symbol(Some(s)) = &e.data {
                        param_names.push(s.symbol_name.clone());
                    } else {
                        eprintln!("Function parameter must be symbol.");
                        std::process::exit(1);
                    }
                }
                cur = el.next.as_ref();
            }
            let body_expr = expr_clone.next.as_ref().unwrap().next.as_ref().unwrap().next.as_ref().unwrap().expression.as_ref().unwrap();
            let func = Function {
                param_symbol_names: param_names,
                body: Some(Box::new(clone_expression_node(body_expr))),
            };
            result.type_ = ObjectType::Function;
            result.value = ObjectValue::FunctionValue(Some(Box::new(func)));
            let result_clone = Box::new(clone_object(result));
            set_object_to_env(env, &fn_name, result_clone);
        }
        "+" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_add(&op1, &op2, result);
        }
        "-" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_sub(&op1, &op2, result);
        }
        "*" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_mul(&op1, &op2, result);
        }
        "/" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_div(&op1, &op2, result);
        }
        "%" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_mod(&op1, &op2, result);
        }
        "||" => {
            let mut cur = expr_clone.next.as_ref();
            while let Some(el) = cur {
                if let Some(e) = &el.expression {
                    let mut operand = new_object_default();
                    evaluate_expression(e, &mut operand, env, context);
                    if bool_val(&operand) {
                        result.type_ = ObjectType::Bool;
                        result.value = ObjectValue::BoolValue(1);
                        return;
                    }
                }
                cur = el.next.as_ref();
            }
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(0);
        }
        "&&" => {
            let mut cur = expr_clone.next.as_ref();
            while let Some(el) = cur {
                if let Some(e) = &el.expression {
                    let mut operand = new_object_default();
                    evaluate_expression(e, &mut operand, env, context);
                    if !bool_val(&operand) {
                        result.type_ = ObjectType::Bool;
                        result.value = ObjectValue::BoolValue(0);
                        return;
                    }
                }
                cur = el.next.as_ref();
            }
            result.type_ = ObjectType::Bool;
            result.value = ObjectValue::BoolValue(1);
        }
        "<" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_lt(&op1, &op2, result);
        }
        ">" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_gt(&op1, &op2, result);
        }
        "eq" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_eq_op(&op1, &op2, result);
        }
        "not" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_not(&op, result);
        }
        "print" => {
            let op = eval_one_operand(&expr_clone, env, context);
            let s = stringify_object(&op);
            println!("{}", s);
            result.type_ = ObjectType::Nil;
            result.value = ObjectValue::IntValue(0);
        }
        "car" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_car(&op, result);
        }
        "cdr" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_cdr(&op, result);
        }
        "cons" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_cons(op1, op2, result);
        }
        "readline" => {
            let mut line = String::new();
            match std::io::stdin().read_line(&mut line) {
                Ok(n) if n > 0 => {
                    if line.ends_with('\n') { line.pop(); }
                    result.type_ = ObjectType::String;
                    result.value = ObjectValue::StringValue(line);
                }
                _ => {
                    result.type_ = ObjectType::Nil;
                    result.value = ObjectValue::IntValue(0);
                }
            }
        }
        "split" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_split(&op1, &op2, result);
        }
        "list-ref" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_list_ref(&op1, &op2, result);
        }
        "progn" => {
            let mut cur = expr_clone.next.as_ref();
            let mut last: Option<Box<Object>> = None;
            while let Some(el) = cur {
                if let Some(e) = &el.expression {
                    let mut operand = new_object_default();
                    evaluate_expression(e, &mut operand, env, context);
                    last = Some(operand);
                }
                cur = el.next.as_ref();
            }
            if let Some(l) = last {
                copy_obj_into(result, &l);
            } else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            }
        }
        "remove-whitespaces" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_remove_whitespaces(&op, result);
        }
        "pop" => {
            let mut op = eval_one_operand(&expr_clone, env, context);
            defined_function_pop(&mut op, result);
            // Update env binding if needed
            let sym_expr = expr_clone.next.as_ref().unwrap().expression.as_ref().unwrap();
            if let ExpressionData::Symbol(Some(s)) = &sym_expr.data {
                set_object_to_env(env, &s.symbol_name, op);
            }
        }
        "push" => {
            // (push list-expr value-expr) - C code evaluates value first (next->next), then list (next)
            let next1 = expr_clone.next.as_ref().unwrap();
            let next2 = next1.next.as_ref().unwrap();
            let mut op_value = new_object_default();
            evaluate_expression(next2.expression.as_ref().unwrap(), &mut op_value, env, context);
            let mut op_list = new_object_default();
            evaluate_expression(next1.expression.as_ref().unwrap(), &mut op_list, env, context);
            defined_function_push(&mut op_list, op_value, result);
            // Update env binding
            let sym_expr = next1.expression.as_ref().unwrap();
            if let ExpressionData::Symbol(Some(s)) = &sym_expr.data {
                set_object_to_env(env, &s.symbol_name, op_list);
            }
        }
        "length" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_length(&op, result);
        }
        "is-int-string" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_is_int_string(&op, result);
        }
        "parse-int" => {
            let op = eval_one_operand(&expr_clone, env, context);
            defined_function_parse_int(&op, result);
        }
        "string-ref" => {
            let (op1, op2) = eval_two_operands(&expr_clone, env, context);
            defined_function_string_ref(&op1, &op2, result);
        }
        _ => {
            // User-defined function call
            let fn_name = sym_name.to_string();
            let func_obj = lookup_function(env, &fn_name);
            if let Some(func_obj) = func_obj {
                if let ObjectValue::FunctionValue(Some(func)) = &func_obj.value {
                    let param_names = func.param_symbol_names.clone();
                    let body = func.body.as_ref().map(|b| clone_expression_node(b));
                    let mut new_env_inner = new_env();
                    // Evaluate params
                    let mut param_cur = expr_clone.next.as_ref();
                    for pname in &param_names {
                        if let Some(el) = param_cur {
                            if let Some(e) = &el.expression {
                                let mut param_val = new_object_default();
                                evaluate_expression(e, &mut param_val, env, context);
                                set_object_to_env(&mut new_env_inner, pname, param_val);
                            }
                            param_cur = el.next.as_ref();
                        }
                    }
                    new_env_inner.parent = Some(Box::new(clone_env(env)));
                    if let Some(body) = &body {
                        evaluate_expression(body, result, &mut new_env_inner, context);
                    }
                    // Copy back any env changes from parent
                    return;
                }
            }
            eprintln!("Undefined function: {}", fn_name);
            std::process::exit(1);
        }
    }
}

fn lookup_function(env: &Env, name: &str) -> Option<Box<Object>> {
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() { break; }
        if env.bindings[i].symbol_name == name {
            return env.bindings[i].value.as_ref().map(|v| Box::new(clone_object(v)));
        }
    }
    if let Some(parent) = &env.parent {
        lookup_function(parent, name)
    } else {
        None
    }
}

fn clone_env(env: &Env) -> Env {
    let mut new_env = new_env();
    for i in 0..MAX_BINDINGS {
        if env.bindings[i].symbol_name.is_empty() { break; }
        new_env.bindings[i].symbol_name = env.bindings[i].symbol_name.clone();
        new_env.bindings[i].value = env.bindings[i].value.as_ref().map(|v| Box::new(clone_object(v)));
    }
    new_env.parent = env.parent.as_ref().map(|p| Box::new(clone_env(p)));
    new_env
}

// =========== Defined functions ===========

fn defined_function_add(op1: &Object, op2: &Object, result: &mut Object) {
    match (op1.type_, op2.type_) {
        (ObjectType::Integer, ObjectType::Integer) => {
            result.type_ = ObjectType::Integer;
            result.value = ObjectValue::IntValue(get_int(&op1.value) + get_int(&op2.value));
        }
        (ObjectType::String, ObjectType::String) => {
            result.type_ = ObjectType::String;
            let s = format!("{}{}", get_string(&op1.value), get_string(&op2.value));
            result.value = ObjectValue::StringValue(s);
        }
        _ => { eprintln!("Type error: operands for + must be integers or strings."); std::process::exit(1); }
    }
}

fn defined_function_sub(op1: &Object, op2: &Object, result: &mut Object) {
    if matches!((op1.type_, op2.type_), (ObjectType::Integer, ObjectType::Integer)) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(get_int(&op1.value) - get_int(&op2.value));
    } else { eprintln!("Type error: operands for - must be integers."); std::process::exit(1); }
}

fn defined_function_mul(op1: &Object, op2: &Object, result: &mut Object) {
    if matches!((op1.type_, op2.type_), (ObjectType::Integer, ObjectType::Integer)) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(get_int(&op1.value) * get_int(&op2.value));
    } else { eprintln!("Type error: operands for * must be integers."); std::process::exit(1); }
}

fn defined_function_div(op1: &Object, op2: &Object, result: &mut Object) {
    if matches!((op1.type_, op2.type_), (ObjectType::Integer, ObjectType::Integer)) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(get_int(&op1.value) / get_int(&op2.value));
    } else { eprintln!("Type error: operands for / must be integers."); std::process::exit(1); }
}

fn defined_function_mod(op1: &Object, op2: &Object, result: &mut Object) {
    if matches!((op1.type_, op2.type_), (ObjectType::Integer, ObjectType::Integer)) {
        result.type_ = ObjectType::Integer;
        result.value = ObjectValue::IntValue(get_int(&op1.value) % get_int(&op2.value));
    } else { eprintln!("Type error: operands for % must be integers."); std::process::exit(1); }
}

fn defined_function_lt(op1: &Object, op2: &Object, result: &mut Object) {
    if matches!((op1.type_, op2.type_), (ObjectType::Integer, ObjectType::Integer)) {
        result.type_ = ObjectType::Bool;
        result.value = ObjectValue::BoolValue(if get_int(&op1.value) < get_int(&op2.value) { 1 } else { 0 });
    } else { eprintln!("Type error: operands for < must be integers."); std::process::exit(1); }
}

fn defined_function_gt(op1: &Object, op2: &Object, result: &mut Object) {
    if matches!((op1.type_, op2.type_), (ObjectType::Integer, ObjectType::Integer)) {
        result.type_ = ObjectType::Bool;
        result.value = ObjectValue::BoolValue(if get_int(&op1.value) > get_int(&op2.value) { 1 } else { 0 });
    } else { eprintln!("Type error: operands for < must be integers."); std::process::exit(1); }
}

fn defined_function_eq_op(op1: &Object, op2: &Object, result: &mut Object) {
    result.type_ = ObjectType::Bool;
    result.value = ObjectValue::BoolValue(if obj_eq(op1, op2) { 1 } else { 0 });
}

fn defined_function_not(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::Bool) {
        eprintln!("Type error: not operand must be boolean.");
        std::process::exit(1);
    }
    result.type_ = ObjectType::Bool;
    result.value = ObjectValue::BoolValue(if get_bool_val(&op.value) != 0 { 0 } else { 1 });
}

fn defined_function_car(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: car operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(cc)) = &op.value {
        if let Some(car) = &cc.car {
            copy_obj_into(result, car);
        }
    }
}

fn defined_function_cdr(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: cdr operand must be list.");
        std::process::exit(1);
    }
    if let ObjectValue::ListValue(Some(cc)) = &op.value {
        if let Some(cdr) = &cc.cdr {
            copy_obj_into(result, cdr);
        }
    }
}

fn defined_function_cons(op1: Box<Object>, op2: Box<Object>, result: &mut Object) {
    result.type_ = ObjectType::List;
    if matches!(op2.type_, ObjectType::List) {
        result.value = ObjectValue::ListValue(Some(Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(op1),
            cdr: Some(op2),
        })));
    } else if matches!(op2.type_, ObjectType::Nil) {
        result.value = ObjectValue::ListValue(Some(Box::new(ConsCell {
            type_: ConsCellType::Nil,
            car: Some(op1),
            cdr: Some(op2),
        })));
    } else {
        let nil_obj = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
        let inner_cc = Box::new(ConsCell { type_: ConsCellType::Cell, car: Some(op2), cdr: Some(nil_obj) });
        let cdr_obj = Box::new(Object { marked: false, type_: ObjectType::List, value: ObjectValue::ListValue(Some(inner_cc)) });
        result.value = ObjectValue::ListValue(Some(Box::new(ConsCell {
            type_: ConsCellType::Cell,
            car: Some(op1),
            cdr: Some(cdr_obj),
        })));
    }
}

fn defined_function_split(op1: &Object, op2: &Object, result: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: split first operand must be string."); std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::String) {
        eprintln!("Type error: split second operand must be string."); std::process::exit(1);
    }
    let s = get_string(&op1.value);
    let delim = get_string(&op2.value);

    let parts: Vec<String> = if delim.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        s.split(delim).map(|p| p.to_string()).collect()
    };

    if parts.is_empty() {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }

    result.type_ = ObjectType::List;
    let mut root_cc: Option<Box<ConsCell>> = None;
    let mut last_cc: *mut ConsCell = std::ptr::null_mut();

    for (i, part) in parts.into_iter().enumerate() {
        let car = Box::new(Object { marked: false, type_: ObjectType::String, value: ObjectValue::StringValue(part) });
        let nil = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
        let mut cc = Box::new(ConsCell { type_: ConsCellType::Cell, car: Some(car), cdr: Some(nil) });
        if i == 0 {
            let ptr: *mut ConsCell = &mut *cc;
            root_cc = Some(cc);
            last_cc = ptr;
        } else {
            let ptr: *mut ConsCell = &mut *cc;
            let wrapper = Box::new(Object { marked: false, type_: ObjectType::List, value: ObjectValue::ListValue(Some(cc)) });
            unsafe { (*last_cc).cdr = Some(wrapper); }
            last_cc = ptr;
        }
    }
    result.value = ObjectValue::ListValue(root_cc);
}

fn defined_function_list_ref(op1: &Object, op2: &Object, result: &mut Object) {
    if !matches!(op1.type_, ObjectType::List) {
        eprintln!("Type error: list-ref first operand must be list."); std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        eprintln!("Type error: list-ref second operand must be integer."); std::process::exit(1);
    }
    let index = get_int(&op2.value);
    let mut current = match &op1.value {
        ObjectValue::ListValue(Some(cc)) => cc,
        _ => { eprintln!("Index out of range."); std::process::exit(1); }
    };
    for _ in 0..index {
        if is_last_cons_cell(current) {
            eprintln!("Index out of range.");
            std::process::exit(1);
        }
        let cdr = current.cdr.as_ref().unwrap();
        current = match &cdr.value {
            ObjectValue::ListValue(Some(cc)) => cc,
            _ => { eprintln!("Index out of range."); std::process::exit(1); }
        };
    }
    if let Some(car) = &current.car {
        copy_obj_into(result, car);
    }
}

fn defined_function_remove_whitespaces(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        eprintln!("Type error: remove-whitespaces operand must be string."); std::process::exit(1);
    }
    let s = get_string(&op.value);
    let new_s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    result.type_ = ObjectType::String;
    result.value = ObjectValue::StringValue(new_s);
}

fn defined_function_pop(op: &mut Object, result: &mut Object) {
    if matches!(op.type_, ObjectType::Nil) {
        result.type_ = ObjectType::Nil;
        result.value = ObjectValue::IntValue(0);
        return;
    }
    if !matches!(op.type_, ObjectType::List) {
        eprintln!("Type error: pop operand must be list."); std::process::exit(1);
    }

    // Count elements
    let count = count_list(op);
    if count == 1 {
        // Single element - return car, set op to nil
        if let ObjectValue::ListValue(Some(cc)) = &op.value {
            if let Some(car) = &cc.car {
                copy_obj_into(result, car);
            }
        }
        op.type_ = ObjectType::Nil;
        op.value = ObjectValue::IntValue(0);
    } else {
        // Find last and second-to-last
        // We need to remove the last element and return its car
        pop_last(op, result);
    }
}

fn count_list(obj: &Object) -> usize {
    let mut count = 0;
    let mut current = match &obj.value {
        ObjectValue::ListValue(Some(cc)) => Some(cc.as_ref()),
        _ => None,
    };
    while let Some(cc) = current {
        count += 1;
        if is_last_cons_cell(cc) { break; }
        current = match &cc.cdr {
            Some(cdr) => match &cdr.value {
                ObjectValue::ListValue(Some(next_cc)) => Some(next_cc.as_ref()),
                _ => None,
            },
            None => None,
        };
    }
    count
}

fn pop_last(obj: &mut Object, result: &mut Object) {
    // Navigate to second-to-last cons cell and remove the last
    let cc = match &mut obj.value {
        ObjectValue::ListValue(Some(cc)) => cc.as_mut(),
        _ => return,
    };

    if is_last_cons_cell(cc) {
        if let Some(car) = &cc.car {
            copy_obj_into(result, car);
        }
        return;
    }

    // Find the second-to-last
    let _path: Vec<usize> = Vec::new();
    let mut depth = 0;
    {
        let mut cur = cc as &ConsCell;
        loop {
            if is_last_cons_cell(cur) { break; }
            depth += 1;
            let cdr = cur.cdr.as_ref().unwrap();
            cur = match &cdr.value {
                ObjectValue::ListValue(Some(next_cc)) => next_cc.as_ref(),
                _ => break,
            };
        }
    }

    // Now navigate depth-1 steps to get second-to-last, then modify
    let mut cur = cc as &mut ConsCell;
    for _ in 0..depth - 1 {
        let cdr = cur.cdr.as_mut().unwrap();
        cur = match &mut cdr.value {
            ObjectValue::ListValue(Some(next_cc)) => next_cc.as_mut(),
            _ => return,
        };
    }
    // cur is second-to-last. cur.cdr is the wrapper for last cons cell
    let last_cdr = cur.cdr.as_ref().unwrap();
    if let ObjectValue::ListValue(Some(last_cc)) = &last_cdr.value {
        if let Some(car) = &last_cc.car {
            copy_obj_into(result, car);
        }
    }
    // Set cur.cdr to nil
    if let Some(cdr) = &mut cur.cdr {
        cdr.type_ = ObjectType::Nil;
        cdr.value = ObjectValue::IntValue(0);
    }
}

fn defined_function_push(op_list: &mut Object, op_value: Box<Object>, result: &mut Object) {
    if matches!(op_list.type_, ObjectType::Nil) {
        copy_obj_into(result, &op_value);
        // Convert op_list to a single-element list
        let nil = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
        let cc = Box::new(ConsCell { type_: ConsCellType::Cell, car: Some(op_value), cdr: Some(nil) });
        op_list.type_ = ObjectType::List;
        op_list.value = ObjectValue::ListValue(Some(cc));
        return;
    }
    if !matches!(op_list.type_, ObjectType::List) {
        eprintln!("Type error: push second operand must be list."); std::process::exit(1);
    }
    copy_obj_into(result, &op_value);
    // Append to end
    append_to_cons_list(op_list, op_value);
}

fn append_to_cons_list(obj: &mut Object, item: Box<Object>) {
    let cc = match &mut obj.value {
        ObjectValue::ListValue(Some(cc)) => cc.as_mut(),
        _ => return,
    };
    let mut cur = cc as *mut ConsCell;
    unsafe {
        while !is_last_cons_cell(&*cur) {
            let cdr = (*cur).cdr.as_mut().unwrap();
            cur = match &mut cdr.value {
                ObjectValue::ListValue(Some(next_cc)) => next_cc.as_mut() as *mut ConsCell,
                _ => return,
            };
        }
        // cur is the last cons cell
        let nil = Box::new(Object { marked: false, type_: ObjectType::Nil, value: ObjectValue::IntValue(0) });
        let new_cc = Box::new(ConsCell { type_: ConsCellType::Cell, car: Some(item), cdr: Some(nil) });
        let wrapper = Box::new(Object { marked: false, type_: ObjectType::List, value: ObjectValue::ListValue(Some(new_cc)) });
        (*cur).cdr = Some(wrapper);
        (*cur).type_ = ConsCellType::Cell;
    }
}

fn defined_function_length(op: &Object, result: &mut Object) {
    match op.type_ {
        ObjectType::Nil => {
            result.type_ = ObjectType::Integer;
            result.value = ObjectValue::IntValue(0);
        }
        ObjectType::List => {
            result.type_ = ObjectType::Integer;
            result.value = ObjectValue::IntValue(count_list(op) as i32);
        }
        ObjectType::String => {
            result.type_ = ObjectType::Integer;
            result.value = ObjectValue::IntValue(get_string(&op.value).len() as i32);
        }
        _ => { eprintln!("Type error: length operand must be list or string."); std::process::exit(1); }
    }
}

fn defined_function_is_int_string(op: &Object, result: &mut Object) {
    result.type_ = ObjectType::Bool;
    if matches!(op.type_, ObjectType::String) {
        let s = get_string(&op.value);
        result.value = ObjectValue::BoolValue(if s.chars().all(|c| c.is_ascii_digit()) { 1 } else { 0 });
    } else {
        result.value = ObjectValue::BoolValue(0);
    }
}

fn defined_function_parse_int(op: &Object, result: &mut Object) {
    if !matches!(op.type_, ObjectType::String) {
        eprintln!("Type error: parse-int operand must be string."); std::process::exit(1);
    }
    let s = get_string(&op.value);
    for c in s.chars() {
        if !c.is_ascii_digit() {
            eprintln!("Type error: parse-int operand must be string of digits.");
            std::process::exit(1);
        }
    }
    result.type_ = ObjectType::Integer;
    result.value = ObjectValue::IntValue(s.parse::<i32>().unwrap_or(0));
}

fn defined_function_string_ref(op1: &Object, op2: &Object, result: &mut Object) {
    if !matches!(op1.type_, ObjectType::String) {
        eprintln!("Type error: string-ref first operand must be string."); std::process::exit(1);
    }
    if !matches!(op2.type_, ObjectType::Integer) {
        eprintln!("Type error: string-ref second operand must be integer."); std::process::exit(1);
    }
    let s = get_string(&op1.value);
    let index = get_int(&op2.value);
    if index < 0 || index as usize >= s.len() {
        eprintln!("Index out of range.");
        std::process::exit(1);
    }
    let ch = s.as_bytes()[index as usize] as char;
    result.type_ = ObjectType::String;
    result.value = ObjectValue::StringValue(ch.to_string());
}

// =========== Public API implementations ===========

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
    if let Some(program) = &result.program {
        let expressions_clone = program.expressions.as_ref().map(|e| Box::new(clone_expression_list(e)));
        let mut env = new_env();
        let mut context = init_allocator();
        let mut cur = expressions_clone.as_ref().map(|b| b.as_ref());
        while let Some(el) = cur {
            if let Some(expr) = &el.expression {
                let mut evaluated = new_object_default();
                evaluate_expression(expr, &mut evaluated, &mut env, &mut context);
            }
            cur = el.next.as_ref().map(|b| b.as_ref());
        }
    }
}

pub fn stringify_object(obj: &Object) -> String {
    match obj.type_ {
        ObjectType::Integer => format!("{}", get_int(&obj.value)),
        ObjectType::String => get_string(&obj.value).to_string(),
        ObjectType::Bool => {
            if get_bool_val(&obj.value) != 0 { "T".to_string() } else { "F".to_string() }
        }
        ObjectType::List => {
            let mut parts = Vec::new();
            let mut current = match &obj.value {
                ObjectValue::ListValue(Some(cc)) => Some(cc.as_ref()),
                _ => None,
            };
            while let Some(cc) = current {
                if let Some(car) = &cc.car {
                    parts.push(stringify_object(car));
                }
                if is_last_cons_cell(cc) { break; }
                current = match &cc.cdr {
                    Some(cdr) => match &cdr.value {
                        ObjectValue::ListValue(Some(next_cc)) => Some(next_cc.as_ref()),
                        _ => None,
                    },
                    None => None,
                };
            }
            format!("({})", parts.join(" "))
        }
        ObjectType::Nil => "nil".to_string(),
        ObjectType::Function => "<function>".to_string(),
    }
}

pub fn init_env(env: &mut Env) {
    env.parent = None;
    for i in 0..MAX_BINDINGS {
        env.bindings[i].symbol_name = String::new();
        env.bindings[i].value = None;
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
    Some(new_object_default())
}
