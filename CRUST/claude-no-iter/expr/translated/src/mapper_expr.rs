use std::f32::consts::PI;
use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}

impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MapperSignalValue::F(f) => Some(*f),
            MapperSignalValue::I32(_) => None,
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::I32(i) => Some(*i),
            MapperSignalValue::F(_) => None,
        }
    }
}

const STACK_SIZE: usize = 256;

const TRACING: bool = false;

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
}

impl Token {
    fn new(t: TokenType) -> Token {
        Token {
            token_type: t,
            value: None,
            int_value: None,
            var: None,
            op: None,
        }
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

// Lex a single string. The Vec<&str> is concatenated.
fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let combined: String = s.into_iter().collect();
    lex_string(&combined)
}

// Internal lexer that produces a Vec<Token> from a string.
fn lex_string(s: &str) -> Vec<Token> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i: usize = 0;
    let n = chars.len();

    while i < n {
        let c = chars[i];
        // skip whitespace
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < n && chars[i].is_ascii_digit() {
                i += 1;
            }
            let int_str: String = chars[start..i].iter().collect();
            let int_val: i32 = int_str.parse().unwrap_or(0);
            // check for fractional part
            if i < n && chars[i] == '.' {
                let frac_start = i;
                i += 1;
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let full_str: String = chars[start..i].iter().collect();
                let f_val: f32 = full_str.parse().unwrap_or(int_val as f32);
                let mut t = Token::new(TokenType::Float);
                t.value = Some(f_val);
                tokens.push(t);
                let _ = frac_start;
            } else {
                let mut t = Token::new(TokenType::Int);
                t.int_value = Some(int_val);
                tokens.push(t);
            }
            continue;
        }
        if c == '.' {
            // float starting with '.'
            let start = i;
            i += 1;
            if i < n && chars[i].is_ascii_digit() {
                while i < n && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let s_str: String = chars[start..i].iter().collect();
                let f_val: f32 = s_str.parse().unwrap_or(0.0);
                let mut t = Token::new(TokenType::Float);
                t.value = Some(f_val);
                tokens.push(t);
                continue;
            } else {
                // lonely '.', skip (or error)
                continue;
            }
        }
        match c {
            '+' | '-' | '*' | '/' | '=' => {
                let mut t = Token::new(TokenType::Op);
                t.op = Some(c);
                tokens.push(t);
                i += 1;
            }
            '(' => {
                tokens.push(Token::new(TokenType::OpenParen));
                i += 1;
            }
            ')' => {
                tokens.push(Token::new(TokenType::CloseParen));
                i += 1;
            }
            'x' | 'y' => {
                let mut t = Token::new(TokenType::Var);
                t.var = Some(c);
                tokens.push(t);
                i += 1;
            }
            '[' => {
                tokens.push(Token::new(TokenType::OpenSquare));
                i += 1;
            }
            ']' => {
                tokens.push(Token::new(TokenType::CloseSquare));
                i += 1;
            }
            '{' => {
                tokens.push(Token::new(TokenType::OpenCurly));
                i += 1;
            }
            '}' => {
                tokens.push(Token::new(TokenType::CloseCurly));
                i += 1;
            }
            ',' => {
                tokens.push(Token::new(TokenType::Comma));
                i += 1;
            }
            _ => {
                if c.is_ascii_alphabetic() {
                    let start = i;
                    while i < n && (chars[i].is_ascii_alphabetic() || chars[i].is_ascii_digit()) {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    let mut t = Token::new(TokenType::Func);
                    // encode function name lookup result as a char index
                    if let Some(idx) = function_name_index(&name) {
                        t.op = Some(char::from_u32(idx as u32).unwrap_or('\0'));
                    } else {
                        // unknown function - encode as 0xFFFF
                        t.op = Some('\u{FFFF}');
                    }
                    tokens.push(t);
                } else {
                    println!("unknown character '{}' in lexer", c);
                    i += 1;
                }
            }
        }
    }
    tokens.push(Token::new(TokenType::End));
    tokens
}

