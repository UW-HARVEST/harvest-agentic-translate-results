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
    fn new(t: TokenType) -> Self {
        Token {
            token_type: t,
            value: None,
            int_value: None,
            var: None,
            op: None,
        }
    }
    // The function name (used for FUNC tokens)
    fn func_name(&self) -> Option<&'static str> {
        // We store the function name in the var field as a placeholder.
        // Instead, let's add a separate way.
        None
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

fn expr_lex(_s: Vec<&str>) -> Vec<Token> {
    // Not used; we use lex_string instead
    Vec::new()
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
        // No-op: Arc handles cleanup
    }
}
fn printtoken(_t: &Token) {}
fn printexprnode(_s: &str, _list: &ExprNode) {}
fn printexpr(_s: &str, _list: &MapperExpr) {}
fn printstack(_stack: &stack_obj_t, _stack_size: i32) {}

// ==================== Internal parser representation ====================

#[derive(Clone, Debug)]
struct RawNode {
    tok: RawToken,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

#[derive(Clone, Debug)]
enum RawToken {
    Float(f32),
    Int(i32),
    Op(char),
    Var(char),
    Func(&'static str), // The function name key
    ToFloat,
    ToInt32,
    OpenParen,
    CloseParen,
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Comma,
    End,
}

#[derive(Clone, Debug)]
enum LexToken {
    Float(f32),
    Int(i32),
    Op(char),
    OpenParen,
    CloseParen,
    Var(char),
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Func(Option<&'static str>), // None means unknown function
    Comma,
    End,
}

// Lex a single token from the string starting at position `pos`. Returns the token
// and the new position. Returns None on error.
fn lex_one(s: &[u8], pos: &mut usize) -> Option<LexToken> {
    // skip whitespace - C code skips at the 'again:' label after specific cases,
    // but in practice whitespace is skipped between tokens.
    // Actually C code does whitespace skip via the case ' '/'\t'/'\r'/'\n' which goto again.
    // Let's emulate that: at the top, if the char is whitespace, skip.
    if *pos >= s.len() {
        return Some(LexToken::End);
    }

    loop {
        if *pos >= s.len() {
            return Some(LexToken::End);
        }
        let c = s[*pos];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => {
                *pos += 1;
                continue;
            }
            _ => break,
        }
    }

    let c = s[*pos];

    let mut integer_found = false;
    let mut int_n: i32 = 0;
    let mut int_start = *pos;

    if c.is_ascii_digit() {
        int_start = *pos;
        while *pos < s.len() && s[*pos].is_ascii_digit() {
            *pos += 1;
        }
        let int_str = std::str::from_utf8(&s[int_start..*pos]).ok()?;
        int_n = int_str.parse().ok()?;
        integer_found = true;
        let next_c = if *pos < s.len() { s[*pos] } else { 0 };
        if next_c != b'.' {
            return Some(LexToken::Int(int_n));
        }
    }

    let c2 = if *pos < s.len() { s[*pos] } else { 0 };

