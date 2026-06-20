use crate::{vm, lexer, parser, ast};
pub const SIMPLE_LANG_COMPILER_H: bool = true;
/// Replicates: Instruction* compile_asts(ASTNode** asts, int* instr_count);
/// In Rust: returns a vector of Instructions, updates an integer by reference.
pub fn compile_asts(asts: &[Box<ast::ASTNode>], instr_count: &mut i32) -> Vec<vm::Instruction> {
    let mut all_instructions = Vec::new();
    *instr_count = 0;

    for ast in asts {
        let mut statement_count = 0;
        let instructions = compile_statement(ast, &mut statement_count);
        *instr_count += statement_count;
        all_instructions.extend(instructions);
    }

    all_instructions
}
/// Replicates: Instruction* compile(const char* source, int* instr_count);
pub fn compile(source: &str, instr_count: &mut i32) -> Vec<vm::Instruction> {
    let tokens = lexer::tokenize(source);
    let ast_nodes = parser::parse(&tokens);
    compile_asts(&ast_nodes, instr_count)
}
/// Replicates: void compile_expression(ASTNode* node, Instruction* instructions, int* count);
/// In Rust: modifies a mutable vector of Instructions and a count reference.
pub fn compile_expression(
node: &ast::ASTNode,
instructions: &mut Vec<vm::Instruction>,
count: &mut i32,
) {
    match node.type_ {
        crate::token::TokenType::TOKEN_INT => {
            emit(instructions, count, vm::OpCode::LOAD_CONST, &node.value);
        }
        crate::token::TokenType::TOKEN_IDENTIFIER => {
            emit(instructions, count, vm::OpCode::LOAD_NAME, &node.value);
        }
        crate::token::TokenType::TOKEN_PLUS => {
            compile_expression(
                node.left.as_deref().expect("plus node missing left operand"),
                instructions,
                count,
            );
            compile_expression(
                node.right.as_deref().expect("plus node missing right operand"),
                instructions,
                count,
            );
            emit(instructions, count, vm::OpCode::BINARY_ADD, "");
        }
        crate::token::TokenType::TOKEN_MINUS => {
            compile_expression(
                node.left.as_deref().expect("minus node missing left operand"),
                instructions,
                count,
            );
            compile_expression(
                node.right.as_deref().expect("minus node missing right operand"),
                instructions,
                count,
            );
            emit(instructions, count, vm::OpCode::BINARY_SUB, "");
        }
        _ => {}
    }
}
/// Replicates: Instruction* compile_statement(ASTNode* ast, int* instr_count);
pub fn compile_statement(ast: &ast::ASTNode, instr_count: &mut i32) -> Vec<vm::Instruction> {
    let mut instructions = Vec::new();
    *instr_count = 0;

    match ast.type_ {
        crate::token::TokenType::TOKEN_LET | crate::token::TokenType::TOKEN_ASSIGN => {
            compile_expression(
                ast.left.as_deref().expect("assignment missing expression"),
                &mut instructions,
                instr_count,
            );
            emit(&mut instructions, instr_count, vm::OpCode::STORE_NAME, &ast.value);
        }
        crate::token::TokenType::TOKEN_DIS => {
            compile_expression(
                ast.left.as_deref().expect("display missing expression"),
                &mut instructions,
                instr_count,
            );
            emit(&mut instructions, instr_count, vm::OpCode::STK_DIS, &ast.value);
        }
        _ => {}
    }

    instructions
}
/// Replicates: void emit(Instruction* instructions, int* count, OpCode opcode, const char* operand);
pub fn emit(
instructions: &mut Vec<vm::Instruction>,
count: &mut i32,
opcode: vm::OpCode,
operand: &str,
) {
    instructions.push(vm::new_instruction(opcode, operand));
    *count += 1;
}
