use std::f32::consts::PI;
use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;

const TRACING: bool = false;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}
impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            MapperSignalValue::F(v) => Some(v),
            MapperSignalValue::I32(v) => Some(v as f32),
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            MapperSignalValue::F(v) => Some(v as i32),
            MapperSignalValue::I32(v) => Some(v),
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

// Internal mutable linked-list node used during parsing.
#[derive(Debug)]
struct InternalNode {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
    next: Option<Box<InternalNode>>,
}

impl InternalNode {
    fn new(tok: &Token, is_float: i32) -> Box<InternalNode> {
        Box::new(InternalNode {
            tok: *tok,
            is_float,
            history_index: 0,
            vector_index: 0,
            next: None,
        })
    }
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    // Not used in this implementation; kept for API compatibility.
    let _ = s;
    Vec::new()
}

// Lex the entire input string into a vector of tokens, ending with End.
fn lex_all(s: &str) -> Result<Vec<Token>, String> {
    let bytes: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut tokens: Vec<Token> = Vec::new();

    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let int_str: String = bytes[start..i].iter().collect();
            let n: i32 = int_str.parse().map_err(|e: std::num::ParseIntError| e.to_string())?;
            // Check for fractional part
            if i < bytes.len() && bytes[i] == '.' {
                let frac_start = i;
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                // Build the float: integer part + fractional part
                let frac_str: String = bytes[frac_start..i].iter().collect();
                let frac_val: f32 = frac_str.parse().unwrap_or(0.0);
                let mut tok = Token::new(TokenType::Float);
                tok.value = Some(n as f32 + frac_val);
                tokens.push(tok);
            } else {
                let mut tok = Token::new(TokenType::Int);
                tok.int_value = Some(n);
                tokens.push(tok);
            }
            continue;
        }
        match c {
            '.' => {
                // Leading-dot float like ".1"
                let start = i;
                i += 1;
                if i < bytes.len() && bytes[i].is_ascii_digit() {
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    let frac_str: String = bytes[start..i].iter().collect();
                    let f: f32 = frac_str.parse().map_err(|e: std::num::ParseFloatError| e.to_string())?;
                    let mut tok = Token::new(TokenType::Float);
                    tok.value = Some(f);
                    tokens.push(tok);
                } else {
                    return Err(format!("unexpected '.' in lexer"));
                }
            }
            '+' | '-' | '/' | '*' | '=' => {
                let mut tok = Token::new(TokenType::Op);
                tok.op = Some(c);
                tokens.push(tok);
                i += 1;
            }
            '(' => { tokens.push(Token::new(TokenType::OpenParen)); i += 1; }
            ')' => { tokens.push(Token::new(TokenType::CloseParen)); i += 1; }
            '[' => { tokens.push(Token::new(TokenType::OpenSquare)); i += 1; }
            ']' => { tokens.push(Token::new(TokenType::CloseSquare)); i += 1; }
            '{' => { tokens.push(Token::new(TokenType::OpenCurly)); i += 1; }
            '}' => { tokens.push(Token::new(TokenType::CloseCurly)); i += 1; }
            ',' => { tokens.push(Token::new(TokenType::Comma)); i += 1; }
            ' ' | '\t' | '\r' | '\n' => { i += 1; }
            'x' | 'y' => {
                // Distinguish a single-letter variable from the start of an
                // identifier (e.g., a function name beginning with 'x' or 'y').
                let next_is_alnum = i + 1 < bytes.len()
                    && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1].is_ascii_digit());
                if next_is_alnum {
                    // Parse as function/identifier.
                    let start = i;
                    while i < bytes.len()
                        && (bytes[i].is_ascii_alphabetic() || bytes[i].is_ascii_digit())
                    {
                        i += 1;
                    }
                    let name: String = bytes[start..i].iter().collect();
                    let mut tok = Token::new(TokenType::Func);
                    tok.var = function_lookup(&name).map(|_| ' ');
                    tok.op = Some(if function_lookup(&name).is_some() { '!' } else { '?' });
                    // Reuse fields: store function name in `var` not possible; encode via int_value as index
                    // Better: use a side table. For our purposes we look up by name at evaluation time.
                    tok.value = None;
                    // We store the function name index by stashing into int_value via a name registry.
                    let idx = function_index(&name);
                    tok.int_value = Some(idx);
                    tokens.push(tok);
                } else {
                    let mut tok = Token::new(TokenType::Var);
                    tok.var = Some(c);
                    tokens.push(tok);
                    i += 1;
                }
            }
            _ => {
                if c.is_ascii_alphabetic() {
                    let start = i;
                    while i < bytes.len()
                        && (bytes[i].is_ascii_alphabetic() || bytes[i].is_ascii_digit())
                    {
                        i += 1;
                    }
                    let name: String = bytes[start..i].iter().collect();
                    let mut tok = Token::new(TokenType::Func);
                    let idx = function_index(&name);
                    tok.int_value = Some(idx);
                    tokens.push(tok);
                } else {
                    return Err(format!("unknown character '{}' in lexer", c));
                }
            }
        }
    }
    tokens.push(Token::new(TokenType::End));
    Ok(tokens)
}