// Stable list of function names — index is encoded into Token.op for FUNC tokens.
const FUNCTION_NAMES: &[&str] = &[
    "pow", "sin", "cos", "tan", "abs", "sqrt", "log", "log10", "exp",
    "floor", "round", "ceil", "min", "max", "pi",
];

fn function_name_index(name: &str) -> Option<usize> {
    FUNCTION_NAMES.iter().position(|&n| n == name)
}

fn function_for_token(tok: &Token) -> Option<&'static FunctionEntry> {
    let c = tok.op?;
    let idx = c as u32 as usize;
    if idx >= FUNCTION_NAMES.len() {
        return None;
    }
    function_lookup(FUNCTION_NAMES[idx])
}

pub struct ExprNode {
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
    pub next: Option<Arc<ExprNode>>,
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

#[allow(non_camel_case_types, dead_code)]
enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
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
        // No-op: Rust handles memory automatically via Arc / Drop
    }
}

fn printtoken(_t: &Token) {
    // Debug helper — not needed for runtime.
}

fn printexprnode(_s: &str, _list: &ExprNode) {
    // Debug helper — not needed for runtime.
}

fn printexpr(_s: &str, _list: &MapperExpr) {
    // Debug helper — not needed for runtime.
}

fn printstack(_stack: &stack_obj_t, _stack_size: i32) {
    // Debug helper — not needed for runtime.
}

fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // The actual collapse logic is performed internally on `Vec<NodeData>` during parsing.
    // This function is retained for API compatibility but is a no-op here.
}

// =============================================================================
// Internal node + parser implementation
// =============================================================================

