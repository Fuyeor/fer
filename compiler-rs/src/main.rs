// src/main.rs
mod interpreter;
mod lexer;
mod parser;

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

fn main() {
    println!("Fer Programming Language Compiler");
    println!("Use `cargo test` to run the test suite.");

    let source_code = r#"
/// @/examples/hello.fer
hello = `hello`
message = `{hello} World! 💜 Fer is breathing!`
print(message)
    "#;

    let lexer = Lexer::new(source_code.trim());

    let mut parser = Parser::new(lexer);
    let ast = parser.parse_module();

    let mut interpreter = Interpreter::new();
    interpreter.run(ast);
}
