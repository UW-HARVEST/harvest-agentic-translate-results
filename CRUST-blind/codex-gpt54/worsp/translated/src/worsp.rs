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
fn runtime_error(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn empty_binding() -> Binding {
    Binding {
        symbol_name: String::new(),
        value: None,
    }
}

fn default_object() -> Object {
    Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    }
}

fn clone_expression_list(list: &ExpressionList) -> ExpressionList {
    ExpressionList {
        expression: list
            .expression
            .as_ref()
            .map(|expr| Box::new(clone_expression(expr))),
        next: list
            .next
            .as_ref()
            .map(|next| Box::new(clone_expression_list(next))),
    }
}

fn clone_expression(expr: &ExpressionNode) -> ExpressionNode {
    let data = match &expr.data {
        ExpressionData::SymbolicExp(node) => ExpressionData::SymbolicExp(node.as_ref().map(|n| {
            Box::new(SymbolicExpNode {
                expressions: n
                    .expressions
                    .as_ref()
                    .map(|list| Box::new(clone_expression_list(list))),
            })
        })),
        ExpressionData::List(node) => ExpressionData::List(node.as_ref().map(|n| {
            Box::new(ListNode {
                expressions: n
                    .expressions
                    .as_ref()
                    .map(|list| Box::new(clone_expression_list(list))),
            })
        })),
        ExpressionData::Literal(node) => ExpressionData::Literal(node.as_ref().map(|n| {
            Box::new(LiteralNode {
                type_: n.type_,
                value: match &n.value {
                    LiteralValue::IntValue(v) => LiteralValue::IntValue(*v),
                    LiteralValue::BooleanValue(v) => LiteralValue::BooleanValue(*v),
                    LiteralValue::StringValue(v) => LiteralValue::StringValue(v.clone()),
                },
            })
        })),
        ExpressionData::Symbol(node) => ExpressionData::Symbol(node.as_ref().map(|n| {
            Box::new(SymbolNode {
                symbol_name: n.symbol_name.clone(),
            })
        })),
    };

    ExpressionNode {
        type_: expr.type_,
        data,
    }
}

fn clone_function(function: &Function) -> Function {
    Function {
        param_symbol_names: function.param_symbol_names.clone(),
        body: function.body.as_ref().map(|body| Box::new(clone_expression(body))),
    }
}

fn clone_cons_cell(cell: &ConsCell) -> ConsCell {
    ConsCell {
        type_: cell.type_,
        car: cell.car.as_ref().map(|obj| Box::new(clone_object(obj))),
        cdr: cell.cdr.as_ref().map(|obj| Box::new(clone_object(obj))),
    }
}

fn clone_object(obj: &Object) -> Object {
    Object {
        marked: obj.marked,
        type_: obj.type_,
        value: match &obj.value {
            ObjectValue::IntValue(v) => ObjectValue::IntValue(*v),
            ObjectValue::StringValue(v) => ObjectValue::StringValue(v.clone()),
            ObjectValue::BoolValue(v) => ObjectValue::BoolValue(*v),
            ObjectValue::ListValue(v) => {
                ObjectValue::ListValue(v.as_ref().map(|cell| Box::new(clone_cons_cell(cell))))
            }
            ObjectValue::FunctionValue(v) => {
                ObjectValue::FunctionValue(v.as_ref().map(|f| Box::new(clone_function(f))))
            }
        },
    }
}

fn clone_env(env: &Env) -> Env {
    Env {
        bindings: std::array::from_fn(|i| Binding {
            symbol_name: env.bindings[i].symbol_name.clone(),
            value: env.bindings[i]
                .value
                .as_ref()
                .map(|value| Box::new(clone_object(value))),
        }),
        parent: env.parent.as_ref().map(|parent| Box::new(clone_env(parent))),
    }
}

fn object_type_eq(lhs: ObjectType, rhs: ObjectType) -> bool {
    std::mem::discriminant(&lhs) == std::mem::discriminant(&rhs)
}

fn expression_type_eq(lhs: ExpressionType, rhs: ExpressionType) -> bool {
    std::mem::discriminant(&lhs) == std::mem::discriminant(&rhs)
}

fn bool_val(obj: &Object) -> bool {
    match obj.type_ {
        ObjectType::Bool => matches!(obj.value, ObjectValue::BoolValue(v) if v != 0),
        ObjectType::Nil => false,
        _ => true,
    }
}

fn eq_objects(lhs: &Object, rhs: &Object) -> bool {
    if !object_type_eq(lhs.type_, rhs.type_) {
        return false;
    }

    match (&lhs.value, &rhs.value, lhs.type_) {
        (ObjectValue::IntValue(a), ObjectValue::IntValue(b), ObjectType::Integer) => a == b,
        (ObjectValue::StringValue(a), ObjectValue::StringValue(b), ObjectType::String) => a == b,
        (ObjectValue::BoolValue(a), ObjectValue::BoolValue(b), ObjectType::Bool) => a == b,
        (_, _, ObjectType::Nil) => true,
        (ObjectValue::ListValue(a), ObjectValue::ListValue(b), ObjectType::List) => {
            match (a.as_ref(), b.as_ref()) {
                (Some(a_cell), Some(b_cell)) => std::ptr::eq(&**a_cell, &**b_cell),
                (None, None) => true,
                _ => false,
            }
        }
        _ => false,
    }
}