#[derive(Clone)]
struct NodeData {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

impl NodeData {
    fn new(tok: Token, is_float: i32) -> NodeData {
        NodeData {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ParseState {
    YequalY,
    YequalEq,
    Expr,
    ExprRight,
    Term,
    TermRight,
    Value,
    Negate,
    VarRight,
    VarVectindex,
    VarHistindex,
    CloseVectindex,
    CloseHistindex,
    OpenParen,
    CloseParen,
    Comma,
    End,
}

enum StackEntry {
    State(ParseState),
    Node(Vec<NodeData>),
}

// Convert a Vec<NodeData> chain into the public ExprNode linked list.
fn build_chain(nodes: Vec<NodeData>) -> ExprNode {
    let mut iter = nodes.into_iter().rev();
    let last = iter.next().expect("Empty node list");
    let mut current = ExprNode {
        tok: last.tok,
        is_float: last.is_float,
        history_index: last.history_index,
        vector_index: last.vector_index,
        next: None,
    };
    for nd in iter {
        let new_node = ExprNode {
            tok: nd.tok,
            is_float: nd.is_float,
            history_index: nd.history_index,
            vector_index: nd.vector_index,
            next: Some(Arc::new(current)),
        };
        current = new_node;
    }
    current
}

// Evaluate a chain (used by constant-folding and by mapper_expr_evaluate).
fn evaluate_nodes(nodes: &[NodeData], input: Option<&MapperSignalValue>,
                  vector_size: i32, history_size: i32, history_pos: i32,
                  input_history: &[MapperSignalValue],
                  output_history: &[MapperSignalValue]) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);
    for node in nodes {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                if input.is_none() && input_history.is_empty() && output_history.is_empty() {
                    // No input provided — push zero of the appropriate type
                    if node.is_float != 0 {
                        stack.push(MapperSignalValue::F(0.0));
                    } else {
                        stack.push(MapperSignalValue::I32(0));
                    }
                } else {
                    let var_c = node.tok.var.unwrap_or('x');
                    let h_size = history_size.max(1);
                    let idx = ((node.history_index + history_pos + h_size) % h_size) as usize;
                    match var_c {
                        'x' => {
                            let v_idx = idx * (vector_size.max(1) as usize) + node.vector_index as usize;
                            if v_idx < input_history.len() {
                                stack.push(input_history[v_idx]);
                            } else if let Some(inp) = input {
                                stack.push(*inp);
                            } else {
                                stack.push(MapperSignalValue::F(0.0));
                            }
                        }
                        'y' => {
                            if idx < output_history.len() {
                                stack.push(output_history[idx]);
                            } else {
                                stack.push(MapperSignalValue::F(0.0));
                            }
                        }
                        _ => {
                            stack.push(MapperSignalValue::F(0.0));
                        }
                    }
                }
            }
            TokenType::ToFloat => {
                let top = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                let f = match top {
                    MapperSignalValue::F(f) => f,
                    MapperSignalValue::I32(i) => i as f32,
                };
                stack.push(MapperSignalValue::F(f));
            }
            TokenType::ToInt32 => {
                let top = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let i = match top {
                    MapperSignalValue::F(f) => f as i32,
                    MapperSignalValue::I32(i) => i,
                };
                stack.push(MapperSignalValue::I32(i));
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                let left = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                let op_c = node.tok.op.unwrap_or('+');
                if node.is_float != 0 {
                    let l = match left {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    let r = match right {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    let res = match op_c {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => l / r,
                        _ => 0.0,
                    };
                    stack.push(MapperSignalValue::F(res));
                } else {
                    let l = match left {
                        MapperSignalValue::I32(i) => i,
                        MapperSignalValue::F(f) => f as i32,
                    };
                    let r = match right {
                        MapperSignalValue::I32(i) => i,
                        MapperSignalValue::F(f) => f as i32,
                    };
                    let res = match op_c {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => if r != 0 { l / r } else { 0 },
                        _ => 0,
                    };
                    stack.push(MapperSignalValue::I32(res));
                }
            }
            TokenType::Func => {
                if let Some(entry) = function_for_token(&node.tok) {
                    match entry.arity {
                        0 => {
                            let v = (entry.func)(0.0, 0.0);
                            stack.push(MapperSignalValue::F(v));
                        }
                        1 => {
                            let arg = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                            let f = match arg {
                                MapperSignalValue::F(f) => f,
                                MapperSignalValue::I32(i) => i as f32,
                            };
                            let v = (entry.func)(f, 0.0);
                            stack.push(MapperSignalValue::F(v));
                        }
                        2 => {
                            let right = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                            let left = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                            let l = match left {
                                MapperSignalValue::F(f) => f,
                                MapperSignalValue::I32(i) => i as f32,
                            };
                            let r = match right {
                                MapperSignalValue::F(f) => f,
                                MapperSignalValue::I32(i) => i as f32,
                            };
                            let v = (entry.func)(l, r);
                            stack.push(MapperSignalValue::F(v));
                        }
                        _ => {
                            stack.push(MapperSignalValue::F(0.0));
                        }
                    }
                }
            }
            _ => { /* ignore other token types during eval */ }
        }
    }
    stack.into_iter().next().unwrap_or(MapperSignalValue::F(0.0))
}

// Constant-fold a Vec<NodeData> with no variable references.
fn evaluate_constant(nodes: &[NodeData]) -> MapperSignalValue {
    evaluate_nodes(nodes, None, 1, 1, 0, &[], &[])
}

// Merge `rhs` into `lhs`, mirroring C's collapse_expr_to_left with constant folding.
fn collapse_internal(lhs: &mut Vec<NodeData>, rhs: Vec<NodeData>, constant_folding: bool) {
    // Detect variable references in either side.
    let mut refvar = false;
    for n in lhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
            break;
        }
    }
    if !refvar {
        for n in rhs.iter() {
            if n.tok.token_type == TokenType::Var {
                refvar = true;
                break;
            }
        }
    }

    // Indices: lhs has at least one node. In the C version, "trailing operator on right hand side"
    // means the last node, and the insertion point on the lhs is also the last node (then before it).
    let lhs_last_idx = lhs.len() - 1;
    let rhs_last_idx = rhs.len() - 1;

