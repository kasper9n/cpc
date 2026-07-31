use cpc::eval;
use std::env;
use std::path::PathBuf;
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

fn history_path() -> Option<PathBuf> {
	#[cfg(target_os = "windows")]
	{
		env::var("APPDATA")
			.ok()
			.map(|p| PathBuf::from(p).join("cpc").join("history"))
	}
	#[cfg(not(target_os = "windows"))]
	{
		env::var("HOME").ok().map(|p| {
			PathBuf::from(p)
				.join(".local")
				.join("share")
				.join("cpc")
				.join("history")
		})
	}
}

// rustyline helper for live next-line results

struct CpcHint(String);

impl rustyline::hint::Hint for CpcHint {
	fn display(&self) -> &str {
		self.0.as_str()
	}
	fn completion(&self) -> Option<&str> {
		None
	}
}

struct CpcHelper;

impl rustyline::hint::Hinter for CpcHelper {
	type Hint = CpcHint;

	fn hint(&self, line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<CpcHint> {
		let expression = line.trim();
		if expression.is_empty() {
			return None;
		}
		match eval(expression, true, false) {
			Ok(answer) => Some(CpcHint(format!("\n{answer}"))),
			Err(_) => None,
		}
	}
}

impl rustyline::highlight::Highlighter for CpcHelper {}

impl rustyline::validate::Validator for CpcHelper {}

impl rustyline::completion::Completer for CpcHelper {
	type Candidate = rustyline::completion::Pair;

	fn complete(
		&self,
		_line: &str,
		_pos: usize,
		_ctx: &rustyline::Context<'_>,
	) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
		Ok((0, Vec::new()))
	}
}

impl rustyline::Helper for CpcHelper {}

// ── REPL ───────────────────────────────────────────────────────────────────

fn repl(verbose: bool) {
	use rustyline::{Editor, config::Builder, history::FileHistory};

	let config = Builder::new()
		.history_ignore_space(true)
		.auto_add_history(true)
		.build();

	let mut rl = Editor::<CpcHelper, FileHistory>::with_config(config)
		.expect("Failed to create rustyline editor");

	rl.set_helper(Some(CpcHelper));

	let hist_path = history_path();
	if let Some(ref path) = hist_path {
		if let Some(parent) = path.parent() {
			let _ = std::fs::create_dir_all(parent);
		}
		let _ = rl.load_history(path);
	}

	loop {
		match rl.readline("> ") {
			Ok(line) => {
				if let Some(ref path) = hist_path {
					let _ = rl.save_history(path);
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
			}
			Err(rustyline::error::ReadlineError::Interrupted) => {
				break;
			}
			Err(rustyline::error::ReadlineError::Eof) => {
				println!();
				break;
			}
			Err(err) => {
				eprintln!("Error: {err}");
				break;
			}
		}
	}
}

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
