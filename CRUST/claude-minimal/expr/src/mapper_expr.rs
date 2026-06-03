use std::f32::consts::PI;
use std::collections::HashMap;
use lazy_static::lazy_static;

const TRACING: bool = false;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}
impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MapperSignalValue::F(f) => Some(*f),
            MapperSignalValue::I32(i) => Some(*i as f32),
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::F(f) => Some(*f as i32),
            MapperSignalValue::I32(i) => Some(*i),
        }
    }
}
const STACK_SIZE: usize = 256;
macro_rules! trace {
    ($($arg:tt)*) => {
        if TRACING {
            println!("-- {}", format!($($arg)*));
        }
    };
}
macro_rules! die_unless {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            println!("-- {}", format!($($arg)*));
            assert!($cond);
        }
    };
}
fn minf(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}
fn maxf(x: f32, y: f32) -> f32 {
    if y > x { y } else { x }
}
fn pif() -> f32 {
    PI
}
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprFunc {
    Unknown = -1,
    Pow = 0,
    Sin,
    Cos,
    Tan,
    Abs,
    Sqrt,
    Log,
    Log10,
    Exp,
    Floor,
    Round,
    Ceil,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Logb,
    Exp2,
    Log2,
    Hypot,
    Cbrt,
    Trunc,
    Min,
    Max,
    Pi,
    NFuncs,
}
#[derive(Debug, Clone, Copy)]
struct FunctionEntry {
    name: &'static str,
    arity: u32,
    func: fn(f32, f32) -> f32,
}
lazy_static::lazy_static! {
    static ref FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        let mut m = HashMap::new();
        m.insert("pow", FunctionEntry { name: "pow", arity: 2, func: f32::powf });
        m.insert("sin", FunctionEntry { name: "sin", arity: 1, func: |x, _| x.sin() });
        m.insert("cos", FunctionEntry { name: "cos", arity: 1, func: |x, _| x.cos() });
        m.insert("tan", FunctionEntry { name: "tan", arity: 1, func: |x, _| x.tan() });
        m.insert("abs", FunctionEntry { name: "abs", arity: 1, func: |x, _| x.abs() });
        m.insert("sqrt", FunctionEntry { name: "sqrt", arity: 1, func: |x, _| x.sqrt() });
        m.insert("log", FunctionEntry { name: "log", arity: 1, func: |x, _| x.ln() });
        m.insert("log10", FunctionEntry { name: "log10", arity: 1, func: |x, _| x.log10() });
        m.insert("exp", FunctionEntry { name: "exp", arity: 1, func: |x, _| x.exp() });
        m.insert("floor", FunctionEntry { name: "floor", arity: 1, func: |x, _| x.floor() });
        m.insert("round", FunctionEntry { name: "round", arity: 1, func: |x, _| x.round() });
        m.insert("ceil", FunctionEntry { name: "ceil", arity: 1, func: |x, _| x.ceil() });
        m.insert("asin", FunctionEntry { name: "asin", arity: 1, func: |x, _| x.asin() });
        m.insert("acos", FunctionEntry { name: "acos", arity: 1, func: |x, _| x.acos() });
        m.insert("atan", FunctionEntry { name: "atan", arity: 1, func: |x, _| x.atan() });
        m.insert("atan2", FunctionEntry { name: "atan2", arity: 2, func: |y, x| y.atan2(x) });
        m.insert("sinh", FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() });
        m.insert("cosh", FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() });
        m.insert("tanh", FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() });
        m.insert("exp2", FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() });
        m.insert("log2", FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() });
        m.insert("hypot", FunctionEntry { name: "hypot", arity: 2, func: |x, y| x.hypot(y) });
        m.insert("cbrt", FunctionEntry { name: "cbrt", arity: 1, func: |x, _| x.cbrt() });
        m.insert("trunc", FunctionEntry { name: "trunc", arity: 1, func: |x, _| x.trunc() });
        m.insert("min", FunctionEntry { name: "min", arity: 2, func: minf });
        m.insert("max", FunctionEntry { name: "max", arity: 2, func: maxf });
        m.insert("pi", FunctionEntry { name: "pi", arity: 0, func: |_, _| pif() });
        m
    };
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Float,
    Int,
    Op,
    OpenParen,
    CloseParen,
    Var,
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Func,
    Comma,
    End,
    ToFloat,
    ToInt32,
}
#[derive(Debug, Clone, Copy)]
struct Token {
    token_type: TokenType,
    value: Option<f32>,
    int_value: Option<i32>,
    var: Option<char>,
    op: Option<char>,
    func: Option<&'static str>,
}
impl Token {
    fn new(t: TokenType) -> Token {
        Token {
            token_type: t,
            value: None,
            int_value: None,
            var: None,
            op: None,
            func: None,
        }
    }
}
fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