fn append_expression(list: &mut Option<Box<ExpressionList>>, expression: ExpressionNode) {
    let node = Box::new(ExpressionList {
        expression: Some(Box::new(expression)),
        next: None,
    });

    let mut cursor = list;
    loop {
        match cursor {
            None => {
                *cursor = Some(node);
                break;
            }
            Some(current) => {
                cursor = &mut current.next;
            }
        }
    }
}

fn current_token(state: &ParseState) -> &Token {
    state
        .token
        .as_deref()
        .unwrap_or_else(|| runtime_error("Unexpected end of token stream."))
}

fn symbolic_expressions(node: &ExpressionNode) -> Option<&ExpressionList> {
    match &node.data {
        ExpressionData::SymbolicExp(data) => data.as_ref().and_then(|n| n.expressions.as_deref()),
        _ => None,
    }
}

fn list_expressions(node: &ExpressionNode) -> Option<&ExpressionList> {
    match &node.data {
        ExpressionData::List(data) => data.as_ref().and_then(|n| n.expressions.as_deref()),
        _ => None,
    }
}

fn symbol_name(node: &ExpressionNode) -> Option<&str> {
    match &node.data {
        ExpressionData::Symbol(symbol) => symbol.as_ref().map(|s| s.symbol_name.as_str()),
        _ => None,
    }
}

fn literal_node(node: &ExpressionNode) -> Option<&LiteralNode> {
    match &node.data {
        ExpressionData::Literal(literal) => literal.as_deref(),
        _ => None,
    }
}

fn cons_cell_ref(obj: &Object) -> Option<&ConsCell> {
    match &obj.value {
        ObjectValue::ListValue(cell) => cell.as_deref(),
        _ => None,
    }
}

fn set_object_to_env(env: &mut Env, symbol_name: &str, obj: &Object) {
    for binding in &mut env.bindings {
        if binding.symbol_name.is_empty() {
            binding.symbol_name = symbol_name.to_string();
            binding.value = Some(Box::new(clone_object(obj)));
            return;
        }

        if binding.symbol_name == symbol_name {
            binding.value = Some(Box::new(clone_object(obj)));
            return;
        }
    }

    runtime_error("Environment is full.");
}

fn update_existing_binding(env: &mut Env, symbol_name: &str, obj: &Object) -> bool {
    for binding in &mut env.bindings {
        if binding.symbol_name.is_empty() {
            break;
        }

        if binding.symbol_name == symbol_name {
            binding.value = Some(Box::new(clone_object(obj)));
            return true;
        }
    }

    if let Some(parent) = env.parent.as_mut() {
        return update_existing_binding(parent, symbol_name, obj);
    }

    false
}

fn lookup_binding<'a>(env: &'a Env, symbol_name: &str) -> Option<&'a Object> {
    for binding in &env.bindings {
        if binding.symbol_name.is_empty() {
            break;
        }

        if binding.symbol_name == symbol_name {
            return binding.value.as_deref();
        }
    }

    env.parent
        .as_deref()
        .and_then(|parent| lookup_binding(parent, symbol_name))
}

fn list_length(obj: &Object) -> usize {
    if object_type_eq(obj.type_, ObjectType::Nil) {
        return 0;
    }

    if !object_type_eq(obj.type_, ObjectType::List) {
        runtime_error("Type error: length operand must be list or string.");
    }

    let mut length = 0usize;
    let mut current = obj;
    loop {
        let cell = cons_cell_ref(current).unwrap_or_else(|| runtime_error("Malformed list."));
        length += 1;
        let cdr = cell.cdr.as_deref().unwrap_or_else(|| runtime_error("Malformed list."));
        if object_type_eq(cdr.type_, ObjectType::Nil) {
            break;
        }
        current = cdr;
    }

    length
}

fn nth_list_item(obj: &Object, index: i32) -> Object {
    if !object_type_eq(obj.type_, ObjectType::List) {
        runtime_error("Type error: list-ref first operand must be list.");
    }

    if index < 0 {
        runtime_error("Index out of range.");
    }

    let mut current = cons_cell_ref(obj).unwrap_or_else(|| runtime_error("Malformed list."));
    for _ in 0..index {
        let cdr = current.cdr.as_deref().unwrap_or_else(|| runtime_error("Malformed list."));
        if object_type_eq(cdr.type_, ObjectType::Nil) {
            runtime_error("Index out of range.");
        }
        current = cons_cell_ref(cdr).unwrap_or_else(|| runtime_error("Malformed list."));
    }

    clone_object(current.car.as_deref().unwrap_or_else(|| runtime_error("Malformed list.")))
}