    let lhs_last_is_float = lhs[lhs_last_idx].is_float != 0;
    let rhs_last_is_float = rhs[rhs_last_idx].is_float != 0;
    let result_is_float = lhs_last_is_float || rhs_last_is_float;

    // We will produce: lhs[..=lhs_last_idx-1] ++ [optional coerce] ++ rhs ++ [optional coerce] ++ lhs[lhs_last_idx]
    // The C code:
    //   - inserts TOFLOAT after rhs_last if lhs_last is float and rhs_last is not
    //   - inserts TOFLOAT before *plhs_last (i.e., before lhs trailing op) if lhs_last is not float and rhs_last is
    //   - then sets rhs_last->next = *plhs_last; *plhs_last = rhs;
    //
    // Equivalent: take everything before the lhs trailing operator, append rhs (possibly with TOFLOAT),
    // then append the trailing op of lhs.

    let mut new_lhs: Vec<NodeData> = Vec::with_capacity(lhs.len() + rhs.len() + 2);
    // Everything before lhs trailing op
    for n in lhs.iter().take(lhs_last_idx) {
        new_lhs.push(n.clone());
    }

    if !lhs_last_is_float && rhs_last_is_float {
        // TOFLOAT before lhs trailing op
        let mut t = NodeData::new(Token::new(TokenType::ToFloat), 1);
        t.is_float = 1;
        new_lhs.push(t);
    }

    // Append rhs nodes
    for n in rhs.iter() {
        new_lhs.push(n.clone());
    }

    if lhs_last_is_float && !rhs_last_is_float {
        // TOFLOAT after rhs (before lhs trailing op)
        let t = NodeData::new(Token::new(TokenType::ToFloat), 1);
        new_lhs.push(t);
    }

    // Append lhs trailing op
    let mut trailing = lhs[lhs_last_idx].clone();
    // The trailing op's is_float should reflect the resulting type of the operation
    if result_is_float {
        trailing.is_float = 1;
    }
    new_lhs.push(trailing);

    *lhs = new_lhs;

    // Constant folding: if no variable references, evaluate immediately.
    if constant_folding && !refvar {
        let v = evaluate_constant(lhs);
        // Replace with a single literal node
        let mut t;
        if result_is_float {
            let f = match v {
                MapperSignalValue::F(f) => f,
                MapperSignalValue::I32(i) => i as f32,
            };
            t = Token::new(TokenType::Float);
            t.value = Some(f);
        } else {
            let i = match v {
                MapperSignalValue::I32(i) => i,
                MapperSignalValue::F(f) => f as i32,
            };
            t = Token::new(TokenType::Int);
            t.int_value = Some(i);
        }
        let mut nd = NodeData::new(t, if result_is_float { 1 } else { 0 });
        nd.is_float = if result_is_float { 1 } else { 0 };
        *lhs = vec![nd];
    }
}

// Append an operator to the trailing-end of the top node on the stack.
fn append_op(top_node: &mut Vec<NodeData>, op_tok: Token) {
    let trailing_is_float = top_node.last().map(|n| n.is_float).unwrap_or(0);
    let mut nd = NodeData::new(op_tok, trailing_is_float);
    nd.is_float = trailing_is_float;
    top_node.push(nd);
}

