use crate::weave::vm::vm::{VM};

mod weave;
use std::env;
use std::io::Write;
use std::process::exit;

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() > 1 {
        run_file(&args[1]);
    } else {
        repl();
    }
}

fn run_file(path: &str) {
    let file_contents = std::fs::read_to_string(path).unwrap();
    let mut vm = VM::new(false);
    let res = vm.interpret(&file_contents);
    match res {
        Ok(_) => {},
        Err(e) => { println!("{:?} reading {}", e, path); exit(e.exit_code()) },
    }
}

fn repl() {
    let mut vm = VM::new(true);
    
    loop {
        // TODO: Allow multi-line input and parsing
        
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
    
        match vm.interpret(&line) {
            Ok(r) => { println!("{}", r) },
            Err(e) => {
                println!("Error: {:?}", e);
            },
        }
    }
}
