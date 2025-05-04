use crate::weave::vm::vm::VM;
use rustyline::error::ReadlineError;
use rustyline::{Editor, Config, Cmd, KeyEvent, Modifiers, KeyCode};
use std::io::{self, Write};

pub fn repl() {
    let mut vm = VM::new(false);
    let config = Config::builder().auto_add_history(true).build();
    let mut rl: Editor<(),_> = Editor::with_config(config).unwrap();
    let mut buffer = String::new();
    let mut prompt = "wv> ";
    loop {
        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => {
                let trimmed = line.trim();
                if buffer.is_empty() && trimmed == "exit" {
                    break;
                }
                buffer.push_str(&line);
                buffer.push('\n');
                // Heuristic: if code block is likely incomplete, prompt for more lines
                // We'll use open braces/brackets/parens as a simple heuristic
                let open_braces = buffer.chars().filter(|&c| c == '{').count();
                let close_braces = buffer.chars().filter(|&c| c == '}').count();
                let open_parens = buffer.chars().filter(|&c| c == '(').count();
                let close_parens = buffer.chars().filter(|&c| c == ')').count();
                let open_brackets = buffer.chars().filter(|&c| c == '[').count();
                let close_brackets = buffer.chars().filter(|&c| c == ']').count();
                let is_incomplete = open_braces > close_braces
                    || open_parens > close_parens
                    || open_brackets > close_brackets;
                if is_incomplete {
                    prompt = "... ";
                    continue;
                } else {
                    prompt = "wv> ";
                }
                let input = std::mem::take(&mut buffer);
                match vm.interpret(&input) {
                    Ok(result) => {
                        if !matches!(result.to_string().as_str(), "") {
                            println!("{}", result);
                        }
                    }
                    Err(e) => {
                        let _ = writeln!(io::stderr(), "Error: {:?}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                let _ = writeln!(io::stderr(), "Readline error: {:?}", err);
                break;
            }
        }
    }
}