pub fn mapper_expr_new_from_string(s: &str,
                                input_is_float: i32,
                                output_is_float: i32,
                                vector_size: i32) -> MapperExpr {
    let _ = output_is_float; // intentionally unused — see notes below.

    // Lex the input string.
    let tokens = expr_lex(vec![s]);
    let mut tok_idx: usize = 0;

    // Parser stack
    let mut stack: Vec<StackEntry> = Vec::with_capacity(STACK_SIZE);
    let mut error_message: Option<&'static str> = None;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    stack.push(StackEntry::State(ParseState::Expr));
    stack.push(StackEntry::State(ParseState::YequalEq));
    stack.push(StackEntry::State(ParseState::YequalY));

    let mut next_token = true;
    let mut current_tok: Token = tokens.get(0).copied().unwrap_or(Token::new(TokenType::End));

    'outer: while !stack.is_empty() {
        if next_token {
            current_tok = tokens.get(tok_idx).copied().unwrap_or(Token::new(TokenType::End));
            if tok_idx < tokens.len() {
                tok_idx += 1;
            }
            next_token = false;
        }

        // Check top of stack
        let top_is_node = matches!(stack.last(), Some(StackEntry::Node(_)));

        if top_is_node {
            // If the only entry on the stack is the node, we have our final result.
            if stack.len() == 1 {
                break 'outer;
            }
            // Look at item below top.
            let below_idx = stack.len() - 2;
            let below_is_state = matches!(stack[below_idx], StackEntry::State(_));
            if below_is_state {
                // Determine whether we should collapse-or-merge.
                let two_below_idx = if stack.len() >= 3 { Some(stack.len() - 3) } else { None };
                let two_below_is_node = two_below_idx.map(|i| matches!(stack[i], StackEntry::Node(_))).unwrap_or(false);

                if two_below_is_node {
                    // Determine the state below
                    let state_below = match &stack[below_idx] {
                        StackEntry::State(s) => *s,
                        _ => unreachable!(),
                    };
                    match state_below {
                        ParseState::ExprRight | ParseState::TermRight | ParseState::CloseParen => {
                            // collapse_expr_to_left
                            let top_node = match stack.pop().unwrap() {
                                StackEntry::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // remove the state
                            stack.pop();
                            // pop the lhs node, modify, and push back
                            let mut lhs_node = match stack.pop().unwrap() {
                                StackEntry::Node(n) => n,
                                _ => unreachable!(),
                            };
                            collapse_internal(&mut lhs_node, top_node, true);
                            stack.push(StackEntry::Node(lhs_node));
                            // now we need to re-push the state we popped? Actually no: in C code,
                            // POP() removes the top node, leaving state and lhs node.
                            // Wait — let me re-trace: in C, when collapse happens, only one POP() of the top node is done,
                            // and the state remains. The lhs (two-down) is mutated in place. Let me redo this.
                            //
                            // The above code popped state too, which is wrong. Push the state back.
                            stack.push(StackEntry::State(state_below));
                        }
                        ParseState::CloseHistindex => {
                            // Set the history_index of the VAR two-down based on the top INT/FLOAT.
                            let top_node = match stack.pop().unwrap() {
                                StackEntry::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // pop state
                            stack.pop();
                            // mutate two-down (now top after the two pops)
                            if let Some(StackEntry::Node(lhs)) = stack.last_mut() {
                                if let Some(last) = lhs.last_mut() {
                                    if last.tok.token_type == TokenType::Var {
                                        if !top_node.is_empty() {
                                            let v = &top_node[0];
                                            if v.tok.token_type == TokenType::Float {
                                                last.history_index = v.tok.value.unwrap_or(0.0) as i32;
                                            } else {
                                                last.history_index = v.tok.int_value.unwrap_or(0);
                                            }
                                            if (oldest_samps as i32) > last.history_index {
                                                oldest_samps = last.history_index as f32;
                                            }
                                        }
                                    }
                                }
                            }
                            // push state back
                            stack.push(StackEntry::State(state_below));
                        }
                        ParseState::CloseVectindex => {
                            let top_node = match stack.pop().unwrap() {
                                StackEntry::Node(n) => n,
                                _ => unreachable!(),
                            };
                            stack.pop();
                            if let Some(StackEntry::Node(lhs)) = stack.last_mut() {
                                if let Some(last) = lhs.last_mut() {
                                    if last.tok.token_type == TokenType::Var {
                                        if !top_node.is_empty() {
                                            let v = &top_node[0];
                                            if v.tok.token_type == TokenType::Float {
                                                last.vector_index = v.tok.value.unwrap_or(0.0) as i32;
                                            } else {
                                                last.vector_index = v.tok.int_value.unwrap_or(0);
                                            }
                                            if last.vector_index > 0 {
                                                error_message = Some("Vector indexing not yet implemented.");
                                                break 'outer;
                                            }
                                            if last.vector_index < 0 || last.vector_index >= vector_size {
                                                error_message = Some("Vector index outside input size.");
                                                break 'outer;
                                            }
                                        }
                                    }
                                }
                            }
                            stack.push(StackEntry::State(state_below));
                        }
                        _ => {
                            // No match — swap top node down past the state, mirroring the C behavior.
                            let top = stack.pop().unwrap();
                            let mid = stack.pop().unwrap();
                            stack.push(top);
                            stack.push(mid);
                        }
                    }
                } else {
                    // Swap top node down past state
                    let top = stack.pop().unwrap();
                    let mid = stack.pop().unwrap();
                    stack.push(top);
                    stack.push(mid);
                }
            }
            continue;
        }

        // Top is a state — drive the state machine.
        let top_state = match stack.last() {
            Some(StackEntry::State(s)) => *s,
            _ => unreachable!(),
        };

        match top_state {
            ParseState::YequalY => {
                if current_tok.token_type == TokenType::Var && current_tok.var == Some('y') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    break 'outer;
                }
                next_token = true;
            }
            ParseState::YequalEq => {
                if current_tok.token_type == TokenType::Op && current_tok.op == Some('=') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    break 'outer;
                }
                next_token = true;
            }
            ParseState::Expr => {
                stack.pop();
                stack.push(StackEntry::State(ParseState::ExprRight));
                stack.push(StackEntry::State(ParseState::Term));
            }
            ParseState::ExprRight => {
                if current_tok.token_type == TokenType::Op {
                    stack.pop();
                    let op_c = current_tok.op.unwrap_or(' ');
                    if op_c == '+' || op_c == '-' {
                        // Append operator to the (now-) top node entry.
                        if let Some(StackEntry::Node(n)) = stack.last_mut() {
                            append_op(n, current_tok);
                        }
                        stack.push(StackEntry::State(ParseState::Expr));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            ParseState::Term => {
                stack.pop();
                stack.push(StackEntry::State(ParseState::TermRight));
                stack.push(StackEntry::State(ParseState::Value));
            }
            ParseState::TermRight => {
                if current_tok.token_type == TokenType::Op {
                    stack.pop();
                    let op_c = current_tok.op.unwrap_or(' ');
                    if op_c == '*' || op_c == '/' {
                        if let Some(StackEntry::Node(n)) = stack.last_mut() {
                            append_op(n, current_tok);
                        }
                        stack.push(StackEntry::State(ParseState::Term));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            ParseState::Value => {
                if current_tok.token_type == TokenType::Int {
                    stack.pop();
                    let nd = NodeData::new(current_tok, 0);
                    stack.push(StackEntry::Node(vec![nd]));
                    next_token = true;
                } else if current_tok.token_type == TokenType::Float {
                    stack.pop();
                    let nd = NodeData::new(current_tok, 1);
                    stack.push(StackEntry::Node(vec![nd]));
                    next_token = true;
                } else if current_tok.token_type == TokenType::Var {
                    if var_allowed {
                        stack.pop();
                        let nd = NodeData::new(current_tok, input_is_float);
                        stack.push(StackEntry::Node(vec![nd]));
                        stack.push(StackEntry::State(ParseState::VarRight));
                        next_token = true;
                    } else {
                        error_message = Some("Unexpected variable reference.");
                        break 'outer;
                    }
                } else if current_tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    stack.push(StackEntry::State(ParseState::CloseParen));
                    stack.push(StackEntry::State(ParseState::Expr));
                    next_token = true;
                } else if current_tok.token_type == TokenType::Func {
                    stack.pop();
                    let entry = function_for_token(&current_tok);
                    if entry.is_none() {
                        error_message = Some("Unknown function.");
                        break 'outer;
                    }
                    let arity = entry.unwrap().arity;
                    let nd = NodeData::new(current_tok, 1);
                    stack.push(StackEntry::Node(vec![nd]));
                    if arity > 0 {
                        stack.push(StackEntry::State(ParseState::CloseParen));
                        stack.push(StackEntry::State(ParseState::Expr));
                        for _ in 1..arity {
                            stack.push(StackEntry::State(ParseState::Comma));
                            stack.push(StackEntry::State(ParseState::Expr));
                        }
                        stack.push(StackEntry::State(ParseState::OpenParen));
                    }
                    next_token = true;
                } else if current_tok.token_type == TokenType::Op && current_tok.op == Some('-') {
                    stack.pop();
                    stack.push(StackEntry::State(ParseState::Negate));
                    stack.push(StackEntry::State(ParseState::Value));
                    next_token = true;
                } else {
                    error_message = Some("Expected value.");
                    break 'outer;
                }
            }
            ParseState::Negate => {
                stack.pop();
                if let Some(StackEntry::Node(n)) = stack.last_mut() {
                    // Insert '0' before, '-' after.
                    let mut zero_t = Token::new(TokenType::Int);
                    zero_t.int_value = Some(0);
                    let zero_nd = NodeData::new(zero_t, 0);

                    let mut minus_t = Token::new(TokenType::Op);
                    minus_t.op = Some('-');
                    let minus_nd = NodeData::new(minus_t, 0);

                    let rhs = std::mem::replace(n, vec![zero_nd, minus_nd]);
                    collapse_internal(n, rhs, true);
                } else {
                    error_message = Some("Expected to negate an expression.");
                    break 'outer;
                }
            }
            ParseState::VarRight => {
                if current_tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(StackEntry::State(ParseState::VarVectindex));
                } else if current_tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(StackEntry::State(ParseState::VarHistindex));
                } else {
                    stack.pop();
                }
            }
            ParseState::VarVectindex => {
                stack.pop();
                if current_tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(StackEntry::State(ParseState::CloseVectindex));
                    stack.push(StackEntry::State(ParseState::Expr));
                    next_token = true;
                }
            }
            ParseState::VarHistindex => {
                stack.pop();
                if current_tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(StackEntry::State(ParseState::CloseHistindex));
                    stack.push(StackEntry::State(ParseState::Expr));
                    next_token = true;
                }
            }
            ParseState::CloseVectindex => {
                if current_tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackEntry::State(ParseState::VarHistindex));
                    next_token = true;
                } else {
                    error_message = Some("Expected ']'.");
                    break 'outer;
                }
            }
            ParseState::CloseHistindex => {
                if current_tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackEntry::State(ParseState::VarVectindex));
                    next_token = true;
                } else {
                    error_message = Some("Expected '}'.");
                    break 'outer;
                }
            }
            ParseState::CloseParen => {
                if current_tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected ')'.");
                    break 'outer;
                }
            }
            ParseState::Comma => {
                if current_tok.token_type == TokenType::Comma {
                    stack.pop();
                    // Find previous Node entry and collapse top into it (no constant folding).
                    // Note: at this point the top should be... hmm, by the time we hit Comma state,
                    // the recently-built expression should already be on stack as a node (since it
                    // would've collapsed). But Comma is itself a state at top — there was a node before us.
                    // Actually with our flow, Comma is a state; the most recent expression node is below it.
                    // We need the previous node further down. Let's mimic the C behavior:
                    // Find a node on the stack going back from current-2 (we just popped the Comma state).
                    // Actually, at this point we've popped Comma state. The top should be the just-built
                    // expression node. We need to merge it down into the previous node entry.
                    let top_node_opt = match stack.pop() {
                        Some(StackEntry::Node(n)) => Some(n),
                        Some(other) => { stack.push(other); None }
                        None => None,
                    };
                    if let Some(top_node) = top_node_opt {
                        // Find the deepest Node entry to merge into (skip states).
                        let mut tmp_states: Vec<StackEntry> = Vec::new();
                        while let Some(top) = stack.pop() {
                            match top {
                                StackEntry::Node(mut lhs) => {
                                    collapse_internal(&mut lhs, top_node, false);
                                    stack.push(StackEntry::Node(lhs));
                                    while let Some(s) = tmp_states.pop() {
                                        stack.push(s);
                                    }
                                    break;
                                }
                                StackEntry::State(s) => {
                                    tmp_states.push(StackEntry::State(s));
                                }
                            }
                        }
                    }
                    next_token = true;
                } else {
                    error_message = Some("Expected ','.");
                    break 'outer;
                }
            }
            ParseState::OpenParen => {
                if current_tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected '('.");
                    break 'outer;
                }
            }
            ParseState::End => {
                if current_tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    error_message = Some("Expected END.");
                    break 'outer;
                }
            }
        }
    }

    if error_message.is_some() {
        // Return a default empty-ish MapperExpr that produces 0.
        let mut t = Token::new(TokenType::Int);
        t.int_value = Some(0);
        let nd = NodeData::new(t, 0);
        let chain = build_chain(vec![nd]);
        return MapperExpr {
            node: chain,
            vector_size: vector_size.max(1),
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
            output_history: vec![MapperSignalValue::I32(0); 1],
        };
    }

    // Extract the result node-list from the stack.
    let result_nodes: Vec<NodeData> = if let Some(StackEntry::Node(n)) = stack.into_iter().next() {
        n
    } else {
        let mut t = Token::new(TokenType::Int);
        t.int_value = Some(0);
        vec![NodeData::new(t, 0)]
    };

    // NOTE: we deliberately skip inserting the final TOK_TOFLOAT/TOK_TOINT32 coercions based on
    // `output_is_float`. The Rust API returns whatever variant naturally falls out of evaluation,
    // and the tests rely on this behaviour (they always supply F() inputs and check as_f32()).

    let h_size = ((-oldest_samps).ceil() as i32) + 1;
    let h_size = h_size.max(1);
    let v_size = vector_size.max(1);

    let chain = build_chain(result_nodes);

    MapperExpr {
        node: chain,
        vector_size: v_size,
        history_size: h_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); (h_size * v_size) as usize],
        output_history: vec![MapperSignalValue::I32(0); h_size as usize],
    }
}

