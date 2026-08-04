use std::f32::consts::PI;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}

impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MapperSignalValue::F(f) => Some(*f),
            _ => None,
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::I32(i) => Some(*i),
            _ => None,
        }
    }
}

const STACK_SIZE: usize = 256;
const TRACING: bool = false;

#[allow(unused_macros)]
macro_rules! trace {
    ($($arg:tt)*) => {
        if TRACING {
            println!("-- {}", format!($($arg)*));
        }
    };
}

#[allow(unused_macros)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
        m.insert("atan2", FunctionEntry { name: "atan2", arity: 2, func: f32::atan2 });
        m.insert("sinh", FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() });
        m.insert("cosh", FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() });
        m.insert("tanh", FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() });
        m.insert("exp2", FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() });
        m.insert("log2", FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() });
        m.insert("hypot", FunctionEntry { name: "hypot", arity: 2, func: f32::hypot });
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
pub struct Token {
    token_type: TokenType,
    value: Option<f32>,
    int_value: Option<i32>,
    var: Option<char>,
    op: Option<char>,
    func_name: Option<&'static str>,
}

impl Token {
    fn new(token_type: TokenType) -> Self {
        Token {
            token_type,
            value: None,
            int_value: None,
            var: None,
            op: None,
            func_name: None,
        }
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

fn function_lookup_name(s: &str) -> Option<&'static str> {
    FUNCTION_TABLE.get_key_value(s).map(|(k, _)| *k)
}

/// Lex one token from the input. Returns Ok((token, new_pos)) or Err.
fn expr_lex_one(bytes: &[u8], mut pos: usize) -> Result<(Token, usize), String> {
    if pos >= bytes.len() {
        return Ok((Token::new(TokenType::End), pos));
    }

    // skip whitespace
    loop {
        if pos >= bytes.len() {
            return Ok((Token::new(TokenType::End), pos));
        }
        match bytes[pos] {
            b' ' | b'\t' | b'\r' | b'\n' => pos += 1,
            _ => break,
        }
    }

    let mut integer_found = false;
    let mut int_val: i32 = 0;

    let c = bytes[pos];
    if c.is_ascii_digit() {
        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
        let s = std::str::from_utf8(&bytes[start..pos]).unwrap_or("0");
        int_val = s.parse().unwrap_or(0);
        integer_found = true;
        if pos >= bytes.len() || bytes[pos] != b'.' {
            let mut t = Token::new(TokenType::Int);
            t.int_value = Some(int_val);
            return Ok((t, pos));
        }
    }

    if pos >= bytes.len() {
        return Ok((Token::new(TokenType::End), pos));
    }

    let c = bytes[pos];
    match c {
        b'.' => {
            let start = pos;
            pos += 1;
            if pos >= bytes.len() || !bytes[pos].is_ascii_digit() {
                if integer_found {
                    let mut t = Token::new(TokenType::Float);
                    t.value = Some(int_val as f32);
                    return Ok((t, pos));
                }
                return Err(format!("Unexpected '.'"));
            }
            while pos < bytes.len() && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let frac_str = std::str::from_utf8(&bytes[start..pos]).unwrap_or("0");
            let frac: f64 = frac_str.parse().unwrap_or(0.0);
            let mut t = Token::new(TokenType::Float);
            t.value = Some((int_val as f64 + frac) as f32);
            Ok((t, pos))
        }
        b'+' | b'-' | b'/' | b'*' | b'=' => {
            let mut t = Token::new(TokenType::Op);
            t.op = Some(c as char);
            Ok((t, pos + 1))
        }
        b'(' => Ok((Token::new(TokenType::OpenParen), pos + 1)),
        b')' => Ok((Token::new(TokenType::CloseParen), pos + 1)),
        b'x' | b'y' => {
            let mut t = Token::new(TokenType::Var);
            t.var = Some(c as char);
            Ok((t, pos + 1))
        }
        b'[' => Ok((Token::new(TokenType::OpenSquare), pos + 1)),
        b']' => Ok((Token::new(TokenType::CloseSquare), pos + 1)),
        b'{' => Ok((Token::new(TokenType::OpenCurly), pos + 1)),
        b'}' => Ok((Token::new(TokenType::CloseCurly), pos + 1)),
        b',' => Ok((Token::new(TokenType::Comma), pos + 1)),
        _ => {
            if !c.is_ascii_alphabetic() {
                return Err(format!("unknown character '{}' in lexer", c as char));
            }
            let start = pos;
            while pos < bytes.len()
                && (bytes[pos].is_ascii_alphabetic() || bytes[pos].is_ascii_digit())
            {
                pos += 1;
            }
            let s = std::str::from_utf8(&bytes[start..pos]).unwrap_or("");
            let mut t = Token::new(TokenType::Func);
            t.func_name = function_lookup_name(s);
            Ok((t, pos))
        }
    }
}

/// Lex an entire input string into a vec of tokens.
fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let joined: String = s.join("");
    let bytes = joined.as_bytes();
    let mut pos = 0usize;
    let mut out = Vec::new();
    loop {
        match expr_lex_one(bytes, pos) {
            Ok((t, np)) => {
                pos = np;
                let is_end = t.token_type == TokenType::End;
                out.push(t);
                if is_end {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct ExprNode {
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
}

impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: Token::new(TokenType::End),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
        }
    }
    pub fn expr_free(&self) {
        // No-op: Rust ownership cleans up automatically
    }
}

pub struct MapperExpr {
    pub nodes: Vec<ExprNode>,
    pub vector_size: i32,
    pub history_size: i32,
    pub history_pos: i32,
    pub input_history: Vec<MapperSignalValue>,
    pub output_history: Vec<MapperSignalValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    Node(Vec<ExprNode>),
}

fn printtoken(t: &Token) {
    match t.token_type {
        TokenType::Float => print!("{}", t.value.unwrap_or(0.0)),
        TokenType::Int => print!("{}", t.int_value.unwrap_or(0)),
        TokenType::Op => print!("{}", t.op.unwrap_or(' ')),
        TokenType::OpenParen => print!("("),
        TokenType::CloseParen => print!(")"),
        TokenType::Var => print!("VAR({})", t.var.unwrap_or(' ')),
        TokenType::OpenSquare => print!("["),
        TokenType::CloseSquare => print!("]"),
        TokenType::OpenCurly => print!("{{"),
        TokenType::CloseCurly => print!("}}"),
        TokenType::Func => print!("FUNC({})", t.func_name.unwrap_or("?")),
        TokenType::Comma => print!(","),
        TokenType::End => print!("END"),
        TokenType::ToFloat => print!("(float)"),
        TokenType::ToInt32 => print!("(int32)"),
    }
}

fn printexprnode(s: &str, list: &[ExprNode]) {
    print!("{}", s);
    let mut first = true;
    for n in list {
        if !first {
            print!(" ");
        }
        first = false;
        if n.is_float != 0
            && n.tok.token_type != TokenType::Float
            && n.tok.token_type != TokenType::ToFloat
        {
            print!(".");
        }
        printtoken(&n.tok);
        if n.tok.token_type == TokenType::Var {
            if n.history_index < 0 {
                print!("{{{}}}", n.history_index);
            }
            if n.vector_index > -1 {
                print!("[{}]", n.vector_index);
            }
        }
    }
}

fn printexpr(s: &str, list: &MapperExpr) {
    printexprnode(s, &list.nodes);
}

fn printstack(stack: &[stack_obj_t], _stack_size: i32) {
    let state_name = [
        "YEQUAL_Y", "YEQUAL_EQ", "EXPR", "EXPR_RIGHT", "TERM",
        "TERM_RIGHT", "VALUE", "NEGATE", "VAR_RIGHT",
        "VAR_VECTINDEX", "VAR_HISTINDEX", "CLOSE_VECTINDEX",
        "CLOSE_HISTINDEX", "OPEN_PAREN", "CLOSE_PAREN",
        "COMMA", "END",
    ];
    print!("Stack: ");
    for so in stack {
        match so {
            stack_obj_t::Node(n) => {
                print!("[");
                printexprnode("", n);
                print!("] ");
            }
            stack_obj_t::State(s) => {
                let idx = *s as usize;
                if idx < state_name.len() {
                    print!("{} ", state_name[idx]);
                }
            }
        }
    }
    println!();
}

#[derive(Clone, Copy, Default)]
struct EvalValue {
    f: f32,
    i: i32,
}

fn msv_to_eval(v: MapperSignalValue) -> EvalValue {
    match v {
        MapperSignalValue::F(f) => EvalValue { f, i: f as i32 },
        MapperSignalValue::I32(i) => EvalValue { f: i as f32, i },
    }
}

fn eval_internal(
    nodes: &[ExprNode],
    input_history: &[MapperSignalValue],
    output_history: &[MapperSignalValue],
    history_pos: i32,
    history_size: i32,
    vector_size: i32,
) -> MapperSignalValue {
    let mut stack: [EvalValue; STACK_SIZE] = [EvalValue::default(); STACK_SIZE];
    let mut top: i32 = -1;

    let hs = history_size.max(1);
    let vs = vector_size.max(1);

    for node in nodes {
        match node.tok.token_type {
            TokenType::Int => {
                top += 1;
                stack[top as usize] = EvalValue {
                    f: 0.0,
                    i: node.tok.int_value.unwrap_or(0),
                };
            }
            TokenType::Float => {
                top += 1;
                stack[top as usize] = EvalValue {
                    f: node.tok.value.unwrap_or(0.0),
                    i: 0,
                };
            }
            TokenType::Var => {
                let idx = (((node.history_index + history_pos) % hs) + hs) % hs;
                top += 1;
                match node.tok.var {
                    Some('x') => {
                        let real_idx = (idx as usize) * (vs as usize) + node.vector_index as usize;
                        if real_idx < input_history.len() {
                            stack[top as usize] = msv_to_eval(input_history[real_idx]);
                        } else {
                            stack[top as usize] = EvalValue::default();
                        }
                    }
                    Some('y') => {
                        let real_idx = idx as usize;
                        if real_idx < output_history.len() {
                            stack[top as usize] = msv_to_eval(output_history[real_idx]);
                        } else {
                            stack[top as usize] = EvalValue::default();
                        }
                    }
                    _ => {
                        stack[top as usize] = EvalValue::default();
                    }
                }
            }
            TokenType::ToFloat => {
                if top >= 0 {
                    stack[top as usize].f = stack[top as usize].i as f32;
                }
            }
            TokenType::ToInt32 => {
                if top >= 0 {
                    stack[top as usize].i = stack[top as usize].f as i32;
                }
            }
            TokenType::Op => {
                if top < 1 {
                    return MapperSignalValue::I32(0);
                }
                let right = stack[top as usize];
                top -= 1;
                let left = stack[top as usize];
                if node.is_float != 0 {
                    let r = match node.tok.op {
                        Some('+') => left.f + right.f,
                        Some('-') => left.f - right.f,
                        Some('*') => left.f * right.f,
                        Some('/') => left.f / right.f,
                        _ => 0.0,
                    };
                    stack[top as usize] = EvalValue { f: r, i: 0 };
                } else {
                    let r = match node.tok.op {
                        Some('+') => left.i.wrapping_add(right.i),
                        Some('-') => left.i.wrapping_sub(right.i),
                        Some('*') => left.i.wrapping_mul(right.i),
                        Some('/') => {
                            if right.i == 0 {
                                0
                            } else {
                                left.i / right.i
                            }
                        }
                        _ => 0,
                    };
                    stack[top as usize] = EvalValue { f: 0.0, i: r };
                }
            }
            TokenType::Func => {
                if let Some(name) = node.tok.func_name {
                    if let Some(entry) = FUNCTION_TABLE.get(name) {
                        match entry.arity {
                            0 => {
                                top += 1;
                                stack[top as usize] = EvalValue {
                                    f: (entry.func)(0.0, 0.0),
                                    i: 0,
                                };
                            }
                            1 => {
                                if top < 0 {
                                    return MapperSignalValue::I32(0);
                                }
                                let arg = stack[top as usize].f;
                                stack[top as usize] = EvalValue {
                                    f: (entry.func)(arg, 0.0),
                                    i: 0,
                                };
                            }
                            2 => {
                                if top < 1 {
                                    return MapperSignalValue::I32(0);
                                }
                                let r = stack[top as usize];
                                top -= 1;
                                let l = stack[top as usize];
                                stack[top as usize] = EvalValue {
                                    f: (entry.func)(l.f, r.f),
                                    i: 0,
                                };
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if top < 0 {
        return MapperSignalValue::I32(0);
    }
    if let Some(last) = nodes.last() {
        if last.is_float != 0 {
            MapperSignalValue::F(stack[top as usize].f)
        } else {
            MapperSignalValue::I32(stack[top as usize].i)
        }
    } else {
        MapperSignalValue::I32(0)
    }
}

fn collapse_expr_to_left(lhs: &mut Vec<ExprNode>, mut rhs: Vec<ExprNode>, constant_folding: bool) {
    if lhs.is_empty() || rhs.is_empty() {
        return;
    }

    // Determine if any node references a variable
    let refvar = lhs.iter().any(|n| n.tok.token_type == TokenType::Var)
        || rhs.iter().any(|n| n.tok.token_type == TokenType::Var);

    let lhs_last_idx = lhs.len() - 1;
    let rhs_last_idx = rhs.len() - 1;
    let lhs_last_is_float = lhs[lhs_last_idx].is_float != 0;
    let rhs_last_is_float = rhs[rhs_last_idx].is_float != 0;
    let combined_is_float = lhs_last_is_float || rhs_last_is_float;

    // Insert float coercion if types disagree
    if lhs_last_is_float && !rhs_last_is_float {
        // Append TOFLOAT to rhs
        rhs.push(ExprNode {
            tok: Token::new(TokenType::ToFloat),
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
    } else if !lhs_last_is_float && rhs_last_is_float {
        // Insert TOFLOAT before lhs's last node, and mark old last as is_float=1
        let coerce_node = ExprNode {
            tok: Token::new(TokenType::ToFloat),
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        };
        lhs.insert(lhs_last_idx, coerce_node);
        // The old last node has shifted to lhs_last_idx + 1
        lhs[lhs_last_idx + 1].is_float = 1;
    }

    // Insert rhs before the last node of lhs (the trailing op or function)
    let insert_at = lhs.len() - 1;
    for (i, n) in rhs.into_iter().enumerate() {
        lhs.insert(insert_at + i, n);
    }

    // Constant folding: if no variable references, evaluate now
    if constant_folding && !refvar {
        let v = eval_internal(lhs, &[], &[], 0, 0, 1);
        lhs.clear();
        if combined_is_float {
            let mut tok = Token::new(TokenType::Float);
            tok.value = Some(match v {
                MapperSignalValue::F(f) => f,
                MapperSignalValue::I32(i) => i as f32,
            });
            lhs.push(ExprNode {
                tok,
                is_float: 1,
                history_index: 0,
                vector_index: 0,
            });
        } else {
            let mut tok = Token::new(TokenType::Int);
            tok.int_value = Some(match v {
                MapperSignalValue::I32(i) => i,
                MapperSignalValue::F(f) => f as i32,
            });
            lhs.push(ExprNode {
                tok,
                is_float: 0,
                history_index: 0,
                vector_index: 0,
            });
        }
    }
}

fn empty_mapper_expr() -> MapperExpr {
    MapperExpr {
        nodes: Vec::new(),
        vector_size: 1,
        history_size: 1,
        history_pos: -1,
        input_history: Vec::new(),
        output_history: Vec::new(),
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    let bytes = s.as_bytes();
    let mut pos: usize = 0;
    let mut stack: Vec<stack_obj_t> = Vec::with_capacity(STACK_SIZE);
    let mut tok = Token::new(TokenType::End);
    let mut next_token = true;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;
    let mut result: Option<Vec<ExprNode>> = None;

    stack.push(stack_obj_t::State(state_t::EXPR));
    stack.push(stack_obj_t::State(state_t::YEQUAL_EQ));
    stack.push(stack_obj_t::State(state_t::YEQUAL_Y));

    while !stack.is_empty() {
        if next_token {
            match expr_lex_one(bytes, pos) {
                Ok((t, np)) => {
                    tok = t;
                    pos = np;
                }
                Err(_) => {
                    return empty_mapper_expr();
                }
            }
            next_token = false;
        }

        if TRACING {
            printstack(&stack, stack.len() as i32);
        }

        // Handle case where top of stack is a Node
        if matches!(stack.last(), Some(stack_obj_t::Node(_))) {
            if stack.len() == 1 {
                if let Some(stack_obj_t::Node(n)) = stack.pop() {
                    result = Some(n);
                }
                break;
            }
            // Check the state below the top node
            let top = stack.len() - 1;
            let top_minus_1_is_state = matches!(stack[top - 1], stack_obj_t::State(_));
            if top_minus_1_is_state {
                let top_minus_1_state = match &stack[top - 1] {
                    stack_obj_t::State(s) => *s,
                    _ => unreachable!(),
                };
                let has_node_below =
                    top >= 2 && matches!(stack[top - 2], stack_obj_t::Node(_));

                if has_node_below {
                    match top_minus_1_state {
                        state_t::EXPR_RIGHT
                        | state_t::TERM_RIGHT
                        | state_t::CLOSE_PAREN => {
                            // Collapse: take top node and merge into top-2 node
                            let top_node = match stack.pop().unwrap() {
                                stack_obj_t::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // Pop the state
                            stack.pop();
                            // Now top is the lhs node
                            if let Some(stack_obj_t::Node(lhs)) = stack.last_mut() {
                                collapse_expr_to_left(lhs, top_node, true);
                            }
                            // Push state back
                            stack.push(stack_obj_t::State(top_minus_1_state));
                            continue;
                        }
                        state_t::CLOSE_HISTINDEX => {
                            let top_node = match stack.pop().unwrap() {
                                stack_obj_t::Node(n) => n,
                                _ => unreachable!(),
                            };
                            stack.pop(); // remove state
                            if top_node.len() != 1 {
                                return empty_mapper_expr();
                            }
                            let n0 = &top_node[0];
                            let hist_idx = match n0.tok.token_type {
                                TokenType::Float => n0.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => n0.tok.int_value.unwrap_or(0),
                                _ => return empty_mapper_expr(),
                            };
                            if let Some(stack_obj_t::Node(var_node)) = stack.last_mut() {
                                if let Some(last) = var_node.last_mut() {
                                    last.history_index = hist_idx;
                                }
                            }
                            if (hist_idx as f32) < oldest_samps {
                                oldest_samps = hist_idx as f32;
                            }
                            stack.push(stack_obj_t::State(top_minus_1_state));
                            continue;
                        }
                        state_t::CLOSE_VECTINDEX => {
                            let top_node = match stack.pop().unwrap() {
                                stack_obj_t::Node(n) => n,
                                _ => unreachable!(),
                            };
                            stack.pop(); // remove state
                            if top_node.len() != 1 {
                                return empty_mapper_expr();
                            }
                            let n0 = &top_node[0];
                            let vec_idx = match n0.tok.token_type {
                                TokenType::Float => n0.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => n0.tok.int_value.unwrap_or(0),
                                _ => return empty_mapper_expr(),
                            };
                            if vec_idx > 0 {
                                return empty_mapper_expr();
                            }
                            if vec_idx < 0 || vec_idx >= vector_size {
                                return empty_mapper_expr();
                            }
                            if let Some(stack_obj_t::Node(var_node)) = stack.last_mut() {
                                if let Some(last) = var_node.last_mut() {
                                    last.vector_index = vec_idx;
                                }
                            }
                            stack.push(stack_obj_t::State(top_minus_1_state));
                            continue;
                        }
                        _ => {
                            // No action; fall through
                        }
                    }
                } else {
                    // Swap node and state below it
                    let len = stack.len();
                    stack.swap(len - 1, len - 2);
                    continue;
                }
            }
            continue;
        }

        // Top is a state
        let state = match &stack[stack.len() - 1] {
            stack_obj_t::State(s) => *s,
            _ => unreachable!(),
        };

        match state {
            state_t::YEQUAL_Y => {
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    return empty_mapper_expr();
                }
                next_token = true;
            }
            state_t::YEQUAL_EQ => {
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    return empty_mapper_expr();
                }
                next_token = true;
            }
            state_t::EXPR => {
                stack.pop();
                stack.push(stack_obj_t::State(state_t::EXPR_RIGHT));
                stack.push(stack_obj_t::State(state_t::TERM));
            }
            state_t::EXPR_RIGHT => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        // APPEND_OP: append op to top node (which is now the new top after popping state)
                        if let Some(stack_obj_t::Node(n)) = stack.last_mut() {
                            let is_float = n.last().map(|x| x.is_float).unwrap_or(0);
                            let mut t = Token::new(TokenType::Op);
                            t.op = tok.op;
                            n.push(ExprNode {
                                tok: t,
                                is_float,
                                history_index: 0,
                                vector_index: 0,
                            });
                        }
                        stack.push(stack_obj_t::State(state_t::EXPR));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            state_t::TERM => {
                stack.pop();
                stack.push(stack_obj_t::State(state_t::TERM_RIGHT));
                stack.push(stack_obj_t::State(state_t::VALUE));
            }
            state_t::TERM_RIGHT => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        if let Some(stack_obj_t::Node(n)) = stack.last_mut() {
                            let is_float = n.last().map(|x| x.is_float).unwrap_or(0);
                            let mut t = Token::new(TokenType::Op);
                            t.op = tok.op;
                            n.push(ExprNode {
                                tok: t,
                                is_float,
                                history_index: 0,
                                vector_index: 0,
                            });
                        }
                        stack.push(stack_obj_t::State(state_t::TERM));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            state_t::VALUE => match tok.token_type {
                TokenType::Int => {
                    stack.pop();
                    stack.push(stack_obj_t::Node(vec![ExprNode {
                        tok,
                        is_float: 0,
                        history_index: 0,
                        vector_index: 0,
                    }]));
                    next_token = true;
                }
                TokenType::Float => {
                    stack.pop();
                    stack.push(stack_obj_t::Node(vec![ExprNode {
                        tok,
                        is_float: 1,
                        history_index: 0,
                        vector_index: 0,
                    }]));
                    next_token = true;
                }
                TokenType::Var => {
                    if !var_allowed {
                        return empty_mapper_expr();
                    }
                    stack.pop();
                    stack.push(stack_obj_t::Node(vec![ExprNode {
                        tok,
                        is_float: input_is_float,
                        history_index: 0,
                        vector_index: 0,
                    }]));
                    stack.push(stack_obj_t::State(state_t::VAR_RIGHT));
                    next_token = true;
                }
                TokenType::OpenParen => {
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::CLOSE_PAREN));
                    stack.push(stack_obj_t::State(state_t::EXPR));
                    next_token = true;
                }
                TokenType::Func => {
                    stack.pop();
                    let name = match tok.func_name {
                        Some(n) => n,
                        None => return empty_mapper_expr(),
                    };
                    let arity = match FUNCTION_TABLE.get(name) {
                        Some(e) => e.arity,
                        None => return empty_mapper_expr(),
                    };
                    stack.push(stack_obj_t::Node(vec![ExprNode {
                        tok,
                        is_float: 1,
                        history_index: 0,
                        vector_index: 0,
                    }]));
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
                _ => return empty_mapper_expr(),
            },
            state_t::NEGATE => {
                stack.pop();
                if let Some(stack_obj_t::Node(node)) = stack.pop() {
                    let mut new_node: Vec<ExprNode> = Vec::new();
                    let mut t0 = Token::new(TokenType::Int);
                    t0.int_value = Some(0);
                    new_node.push(ExprNode {
                        tok: t0,
                        is_float: 0,
                        history_index: 0,
                        vector_index: 0,
                    });
                    let mut tm = Token::new(TokenType::Op);
                    tm.op = Some('-');
                    new_node.push(ExprNode {
                        tok: tm,
                        is_float: 0,
                        history_index: 0,
                        vector_index: 0,
                    });
                    collapse_expr_to_left(&mut new_node, node, true);
                    stack.push(stack_obj_t::Node(new_node));
                } else {
                    return empty_mapper_expr();
                }
            }
            state_t::VAR_RIGHT => {
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
            state_t::VAR_VECTINDEX => {
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(stack_obj_t::State(state_t::CLOSE_VECTINDEX));
                    stack.push(stack_obj_t::State(state_t::EXPR));
                    next_token = true;
                }
            }
            state_t::VAR_HISTINDEX => {
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(stack_obj_t::State(state_t::CLOSE_HISTINDEX));
                    stack.push(stack_obj_t::State(state_t::EXPR));
                    next_token = true;
                }
            }
            state_t::CLOSE_VECTINDEX => {
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::VAR_HISTINDEX));
                    next_token = true;
                } else {
                    return empty_mapper_expr();
                }
            }
            state_t::CLOSE_HISTINDEX => {
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(stack_obj_t::State(state_t::VAR_VECTINDEX));
                    next_token = true;
                } else {
                    return empty_mapper_expr();
                }
            }
            state_t::CLOSE_PAREN => {
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    return empty_mapper_expr();
                }
            }
            state_t::COMMA => {
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    let top_node = match stack.pop() {
                        Some(stack_obj_t::Node(n)) => n,
                        _ => return empty_mapper_expr(),
                    };
                    // Find previous expression on the stack
                    let mut found_idx: Option<usize> = None;
                    for i in (0..stack.len()).rev() {
                        if matches!(stack[i], stack_obj_t::Node(_)) {
                            found_idx = Some(i);
                            break;
                        }
                    }
                    if let Some(idx) = found_idx {
                        if let stack_obj_t::Node(prev) = &mut stack[idx] {
                            collapse_expr_to_left(prev, top_node, false);
                        }
                    }
                    next_token = true;
                } else {
                    return empty_mapper_expr();
                }
            }
            state_t::OPEN_PAREN => {
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    return empty_mapper_expr();
                }
            }
            state_t::END => {
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    return empty_mapper_expr();
                }
            }
        }
    }

    let mut nodes = match result {
        Some(n) => n,
        None => return empty_mapper_expr(),
    };

    // Bound check the history
    if oldest_samps < -100.0 {
        return empty_mapper_expr();
    }

    // Coerce final output if necessary
    let last_is_float = nodes.last().map(|n| n.is_float != 0).unwrap_or(false);
    if last_is_float && output_is_float == 0 {
        nodes.push(ExprNode {
            tok: Token::new(TokenType::ToInt32),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
        });
    } else if !last_is_float && output_is_float != 0 {
        nodes.push(ExprNode {
            tok: Token::new(TokenType::ToFloat),
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
    }

    // Special check: vector indexing not yet supported when vector_size > 1
    if vector_size > 1 {
        for n in &nodes {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return empty_mapper_expr();
            }
        }
    }

    let history_size = (-oldest_samps).ceil() as i32 + 1;
    let history_size_usize = history_size.max(1) as usize;
    let vec_size_usize = vector_size.max(1) as usize;

    MapperExpr {
        nodes,
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); history_size_usize * vec_size_usize],
        output_history: vec![MapperSignalValue::I32(0); history_size_usize],
    }
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    if mapper.history_size > 0 && !mapper.input_history.is_empty() {
        mapper.history_pos = (mapper.history_pos + 1).rem_euclid(mapper.history_size);
        let idx = (mapper.history_pos as usize) * (mapper.vector_size.max(1) as usize);
        if idx < mapper.input_history.len() {
            mapper.input_history[idx] = *input;
        }
    }

    let result = eval_internal(
        &mapper.nodes,
        &mapper.input_history,
        &mapper.output_history,
        mapper.history_pos,
        mapper.history_size,
        mapper.vector_size,
    );

    if mapper.history_pos >= 0
        && (mapper.history_pos as usize) < mapper.output_history.len()
    {
        mapper.output_history[mapper.history_pos as usize] = result;
    }

    // Suppress unused warnings for debug helpers
    let _ = (printtoken, printexprnode, printexpr, printstack, function_lookup, expr_lex);

    result
}
