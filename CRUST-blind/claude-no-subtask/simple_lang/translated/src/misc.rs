use crate::{ast, settings, token, vm};
use crate::token::TokenType;
use std::fs;
pub const SIMPLE_LANG_MISC_H: bool = true;

/// Replicates: char* get_token_type_name(TokenType tokenType);
fn get_token_type_name(token_type: &TokenType) -> &'static str {
    match token_type {
        TokenType::TOKEN_INT => "TOKEN_INT",
        TokenType::TOKEN_IDENTIFIER => "TOKEN_IDENTIFIER",
        TokenType::TOKEN_ASSIGN => "TOKEN_ASSIGN",
        TokenType::TOKEN_PLUS => "TOKEN_PLUS",
        TokenType::TOKEN_MINUS => "TOKEN_MINUS",
        TokenType::TOKEN_SEMICOLON => "TOKEN_SEMICOLON",
        TokenType::TOKEN_LET => "TOKEN_LET",
        TokenType::TOKEN_EOF => "TOKEN_EOF",
        TokenType::TOKEN_DIS => "TOKEN_DIS",
    }
}

/// Replicates: void show_opcode(Instruction* instruction, int instr_count);
pub fn show_opcode(instruction: &vm::Instruction, instr_count: i32) {
    // The C function takes a pointer and a count and iterates instr_count
    // contiguous Instructions. Our Rust signature only gives us a single
    // Instruction reference; we replicate the visible behaviour for one
    // Instruction (header + that single line + trailing newline).
    println!("bytecode:");
    let _ = instr_count; // unused: signature constraint
    if !instruction.operand.is_empty() {
        println!(
            "{:<15} \t ({})",
            get_opcode_name(instruction.opcode),
            instruction.operand
        );
    } else {
        println!("{:<15} \t *", get_opcode_name(instruction.opcode));
    }
    println!();
}

/// Replicates: void show_tokens(Token* tokens);
/// Interpreted as working with a slice of tokens in Rust.
pub fn show_tokens(tokens: &[token::Token]) {
    println!("tokens:");
    for tok in tokens.iter() {
        if tok.token_type == TokenType::TOKEN_EOF {
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

/// Replicates: void print_asts(ASTNode** asts);
/// Interpreted as working with a slice of ASTNode pointers in Rust.
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
        Ok(mut s) => {
            // Truncate to MAX_SOURCE_LENGTH - 1 (matching the fixed buffer
            // in the C version).
            let max = (settings::MAX_SOURCE_LENGTH as usize).saturating_sub(1);
            if s.len() > max {
                s.truncate(max);
            }
            s
        }
        Err(_) => {
            eprintln!("Could not open file");
            std::process::exit(-1);
        }
    }
}

/// Replicates: void print_ast(ASTNode* node, int level);
pub fn print_ast(node: &ast::ASTNode, level: i32) {
    for _ in 0..level {
        print!("  ");
    }

    match node.type_ {
        TokenType::TOKEN_INT => println!("INT: {}", node.value),
        TokenType::TOKEN_IDENTIFIER => println!("IDENTIFIER: {}", node.value),
        TokenType::TOKEN_ASSIGN => println!("ASSIGN: {}", node.value),
        TokenType::TOKEN_LET => println!("LET: {}", node.value),
        TokenType::TOKEN_PLUS => println!("PLUS"),
        TokenType::TOKEN_MINUS => println!("MINUS"),
        TokenType::TOKEN_DIS => println!("DISPLAY"),
        _ => println!("UNKNOWN"),
    }

    if let Some(left) = &node.left {
        print_ast(left, level + 1);
    }
    if let Some(right) = &node.right {
        print_ast(right, level + 1);
    }
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