fn list_from_objects(items: Vec<Object>) -> Object {
    if items.is_empty() {
        return Object {
            marked: false,
            type_: ObjectType::Nil,
            value: ObjectValue::IntValue(0),
        };
    }

    let mut tail = Object {
        marked: false,
        type_: ObjectType::Nil,
        value: ObjectValue::IntValue(0),
    };

    for item in items.into_iter().rev() {
        tail = Object {
            marked: false,
            type_: ObjectType::List,
            value: ObjectValue::ListValue(Some(Box::new(ConsCell {
                type_: ConsCellType::Cell,
                car: Some(Box::new(item)),
                cdr: Some(Box::new(tail)),
            }))),
        };
    }

    tail
}

fn object_to_items(obj: &Object) -> Vec<Object> {
    if object_type_eq(obj.type_, ObjectType::Nil) {
        return Vec::new();
    }

    if !object_type_eq(obj.type_, ObjectType::List) {
        runtime_error("Type error: operand must be list.");
    }

    let mut items = Vec::new();
    let mut current = obj;
    loop {
        let cell = cons_cell_ref(current).unwrap_or_else(|| runtime_error("Malformed list."));
        items.push(clone_object(
            cell.car
                .as_deref()
                .unwrap_or_else(|| runtime_error("Malformed list.")),
        ));
        let cdr = cell.cdr.as_deref().unwrap_or_else(|| runtime_error("Malformed list."));
        if object_type_eq(cdr.type_, ObjectType::Nil) {
            break;
        }
        current = cdr;
    }
    items
}

fn eval_to_object(
    expression: &ExpressionNode,
    env: &mut Env,
    context: &mut AllocatorContext,
) -> Object {
    let mut value = default_object();
    evaluate_expression(expression, &mut value, env, context);
    value
}

