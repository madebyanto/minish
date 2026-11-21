use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn main() {
    loop {
        let current_dir = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| String::from("?"));
        
        print!("{} > ", current_dir);
        if io::stdout().flush().is_err() {
            break;
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                continue;
            }
        }

        let mut parts = input.trim().split_whitespace();
        let command = match parts.next() {
            Some(cmd) => cmd,
            None => continue, 
        };
        let args: Vec<&str> = parts.collect();

        match command {
            "exit" => break,
            "cd" => {
                let target = args.first().copied().unwrap_or("/");
                if let Err(e) = env::set_current_dir(Path::new(target)) {
                    eprintln!("cd: {}: {}", target, e);
                }
            }
            _ => execute_external(command, &args),
        }
    }
}

fn execute_external(command: &str, args: &[&str]) {
    let bin_path = if command.starts_with('/') || command.starts_with("./") {
        Path::new(command).to_path_buf()
    } else {
        Path::new("/usr/bin").join(command)
    };

    if !bin_path.exists() {
        eprintln!("minish: command not found in /usr/bin: {}", command);
        return;
    }

    match Command::new(&bin_path).args(args).spawn() {
        Ok(mut child) => {
            if let Err(e) = child.wait() {
                eprintln!("Error waiting for process: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Error executing {}: {}", bin_path.display(), e);
        }
    }
}