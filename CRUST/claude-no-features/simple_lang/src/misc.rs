use crate::{ast, settings, token, vm};
use std::fs;
pub const SIMPLE_LANG_MISC_H: bool = true;

fn _settings_marker() -> i32 {
    settings::DEBUG
}

/// Replicates: char* get_opcode_name(OpCode opcode);
pub fn get_opcode_name(opcode: vm::OpCode) -> String {
    match opcode {
        vm::OpCode::LOAD_CONST => "LOAD_CONST".to_string(),
        vm::OpCode::LOAD_NAME => "LOAD_NAME".to_string(),
        vm::OpCode::STORE_NAME => "STORE_NAME".to_string(),
        vm::OpCode::BINARY_SUB => "BINARY_SUB".to_string(),
        vm::OpCode::BINARY_ADD => "BINARY_ADD".to_string(),
        vm::OpCode::STK_DIS => "STK_DIS".to_string(),
    }
}

fn get_token_type_name(t: &token::TokenType) -> &'static str {
    match t {
        token::TokenType::TOKEN_INT => "TOKEN_INT",
        token::TokenType::TOKEN_IDENTIFIER => "TOKEN_IDENTIFIER",
        token::TokenType::TOKEN_ASSIGN => "TOKEN_ASSIGN",
        token::TokenType::TOKEN_PLUS => "TOKEN_PLUS",
        token::TokenType::TOKEN_MINUS => "TOKEN_MINUS",
        token::TokenType::TOKEN_SEMICOLON => "TOKEN_SEMICOLON",
        token::TokenType::TOKEN_LET => "TOKEN_LET",
        token::TokenType::TOKEN_EOF => "TOKEN_EOF",
        token::TokenType::TOKEN_DIS => "TOKEN_DIS",
    }
}

/// Replicates: void show_opcode(Instruction* instruction, int instr_count);
pub fn show_opcode(instruction: &vm::Instruction, instr_count: i32) {
    println!("bytecode:");
    // The C signature took a pointer/array of instructions; the Rust signature here
    // accepts a single instruction. We display only that single instruction
    // (replicated `instr_count` times to mirror the C loop semantics).
    for _ in 0..instr_count {
        if !instruction.operand.is_empty() {
            println!(
                "{:<15} \t ({})",
                get_opcode_name(instruction.opcode),
                instruction.operand
            );
        } else {
            println!("{:<15} \t *", get_opcode_name(instruction.opcode));
        }
    }
    println!();
}

/// Replicates: void show_tokens(Token* tokens);
pub fn show_tokens(tokens: &[token::Token]) {
    println!("tokens:");
    for tok in tokens.iter() {
        if tok.token_type == token::TokenType::TOKEN_EOF {
            break;
        }
        println!(
            "{:<15} \t ({})",
            get_token_type_name(&tok.token_type),
            tok.value
        );
    }
    println!();
}

/// Replicates: void print_ast(ASTNode* node, int level);
pub fn print_ast(node: &ast::ASTNode, level: i32) {
    for _ in 0..level {
        print!("  ");
    }
    match node.type_ {
        token::TokenType::TOKEN_INT => println!("INT: {}", node.value),
        token::TokenType::TOKEN_IDENTIFIER => println!("IDENTIFIER: {}", node.value),
        token::TokenType::TOKEN_ASSIGN => println!("ASSIGN: {}", node.value),
        token::TokenType::TOKEN_LET => println!("LET: {}", node.value),
        token::TokenType::TOKEN_PLUS => println!("PLUS"),
        token::TokenType::TOKEN_MINUS => println!("MINUS"),
        token::TokenType::TOKEN_DIS => println!("DISPLAY"),
        _ => println!("UNKNOWN"),
    }
    if let Some(left) = &node.left {
        print_ast(left, level + 1);
    }
    if let Some(right) = &node.right {
        print_ast(right, level + 1);
    }
}

/// Replicates: void print_asts(ASTNode** asts);
pub fn print_asts(asts: &[Box<ast::ASTNode>]) {
    println!("AST:");
    for node in asts.iter() {
        print_ast(node, 0);
        println!();
    }
    println!();
}

/// Replicates: char* read_file(const char* source_path);
pub fn read_file(source_path: &str) -> String {
    match fs::read_to_string(source_path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Could not open file");
            std::process::exit(-1);
        }
    }
}