fn parse_expression(source: &str, state: &mut ParseState) -> ExpressionNode {
    if match_token(state, TokenKind::LParen) == 1 {
        let mut expressions = None;
        next(source, state);
        while match_token(state, TokenKind::RParen) == 0 {
            let expression = parse_expression(source, state);
            append_expression(&mut expressions, expression);
        }
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::SymbolicExp,
            data: ExpressionData::SymbolicExp(Some(Box::new(SymbolicExpNode { expressions }))),
        }
    } else if match_token(state, TokenKind::Quote) == 1 {
        let mut expressions = None;
        next(source, state);
        next(source, state);
        while match_token(state, TokenKind::RParen) == 0 {
            let expression = parse_expression(source, state);
            append_expression(&mut expressions, expression);
        }
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::List,
            data: ExpressionData::List(Some(Box::new(ListNode { expressions }))),
        }
    } else if match_token(state, TokenKind::Symbol) == 1 {
        let symbol = current_token(state).str.clone();
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Symbol,
            data: ExpressionData::Symbol(Some(Box::new(SymbolNode { symbol_name: symbol }))),
        }
    } else if match_token(state, TokenKind::Digit) == 1 {
        let value = current_token(state).val;
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Integer,
                value: LiteralValue::IntValue(value),
            }))),
        }
    } else if match_token(state, TokenKind::String) == 1 {
        let value = current_token(state).str.clone();
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::String,
                value: LiteralValue::StringValue(value),
            }))),
        }
    } else if match_token(state, TokenKind::True) == 1 {
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Boolean,
                value: LiteralValue::BooleanValue(true),
            }))),
        }
    } else if match_token(state, TokenKind::False) == 1 {
        next(source, state);
        ExpressionNode {
            type_: ExpressionType::Literal,
            data: ExpressionData::Literal(Some(Box::new(LiteralNode {
                type_: LiteralType::Boolean,
                value: LiteralValue::BooleanValue(false),
            }))),
        }
    } else {
        runtime_error(&format!("Unexpected token: {}", current_token(state).str));
    }
}
pub fn match_token(state: &mut ParseState, kind: TokenKind) -> i32 {
    match state.token.as_deref() {
        Some(token) if token.kind == kind => 1,
        _ => 0,
    }
}
pub fn next(source: &str, state: &mut ParseState) {
    let chars: Vec<char> = source.chars().collect();
    while (state.pos as usize) < chars.len() && chars[state.pos as usize].is_whitespace() {
        state.pos += 1;
    }

    let pos = state.pos as usize;
    let token = if pos >= chars.len() {
        Token {
            kind: TokenKind::Eof,
            next: None,
            val: 0,
            str: "\0".to_string(),
        }
    } else {
        match chars[pos] {
            '(' => {
                state.pos += 1;
                Token {
                    kind: TokenKind::LParen,
                    next: None,
                    val: 0,
                    str: "(".to_string(),
                }
            }
            ')' => {
                state.pos += 1;
                Token {
                    kind: TokenKind::RParen,
                    next: None,
                    val: 0,
                    str: ")".to_string(),
                }
            }
            '\'' => {
                state.pos += 1;
                Token {
                    kind: TokenKind::Quote,
                    next: None,
                    val: 0,
                    str: "'".to_string(),
                }
            }
            '"' => {
                state.pos += 1;
                let start = state.pos as usize;
                while (state.pos as usize) < chars.len() && chars[state.pos as usize] != '"' {
                    state.pos += 1;
                }
                let end = state.pos as usize;
                let token_str: String = chars[start..end].iter().collect();
                if (state.pos as usize) < chars.len() && chars[state.pos as usize] == '"' {
                    state.pos += 1;
                }
                Token {
                    kind: TokenKind::String,
                    next: None,
                    val: 0,
                    str: token_str,
                }
            }
            ';' => {
                while (state.pos as usize) < chars.len() && chars[state.pos as usize] != '\n' {
                    state.pos += 1;
                }
                next(source, state);
                return;
            }
            ch if ch.is_ascii_digit() => {
                let start = state.pos as usize;
                while (state.pos as usize) < chars.len()
                    && chars[state.pos as usize].is_ascii_digit()
                {
                    state.pos += 1;
                }
                let token_str: String = chars[start..state.pos as usize].iter().collect();
                let value = token_str.parse::<i32>().unwrap_or(0);
                Token {
                    kind: TokenKind::Digit,
                    next: None,
                    val: value,
                    str: String::new(),
                }
            }
            ch if ch.is_ascii_alphabetic() || "+-*/%|&=<>".contains(ch) => {
                let start = state.pos as usize;
                while (state.pos as usize) < chars.len() {
                    let current = chars[state.pos as usize];
                    if current.is_ascii_alphanumeric() || "+-*/%|&=<>".contains(current) {
                        state.pos += 1;
                    } else {
                        break;
                    }
                }
                let token_str: String = chars[start..state.pos as usize].iter().collect();
                let kind = match token_str.as_str() {
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    _ => TokenKind::Symbol,
                };
                Token {
                    kind,
                    next: None,
                    val: 0,
                    str: token_str,
                }
            }
            unexpected => runtime_error(&format!("Unexpected token: {unexpected}")),
        }
    };

    state.token = Some(Box::new(token));
}
pub fn parse(source: &str, state: &mut ParseState, result: &mut ParseResult) {
    next(source, state);
    let mut expressions = None;

    while match_token(state, TokenKind::Eof) == 0 {
        let expression = parse_expression(source, state);
        append_expression(&mut expressions, expression);
    }

    result.program = Some(Box::new(ProgramNode { expressions }));
}
pub fn evaluate_expression(
expression: &ExpressionNode,
result: &mut Object,
env: &mut Env,
context: &mut AllocatorContext,
) {
    if let Some(stack) = context.stack.as_mut() {
        let next_index = stack.top + 1;
        if next_index >= 0 && (next_index as usize) < OBJECT_NUMBER {
            stack.top = next_index;
            stack.objects[next_index as usize] = Some(Box::new(clone_object(result)));
        }
    }

    match expression.type_ {
        ExpressionType::Literal => {
            let literal = literal_node(expression)
                .unwrap_or_else(|| runtime_error("Malformed literal expression."));
            match &literal.value {
                LiteralValue::IntValue(value) => {
                    result.type_ = ObjectType::Integer;
                    result.value = ObjectValue::IntValue(*value);
                }
                LiteralValue::StringValue(value) => {
                    result.type_ = ObjectType::String;
                    result.value = ObjectValue::StringValue(value.clone());
                }
                LiteralValue::BooleanValue(value) => {
                    result.type_ = ObjectType::Bool;
                    result.value = ObjectValue::BoolValue(i32::from(*value));
                }
            }
        }
        ExpressionType::Symbol => {
            let name = symbol_name(expression)
                .unwrap_or_else(|| runtime_error("Malformed symbol expression."));
            if name == "nil" {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
            } else if let Some(value) = lookup_binding(env, name) {
                *result = clone_object(value);
            } else {
                runtime_error(&format!("Undefined symbol: {name}"));
            }
        }
        ExpressionType::List => {
            let mut items = Vec::new();
            let mut cursor = list_expressions(expression);
            while let Some(node) = cursor {
                let expr = node
                    .expression
                    .as_deref()
                    .unwrap_or_else(|| runtime_error("Malformed list expression."));
                items.push(eval_to_object(expr, env, context));
                cursor = node.next.as_deref();
            }
            *result = list_from_objects(items);
        }
        ExpressionType::SymbolicExp => {
            let Some(expressions) = symbolic_expressions(expression) else {
                result.type_ = ObjectType::Nil;
                result.value = ObjectValue::IntValue(0);
                if let Some(stack) = context.stack.as_mut() {
                    if stack.top >= 0 {
                        stack.objects[stack.top as usize] = None;
                        stack.top -= 1;
                    }
                }
                return;
            };

            let head = expressions
                .expression
                .as_deref()
                .unwrap_or_else(|| runtime_error("Malformed symbolic expression."));
            let head_name = symbol_name(head)
                .unwrap_or_else(|| runtime_error("S-exp must be started with symbol."));

            match head_name {
                "if" => {
                    let cond = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("if must have condition."));
                    let then_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("if must have then clause."));
                    let cond_obj = eval_to_object(cond, env, context);
                    if bool_val(&cond_obj) {
                        evaluate_expression(then_expr, result, env, context);
                    } else if let Some(else_expr) = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                    {
                        evaluate_expression(else_expr, result, env, context);
                    } else {
                        result.type_ = ObjectType::Nil;
                        result.value = ObjectValue::IntValue(0);
                    }
                }
                "while" => {
                    let cond = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("if must have condition."));
                    let body = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("if must have then clause."));
                    loop {
                        let cond_obj = eval_to_object(cond, env, context);
                        if bool_val(&cond_obj) {
                            evaluate_expression(body, result, env, context);
                        } else {
                            result.type_ = ObjectType::Nil;
                            result.value = ObjectValue::IntValue(0);
                            break;
                        }
                    }
                }
                "=" => {
                    let symbol_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Variable name must be symbol."));
                    let target_name = symbol_name(symbol_expr)
                        .unwrap_or_else(|| runtime_error("Variable name must be symbol."));
                    let value_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("assignment must have expression."));
                    let evaluated_value = eval_to_object(value_expr, env, context);
                    *result = clone_object(&evaluated_value);
                    set_object_to_env(env, target_name, &evaluated_value);
                }
                "defun" => {
                    let name_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Function name must be symbol."));
                    let fn_name = symbol_name(name_expr)
                        .unwrap_or_else(|| runtime_error("Function name must be symbol."));
                    let params_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Function must have parameter."));

                    if !expression_type_eq(params_expr.type_, ExpressionType::SymbolicExp) {
                        runtime_error("Function parameter must be list.");
                    }

                    let mut param_names = Vec::new();
                    let mut cursor = symbolic_expressions(params_expr);
                    while let Some(param) = cursor {
                        let param_expr = param
                            .expression
                            .as_deref()
                            .unwrap_or_else(|| runtime_error("Function parameter must be symbol."));
                        let param_name = symbol_name(param_expr)
                            .unwrap_or_else(|| runtime_error("Function parameter must be symbol."));
                        param_names.push(param_name.to_string());
                        cursor = param.next.as_deref();
                    }

                    let body_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Function must have body."));

                    result.type_ = ObjectType::Function;
                    result.value = ObjectValue::FunctionValue(Some(Box::new(Function {
                        param_symbol_names: param_names,
                        body: Some(Box::new(clone_expression(body_expr))),
                    })));
                    let current = clone_object(result);
                    set_object_to_env(env, fn_name, &current);
                }
                "+" => {
                    let lhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let rhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.next.as_deref())
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                        (
                            ObjectValue::IntValue(a),
                            ObjectValue::IntValue(b),
                            ObjectType::Integer,
                            ObjectType::Integer,
                        ) => {
                            result.type_ = ObjectType::Integer;
                            result.value = ObjectValue::IntValue(a + b);
                        }
                        (
                            ObjectValue::StringValue(a),
                            ObjectValue::StringValue(b),
                            ObjectType::String,
                            ObjectType::String,
                        ) => {
                            result.type_ = ObjectType::String;
                            result.value = ObjectValue::StringValue(format!("{a}{b}"));
                        }
                        _ => runtime_error(
                            "Type error: operands for + must be integers or strings.",
                        ),
                    }
                }
                "-" | "*" | "/" | "%" | "<" | ">" | "eq" => {
                    let lhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let rhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.next.as_deref())
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );

                    match head_name {
                        "-" => match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                            (
                                ObjectValue::IntValue(a),
                                ObjectValue::IntValue(b),
                                ObjectType::Integer,
                                ObjectType::Integer,
                            ) => {
                                result.type_ = ObjectType::Integer;
                                result.value = ObjectValue::IntValue(a - b);
                            }
                            _ => runtime_error("Type error: operands for - must be integers."),
                        },
                        "*" => match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                            (
                                ObjectValue::IntValue(a),
                                ObjectValue::IntValue(b),
                                ObjectType::Integer,
                                ObjectType::Integer,
                            ) => {
                                result.type_ = ObjectType::Integer;
                                result.value = ObjectValue::IntValue(a * b);
                            }
                            _ => runtime_error("Type error: operands for * must be integers."),
                        },
                        "/" => match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                            (
                                ObjectValue::IntValue(a),
                                ObjectValue::IntValue(b),
                                ObjectType::Integer,
                                ObjectType::Integer,
                            ) => {
                                result.type_ = ObjectType::Integer;
                                result.value = ObjectValue::IntValue(a / b);
                            }
                            _ => runtime_error("Type error: operands for / must be integers."),
                        },
                        "%" => match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                            (
                                ObjectValue::IntValue(a),
                                ObjectValue::IntValue(b),
                                ObjectType::Integer,
                                ObjectType::Integer,
                            ) => {
                                result.type_ = ObjectType::Integer;
                                result.value = ObjectValue::IntValue(a % b);
                            }
                            _ => runtime_error("Type error: operands for % must be integers."),
                        },
                        "<" => match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                            (
                                ObjectValue::IntValue(a),
                                ObjectValue::IntValue(b),
                                ObjectType::Integer,
                                ObjectType::Integer,
                            ) => {
                                result.type_ = ObjectType::Bool;
                                result.value = ObjectValue::BoolValue(i32::from(a < b));
                            }
                            _ => runtime_error("Type error: operands for < must be integers."),
                        },
                        ">" => match (&lhs.value, &rhs.value, lhs.type_, rhs.type_) {
                            (
                                ObjectValue::IntValue(a),
                                ObjectValue::IntValue(b),
                                ObjectType::Integer,
                                ObjectType::Integer,
                            ) => {
                                result.type_ = ObjectType::Bool;
                                result.value = ObjectValue::BoolValue(i32::from(a > b));
                            }
                            _ => runtime_error("Type error: operands for < must be integers."),
                        },
                        "eq" => {
                            result.type_ = ObjectType::Bool;
                            result.value = ObjectValue::BoolValue(i32::from(eq_objects(&lhs, &rhs)));
                        }
                        _ => runtime_error("Unexpected arithmetic operator."),
                    }
                }
                "||" | "&&" => {
                    let mut cursor = expressions.next.as_deref();
                    if head_name == "||" {
                        while let Some(expr_node) = cursor {
                            let operand = eval_to_object(
                                expr_node
                                    .expression
                                    .as_deref()
                                    .unwrap_or_else(|| runtime_error("Malformed expression.")),
                                env,
                                context,
                            );
                            if bool_val(&operand) {
                                result.type_ = ObjectType::Bool;
                                result.value = ObjectValue::BoolValue(1);
                                if let Some(stack) = context.stack.as_mut() {
                                    if stack.top >= 0 {
                                        stack.objects[stack.top as usize] = None;
                                        stack.top -= 1;
                                    }
                                }
                                return;
                            }
                            cursor = expr_node.next.as_deref();
                        }
                        result.type_ = ObjectType::Bool;
                        result.value = ObjectValue::BoolValue(0);
                    } else {
                        while let Some(expr_node) = cursor {
                            let operand = eval_to_object(
                                expr_node
                                    .expression
                                    .as_deref()
                                    .unwrap_or_else(|| runtime_error("Malformed expression.")),
                                env,
                                context,
                            );
                            if !bool_val(&operand) {
                                result.type_ = ObjectType::Bool;
                                result.value = ObjectValue::BoolValue(0);
                                if let Some(stack) = context.stack.as_mut() {
                                    if stack.top >= 0 {
                                        stack.objects[stack.top as usize] = None;
                                        stack.top -= 1;
                                    }
                                }
                                return;
                            }
                            cursor = expr_node.next.as_deref();
                        }
                        result.type_ = ObjectType::Bool;
                        result.value = ObjectValue::BoolValue(1);
                    }
                }
                "not" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    if !object_type_eq(operand.type_, ObjectType::Bool) {
                        runtime_error("Type error: not operand must be boolean.");
                    }
                    result.type_ = ObjectType::Bool;
                    result.value = ObjectValue::BoolValue(i32::from(!bool_val(&operand)));
                }
                "print" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    println!("{}", stringify_object(&operand));
                    result.type_ = ObjectType::Nil;
                    result.value = ObjectValue::IntValue(0);
                }
                "car" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    if !object_type_eq(operand.type_, ObjectType::List) {
                        runtime_error("Type error: car operand must be list.");
                    }
                    *result = clone_object(
                        cons_cell_ref(&operand)
                            .and_then(|cell| cell.car.as_deref())
                            .unwrap_or_else(|| runtime_error("Malformed list.")),
                    );
                }
                "cdr" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    if !object_type_eq(operand.type_, ObjectType::List) {
                        runtime_error("Type error: cdr operand must be list.");
                    }
                    *result = clone_object(
                        cons_cell_ref(&operand)
                            .and_then(|cell| cell.cdr.as_deref())
                            .unwrap_or_else(|| runtime_error("Malformed list.")),
                    );
                }
                "cons" => {
                    let lhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let rhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.next.as_deref())
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let mut items = vec![clone_object(&lhs)];
                    if object_type_eq(rhs.type_, ObjectType::List)
                        || object_type_eq(rhs.type_, ObjectType::Nil)
                    {
                        items.extend(object_to_items(&rhs));
                    } else {
                        items.push(clone_object(&rhs));
                    }
                    *result = list_from_objects(items);
                }
                "readline" => {
                    use std::io::{self, Read};
                    let mut input = String::new();
                    let mut handle = io::stdin().lock();
                    match handle.read_to_string(&mut input) {
                        Ok(0) => {
                            result.type_ = ObjectType::Nil;
                            result.value = ObjectValue::IntValue(0);
                        }
                        Ok(_) => {
                            if let Some(idx) = input.find('\n') {
                                input.truncate(idx);
                            }
                            if input.ends_with('\r') {
                                input.pop();
                            }
                            result.type_ = ObjectType::String;
                            result.value = ObjectValue::StringValue(input);
                        }
                        Err(_) => {
                            result.type_ = ObjectType::Nil;
                            result.value = ObjectValue::IntValue(0);
                        }
                    }
                }
                "split" => {
                    let lhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let rhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.next.as_deref())
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );

                    let ObjectValue::StringValue(input) = &lhs.value else {
                        runtime_error("Type error: split first operand must be string.");
                    };
                    let ObjectValue::StringValue(delimiter) = &rhs.value else {
                        runtime_error("Type error: split second operand must be string.");
                    };

                    let parts = if delimiter.is_empty() {
                        input.chars().map(|ch| ch.to_string()).collect::<Vec<_>>()
                    } else {
                        input
                            .split(|ch| delimiter.contains(ch))
                            .filter(|part| !part.is_empty())
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                    };

                    let objects = parts
                        .into_iter()
                        .map(|part| Object {
                            marked: false,
                            type_: ObjectType::String,
                            value: ObjectValue::StringValue(part),
                        })
                        .collect::<Vec<_>>();
                    *result = list_from_objects(objects);
                }
                "list-ref" => {
                    let lhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let rhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.next.as_deref())
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let index = match rhs.value {
                        ObjectValue::IntValue(v) if object_type_eq(rhs.type_, ObjectType::Integer) => v,
                        _ => runtime_error("Type error: list-ref second operand must be integer."),
                    };
                    *result = nth_list_item(&lhs, index);
                }
                "progn" => {
                    let mut cursor = expressions.next.as_deref();
                    let mut last = None;
                    while let Some(expr_node) = cursor {
                        let value = eval_to_object(
                            expr_node
                                .expression
                                .as_deref()
                                .unwrap_or_else(|| runtime_error("Malformed expression.")),
                            env,
                            context,
                        );
                        last = Some(value);
                        cursor = expr_node.next.as_deref();
                    }
                    if let Some(value) = last {
                        *result = value;
                    } else {
                        result.type_ = ObjectType::Nil;
                        result.value = ObjectValue::IntValue(0);
                    }
                }
                "remove-whitespaces" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let ObjectValue::StringValue(value) = &operand.value else {
                        runtime_error("Type error: remove-whitespaces operand must be string.");
                    };
                    result.type_ = ObjectType::String;
                    result.value =
                        ObjectValue::StringValue(value.chars().filter(|c| !c.is_whitespace()).collect());
                }
                "pop" => {
                    let arg_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Missing operand."));
                    let operand = eval_to_object(arg_expr, env, context);
                    if object_type_eq(operand.type_, ObjectType::Nil) {
                        result.type_ = ObjectType::Nil;
                        result.value = ObjectValue::IntValue(0);
                    } else if !object_type_eq(operand.type_, ObjectType::List) {
                        runtime_error("Type error: pop operand must be list.");
                    } else {
                        let mut items = object_to_items(&operand);
                        let popped = items.pop().unwrap_or_else(|| default_object());
                        *result = popped;
                        let updated_list = list_from_objects(items);
                        if let Some(name) = symbol_name(arg_expr) {
                            let _ = update_existing_binding(env, name, &updated_list);
                        }
                    }
                }
                "push" => {
                    let list_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Missing operand."));
                    let value_expr = expressions
                        .next
                        .as_deref()
                        .and_then(|n| n.next.as_deref())
                        .and_then(|n| n.expression.as_deref())
                        .unwrap_or_else(|| runtime_error("Missing operand."));
                    let list_obj = eval_to_object(list_expr, env, context);
                    let value_obj = eval_to_object(value_expr, env, context);

                    if object_type_eq(list_obj.type_, ObjectType::Nil) {
                        *result = clone_object(&value_obj);
                        if let Some(name) = symbol_name(list_expr) {
                            let new_list = list_from_objects(vec![clone_object(&value_obj)]);
                            let _ = update_existing_binding(env, name, &new_list);
                        }
                    } else if !object_type_eq(list_obj.type_, ObjectType::List) {
                        runtime_error("Type error: push second operand must be list.");
                    } else {
                        let mut items = object_to_items(&list_obj);
                        items.push(clone_object(&value_obj));
                        let updated_list = list_from_objects(items);
                        if let Some(name) = symbol_name(list_expr) {
                            let _ = update_existing_binding(env, name, &updated_list);
                        }
                        *result = clone_object(&value_obj);
                    }
                }
                "length" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let length = match &operand.value {
                        ObjectValue::StringValue(value)
                            if object_type_eq(operand.type_, ObjectType::String) =>
                        {
                            value.len()
                        }
                        _ => list_length(&operand),
                    };
                    result.type_ = ObjectType::Integer;
                    result.value = ObjectValue::IntValue(length as i32);
                }
                "is-int-string" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let is_int = matches!(
                        &operand.value,
                        ObjectValue::StringValue(value) if value.chars().all(|c| c.is_ascii_digit())
                    );
                    result.type_ = ObjectType::Bool;
                    result.value = ObjectValue::BoolValue(i32::from(is_int));
                }
                "parse-int" => {
                    let operand = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let ObjectValue::StringValue(value) = &operand.value else {
                        runtime_error("Type error: parse-int operand must be string.");
                    };
                    if !value.chars().all(|c| c.is_ascii_digit()) {
                        runtime_error("Type error: parse-int operand must be string of digits.");
                    }
                    result.type_ = ObjectType::Integer;
                    result.value = ObjectValue::IntValue(value.parse::<i32>().unwrap_or(0));
                }
                "string-ref" => {
                    let lhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let rhs = eval_to_object(
                        expressions
                            .next
                            .as_deref()
                            .and_then(|n| n.next.as_deref())
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing operand.")),
                        env,
                        context,
                    );
                    let ObjectValue::StringValue(value) = &lhs.value else {
                        runtime_error("Type error: string-ref first operand must be string.");
                    };
                    let index = match rhs.value {
                        ObjectValue::IntValue(v) if object_type_eq(rhs.type_, ObjectType::Integer) => v,
                        _ => runtime_error("Type error: string-ref second operand must be integer."),
                    };
                    if index < 0 || index as usize >= value.len() {
                        runtime_error("Index out of range.");
                    }
                    let ch = value
                        .chars()
                        .nth(index as usize)
                        .unwrap_or_else(|| runtime_error("Index out of range."));
                    result.type_ = ObjectType::String;
                    result.value = ObjectValue::StringValue(ch.to_string());
                }
                _ => {
                    let function_object = lookup_binding(env, head_name)
                        .map(clone_object)
                        .unwrap_or_else(|| runtime_error(&format!("Undefined function: {head_name}")));
                    if !object_type_eq(function_object.type_, ObjectType::Function) {
                        runtime_error(&format!("Undefined function: {head_name}"));
                    }
                    let ObjectValue::FunctionValue(function) = function_object.value else {
                        runtime_error(&format!("Undefined function: {head_name}"));
                    };
                    let function = function
                        .as_deref()
                        .unwrap_or_else(|| runtime_error(&format!("Undefined function: {head_name}")));

                    let mut new_env = Env {
                        bindings: std::array::from_fn(|_| empty_binding()),
                        parent: Some(Box::new(clone_env(env))),
                    };

                    let mut arg_expr = expressions.next.as_deref();
                    for param_name in &function.param_symbol_names {
                        let expr_node = arg_expr
                            .and_then(|n| n.expression.as_deref())
                            .unwrap_or_else(|| runtime_error("Missing function argument."));
                        let value = eval_to_object(expr_node, env, context);
                        set_object_to_env(&mut new_env, param_name, &value);
                        arg_expr = arg_expr.and_then(|n| n.next.as_deref());
                    }

                    let body = function
                        .body
                        .as_deref()
                        .unwrap_or_else(|| runtime_error("Function must have body."));
                    evaluate_expression(body, result, &mut new_env, context);
                }
            }
        }
    }

    if let Some(stack) = context.stack.as_mut() {
        if stack.top >= 0 {
            stack.objects[stack.top as usize] = None;
            stack.top -= 1;
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
    let Some(program) = result.program.as_deref() else {
        return;
    };
    let mut env = Env {
        bindings: std::array::from_fn(|_| empty_binding()),
        parent: None,
    };
    init_env(&mut env);
    let mut context = init_allocator();
    let mut cursor = program.expressions.as_deref();
    while let Some(node) = cursor {
        let expression = node
            .expression
            .as_deref()
            .unwrap_or_else(|| runtime_error("Malformed program expression."));
        let mut value = default_object();
        evaluate_expression(expression, &mut value, &mut env, &mut context);
        cursor = node.next.as_deref();
    }
}
pub fn stringify_object(obj: &Object) -> String {
    match (&obj.type_, &obj.value) {
        (ObjectType::Integer, ObjectValue::IntValue(value)) => value.to_string(),
        (ObjectType::String, ObjectValue::StringValue(value)) => value.clone(),
        (ObjectType::Bool, ObjectValue::BoolValue(value)) => {
            if *value != 0 {
                "T".to_string()
            } else {
                "F".to_string()
            }
        }
        (ObjectType::Nil, _) => "nil".to_string(),
        (ObjectType::Function, _) => "<function>".to_string(),
        (ObjectType::List, ObjectValue::ListValue(_)) => {
            let parts = object_to_items(obj)
                .into_iter()
                .map(|item| stringify_object(&item))
                .collect::<Vec<_>>();
            format!("({})", parts.join(" "))
        }
        _ => runtime_error(&format!("Unexpected object type: {:?}", obj.type_)),
    }
}
pub fn init_env(env: &mut Env) {
    env.parent = None;
    env.bindings = std::array::from_fn(|_| empty_binding());
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
pub fn allocate(context: &mut AllocatorContext, env: &mut Env) -> Option<Box<Object>> {
    let _ = env;
    if let Some(index) = context.free_bitmap.iter().position(|bit| *bit == 0) {
        context.free_bitmap[index] = 1;
        Some(Box::new(default_object()))
    } else {
        Some(Box::new(default_object()))
    }
}