fn expr_lex(chars: &[char], pos: &mut usize) -> Result<Token, String> {
    if *pos >= chars.len() {
        return Ok(Token::new(TokenType::End));
    }
    let mut c = chars[*pos];
    let mut integer_found = false;
    let mut n: i32 = 0;

    loop {
        if c.is_ascii_digit() {
            let start = *pos;
            while *pos < chars.len() && chars[*pos].is_ascii_digit() {
                *pos += 1;
            }
            let s: String = chars[start..*pos].iter().collect();
            n = s.parse().unwrap_or(0);
            integer_found = true;
            c = if *pos < chars.len() { chars[*pos] } else { '\0' };
            if c != '.' {
                let mut t = Token::new(TokenType::Int);
                t.int_value = Some(n);
                return Ok(t);
            }
        }

        match c {
            '.' => {
                let start = *pos;
                *pos += 1;
                let next_c = if *pos < chars.len() { chars[*pos] } else { '\0' };
                if !next_c.is_ascii_digit() && integer_found {
                    let mut t = Token::new(TokenType::Float);
                    t.value = Some(n as f32);
                    return Ok(t);
                }
                if !next_c.is_ascii_digit() {
                    return Err(format!("Unexpected '.' at position {}", *pos));
                }
                while *pos < chars.len() && chars[*pos].is_ascii_digit() {
                    *pos += 1;
                }
                let frac_str: String = chars[start..*pos].iter().collect();
                let frac: f32 = frac_str.parse().unwrap_or(0.0);
                let mut t = Token::new(TokenType::Float);
                t.value = Some(n as f32 + frac);
                return Ok(t);
            }
            '+' | '-' | '/' | '*' | '=' => {
                *pos += 1;
                let mut t = Token::new(TokenType::Op);
                t.op = Some(c);
                return Ok(t);
            }
            '(' => { *pos += 1; return Ok(Token::new(TokenType::OpenParen)); }
            ')' => { *pos += 1; return Ok(Token::new(TokenType::CloseParen)); }
            'x' | 'y' => {
                // Look ahead: if more alpha chars follow, treat as identifier (function)
                let mut end = *pos + 1;
                while end < chars.len() && (chars[end].is_ascii_alphabetic() || chars[end].is_ascii_digit()) {
                    end += 1;
                }
                if end == *pos + 1 {
                    *pos += 1;
                    let mut t = Token::new(TokenType::Var);
                    t.var = Some(c);
                    return Ok(t);
                } else {
                    let name: String = chars[*pos..end].iter().collect();
                    *pos = end;
                    let mut t = Token::new(TokenType::Func);
                    t.func = function_lookup(&name).map(|e| e.name);
                    return Ok(t);
                }
            }
            '[' => { *pos += 1; return Ok(Token::new(TokenType::OpenSquare)); }
            ']' => { *pos += 1; return Ok(Token::new(TokenType::CloseSquare)); }
            '{' => { *pos += 1; return Ok(Token::new(TokenType::OpenCurly)); }
            '}' => { *pos += 1; return Ok(Token::new(TokenType::CloseCurly)); }
            ' ' | '\t' | '\r' | '\n' => {
                *pos += 1;
                if *pos >= chars.len() {
                    return Ok(Token::new(TokenType::End));
                }
                c = chars[*pos];
                continue;
            }
            ',' => { *pos += 1; return Ok(Token::new(TokenType::Comma)); }
            _ => {
                if !c.is_ascii_alphabetic() {
                    return Err(format!("unknown character '{}' in lexer", c));
                }
                let start = *pos;
                while *pos < chars.len() && (chars[*pos].is_ascii_alphabetic() || chars[*pos].is_ascii_digit()) {
                    *pos += 1;
                }
                let name: String = chars[start..*pos].iter().collect();
                let mut t = Token::new(TokenType::Func);
                t.func = function_lookup(&name).map(|e| e.name);
                return Ok(t);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExprNode {
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
    pub next: Option<Box<ExprNode>>,
}
impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: Token::new(TokenType::End),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // Rust will free automatically when dropped; nothing to do.
    }
}

fn exprnode_new(tok: &Token, is_float: i32) -> Box<ExprNode> {
    Box::new(ExprNode {
        tok: *tok,
        is_float,
        history_index: 0,
        vector_index: 0,
        next: None,
    })
}

pub struct MapperExpr {
    pub node: ExprNode,
    pub vector_size: i32,
    pub history_size: i32,
    pub history_pos: i32,
    pub input_history: Vec<MapperSignalValue>,
    pub output_history: Vec<MapperSignalValue>,
}

#[allow(non_camel_case_types)]
pub enum state_t {
    YEQUAL_Y,
    YEQUAL_EQ,
    EXPR,
    EXPR_RIGHT,
    TERM,
    TERM_RIGHT,
    VALUE,
    NEGATE,
    VAR_RIGHT,
    VAR_VECTINDEX,
    VAR_HISTINDEX,
    CLOSE_VECTINDEX,
    CLOSE_HISTINDEX,
    OPEN_PAREN,
    CLOSE_PAREN,
    COMMA,
    END,
}

#[allow(non_camel_case_types)]
enum stack_obj_t {
    State(state_t),
    Node(Box<ExprNode>),
}

fn printtoken(_t: &Token) {}
fn printexprnode(_s: &str, _list: &ExprNode) {}
fn printexpr(_s: &str, _list: &MapperExpr) {}
fn printstack(_stack: &stack_obj_t, _stack_size: i32) {}

/// Linearize a linked-list ExprNode chain into a Vec of boxed nodes,
/// also returning whether any variable references appear.
fn linearize(start: Box<ExprNode>) -> (Vec<Box<ExprNode>>, bool) {
    let mut chain: Vec<Box<ExprNode>> = Vec::new();
    let mut refvar = false;
    let mut cur = Some(start);
    while let Some(mut node) = cur {
        if node.tok.token_type == TokenType::Var {
            refvar = true;
        }
        cur = node.next.take();
        chain.push(node);
    }
    (chain, refvar)
}

/// Reassemble a Vec of nodes into a linked list.
fn assemble(chain: Vec<Box<ExprNode>>) -> Option<Box<ExprNode>> {
    let mut head: Option<Box<ExprNode>> = None;
    for mut node in chain.into_iter().rev() {
        node.next = head;
        head = Some(node);
    }
    head
}

fn collapse_expr_to_left(
    lhs: Box<ExprNode>,
    rhs: Box<ExprNode>,
    constant_folding: bool,
) -> Box<ExprNode> {
    let (mut lhs_chain, refvar_l) = linearize(lhs);
    let (rhs_chain, refvar_r) = linearize(rhs);
    let refvar = refvar_l || refvar_r;

    let lhs_last_is_float = lhs_chain.last().unwrap().is_float != 0;
    let rhs_last_is_float = rhs_chain.last().unwrap().is_float != 0;
    let is_float = lhs_last_is_float || rhs_last_is_float;

    let lhs_last = lhs_chain.pop().unwrap();

    let mut result: Vec<Box<ExprNode>> = Vec::new();
    result.extend(lhs_chain);

    if lhs_last_is_float && !rhs_last_is_float {
        // Append rhs, then TOFLOAT, then trailing op
        result.extend(rhs_chain);
        let mut tofloat = exprnode_new(&Token::new(TokenType::ToFloat), 1);
        tofloat.is_float = 1;
        result.push(tofloat);
        result.push(lhs_last);
    } else if !lhs_last_is_float && rhs_last_is_float {
        // Insert TOFLOAT before rhs, mark trailing op as float
        let tofloat = exprnode_new(&Token::new(TokenType::ToFloat), 1);
        result.push(tofloat);
        result.extend(rhs_chain);
        let mut lhs_last_modified = lhs_last;
        lhs_last_modified.is_float = 1;
        result.push(lhs_last_modified);
    } else {
        result.extend(rhs_chain);
        result.push(lhs_last);
    }

    let head = assemble(result).unwrap();

    if constant_folding && !refvar {
        // Evaluate the expression with no input
        let mut e = MapperExpr {
            node: *head,
            vector_size: 1,
            history_size: 1,
            history_pos: 0,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };
        let v = mapper_expr_evaluate_internal(&mut e, None);

        let mut tok;
        let new_is_float;
        if is_float {
            tok = Token::new(TokenType::Float);
            tok.value = Some(v.as_f32().unwrap());
            new_is_float = 1;
        } else {
            tok = Token::new(TokenType::Int);
            tok.int_value = Some(v.as_i32().unwrap());
            new_is_float = 0;
        }
        exprnode_new(&tok, new_is_float)
    } else {
        head
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    _output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    let chars: Vec<char> = s.chars().collect();
    let mut pos: usize = 0;

    let mut stack: Vec<stack_obj_t> = Vec::new();
    let mut tok = Token::new(TokenType::End);
    let mut next_token = true;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    stack.push(stack_obj_t::State(state_t::EXPR));
    stack.push(stack_obj_t::State(state_t::YEQUAL_EQ));
    stack.push(stack_obj_t::State(state_t::YEQUAL_Y));

    let mut result: Option<Box<ExprNode>> = None;
    let mut error_message: Option<&'static str> = None;

    'outer: while !stack.is_empty() {
        if next_token {
            match expr_lex(&chars, &mut pos) {
                Ok(t) => tok = t,
                Err(_) => {
                    error_message = Some("Error in lexical analysis.");
                    break 'outer;
                }
            }
            next_token = false;
        }

        let top = stack.len() - 1;

        // Handle case: top is a Node
        if matches!(stack[top], stack_obj_t::Node(_)) {
            if top == 0 {
                if let stack_obj_t::Node(n) = stack.pop().unwrap() {
                    result = Some(n);
                }
                break 'outer;
            }
            // top-1 must be a State here (otherwise structure is invalid)
            if matches!(stack[top - 1], stack_obj_t::State(_)) {
                if top >= 2 && matches!(stack[top - 2], stack_obj_t::Node(_)) {
                    let state_ref = if let stack_obj_t::State(ref s) = stack[top - 1] { s } else { unreachable!() };
                    let action = match state_ref {
                        state_t::EXPR_RIGHT | state_t::TERM_RIGHT | state_t::CLOSE_PAREN => 1,
                        state_t::CLOSE_HISTINDEX => 2,
                        state_t::CLOSE_VECTINDEX => 3,
                        _ => 0,
                    };
                    match action {
                        1 => {
                            // Pop the top Node, collapse into Node at top-2
                            let rhs = if let stack_obj_t::Node(n) = stack.pop().unwrap() { n } else { unreachable!() };
                            // Remove the lhs node from stack[top-2], collapse, and put back
                            let lhs_idx = stack.len() - 2; // since we popped one
                            // stack[lhs_idx] is the lhs node
                            // Replace temporarily with placeholder
                            let placeholder = stack_obj_t::State(state_t::END);
                            let lhs_obj = std::mem::replace(&mut stack[lhs_idx], placeholder);
                            let lhs_box = if let stack_obj_t::Node(n) = lhs_obj { n } else { unreachable!() };
                            let merged = collapse_expr_to_left(lhs_box, rhs, true);
                            stack[lhs_idx] = stack_obj_t::Node(merged);
                        }
                        2 => {
                            // CLOSE_HISTINDEX: set history_index of var node at top-2
                            let top_node = if let stack_obj_t::Node(n) = stack.pop().unwrap() { n } else { unreachable!() };
                            // top_node should be lonely Int or Float
                            let idx_val: i32 = match top_node.tok.token_type {
                                TokenType::Float => top_node.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => top_node.tok.int_value.unwrap_or(0),
                                _ => 0,
                            };
                            let var_idx = stack.len() - 2;
                            if let stack_obj_t::Node(ref mut var_node) = stack[var_idx] {
                                var_node.history_index = idx_val;
                                if (oldest_samps as i32) > var_node.history_index {
                                    oldest_samps = var_node.history_index as f32;
                                }
                            }
                        }
                        3 => {
                            // CLOSE_VECTINDEX
                            let top_node = if let stack_obj_t::Node(n) = stack.pop().unwrap() { n } else { unreachable!() };
                            let idx_val: i32 = match top_node.tok.token_type {
                                TokenType::Float => top_node.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => top_node.tok.int_value.unwrap_or(0),
                                _ => 0,
                            };
                            let var_idx = stack.len() - 2;
                            if let stack_obj_t::Node(ref mut var_node) = stack[var_idx] {
                                var_node.vector_index = idx_val;
                                if var_node.vector_index > 0 {
                                    error_message = Some("Vector indexing not yet implemented.");
                                    break 'outer;
                                }
                                if var_node.vector_index < 0 || var_node.vector_index >= vector_size {
                                    error_message = Some("Vector index outside input size.");
                                    break 'outer;
                                }
                            }
                        }
                        _ => {
                            // No matching action; do nothing
                        }
                    }
                } else {
                    // Swap top and top-1
                    stack.swap(top - 1, top);
                }
            }
            continue;
        }

        // Top is a State
        let state = if let stack_obj_t::State(ref s) = stack[top] { s } else { unreachable!() };
        // Decide what to do, but be careful to release the borrow before mutating the stack.
        let action_state: u32 = match state {
            state_t::YEQUAL_Y => 1,
            state_t::YEQUAL_EQ => 2,
            state_t::EXPR => 3,
            state_t::EXPR_RIGHT => 4,
            state_t::TERM => 5,
            state_t::TERM_RIGHT => 6,
            state_t::VALUE => 7,
            state_t::NEGATE => 8,
            state_t::VAR_RIGHT => 9,
            state_t::VAR_VECTINDEX => 10,
            state_t::VAR_HISTINDEX => 11,
            state_t::CLOSE_VECTINDEX => 12,
            state_t::CLOSE_HISTINDEX => 13,
            state_t::OPEN_PAREN => 14,
            state_t::CLOSE_PAREN => 15,
            state_t::COMMA => 16,
            state_t::END => 17,
        };

        match action_state {
            1 => {
                // YEQUAL_Y
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    break 'outer;
                }
                next_token = true;
            }
            2 => {
                // YEQUAL_EQ
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    break 'outer;
                }
                next_token = true;
            }
            3 => {
                // EXPR
                stack.pop();
                stack.push(stack_obj_t::State(state_t::EXPR_RIGHT));
                stack.push(stack_obj_t::State(state_t::TERM));
            }
            4 => {
                // EXPR_RIGHT
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        // APPEND_OP
                        append_op_to_top(&mut stack, &tok);
                        stack.push(stack_obj_t::State(state_t::EXPR));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            5 => {
                // TERM
                stack.pop();
                stack.push(stack_obj_t::State(state_t::TERM_RIGHT));
                stack.push(stack_obj_t::State(state_t::VALUE));
            }
            6 => {
                // TERM_RIGHT
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        append_op_to_top(&mut stack, &tok);
                        stack.push(stack_obj_t::State(state_t::TERM));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            7 => {
                // VALUE
                match tok.token_type {
                    TokenType::Int => {
                        stack.pop();
                        stack.push(stack_obj_t::Node(exprnode_new(&tok, 0)));
                        next_token = true;
                    }
                    TokenType::Float => {
                        stack.pop();
                        stack.push(stack_obj_t::Node(exprnode_new(&tok, 1)));
                        next_token = true;
                    }
                    TokenType::Var => {
                        if var_allowed {
                            stack.pop();
                            stack.push(stack_obj_t::Node(exprnode_new(&tok, input_is_float)));
                            stack.push(stack_obj_t::State(state_t::VAR_RIGHT));
                            next_token = true;
                        } else {
                            error_message = Some("Unexpected variable reference.");
                            break 'outer;
                        }
                    }
                    TokenType::OpenParen => {
                        stack.pop();
                        stack.push(stack_obj_t::State(state_t::CLOSE_PAREN));
                        stack.push(stack_obj_t::State(state_t::EXPR));
                        next_token = true;
                    }
                    TokenType::Func => {
                        stack.pop();
                        let func_name = match tok.func {
                            Some(n) => n,
                            None => {
                                error_message = Some("Unknown function.");
                                break 'outer;
                            }
                        };
                        let arity = FUNCTION_TABLE.get(func_name).map(|e| e.arity).unwrap_or(0);
                        stack.push(stack_obj_t::Node(exprnode_new(&tok, 1)));
                        if arity > 0 {
                            stack.push(stack_obj_t::State(state_t::CLOSE_PAREN));
                            stack.push(stack_obj_t::State(state_t::EXPR));
                            for _ in 1..arity {
                                stack.push(stack_obj_t::State(state_t::COMMA));
                                stack.push(stack_obj_t::State(state_t::EXPR));
                            }
                            stack.push(stack_obj_t::State(state_t::OPEN_PAREN));
                        }
                        next_token = true;
                    }
                    TokenType::Op if tok.op == Some('-') => {
                        stack.pop();
                        stack.push(stack_obj_t::State(state_t::NEGATE));
                        stack.push(stack_obj_t::State(state_t::VALUE));
                        next_token = true;
                    }
                    _ => {
                        error_message = Some("Expected value.");
                        break 'outer;
                    }
                }
            }
            8 => {
                // NEGATE
                stack.pop();
                let top_idx = stack.len() - 1;
                if matches!(stack[top_idx], stack_obj_t::Node(_)) {
                    let placeholder = stack_obj_t::State(state_t::END);
                    let node_obj = std::mem::replace(&mut stack[top_idx], placeholder);
                    let inner = if let stack_obj_t::Node(n) = node_obj { n } else { unreachable!() };
                    let mut zero_tok = Token::new(TokenType::Int);
                    zero_tok.int_value = Some(0);
                    let mut zero = exprnode_new(&zero_tok, 0);
                    let mut minus_tok = Token::new(TokenType::Op);
                    minus_tok.op = Some('-');
                    let minus = exprnode_new(&minus_tok, 0);
                    zero.next = Some(minus);
                    let merged = collapse_expr_to_left(zero, inner, true);
                    stack[top_idx] = stack_obj_t::Node(merged);
                } else {
                    error_message = Some("Expected to negate an expression.");
                    break 'outer;
                }
            }
            9 => {
                // VAR_RIGHT
                if tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::VAR_VECTINDEX));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::VAR_HISTINDEX));
                } else {
                    stack.pop();
                }
            }
            10 => {
                // VAR_VECTINDEX
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(stack_obj_t::State(state_t::CLOSE_VECTINDEX));
                    stack.push(stack_obj_t::State(state_t::EXPR));
                    next_token = true;
                }
            }
            11 => {
                // VAR_HISTINDEX
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(stack_obj_t::State(state_t::CLOSE_HISTINDEX));
                    stack.push(stack_obj_t::State(state_t::EXPR));
                    next_token = true;
                }
            }
            12 => {
                // CLOSE_VECTINDEX
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::VAR_HISTINDEX));
                    next_token = true;
                } else {
                    error_message = Some("Expected ']'.");
                    break 'outer;
                }
            }
            13 => {
                // CLOSE_HISTINDEX
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::VAR_VECTINDEX));
                    next_token = true;
                } else {
                    error_message = Some("Expected '}'.");
                    break 'outer;
                }
            }
            14 => {
                // OPEN_PAREN
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected '('.");
                    break 'outer;
                }
            }
            15 => {
                // CLOSE_PAREN
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected ')'.");
                    break 'outer;
                }
            }
            16 => {
                // COMMA
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // find the previous Node on the stack
                    let cur_top = stack.len() - 1;
                    let mut i = cur_top as isize - 1;
                    while i >= 0 && !matches!(stack[i as usize], stack_obj_t::Node(_)) {
                        i -= 1;
                    }
                    if i >= 0 {
                        // Pop top node and merge into stack[i]
                        let rhs = if let stack_obj_t::Node(n) = stack.pop().unwrap() { n } else { unreachable!() };
                        let placeholder = stack_obj_t::State(state_t::END);
                        let lhs_obj = std::mem::replace(&mut stack[i as usize], placeholder);
                        let lhs = if let stack_obj_t::Node(n) = lhs_obj { n } else { unreachable!() };
                        let merged = collapse_expr_to_left(lhs, rhs, false);
                        stack[i as usize] = stack_obj_t::Node(merged);
                    }
                    next_token = true;
                } else {
                    error_message = Some("Expected ','.");
                    break 'outer;
                }
            }
            17 => {
                // END
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    error_message = Some("Expected END.");
                    break 'outer;
                }
            }
            _ => {
                error_message = Some("Unexpected parser state.");
                break 'outer;
            }
        }
    }

    if result.is_none() {
        if let Some(msg) = error_message {
            println!("{}", msg);
        }
        // Return an empty MapperExpr (will basically be a no-op)
        return MapperExpr {
            node: ExprNode::new(),
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };
    }

    if oldest_samps < -100.0 {
        trace!("Expression contains history reference of {}\n", oldest_samps);
        return MapperExpr {
            node: ExprNode::new(),
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };
    }

    // No final output coercion: we use a typed enum (MapperSignalValue) so
    // the result already carries the right concrete variant for as_f32/as_i32.

    let mut node = result.unwrap();

    // Vector indexing check
    if vector_size > 1 {
        let mut cur: Option<&ExprNode> = Some(node.as_ref());
        while let Some(n) = cur {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                trace!("vector indexing not yet implemented\n");
                return MapperExpr {
                    node: ExprNode::new(),
                    vector_size: 1,
                    history_size: 1,
                    history_pos: -1,
                    input_history: vec![MapperSignalValue::I32(0)],
                    output_history: vec![MapperSignalValue::I32(0)],
                };
            }
            cur = n.next.as_deref();
        }
    }

    let history_size = ((-oldest_samps).ceil() as i32) + 1;
    let vs = vector_size.max(1) as usize;
    let hs = history_size.max(1) as usize;

    MapperExpr {
        node: *std::mem::replace(&mut node, Box::new(ExprNode::new())),
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); vs * hs],
        output_history: vec![MapperSignalValue::I32(0); hs],
    }
}

