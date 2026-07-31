use cpc::eval;
use std::env;
use std::io::{self, BufRead, Write};
use std::process::exit;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
	println!(concat!(
		"Usage: cpc '<expression>' [options]",
		"\n",
		"\nOptions:",
		"\n    --verbose   Enable verbose logging",
		"\n    --version   Show cpc version",
		"\n    --help      Show this help page",
	));
}

fn get_args() -> env::Args {
	let mut args = env::args();
	args.next(); // skip binary name
	args
}

/// Interactive REPL, used when no expression argument is passed
fn repl(verbose: bool) {
	let stdin = io::stdin();
	let mut stdout = io::stdout();

	loop {
		print!("> ");
		if stdout.flush().is_err() {
			break;
		}

		let mut line = String::new();
		match stdin.lock().read_line(&mut line) {
			Ok(0) => break, // EOF (e.g. Ctrl-D)
			Ok(_) => {}
			Err(_) => break,
		}

		let expression = line.trim();
		if expression.is_empty() {
			continue;
		}

		match eval(expression, true, verbose) {
			Ok(answer) => {
				if !verbose {
					println!("{answer}");
				}
			}
			Err(e) => {
				eprintln!("{e}");
			}
		}
		if stdout.flush().is_err() {
			break;
		}
	}
}

/// CLI interface
fn main() {
	// parse these first so they work if there are unexpected args
	for arg in get_args() {
		match arg.as_str() {
			"--version" => {
				println!("{VERSION}");
				exit(0);
			}
			"--help" => {
				print_help();
				exit(0);
			}
			_ => {}
		}
	}
	let mut verbose = false;
	let mut expression_opt = None;
	for arg in get_args() {
		match arg.as_str() {
			"-v" | "--verbose" => verbose = true,
			_ => {
				if expression_opt.is_none() {
					expression_opt = Some(arg);
				} else {
					eprintln!("Unexpected argument: {}", arg);
					exit(1);
				}
			}
		}
	}
	let expression = match expression_opt {
		Some(expression) => expression,
		None => {
			repl(verbose);
			exit(0);
		}
	};

	match eval(&expression, true, verbose) {
		Ok(answer) => {
			if !verbose {
				println!("{answer}");
			}
		}
		Err(e) => {
			eprintln!("{e}");
			exit(1);
		}
	}
}