    match c2 {
        b'.' => {
            let dot_pos = *pos;
            *pos += 1;
            let next_c = if *pos < s.len() { s[*pos] } else { 0 };
            if !next_c.is_ascii_digit() && integer_found {
                return Some(LexToken::Float(int_n as f32));
            }
            if !next_c.is_ascii_digit() {
                return None;
            }
            // Float: parse from dot_pos (or int_start..*pos including the dot and decimals)
            let frac_start = dot_pos;
            while *pos < s.len() && s[*pos].is_ascii_digit() {
                *pos += 1;
            }
            // The C code: tok->f = (float)n + atof(s) where s starts at the '.'
            // atof of ".5" = 0.5
            let frac_str = std::str::from_utf8(&s[frac_start..*pos]).ok()?;
            let frac: f64 = frac_str.parse().unwrap_or(0.0);
            let val = (int_n as f64) + frac;
            Some(LexToken::Float(val as f32))
        }
        b'+' | b'-' | b'/' | b'*' | b'=' => {
            *pos += 1;
            Some(LexToken::Op(c2 as char))
        }
        b'(' => {
            *pos += 1;
            Some(LexToken::OpenParen)
        }
        b')' => {
            *pos += 1;
            Some(LexToken::CloseParen)
        }
        b'x' | b'y' => {
            *pos += 1;
            Some(LexToken::Var(c2 as char))
        }
        b'[' => {
            *pos += 1;
            Some(LexToken::OpenSquare)
        }
        b']' => {
            *pos += 1;
            Some(LexToken::CloseSquare)
        }
        b'{' => {
            *pos += 1;
            Some(LexToken::OpenCurly)
        }
        b'}' => {
            *pos += 1;
            Some(LexToken::CloseCurly)
        }
        b',' => {
            *pos += 1;
            Some(LexToken::Comma)
        }
        0 => Some(LexToken::End),
        _ => {
            if !c2.is_ascii_alphabetic() {
                println!("unknown character '{}' in lexer", c2 as char);
                return None;
            }
            let name_start = *pos;
            while *pos < s.len()
                && (s[*pos].is_ascii_alphabetic() || s[*pos].is_ascii_digit())
            {
                *pos += 1;
            }
            let name = std::str::from_utf8(&s[name_start..*pos]).ok()?;
            // Look up the function name - return canonical static str if known
            match name {
                "pow" => Some(LexToken::Func(Some("pow"))),
                "sin" => Some(LexToken::Func(Some("sin"))),
                "cos" => Some(LexToken::Func(Some("cos"))),
                "tan" => Some(LexToken::Func(Some("tan"))),
                "abs" => Some(LexToken::Func(Some("abs"))),
                "sqrt" => Some(LexToken::Func(Some("sqrt"))),
                "log" => Some(LexToken::Func(Some("log"))),
                "log10" => Some(LexToken::Func(Some("log10"))),
                "exp" => Some(LexToken::Func(Some("exp"))),
                "floor" => Some(LexToken::Func(Some("floor"))),
                "round" => Some(LexToken::Func(Some("round"))),
                "ceil" => Some(LexToken::Func(Some("ceil"))),
                "min" => Some(LexToken::Func(Some("min"))),
                "max" => Some(LexToken::Func(Some("max"))),
                "pi" => Some(LexToken::Func(Some("pi"))),
                _ => Some(LexToken::Func(None)),
            }
        }
    }
}

// ==================== collapse_expr_to_left ====================

fn collapse_expr_to_left_internal(
    lhs: &mut Vec<RawNode>,
    rhs: Vec<RawNode>,
    constant_folding: bool,
) {
    // track whether any variable references exist
    let mut refvar = false;
    for n in &rhs {
        if let RawToken::Var(_) = n.tok {
            refvar = true;
        }
    }
    for n in lhs.iter() {
        if let RawToken::Var(_) = n.tok {
            refvar = true;
        }
    }

    // The LHS structure: there's potentially a trailing operator at the end.
    // The C code finds the LAST node of LHS, then inserts RHS *before* that last node.
    // Actually re-reading: "find pointer to insertion place: could be a function that needs args,
    // otherwise assume it's before the trailing operator"
    // Looking at C code:
    //   exprnode *plhs_last = plhs;
    //   while ((*plhs_last)->next) {
    //       plhs_last = &(*plhs_last)->next;
    //   }
    // So plhs_last points to the LAST node's pointer. That last node's `next` is currently NULL.
    // Then: rhs_last->next = (*plhs_last);  -- rhs's last points to the LHS's last node
    //       (*plhs_last) = rhs;             -- replace last node's slot with start of rhs
    // So actually it replaces the trailing op slot with rhs, and rhs's last->next points to old trailing op.
    //
    // Wait, plhs_last is a pointer to the pointer. Initially plhs_last = plhs (pointer to head of list).
    // While (*plhs_last)->next exists, plhs_last = &(*plhs_last)->next.
    // This means after the loop, *plhs_last is the LAST node.
    //
    // Then: rhs_last->next = (*plhs_last);  // rhs's tail's .next = last LHS node
    //       (*plhs_last) = rhs;             // The slot that previously held the last node now holds the rhs list
    //
    // Net effect: The last node of LHS is moved to the END of (rhs appended in its place).
    // I.e., everything except the trailing op goes first, then rhs, then the trailing op.

    // is_float of last LHS node || is_float of last RHS node
    let last_lhs_idx = lhs.len() - 1;
    let last_rhs_is_float = rhs.last().map(|n| n.is_float).unwrap_or(0);
    let last_lhs_is_float = lhs[last_lhs_idx].is_float;
    let is_float_combined = if last_lhs_is_float != 0 || last_rhs_is_float != 0 {
        1
    } else {
        0
    };

    // Coercion: insert TOFLOAT on whichever side is non-float (when combined is float)
    // The C code:
    //   if ((*plhs_last)->is_float && !rhs_last->is_float) {
    //       rhs_last = rhs_last->next = exprnode_new(&coerce, 1);
    //   }
    // So append a TOFLOAT to the END of RHS.
    //   else if (!(*plhs_last)->is_float && rhs_last->is_float) {
    //       exprnode e = exprnode_new(&coerce, 1);
    //       e->next = (*plhs_last);
    //       (*plhs_last) = e;
    //       plhs_last = &e->next;
    //       e->next->is_float = 1;
    //   }
    // Insert a TOFLOAT before the last node of LHS. So LHS becomes: ..., TOFLOAT, last_op
    // And then the last node's is_float is set to 1.

    let mut rhs = rhs;
    if last_lhs_is_float != 0 && last_rhs_is_float == 0 {
        rhs.push(RawNode {
            tok: RawToken::ToFloat,
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
    } else if last_lhs_is_float == 0 && last_rhs_is_float != 0 {
        // Insert TOFLOAT before last LHS node, then mark last LHS as is_float=1
        let last = lhs.pop().unwrap();
        lhs.push(RawNode {
            tok: RawToken::ToFloat,
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
        let mut last = last;
        last.is_float = 1;
        lhs.push(last);
    }

    // Now: rhs_last->next = (*plhs_last); (*plhs_last) = rhs;
    // We need to: remove last node of lhs, then append rhs, then append the original last node.
    let last_lhs_node = lhs.pop().unwrap();
    lhs.extend(rhs.into_iter());
    lhs.push(last_lhs_node);

    // If there were no variable references, evaluate the constant expression
    if constant_folding && !refvar {
        let mut tmp_expr = MapperExpr {
            node: ExprNode::new(),
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        };
        // Build a temporary chain from lhs
        let chain = build_chain_from_raw(lhs.clone());
        tmp_expr.node = chain;

        // Evaluate
        let result = evaluate_internal(&mut tmp_expr, None);

        // Replace lhs with single node containing result
        lhs.clear();
        let new_tok = if is_float_combined != 0 {
            let f = match result {
                MapperSignalValue::F(f) => f,
                MapperSignalValue::I32(i) => i as f32,
            };
            RawToken::Float(f)
        } else {
            let i = match result {
                MapperSignalValue::I32(i) => i,
                MapperSignalValue::F(f) => f as i32,
            };
            RawToken::Int(i)
        };
        lhs.push(RawNode {
            tok: new_tok,
            is_float: is_float_combined,
            history_index: 0,
            vector_index: 0,
        });
    }
}

// ==================== Building the public chain ====================

fn raw_to_token(raw: &RawToken) -> Token {
    match raw {
        RawToken::Float(f) => {
            let mut t = Token::new(TokenType::Float);
            t.value = Some(*f);
            t
        }
        RawToken::Int(i) => {
            let mut t = Token::new(TokenType::Int);
            t.int_value = Some(*i);
            t
        }
        RawToken::Op(c) => {
            let mut t = Token::new(TokenType::Op);
            t.op = Some(*c);
            t
        }
        RawToken::Var(c) => {
            let mut t = Token::new(TokenType::Var);
            t.var = Some(*c);
            t
        }
        RawToken::Func(name) => {
            let mut t = Token::new(TokenType::Func);
            // Store function name in var field as character placeholder?
            // Better: we'll use op to encode... Actually we need the name. Since
            // Token doesn't have a name field, encode via the FUNCTION_TABLE keys.
            // We'll use the var field with a special marker character mapping.
            // Simplest: store first char of name in var, but that's ambiguous.
            // Let's add the function name to a side table keyed by node identity.
            // Actually, we have a separate "name" stash idea: extend Token via op
            // to be the first char... Actually simplest: do not modify Token, but
            // store the function name as a static string in a separate map.
            // BUT that won't survive Arc.
            //
            // Cleaner: encode the function name index into int_value. We have
            // FUNCTION_TABLE keyed by name. We need a stable index. Let's add
            // a parallel ordered list.
            t.int_value = Some(func_name_to_index(name));
            t
        }
        RawToken::ToFloat => Token::new(TokenType::ToFloat),
        RawToken::ToInt32 => Token::new(TokenType::ToInt32),
        RawToken::OpenParen => Token::new(TokenType::OpenParen),
        RawToken::CloseParen => Token::new(TokenType::CloseParen),
        RawToken::OpenSquare => Token::new(TokenType::OpenSquare),
        RawToken::CloseSquare => Token::new(TokenType::CloseSquare),
        RawToken::OpenCurly => Token::new(TokenType::OpenCurly),
        RawToken::CloseCurly => Token::new(TokenType::CloseCurly),
        RawToken::Comma => Token::new(TokenType::Comma),
        RawToken::End => Token::new(TokenType::End),
    }
}

const FUNC_NAMES: &[&str] = &[
    "pow", "sin", "cos", "tan", "abs", "sqrt", "log", "log10", "exp", "floor",
    "round", "ceil", "min", "max", "pi",
];

fn func_name_to_index(name: &str) -> i32 {
    for (i, n) in FUNC_NAMES.iter().enumerate() {
        if *n == name {
            return i as i32;
        }
    }
    -1
}

fn func_index_to_entry(idx: i32) -> Option<&'static FunctionEntry> {
    if idx < 0 || idx as usize >= FUNC_NAMES.len() {
        return None;
    }
    FUNCTION_TABLE.get(FUNC_NAMES[idx as usize])
}

fn build_chain_from_raw(mut raw: Vec<RawNode>) -> ExprNode {
    if raw.is_empty() {
        return ExprNode::new();
    }
    let head = raw.remove(0);
    let mut tail_arc: Option<Arc<ExprNode>> = None;
    // Build remainder in reverse
    while let Some(n) = raw.pop() {
        let node = ExprNode {
            tok: raw_to_token(&n.tok),
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
            next: tail_arc.take(),
        };
        tail_arc = Some(Arc::new(node));
    }
    ExprNode {
        tok: raw_to_token(&head.tok),
        is_float: head.is_float,
        history_index: head.history_index,
        vector_index: head.vector_index,
        next: tail_arc,
    }
}

// ==================== Evaluation ====================

fn evaluate_internal(
    expr: &mut MapperExpr,
    input: Option<&MapperSignalValue>,
) -> MapperSignalValue {
    // Stack: store as union-like via MapperSignalValue
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);

    if let Some(inp) = input {
        expr.history_pos = (expr.history_pos + 1).rem_euclid(expr.history_size);
        // Write input into history at history_pos*vector_size for each vector element.
        // C: memcpy(&expr->input_history[expr->history_pos*expr->vector_size],
        //          input_vector, expr->vector_size * sizeof(...));
        // Since vector_size is typically 1, write a single value.
        for i in 0..expr.vector_size {
            let idx = (expr.history_pos * expr.vector_size + i) as usize;
            if idx < expr.input_history.len() {
                expr.input_history[idx] = *inp;
            }
        }
    }

    // Walk the chain
    let mut cur: Option<&ExprNode> = Some(&expr.node);
    // We'll iterate through; cur points at the current node.
    // We need to traverse the Arc chain.
    // Use unsafe-free iteration: descend via next.
    let head_ref: &ExprNode = &expr.node;
    // Manually traverse:
    let mut node_opt: Option<&ExprNode> = Some(head_ref);
    while let Some(node) = node_opt {
        let _ = cur;
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                let var = node.tok.var.unwrap_or(' ');
                let hsz = expr.history_size;
                let idx = (node.history_index + expr.history_pos + hsz).rem_euclid(hsz);
                match var {
                    'x' => {
                        let i = (idx * expr.vector_size + node.vector_index) as usize;
                        let v = if i < expr.input_history.len() {
                            expr.input_history[i]
                        } else {
                            MapperSignalValue::I32(0)
                        };
                        stack.push(v);
                    }
                    'y' => {
                        let i = idx as usize;
                        let v = if i < expr.output_history.len() {
                            expr.output_history[i]
                        } else {
                            MapperSignalValue::I32(0)
                        };
                        stack.push(v);
                    }
                    _ => {
                        // error path - return 0
                        return MapperSignalValue::I32(0);
                    }
                }
            }
            TokenType::ToFloat => {
                let top_idx = stack.len() - 1;
                let v = stack[top_idx];
                let f = match v {
                    MapperSignalValue::F(f) => f,
                    MapperSignalValue::I32(i) => i as f32,
                };
                stack[top_idx] = MapperSignalValue::F(f);
            }
            TokenType::ToInt32 => {
                let top_idx = stack.len() - 1;
                let v = stack[top_idx];
                let i = match v {
                    MapperSignalValue::F(f) => f as i32,
                    MapperSignalValue::I32(i) => i,
                };
                stack[top_idx] = MapperSignalValue::I32(i);
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let op = node.tok.op.unwrap_or('+');
                if node.is_float != 0 {
                    let lf = match left {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    let rf = match right {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    let r = match op {
                        '+' => lf + rf,
                        '-' => lf - rf,
                        '*' => lf * rf,
                        '/' => lf / rf,
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack.push(MapperSignalValue::F(r));
                } else {
                    let li = match left {
                        MapperSignalValue::I32(i) => i,
                        MapperSignalValue::F(f) => f as i32,
                    };
                    let ri = match right {
                        MapperSignalValue::I32(i) => i,
                        MapperSignalValue::F(f) => f as i32,
                    };
                    let r = match op {
                        '+' => li + ri,
                        '-' => li - ri,
                        '*' => li * ri,
                        '/' => {
                            if ri == 0 {
                                0
                            } else {
                                li / ri
                            }
                        }
                        _ => return MapperSignalValue::I32(0),
                    };
                    stack.push(MapperSignalValue::I32(r));
                }
            }
            TokenType::Func => {
                let func_idx = node.tok.int_value.unwrap_or(-1);
                let entry = match func_index_to_entry(func_idx) {
                    Some(e) => e,
                    None => return MapperSignalValue::I32(0),
                };
                match entry.arity {
                    0 => {
                        let r = (entry.func)(0.0, 0.0);
                        stack.push(MapperSignalValue::F(r));
                    }
                    1 => {
                        let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                        let rf = match right {
                            MapperSignalValue::F(f) => f,
                            MapperSignalValue::I32(i) => i as f32,
                        };
                        let r = (entry.func)(rf, 0.0);
                        stack.push(MapperSignalValue::F(r));
                    }
                    2 => {
                        let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                        let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                        let lf = match left {
                            MapperSignalValue::F(f) => f,
                            MapperSignalValue::I32(i) => i as f32,
                        };
                        let rf = match right {
                            MapperSignalValue::F(f) => f,
                            MapperSignalValue::I32(i) => i as f32,
                        };
                        let r = (entry.func)(lf, rf);
                        stack.push(MapperSignalValue::F(r));
                    }
                    _ => return MapperSignalValue::I32(0),
                }
            }
            _ => return MapperSignalValue::I32(0),
        }
        // Advance
        node_opt = node.next.as_deref();
    }

    let result = stack.first().copied().unwrap_or(MapperSignalValue::I32(0));
    if input.is_some() {
        let pos = expr.history_pos as usize;
        if pos < expr.output_history.len() {
            expr.output_history[pos] = result;
        }
    }
    result
}

// ==================== Public interface ====================

#[derive(Clone, Copy)]
enum StateOrNodeIdx {
    State(StateE),
    NodeIdx(usize), // index into the nodes vector
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StateE {
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

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    let bytes = s.as_bytes();
    let mut pos: usize = 0;

    // The stack stores either States or pointers to "nodes" (Vec<RawNode>).
    // We store node-lists in a separate Vec, and the stack holds either
    // a state or an index into the node-list vector.
    let mut stack: Vec<StackItem> = Vec::with_capacity(STACK_SIZE);

    let mut tok: LexToken = LexToken::End;
    let mut next_token = true;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;
    let mut error_message: Option<&'static str> = None;
    let mut result_nodes: Option<Vec<RawNode>> = None;

    stack.push(StackItem::State(StateE::Expr));
    stack.push(StackItem::State(StateE::YEqualEq));
    stack.push(StackItem::State(StateE::YEqualY));

    'main: while !stack.is_empty() {
        if next_token {
            tok = match lex_one(bytes, &mut pos) {
                Some(t) => t,
                None => {
                    error_message = Some("Error in lexical analysis.");
                    break 'main;
                }
            };
            next_token = false;
        }

        // Check if top of stack is a node
        let top_is_node = matches!(stack.last(), Some(StackItem::Node(_)));

        if top_is_node {
            let top = stack.len() - 1;
            if top == 0 {
                // success - extract the nodes
                if let StackItem::Node(n) = stack.pop().unwrap() {
                    result_nodes = Some(n);
                }
                break 'main;
            }
            // Check stack[top-1]
            let top_minus_1_is_state = matches!(stack[top - 1], StackItem::State(_));
            if top_minus_1_is_state {
                if top >= 2 && matches!(stack[top - 2], StackItem::Node(_)) {
                    // Get state at top-1
                    let state = match stack[top - 1] {
                        StackItem::State(s) => s,
                        _ => unreachable!(),
                    };
                    match state {
                        StateE::ExprRight | StateE::TermRight | StateE::CloseParen => {
                            // collapse_expr_to_left(plhs=&stack[top-2], stack[top])
                            let rhs = match stack.pop().unwrap() {
                                StackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // Now stack top is the state; we POP it.
                            // Wait - re-read C code:
                            //   collapse_expr_to_left(&stack[top-2].node, stack[top].node, 1);
                            //   POP();
                            // Only one POP - which removes the node (top). The state stays.
                            // So we pop only the rhs node. We've already popped it.
                            // Now stack[top-2] in original indexing is currently stack[len-2]
                            // (since we popped one). The state stack[len-1] stays.
                            // Wait: after popping rhs, the state is now the new top.
                            // The lhs node is at len-2.
                            let lhs_idx = stack.len() - 2;
                            if let StackItem::Node(ref mut lhs) = stack[lhs_idx] {
                                collapse_expr_to_left_internal(lhs, rhs, true);
                            }
                        }
                        StateE::CloseHistIndex => {
                            // var node is at top-2; this rhs node is at top
                            let rhs = match stack.pop().unwrap() {
                                StackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            // The state stays. Find node at len-2.
                            let var_idx = stack.len() - 2;
                            // Verify it's a node with VAR token
                            if rhs.len() != 1 {
                                error_message = Some("expected lonely INT or FLOAT.");
                                break 'main;
                            }
                            let val = &rhs[0].tok;
                            let history_index = match val {
                                RawToken::Float(f) => *f as i32,
                                RawToken::Int(i) => *i,
                                _ => {
                                    error_message = Some("expected lonely INT or FLOAT.");
                                    break 'main;
                                }
                            };
                            if let StackItem::Node(ref mut var_list) = stack[var_idx] {
                                let var_list_len = var_list.len();
                                if let Some(n) = var_list.first_mut() {
                                    if !matches!(n.tok, RawToken::Var(_)) || var_list_len != 1 {
                                        error_message = Some("expected VAR two-down on stack.");
                                        break 'main;
                                    }
                                    n.history_index = history_index;
                                    if (oldest_samps as i32) > history_index {
                                        oldest_samps = history_index as f32;
                                    }
                                }
                            }
                        }
                        StateE::CloseVectIndex => {
                            let rhs = match stack.pop().unwrap() {
                                StackItem::Node(n) => n,
                                _ => unreachable!(),
                            };
                            let var_idx = stack.len() - 2;
                            if rhs.len() != 1 {
                                error_message = Some("expected lonely INT or FLOAT.");
                                break 'main;
                            }
                            let val = &rhs[0].tok;
                            let vector_index = match val {
                                RawToken::Float(f) => *f as i32,
                                RawToken::Int(i) => *i,
                                _ => {
                                    error_message = Some("expected lonely INT or FLOAT.");
                                    break 'main;
                                }
                            };
                            if let StackItem::Node(ref mut var_list) = stack[var_idx] {
                                let var_list_len = var_list.len();
                                if let Some(n) = var_list.first_mut() {
                                    if !matches!(n.tok, RawToken::Var(_)) || var_list_len != 1 {
                                        error_message = Some("expected VAR two-down on stack.");
                                        break 'main;
                                    }
                                    n.vector_index = vector_index;
                                    if vector_index > 0 {
                                        error_message =
                                            Some("Vector indexing not yet implemented.");
                                        break 'main;
                                    }
                                    if vector_index < 0 || vector_index >= vector_size {
                                        error_message = Some("Vector index outside input size.");
                                        break 'main;
                                    }
                                }
                            }
                        }
                        _ => {
                            // Don't treat as collapse
                            // Fall through to swap
                            // swap top with top-1
                            let len = stack.len();
                            stack.swap(len - 1, len - 2);
                        }
                    }
                } else {
                    // swap top with top-1
                    let len = stack.len();
                    stack.swap(len - 1, len - 2);
                }
            }
            continue 'main;
        }

        // Top is a state
        let state = match stack.last().unwrap() {
            StackItem::State(s) => *s,
            _ => unreachable!(),
        };

        match state {
            StateE::YEqualY => {
                if let LexToken::Var(c) = tok {
                    if c == 'y' {
                        stack.pop();
                    } else {
                        error_message = Some("Error in y= prefix.");
                        break 'main;
                    }
                } else {
                    error_message = Some("Error in y= prefix.");
                    break 'main;
                }
                next_token = true;
            }
            StateE::YEqualEq => {
                if let LexToken::Op(op) = tok {
                    if op == '=' {
                        stack.pop();
                    } else {
                        error_message = Some("Error in y= prefix.");
                        break 'main;
                    }
                } else {
                    error_message = Some("Error in y= prefix.");
                    break 'main;
                }
                next_token = true;
            }
            StateE::Expr => {
                stack.pop();
                stack.push(StackItem::State(StateE::ExprRight));
                stack.push(StackItem::State(StateE::Term));
            }
            StateE::ExprRight => {
                if let LexToken::Op(op) = tok {
                    stack.pop();
                    if op == '+' || op == '-' {
                        // APPEND_OP
                        append_op_to_top_node(&mut stack, op);
                        stack.push(StackItem::State(StateE::Expr));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateE::Term => {
                stack.pop();
                stack.push(StackItem::State(StateE::TermRight));
                stack.push(StackItem::State(StateE::Value));
            }
            StateE::TermRight => {
                if let LexToken::Op(op) = tok {
                    stack.pop();
                    if op == '*' || op == '/' {
                        append_op_to_top_node(&mut stack, op);
                        stack.push(StackItem::State(StateE::Term));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateE::Value => match tok {
                LexToken::Int(i) => {
                    stack.pop();
                    stack.push(StackItem::Node(vec![RawNode {
                        tok: RawToken::Int(i),
                        is_float: 0,
                        history_index: 0,
                        vector_index: 0,
                    }]));
                    next_token = true;
                }
                LexToken::Float(f) => {
                    stack.pop();
                    stack.push(StackItem::Node(vec![RawNode {
                        tok: RawToken::Float(f),
                        is_float: 1,
                        history_index: 0,
                        vector_index: 0,
                    }]));
                    next_token = true;
                }
                LexToken::Var(c) => {
                    if var_allowed {
                        stack.pop();
                        stack.push(StackItem::Node(vec![RawNode {
                            tok: RawToken::Var(c),
                            is_float: input_is_float,
                            history_index: 0,
                            vector_index: 0,
                        }]));
                        stack.push(StackItem::State(StateE::VarRight));
                        next_token = true;
                    } else {
                        error_message = Some("Unexpected variable reference.");
                        break 'main;
                    }
                }
                LexToken::OpenParen => {
                    stack.pop();
                    stack.push(StackItem::State(StateE::CloseParen));
                    stack.push(StackItem::State(StateE::Expr));
                    next_token = true;
                }
                LexToken::Func(name_opt) => {
                    stack.pop();
                    let name = match name_opt {
                        Some(n) => n,
                        None => {
                            error_message = Some("Unknown function.");
                            break 'main;
                        }
                    };
                    let entry = FUNCTION_TABLE.get(name).unwrap();
                    let arity = entry.arity;
                    stack.push(StackItem::Node(vec![RawNode {
                        tok: RawToken::Func(name),
                        is_float: 1,
                        history_index: 0,
                        vector_index: 0,
                    }]));
                    if arity > 0 {
                        stack.push(StackItem::State(StateE::CloseParen));
                        stack.push(StackItem::State(StateE::Expr));
                        for _ in 1..arity {
                            stack.push(StackItem::State(StateE::Comma));
                            stack.push(StackItem::State(StateE::Expr));
                        }
                        stack.push(StackItem::State(StateE::OpenParen));
                    }
                    next_token = true;
                }
                LexToken::Op(op) if op == '-' => {
                    stack.pop();
                    stack.push(StackItem::State(StateE::Negate));
                    stack.push(StackItem::State(StateE::Value));
                    next_token = true;
                }
                _ => {
                    error_message = Some("Expected value.");
                    break 'main;
                }
            },
            StateE::Negate => {
                stack.pop();
                // top is now expected to be a node
                if matches!(stack.last(), Some(StackItem::Node(_))) {
                    let inner = match stack.pop().unwrap() {
                        StackItem::Node(n) => n,
                        _ => unreachable!(),
                    };
                    // Build: 0 - inner
                    let mut e = vec![
                        RawNode {
                            tok: RawToken::Int(0),
                            is_float: 0,
                            history_index: 0,
                            vector_index: 0,
                        },
                        RawNode {
                            tok: RawToken::Op('-'),
                            is_float: 0,
                            history_index: 0,
                            vector_index: 0,
                        },
                    ];
                    collapse_expr_to_left_internal(&mut e, inner, true);
                    stack.push(StackItem::Node(e));
                } else {
                    error_message = Some("Expected to negate an expression.");
                    break 'main;
                }
            }
            StateE::VarRight => {
                if matches!(tok, LexToken::OpenSquare) {
                    stack.pop();
                    stack.push(StackItem::State(StateE::VarVectIndex));
                } else if matches!(tok, LexToken::OpenCurly) {
                    stack.pop();
                    stack.push(StackItem::State(StateE::VarHistIndex));
                } else {
                    stack.pop();
                }
            }
            StateE::VarVectIndex => {
                stack.pop();
                if matches!(tok, LexToken::OpenSquare) {
                    var_allowed = false;
                    stack.push(StackItem::State(StateE::CloseVectIndex));
                    stack.push(StackItem::State(StateE::Expr));
                    next_token = true;
                }
            }
            StateE::VarHistIndex => {
                stack.pop();
                if matches!(tok, LexToken::OpenCurly) {
                    var_allowed = false;
                    stack.push(StackItem::State(StateE::CloseHistIndex));
                    stack.push(StackItem::State(StateE::Expr));
                    next_token = true;
                }
            }
            StateE::CloseVectIndex => {
                if matches!(tok, LexToken::CloseSquare) {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackItem::State(StateE::VarHistIndex));
                    next_token = true;
                } else {
                    error_message = Some("Expected ']'.");
                    break 'main;
                }
            }
            StateE::CloseHistIndex => {
                if matches!(tok, LexToken::CloseCurly) {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackItem::State(StateE::VarVectIndex));
                    next_token = true;
                } else {
                    error_message = Some("Expected '}'.");
                    break 'main;
                }
            }
            StateE::CloseParen => {
                if matches!(tok, LexToken::CloseParen) {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected ')'.");
                    break 'main;
                }
            }
            StateE::Comma => {
                if matches!(tok, LexToken::Comma) {
                    stack.pop();
                    // After pop, find previous expression node on the stack (search backwards).
                    // The C code: for (i=top-1; i>=0 && stack[i].type!=ST_NODE; --i){}
                    //              if (i>=0) { collapse_expr_to_left(&stack[i].node, stack[top].node, 0); POP(); }
                    // But here we already popped the COMMA state. In C, top is the COMMA's index; stack[top].node is one above? Re-read:
                    //   case COMMA:
                    //       if (tok.type == TOK_COMMA) {
                    //           POP();  // pops the COMMA state
                    //           // find previous expression on the stack
                    //           for (i=top-1; i>=0 && stack[i].type!=ST_NODE; --i) {};
                    //           if (i>=0) {
                    //               collapse_expr_to_left(&stack[i].node, stack[top].node, 0);
                    //               POP();
                    //           }
                    //           next_token = 1;
                    //       }
                    // After POP() of COMMA, top points to where COMMA was - 1. So stack[top] is the item below COMMA.
                    // In our terms: after pop, the top of stack is what was below the comma. Likely a node (the rhs of comma).
                    // Then search from top-1 (i.e., one below that) for a node.
                    // If found, collapse_expr_to_left(&stack[i].node, stack[top].node, 0) and pop.

                    // After the POP above, the stack top is the rhs node. Find lhs node.
                    let rhs_idx = stack.len() - 1;
                    let mut found: Option<usize> = None;
                    if rhs_idx > 0 {
                        for i in (0..rhs_idx).rev() {
                            if matches!(stack[i], StackItem::Node(_)) {
                                found = Some(i);
                                break;
                            }
                        }
                    }
                    if let Some(lhs_idx) = found {
                        let rhs = match stack.pop().unwrap() {
                            StackItem::Node(n) => n,
                            _ => unreachable!(),
                        };
                        if let StackItem::Node(ref mut lhs) = stack[lhs_idx] {
                            collapse_expr_to_left_internal(lhs, rhs, false);
                        }
                    }
                    next_token = true;
                } else {
                    error_message = Some("Expected ','.");
                    break 'main;
                }
            }
            StateE::OpenParen => {
                if matches!(tok, LexToken::OpenParen) {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected '('.");
                    break 'main;
                }
            }
            StateE::End => {
                if matches!(tok, LexToken::End) {
                    stack.pop();
                } else {
                    error_message = Some("Expected END.");
                    break 'main;
                }
            }
        }
    }

    // If we have result_nodes, build the final expression
    if let Some(mut nodes) = result_nodes {
        if let Some(_) = error_message {
            // shouldn't happen
        }
        if (oldest_samps as i32) < -100 {
            return MapperExpr {
                node: ExprNode::new(),
                vector_size: 1,
                history_size: 1,
                history_pos: -1,
                input_history: vec![MapperSignalValue::I32(0)],
                output_history: vec![MapperSignalValue::I32(0)],
            };
        }
        // Final coercion: in this Rust port, MapperSignalValue is dynamic so we
        // skip the final coercion to preserve the computed value's actual type.
        // The caller can use as_f32()/as_i32() to convert as needed.
        let _ = output_is_float;

        // Vector size > 1 check: error if any vector_index > 0 on a Var
        if vector_size > 1 {
            for n in &nodes {
                if let RawToken::Var(_) = n.tok {
                    if n.vector_index > 0 {
                        return MapperExpr {
                            node: ExprNode::new(),
                            vector_size: 1,
                            history_size: 1,
                            history_pos: -1,
                            input_history: vec![MapperSignalValue::I32(0)],
                            output_history: vec![MapperSignalValue::I32(0)],
                        };
                    }
                }
            }
        }

        let history_size = ((-oldest_samps).ceil() as i32) + 1;
        let history_size = if history_size < 1 { 1 } else { history_size };

        let chain = build_chain_from_raw(nodes);
        let input_history_len = (vector_size * history_size) as usize;
        let output_history_len = history_size as usize;

        return MapperExpr {
            node: chain,
            vector_size,
            history_size,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0); input_history_len],
            output_history: vec![MapperSignalValue::I32(0); output_history_len],
        };
    }

    // Failure - return a default expression
    if let Some(msg) = error_message {
        println!("{}", msg);
    }
    MapperExpr {
        node: ExprNode::new(),
        vector_size: 1,
        history_size: 1,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0)],
        output_history: vec![MapperSignalValue::I32(0)],
    }
}

enum StackItem {
    State(StateE),
    Node(Vec<RawNode>),
}

fn append_op_to_top_node(stack: &mut Vec<StackItem>, op: char) {
    if let Some(StackItem::Node(nodes)) = stack.last_mut() {
        let last_is_float = nodes.last().map(|n| n.is_float).unwrap_or(0);
        nodes.push(RawNode {
            tok: RawToken::Op(op),
            is_float: last_is_float,
            history_index: 0,
            vector_index: 0,
        });
    }
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    evaluate_internal(mapper, Some(input))
}