fn append_op_to_top(stack: &mut Vec<stack_obj_t>, tok: &Token) {
    let top = stack.len() - 1;
    if let stack_obj_t::Node(ref mut head) = stack[top] {
        let mut cur: &mut Box<ExprNode> = head;
        while cur.next.is_some() {
            cur = cur.next.as_mut().unwrap();
        }
        let parent_is_float = cur.is_float;
        let mut new_node = exprnode_new(tok, 0);
        new_node.is_float = parent_is_float;
        cur.next = Some(new_node);
    }
}

fn mapper_expr_evaluate_internal(
    expr: &mut MapperExpr,
    input: Option<&MapperSignalValue>,
) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = vec![MapperSignalValue::I32(0); STACK_SIZE];
    let mut top: i32 = -1;

    if let Some(inp) = input {
        expr.history_pos = (expr.history_pos + 1).rem_euclid(expr.history_size);
        let pos = expr.history_pos as usize;
        let vs = expr.vector_size as usize;
        // Treat the input as a single-element "vector" (vector_size assumed >=1).
        for i in 0..vs {
            let idx = pos * vs + i;
            if idx < expr.input_history.len() {
                expr.input_history[idx] = *inp;
            }
        }
    }

    let mut node_opt: Option<&ExprNode> = Some(&expr.node);

    while let Some(n) = node_opt {
        match n.tok.token_type {
            TokenType::Int => {
                top += 1;
                stack[top as usize] = MapperSignalValue::I32(n.tok.int_value.unwrap_or(0));
            }
            TokenType::Float => {
                top += 1;
                stack[top as usize] = MapperSignalValue::F(n.tok.value.unwrap_or(0.0));
            }
            TokenType::Var => {
                let hs = expr.history_size.max(1);
                let idx = ((n.history_index + expr.history_pos + hs).rem_euclid(hs)) as usize;
                let raw = match n.tok.var {
                    Some('x') => {
                        let vs = expr.vector_size.max(1) as usize;
                        let i = idx * vs + n.vector_index as usize;
                        if i < expr.input_history.len() {
                            expr.input_history[i]
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    Some('y') => {
                        if idx < expr.output_history.len() {
                            expr.output_history[idx]
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    _ => return MapperSignalValue::I32(0),
                };
                // Coerce to match the node's is_float type since the input
                // enum variant may not match the parser's expected type.
                let coerced = if n.is_float != 0 {
                    MapperSignalValue::F(raw.as_f32().unwrap_or(0.0))
                } else {
                    MapperSignalValue::I32(raw.as_i32().unwrap_or(0))
                };
                top += 1;
                stack[top as usize] = coerced;
            }
            TokenType::ToFloat => {
                let v = stack[top as usize];
                stack[top as usize] = MapperSignalValue::F(v.as_f32().unwrap_or(0.0));
            }
            TokenType::ToInt32 => {
                let v = stack[top as usize];
                stack[top as usize] = MapperSignalValue::I32(v.as_i32().unwrap_or(0));
            }
            TokenType::Op => {
                let right = stack[top as usize]; top -= 1;
                let left = stack[top as usize]; top -= 1;
                top += 1;
                let op = n.tok.op.unwrap_or('+');
                if n.is_float != 0 {
                    let l = left.as_f32().unwrap_or(0.0);
                    let r = right.as_f32().unwrap_or(0.0);
                    let res = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => l / r,
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack[top as usize] = MapperSignalValue::F(res);
                } else {
                    let l = left.as_i32().unwrap_or(0);
                    let r = right.as_i32().unwrap_or(0);
                    let res = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => if r == 0 { 0 } else { l / r },
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack[top as usize] = MapperSignalValue::I32(res);
                }
            }
            TokenType::Func => {
                let fname = match n.tok.func {
                    Some(s) => s,
                    None => return MapperSignalValue::I32(0),
                };
                let entry = match FUNCTION_TABLE.get(fname) {
                    Some(e) => e,
                    None => return MapperSignalValue::I32(0),
                };
                let arity = entry.arity;
                match arity {
                    0 => {
                        let res = (entry.func)(0.0, 0.0);
                        top += 1;
                        stack[top as usize] = MapperSignalValue::F(res);
                    }
                    1 => {
                        let r = stack[top as usize].as_f32().unwrap_or(0.0);
                        // top stays the same (we pop one and push one)
                        let res = (entry.func)(r, 0.0);
                        stack[top as usize] = MapperSignalValue::F(res);
                    }
                    2 => {
                        let r = stack[top as usize].as_f32().unwrap_or(0.0);
                        top -= 1;
                        let l = stack[top as usize].as_f32().unwrap_or(0.0);
                        let res = (entry.func)(l, r);
                        stack[top as usize] = MapperSignalValue::F(res);
                    }
                    _ => return MapperSignalValue::I32(0),
                }
            }
            _ => return MapperSignalValue::I32(0),
        }
        node_opt = n.next.as_deref();
    }

    let result = if top >= 0 { stack[0] } else { MapperSignalValue::I32(0) };
    if input.is_some() {
        let pos = expr.history_pos as usize;
        if pos < expr.output_history.len() {
            expr.output_history[pos] = result;
        }
    }
    result
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    mapper_expr_evaluate_internal(mapper, Some(input))
}