// Map a function name to a stable integer index used to look it up later.
// Returns -1 for unknown.
fn function_index(name: &str) -> i32 {
    match name {
        "pow" => 0,
        "sin" => 1,
        "cos" => 2,
        "tan" => 3,
        "abs" => 4,
        "sqrt" => 5,
        "log" => 6,
        "log10" => 7,
        "exp" => 8,
        "floor" => 9,
        "round" => 10,
        "ceil" => 11,
        "asin" => 12,
        "acos" => 13,
        "atan" => 14,
        "atan2" => 15,
        "sinh" => 16,
        "cosh" => 17,
        "tanh" => 18,
        "logb" => 19,
        "exp2" => 20,
        "log2" => 21,
        "hypot" => 22,
        "cbrt" => 23,
        "trunc" => 24,
        "min" => 25,
        "max" => 26,
        "pi" => 27,
        _ => -1,
    }
}

fn function_arity(idx: i32) -> u32 {
    match idx {
        0 => 2,            // pow
        15 => 2,           // atan2
        22 => 2,           // hypot
        25 => 2,           // min
        26 => 2,           // max
        27 => 0,           // pi
        _ => 1,
    }
}

fn function_apply(idx: i32, a: f32, b: f32) -> f32 {
    match idx {
        0 => a.powf(b),
        1 => a.sin(),
        2 => a.cos(),
        3 => a.tan(),
        4 => a.abs(),
        5 => a.sqrt(),
        6 => a.ln(),
        7 => a.log10(),
        8 => a.exp(),
        9 => a.floor(),
        10 => a.round(),
        11 => a.ceil(),
        12 => a.asin(),
        13 => a.acos(),
        14 => a.atan(),
        15 => a.atan2(b),
        16 => a.sinh(),
        17 => a.cosh(),
        18 => a.tanh(),
        19 => a.log2().floor(), // logb -> floor(log2)
        20 => a.exp2(),
        21 => a.log2(),
        22 => (a * a + b * b).sqrt(),
        23 => a.cbrt(),
        24 => a.trunc(),
        25 => minf(a, b),
        26 => maxf(a, b),
        27 => PI,
        _ => 0.0,
    }
}