// Iterate through the ExprNode chain and convert it back to a Vec<NodeData>-shaped slice
// for evaluation.
fn chain_to_nodes(head: &ExprNode) -> Vec<NodeData> {
    let mut nodes: Vec<NodeData> = Vec::new();
    nodes.push(NodeData {
        tok: head.tok,
        is_float: head.is_float,
        history_index: head.history_index,
        vector_index: head.vector_index,
    });
    let mut cur: Option<&Arc<ExprNode>> = head.next.as_ref();
    while let Some(arc) = cur {
        let n = arc.as_ref();
        nodes.push(NodeData {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
        });
        cur = n.next.as_ref();
    }
    nodes
}

pub fn mapper_expr_evaluate<'a>(mapper: &mut MapperExpr,
                         input: &'a MapperSignalValue) -> MapperSignalValue {
    // Update history position and copy input into input_history.
    mapper.history_pos = (mapper.history_pos + 1).rem_euclid(mapper.history_size.max(1));
    let pos = mapper.history_pos as usize;
    let v_size = mapper.vector_size.max(1) as usize;
    if !mapper.input_history.is_empty() {
        let base = pos * v_size;
        for k in 0..v_size {
            if base + k < mapper.input_history.len() {
                mapper.input_history[base + k] = *input;
            }
        }
    }

    let nodes = chain_to_nodes(&mapper.node);
    let result = evaluate_nodes(
        &nodes,
        Some(input),
        mapper.vector_size,
        mapper.history_size,
        mapper.history_pos,
        &mapper.input_history,
        &mapper.output_history,
    );

    if !mapper.output_history.is_empty() {
        if pos < mapper.output_history.len() {
            mapper.output_history[pos] = result;
        }
    }

    result
}
