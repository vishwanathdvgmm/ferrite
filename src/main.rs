pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod semantic;
pub mod stdlib;

use crate::runtime::Interp;
use std::io::{self, Write};

fn run_main() {
    let args: Vec<String> = std::env::args().collect();
    let mut vm = Interp::new();

    if args.len() > 1 {
        let path = &args[1];
        let src = std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("ferrite: {}", e);
            std::process::exit(1);
        });
        vm.import_base = std::path::Path::new(path).parent().map(|p| p.to_path_buf());
        if let Err(e) = vm.run(&src) {
            eprintln!("\x1b[31mError:\x1b[0m {}", e);
            std::process::exit(1);
        }
    } else {
        println!("\x1b[36m╔══════════════════════════════════════╗");
        println!("║   Ferrite v1.4  —  built in Rust     ║");
        println!("║   Type 'exit' or Ctrl+D to quit       ║");
        println!("╚══════════════════════════════════════╝\x1b[0m");

        let mut buffer = String::new();
        let mut depth: i32 = 0;
        loop {
            let prompt = if depth > 0 {
                "\x1b[90m... \x1b[0m"
            } else {
                "\x1b[33m»   \x1b[0m"
            };
            print!("{}", prompt);
            io::stdout().flush().unwrap();
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => {
                    println!();
                    break;
                }
                _ => {}
            }
            let trimmed = line.trim();
            if depth == 0 && (trimmed == "exit" || trimmed == "quit") {
                break;
            }
            if trimmed.is_empty() && depth == 0 {
                continue;
            }
            for c in line.chars() {
                match c {
                    '{' | '(' | '[' => depth += 1,
                    '}' | ')' | ']' => depth -= 1,
                    _ => {}
                }
            }
            buffer.push_str(&line);
            if depth <= 0 {
                depth = 0;
                let src = buffer.trim().to_string();
                buffer.clear();
                if src.is_empty() {
                    continue;
                }
                if let Err(e) = vm.run(&src) {
                    eprintln!("\x1b[31m  Error: {}\x1b[0m", e);
                }
            }
        }
        println!("Goodbye! 🦀");
    }
}

fn main() {
    let builder = std::thread::Builder::new()
        .name("ferrite-main".into())
        .stack_size(64 * 1024 * 1024);
    let handler = builder
        .spawn(run_main)
        .expect("failed to spawn interpreter thread");
    handler.join().expect("interpreter thread panicked");
}