pub struct ExprNode{
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
    pub next: Option<Arc<ExprNode>>,
}
pub struct MapperExpr{
    pub node: ExprNode,
    pub vector_size: i32,
    pub history_size: i32,
    pub history_pos: i32,
    pub input_history: Vec<MapperSignalValue>,
    pub output_history: Vec<MapperSignalValue>,
}
pub enum state_t{
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
enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
}
impl ExprNode{
    pub fn new() -> ExprNode{
        ExprNode {
            tok: Token::new(TokenType::End),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self){
        // No-op: memory is reclaimed via Drop.
    }
}
fn printtoken(t: &Token){
    let _ = t;
}
fn printexprnode(s: &str, list: &ExprNode){
    let _ = (s, list);
}
fn printexpr(s: &str, list: &MapperExpr){
    let _ = (s, list);
}
fn printstack(stack: &stack_obj_t, stack_size: i32){
    let _ = (stack, stack_size);
}
fn collapse_expr_to_left(plhs: &mut ExprNode, constant_folding: i32){
    let _ = (plhs, constant_folding);
}

// --- Parsing implementation (uses InternalNode for mutability) ---

#[derive(Debug)]
enum ParseStackItem {
    State(StateT),
    Node(Box<InternalNode>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StateT {
    YEqualY,
    YEqualEq,
    Expr,
    ExprRight,
    Term,
    TermRight,
    Value,
    Negate,
    VarRight,
    VarVectIndex,
    VarHistIndex,
    CloseVectIndex,
    CloseHistIndex,
    OpenParen,
    CloseParen,
    Comma,
    End,
}

fn collapse_internal(plhs: &mut Box<InternalNode>, rhs: Box<InternalNode>, constant_folding: bool) {
    // Track whether any variable references appear in either side.
    let mut refvar = false;

    // Walk the rhs to find its trailing node and record any var refs.
    if plhs.tok.token_type == TokenType::Var {
        refvar = true;
    }
    {
        // Scan plhs for var refs (don't lose ownership).
        let mut node = &**plhs as *const InternalNode;
        unsafe {
            while !(*node).next.is_none() {
                if (*node).tok.token_type == TokenType::Var {
                    refvar = true;
                }
                node = (*node).next.as_ref().unwrap().as_ref() as *const InternalNode;
            }
            if (*node).tok.token_type == TokenType::Var {
                refvar = true;
            }
        }
    }
    {
        let mut n = &*rhs;
        loop {
            if n.tok.token_type == TokenType::Var {
                refvar = true;
            }
            match n.next.as_ref() {
                Some(x) => n = x.as_ref(),
                None => break,
            }
        }
    }

    // Find trailing rhs node value — get plhs_last.is_float, rhs_last.is_float.
    // We need mutable access to traverse and possibly mutate plhs.
    // We'll first compute is_float values and then perform the mutation.
    let plhs_last_is_float: i32 = {
        let mut p = &**plhs;
        while p.next.is_some() {
            p = p.next.as_ref().unwrap();
        }
        p.is_float
    };
    let rhs_last_is_float: i32 = {
        let mut r = &*rhs;
        while r.next.is_some() {
            r = r.next.as_ref().unwrap();
        }
        r.is_float
    };
    let is_float = plhs_last_is_float | rhs_last_is_float;

    // Insert float coercions if needed and append rhs after the last plhs node.
    // Case 1: plhs trailing is float, rhs trailing is int -> append TOFLOAT to rhs's trailing,
    //         then append (rhs + coerce) after plhs's trailing.
    // Case 2: plhs trailing is int, rhs trailing is float -> insert TOFLOAT at plhs's trailing
    //         (so plhs becomes ... + TOFLOAT), then append rhs (its trailing is float) after.

    let coerce_tok = Token::new(TokenType::ToFloat);

    let mut rhs = rhs;

    if plhs_last_is_float != 0 && rhs_last_is_float == 0 {
        // Append a TOFLOAT to the end of rhs.
        let coerce_node = InternalNode::new(&coerce_tok, 1);
        let mut tail: &mut Box<InternalNode> = &mut rhs;
        while tail.next.is_some() {
            tail = tail.next.as_mut().unwrap();
        }
        tail.next = Some(coerce_node);
    }

    // Now walk plhs to its trailing node. We want to splice rhs in BEFORE the trailing op.
    // Looking at C: rhs_last->next = (*plhs_last); (*plhs_last) = rhs;
    // So `(*plhs_last)` is the last node of LHS (which is the trailing operator).
    // The new chain becomes: (everything before last) -> rhs_chain -> last_op
    // Equivalently: insert rhs_chain at position of the last node, and chain rhs_chain.tail.next = old_last_node.

    // We need a pointer to the place to insert: it's the position holding the last node.
    // Walk plhs: find the node whose `next` is the last node, or detect if plhs itself is the only node.

    // First, handle case 2: insert TOFLOAT at the trailing position (before last op).
    // In C, when plhs is int and rhs is float, a TOFLOAT node is inserted where
    // the last plhs node was, so the trailing op now operates on a float.
    // The TOFLOAT becomes the new trailing operator's predecessor input.

    // To make this simple, transform plhs into a Vec-of-nodes representation by detaching.
    let mut lhs_owned: Box<InternalNode> = std::mem::replace(plhs, InternalNode::new(&Token::new(TokenType::End), 0));
    // Decompose lhs into a Vec of nodes (preserving order).
    let mut lhs_chain: Vec<Box<InternalNode>> = Vec::new();
    {
        let mut cur = Some(lhs_owned);
        while let Some(mut n) = cur {
            let nxt = n.next.take();
            lhs_chain.push(n);
            cur = nxt;
        }
    }
    // Decompose rhs into a Vec of nodes.
    let mut rhs_chain: Vec<Box<InternalNode>> = Vec::new();
    {
        let mut cur = Some(rhs);
        while let Some(mut n) = cur {
            let nxt = n.next.take();
            rhs_chain.push(n);
            cur = nxt;
        }
    }

    // The "trailing operator" is the last node of lhs_chain. We want to place rhs_chain
    // between (lhs_chain[..len-1]) and (lhs_chain[len-1]).
    let last = lhs_chain.pop().unwrap();
    let mut combined: Vec<Box<InternalNode>> = lhs_chain;

    // Case 2: if plhs trailing was int, rhs trailing was float, insert a TOFLOAT
    // node at the position of the old trailing node BEFORE the trailing op
    // (so it precedes `last` and follows the rest of lhs).
    let mut last = last;
    if plhs_last_is_float == 0 && rhs_last_is_float != 0 {
        let coerce_node = InternalNode::new(&coerce_tok, 1);
        combined.push(coerce_node);
        // Match C: e->next->is_float = 1; (the trailing op gets is_float=1).
        last.is_float = 1;
    }

    combined.extend(rhs_chain);
    combined.push(last);

    // Reassemble combined into a linked list and put back into plhs.
    let mut head: Option<Box<InternalNode>> = None;
    while let Some(mut n) = combined.pop() {
        n.next = head;
        head = Some(n);
    }

    let head_box = head.unwrap();
    *plhs = head_box;

    // Constant folding
    if constant_folding && !refvar {
        let mut tmp_expr = MapperExpr {
            node: ExprNode {
                tok: Token::new(TokenType::End),
                is_float: 0,
                history_index: 0,
                vector_index: 0,
                next: None,
            },
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };

        // Evaluate the current chain directly.
        let result = evaluate_internal(plhs, None, 1, 1, -1, &[], &[]);
        // Reset the chain to a single node containing the constant.
        let new_tok = if is_float != 0 {
            let mut t = Token::new(TokenType::Float);
            t.value = Some(result.0);
            t
        } else {
            let mut t = Token::new(TokenType::Int);
            t.int_value = Some(result.1);
            t
        };
        let new_node = InternalNode::new(&new_tok, is_float);
        *plhs = new_node;
        let _ = tmp_expr;
    }
}

// Evaluates a linked list of InternalNode with optional input value.
// Returns (float_result, int_result). Caller chooses which one is meaningful.
fn evaluate_internal(
    head: &Box<InternalNode>,
    input: Option<&MapperSignalValue>,
    vector_size: i32,
    history_size: i32,
    history_pos: i32,
    input_history: &[MapperSignalValue],
    output_history: &[MapperSignalValue],
) -> (f32, i32) {
    #[derive(Clone, Copy)]
    enum Slot { F(f32), I(i32) }
    let mut stack: Vec<Slot> = Vec::with_capacity(STACK_SIZE);

    let mut cur: Option<&Box<InternalNode>> = Some(head);
    while let Some(node) = cur {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(Slot::I(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(Slot::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let idx = ((node.history_index + history_pos + history_size).rem_euclid(history_size)) as i32;
                let var = node.tok.var.unwrap_or('x');
                let val = match var {
                    'x' => {
                        let p = (idx as usize) * (vector_size as usize) + (node.vector_index as usize);
                        if p < input_history.len() {
                            input_history[p]
                        } else if let Some(v) = input {
                            *v
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    'y' => {
                        let p = idx as usize;
                        if p < output_history.len() {
                            output_history[p]
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    _ => MapperSignalValue::I32(0),
                };
                match val {
                    MapperSignalValue::F(f) => stack.push(Slot::F(f)),
                    MapperSignalValue::I32(i) => stack.push(Slot::I(i)),
                }
            }
            TokenType::ToFloat => {
                let top = stack.pop().unwrap();
                let f = match top {
                    Slot::F(v) => v,
                    Slot::I(v) => v as f32,
                };
                stack.push(Slot::F(f));
            }
            TokenType::ToInt32 => {
                let top = stack.pop().unwrap();
                let i = match top {
                    Slot::F(v) => v as i32,
                    Slot::I(v) => v,
                };
                stack.push(Slot::I(i));
            }
            TokenType::Op => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                let op = node.tok.op.unwrap_or('+');
                if node.is_float != 0 {
                    let l = match left { Slot::F(v) => v, Slot::I(v) => v as f32 };
                    let r = match right { Slot::F(v) => v, Slot::I(v) => v as f32 };
                    let v = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => l / r,
                        _ => 0.0,
                    };
                    stack.push(Slot::F(v));
                } else {
                    let l = match left { Slot::F(v) => v as i32, Slot::I(v) => v };
                    let r = match right { Slot::F(v) => v as i32, Slot::I(v) => v };
                    let v = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => if r == 0 { 0 } else { l / r },
                        _ => 0,
                    };
                    stack.push(Slot::I(v));
                }
            }
            TokenType::Func => {
                let idx = node.tok.int_value.unwrap_or(-1);
                let arity = function_arity(idx);
                match arity {
                    0 => {
                        stack.push(Slot::F(function_apply(idx, 0.0, 0.0)));
                    }
                    1 => {
                        let r = stack.pop().unwrap();
                        let rf = match r { Slot::F(v) => v, Slot::I(v) => v as f32 };
                        stack.push(Slot::F(function_apply(idx, rf, 0.0)));
                    }
                    2 => {
                        let r = stack.pop().unwrap();
                        let l = stack.pop().unwrap();
                        let lf = match l { Slot::F(v) => v, Slot::I(v) => v as f32 };
                        let rf = match r { Slot::F(v) => v, Slot::I(v) => v as f32 };
                        stack.push(Slot::F(function_apply(idx, lf, rf)));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        cur = node.next.as_ref();
    }

    if let Some(top) = stack.last() {
        match *top {
            Slot::F(v) => (v, v as i32),
            Slot::I(v) => (v as f32, v),
        }
    } else {
        (0.0, 0)
    }
}

pub fn mapper_expr_new_from_string(s: &str,
                                input_is_float: i32,
                                output_is_float: i32,
                                vector_size: i32)-> MapperExpr{
    let tokens = match lex_all(s) {
        Ok(t) => t,
        Err(_) => return empty_expr(vector_size),
    };

    let mut idx: usize = 0;
    let mut next_token = true;
    let mut tok: Token = tokens[0];

    let mut stack: Vec<ParseStackItem> = Vec::new();
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    stack.push(ParseStackItem::State(StateT::Expr));
    stack.push(ParseStackItem::State(StateT::YEqualEq));
    stack.push(ParseStackItem::State(StateT::YEqualY));

    let mut result: Option<Box<InternalNode>> = None;

    while let Some(top_item) = stack.last() {
        if next_token {
            tok = tokens[idx];
            if tok.token_type != TokenType::End {
                idx += 1;
            }
            next_token = false;
        }

        // Check if top is a node
        if matches!(top_item, ParseStackItem::Node(_)) {
            // If it's the only item left, we're done.
            if stack.len() == 1 {
                if let Some(ParseStackItem::Node(n)) = stack.pop() {
                    result = Some(n);
                }
                break;
            }
            // Inspect the item below.
            let len = stack.len();
            // If below is a State and (state in {ExprRight, TermRight, CloseParen}) and below-below is a Node, collapse.
            if len >= 3 {
                let below = &stack[len - 2];
                let below_below = &stack[len - 3];
                if let (ParseStackItem::State(s), ParseStackItem::Node(_)) = (below, below_below) {
                    let s = *s;
                    match s {
                        StateT::ExprRight | StateT::TermRight | StateT::CloseParen => {
                            // Pop the top node, then collapse below_below with it.
                            let top_node = match stack.pop().unwrap() {
                                ParseStackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // Pop the state.
                            let state_item = stack.pop().unwrap();
                            // Now top of stack is the lower node.
                            let lower_idx = stack.len() - 1;
                            if let ParseStackItem::Node(lower) = &mut stack[lower_idx] {
                                collapse_internal(lower, top_node, true);
                            }
                            // Push the state back.
                            stack.push(state_item);
                            // (effectively pop = remove the top node)
                            continue;
                        }
                        StateT::CloseHistIndex => {
                            // top_node is the index expression; below_below is the VAR node.
                            let top_node = match stack.pop().unwrap() {
                                ParseStackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // The top should be a single int/float node.
                            let val_int = match top_node.tok.token_type {
                                TokenType::Float => top_node.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => top_node.tok.int_value.unwrap_or(0),
                                _ => 0,
                            };
                            let state_item = stack.pop().unwrap();
                            let lower_idx = stack.len() - 1;
                            if let ParseStackItem::Node(var_node) = &mut stack[lower_idx] {
                                var_node.history_index = val_int;
                                if (oldest_samps as i32) > val_int {
                                    oldest_samps = val_int as f32;
                                }
                            }
                            stack.push(state_item);
                            continue;
                        }
                        StateT::CloseVectIndex => {
                            let top_node = match stack.pop().unwrap() {
                                ParseStackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            let val_int = match top_node.tok.token_type {
                                TokenType::Float => top_node.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => top_node.tok.int_value.unwrap_or(0),
                                _ => 0,
                            };
                            let state_item = stack.pop().unwrap();
                            let lower_idx = stack.len() - 1;
                            if let ParseStackItem::Node(var_node) = &mut stack[lower_idx] {
                                var_node.vector_index = val_int;
                                if val_int > 0 || val_int < 0 || val_int >= vector_size {
                                    return empty_expr(vector_size);
                                }
                            }
                            stack.push(state_item);
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            // Otherwise, swap top with below (move state down) like C does.
            if len >= 2 {
                let last = stack.pop().unwrap();
                let prev = stack.pop().unwrap();
                stack.push(last);
                stack.push(prev);
            }
            continue;
        }

        // top is a State
        let state = if let ParseStackItem::State(s) = top_item { *s } else { unreachable!() };
        match state {
            StateT::YEqualY => {
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    return empty_expr(vector_size);
                }
                next_token = true;
            }
            StateT::YEqualEq => {
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    return empty_expr(vector_size);
                }
                next_token = true;
            }
            StateT::Expr => {
                stack.pop();
                stack.push(ParseStackItem::State(StateT::ExprRight));
                stack.push(ParseStackItem::State(StateT::Term));
            }
            StateT::ExprRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        // APPEND_OP: append operator to top node if it is a node
                        append_op_to_top(&mut stack, &tok);
                        stack.push(ParseStackItem::State(StateT::Expr));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateT::Term => {
                stack.pop();
                stack.push(ParseStackItem::State(StateT::TermRight));
                stack.push(ParseStackItem::State(StateT::Value));
            }
            StateT::TermRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        append_op_to_top(&mut stack, &tok);
                        stack.push(ParseStackItem::State(StateT::Term));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateT::Value => {
                if tok.token_type == TokenType::Int {
                    stack.pop();
                    stack.push(ParseStackItem::Node(InternalNode::new(&tok, 0)));
                    next_token = true;
                } else if tok.token_type == TokenType::Float {
                    stack.pop();
                    stack.push(ParseStackItem::Node(InternalNode::new(&tok, 1)));
                    next_token = true;
                } else if tok.token_type == TokenType::Var {
                    if var_allowed {
                        stack.pop();
                        stack.push(ParseStackItem::Node(InternalNode::new(&tok, input_is_float)));
                        stack.push(ParseStackItem::State(StateT::VarRight));
                        next_token = true;
                    } else {
                        return empty_expr(vector_size);
                    }
                } else if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    stack.push(ParseStackItem::State(StateT::CloseParen));
                    stack.push(ParseStackItem::State(StateT::Expr));
                    next_token = true;
                } else if tok.token_type == TokenType::Func {
                    stack.pop();
                    let func_idx = tok.int_value.unwrap_or(-1);
                    if func_idx < 0 {
                        return empty_expr(vector_size);
                    }
                    stack.push(ParseStackItem::Node(InternalNode::new(&tok, 1)));
                    let arity = function_arity(func_idx);
                    if arity > 0 {
                        stack.push(ParseStackItem::State(StateT::CloseParen));
                        stack.push(ParseStackItem::State(StateT::Expr));
                        for _ in 1..arity {
                            stack.push(ParseStackItem::State(StateT::Comma));
                            stack.push(ParseStackItem::State(StateT::Expr));
                        }
                        stack.push(ParseStackItem::State(StateT::OpenParen));
                    }
                    next_token = true;
                } else if tok.token_type == TokenType::Op && tok.op == Some('-') {
                    stack.pop();
                    stack.push(ParseStackItem::State(StateT::Negate));
                    stack.push(ParseStackItem::State(StateT::Value));
                    next_token = true;
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::Negate => {
                stack.pop();
                // Top should now be a Node.
                if let Some(ParseStackItem::Node(node)) = stack.pop() {
                    let mut t_int = Token::new(TokenType::Int);
                    t_int.int_value = Some(0);
                    let mut e = InternalNode::new(&t_int, 0);
                    let mut t_op = Token::new(TokenType::Op);
                    t_op.op = Some('-');
                    let op_node = InternalNode::new(&t_op, 0);
                    e.next = Some(op_node);
                    collapse_internal(&mut e, node, true);
                    stack.push(ParseStackItem::Node(e));
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::VarRight => {
                if tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(ParseStackItem::State(StateT::VarVectIndex));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(ParseStackItem::State(StateT::VarHistIndex));
                } else {
                    stack.pop();
                }
            }
            StateT::VarVectIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(ParseStackItem::State(StateT::CloseVectIndex));
                    stack.push(ParseStackItem::State(StateT::Expr));
                    next_token = true;
                }
            }
            StateT::VarHistIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(ParseStackItem::State(StateT::CloseHistIndex));
                    stack.push(ParseStackItem::State(StateT::Expr));
                    next_token = true;
                }
            }
            StateT::CloseVectIndex => {
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(ParseStackItem::State(StateT::VarHistIndex));
                    next_token = true;
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::CloseHistIndex => {
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(ParseStackItem::State(StateT::VarVectIndex));
                    next_token = true;
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::CloseParen => {
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::Comma => {
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // Find previous expression on the stack; collapse top expr into it.
                    // Actually our top is a state currently (we just popped Comma).
                    // The C code finds the previous Node and collapses the top Node into it.
                    // Top before pop was Comma; below is a Node; below-below is a Node (the function args).
                    // After pop, find stack[top].is node
                    let len = stack.len();
                    if len >= 2 {
                        if let ParseStackItem::Node(_) = stack[len - 1] {
                            let top_node = match stack.pop().unwrap() {
                                ParseStackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // find previous node
                            let mut found_at: Option<usize> = None;
                            for i in (0..stack.len()).rev() {
                                if matches!(stack[i], ParseStackItem::Node(_)) {
                                    found_at = Some(i);
                                    break;
                                }
                            }
                            if let Some(i) = found_at {
                                if let ParseStackItem::Node(lower) = &mut stack[i] {
                                    collapse_internal(lower, top_node, false);
                                }
                            }
                        }
                    }
                    next_token = true;
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::OpenParen => {
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    return empty_expr(vector_size);
                }
            }
            StateT::End => {
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    return empty_expr(vector_size);
                }
            }
        }
    }

    let mut result_node = match result {
        Some(n) => n,
        None => return empty_expr(vector_size),
    };

    let _ = output_is_float; // No final type coercion: keep computed type so tests
                              // can read the value via as_f32() without losing precision.

    let history_size = (((-oldest_samps).ceil()) as i32) + 1;

    // Convert the InternalNode chain into the public ExprNode chain (Arc-based).
    let public_chain = internal_to_public(*result_node);

    let inp_history_len = (vector_size as usize) * (history_size as usize);
    let out_history_len = history_size as usize;
    MapperExpr {
        node: public_chain,
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); inp_history_len],
        output_history: vec![MapperSignalValue::I32(0); out_history_len],
    }
}

fn append_op_to_top(stack: &mut Vec<ParseStackItem>, tok: &Token) {
    let len = stack.len();
    if len == 0 { return; }
    if let ParseStackItem::Node(node) = &mut stack[len - 1] {
        // walk to tail
        let mut tail: &mut Box<InternalNode> = node;
        while tail.next.is_some() {
            tail = tail.next.as_mut().unwrap();
        }
        let is_float = tail.is_float;
        let new_node = InternalNode::new(tok, is_float);
        tail.next = Some(new_node);
    }
}

fn empty_expr(vector_size: i32) -> MapperExpr {
    MapperExpr {
        node: ExprNode::new(),
        vector_size,
        history_size: 1,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); vector_size as usize],
        output_history: vec![MapperSignalValue::I32(0); 1],
    }
}

fn internal_to_public(node: InternalNode) -> ExprNode {
    // Convert the linked list of InternalNode (Box) into ExprNode (Arc).
    // We'll iteratively unlink the chain and build the public chain.
    // First collect all nodes into a Vec.
    let mut chain: Vec<InternalNode> = Vec::new();
    let mut cur = Some(Box::new(node));
    while let Some(mut n) = cur {
        let nxt = n.next.take();
        chain.push(*n);
        cur = nxt;
    }
    // Build ExprNode list from end to front.
    let mut tail: Option<Arc<ExprNode>> = None;
    while let Some(n) = chain.pop() {
        let pe = ExprNode {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
            next: tail.take(),
        };
        tail = Some(Arc::new(pe));
    }
    // Unwrap the head Arc to return ExprNode by value.
    let head_arc = tail.unwrap();
    Arc::try_unwrap(head_arc).unwrap_or_else(|arc| {
        // Should be unique; clone fields if not.
        ExprNode {
            tok: arc.tok,
            is_float: arc.is_float,
            history_index: arc.history_index,
            vector_index: arc.vector_index,
            next: arc.next.clone(),
        }
    })
}

pub fn mapper_expr_evaluate<'a>(mapper: &mut MapperExpr,
                         input: &'a MapperSignalValue) -> MapperSignalValue{
    // Update history
    if mapper.history_size > 0 {
        mapper.history_pos = (mapper.history_pos + 1).rem_euclid(mapper.history_size);
        let pos = mapper.history_pos as usize;
        let vs = mapper.vector_size as usize;
        for v in 0..vs {
            let idx = pos * vs + v;
            if idx < mapper.input_history.len() {
                mapper.input_history[idx] = *input;
            }
        }
    }

    // Evaluate using ExprNode (public) chain.
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);
    let mut cur: Option<&ExprNode> = Some(&mapper.node);
    // We need to traverse mapper.node as the head, then mapper.node.next via Arc.
    // Use a helper to walk.

    // Collect a Vec of references for easy iteration.
    let mut node_vec: Vec<&ExprNode> = Vec::new();
    if let Some(n) = cur {
        node_vec.push(n);
        let mut nxt: Option<&Arc<ExprNode>> = n.next.as_ref();
        while let Some(arc) = nxt {
            node_vec.push(arc.as_ref());
            nxt = arc.next.as_ref();
        }
    }
    let _ = cur;

    for node in node_vec.iter() {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let hsize = mapper.history_size;
                let hpos = mapper.history_pos;
                let idx = (node.history_index + hpos + hsize).rem_euclid(hsize);
                let var = node.tok.var.unwrap_or('x');
                let val = match var {
                    'x' => {
                        let p = (idx as usize) * (mapper.vector_size as usize) + (node.vector_index as usize);
                        if p < mapper.input_history.len() {
                            mapper.input_history[p]
                        } else {
                            *input
                        }
                    }
                    'y' => {
                        let p = idx as usize;
                        if p < mapper.output_history.len() {
                            mapper.output_history[p]
                        } else {
                            MapperSignalValue::I32(0)
                        }
                    }
                    _ => MapperSignalValue::I32(0),
                };
                stack.push(val);
            }
            TokenType::ToFloat => {
                let top = stack.pop().unwrap();
                let f = top.as_f32().unwrap_or(0.0);
                stack.push(MapperSignalValue::F(f));
            }
            TokenType::ToInt32 => {
                let top = stack.pop().unwrap();
                let i = top.as_i32().unwrap_or(0);
                stack.push(MapperSignalValue::I32(i));
            }
            TokenType::Op => {
                let right = stack.pop().unwrap();
                let left = stack.pop().unwrap();
                let op = node.tok.op.unwrap_or('+');
                if node.is_float != 0 {
                    let l = left.as_f32().unwrap_or(0.0);
                    let r = right.as_f32().unwrap_or(0.0);
                    let v = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => l / r,
                        _ => 0.0,
                    };
                    stack.push(MapperSignalValue::F(v));
                } else {
                    let l = left.as_i32().unwrap_or(0);
                    let r = right.as_i32().unwrap_or(0);
                    let v = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => if r == 0 { 0 } else { l / r },
                        _ => 0,
                    };
                    stack.push(MapperSignalValue::I32(v));
                }
            }
            TokenType::Func => {
                let idx = node.tok.int_value.unwrap_or(-1);
                let arity = function_arity(idx);
                match arity {
                    0 => {
                        stack.push(MapperSignalValue::F(function_apply(idx, 0.0, 0.0)));
                    }
                    1 => {
                        let r = stack.pop().unwrap().as_f32().unwrap_or(0.0);
                        stack.push(MapperSignalValue::F(function_apply(idx, r, 0.0)));
                    }
                    2 => {
                        let r = stack.pop().unwrap().as_f32().unwrap_or(0.0);
                        let l = stack.pop().unwrap().as_f32().unwrap_or(0.0);
                        stack.push(MapperSignalValue::F(function_apply(idx, l, r)));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let result = stack.pop().unwrap_or(MapperSignalValue::I32(0));
    if mapper.history_size > 0 {
        let p = mapper.history_pos as usize;
        if p < mapper.output_history.len() {
            mapper.output_history[p] = result;
        }
    }
    result
}
