//! calculation + conversion
//!
//! cpc parses and evaluates strings of math, with support for units and conversion. 128-bit decimal floating points are used for high accuracy.
//!
//! cpc lets you mix units, so for example 1 km - 1m results in Number { value: 999, unit: Meter }.
//!
//! Check out the [list of supported units](units/enum.Unit.html)
//!
//! # Example usage
//! ```rust
//! use cpc::eval;
//! use cpc::units::Unit;
//!
//! match eval("3m + 1cm", true, false) {
//!     Ok(answer) => {
//!         // answer: Number { value: 301, unit: Unit::Centimeter }
//!         println!("Evaluated value: {} {:?}", answer.value, answer.unit)
//!     },
//!     Err(e) => {
//!         println!("{e}")
//!     }
//! }
//! ```

use crate::units::{Unit, UnitType, primitive_unit, sort_units};
use fastnum::{D128, dec128 as d};
use serde::{Deserialize, Serialize};
use std::{
	fmt::{self, Debug, Display},
	sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use web_time::Instant;

/// Currency exchange rates
pub mod currency;
/// Turns an [`AstNode`](parser::AstNode) into a [`Number`]
pub mod evaluator;
/// Turns a string into [`Token`]s
pub mod lexer;
mod lookup;
/// Turns [`Token`]s into an [`AstNode`](parser::AstNode)
pub mod parser;
/// Units, and functions you can use with them
pub mod units;

#[derive(Clone)]
/// A number with a `Unit`.
///
/// Example:
/// ```rust
/// use cpc::{eval,Number};
/// use cpc::units::Unit;
/// use fastnum::dec128;
///
/// let x = Number::with_basic_unit(dec128!(100), Unit::Meter);
/// ```
pub struct Number {
	/// The value of the number
	pub value: D128,
	/// The unit and exponent
	pub unit: Vec<(Unit, isize)>,
}
impl Number {
	pub fn new_unitless(value: D128) -> Number {
		Number {
			value,
			unit: vec![],
		}
	}
	pub fn with_basic_unit(value: D128, unit: Unit) -> Number {
		Number {
			value,
			unit: vec![(unit, 1)],
		}
	}
	pub fn with_unit(value: D128, unit: Vec<(Unit, isize)>) -> Number {
		Number { value, unit }
	}
	pub fn has_unit(&self) -> bool {
		!self.unit.is_empty()
	}
	pub fn is_unitless(&self) -> bool {
		self.unit.is_empty()
	}
	pub fn get_simplified_value(&self) -> D128 {
		self.value.reduce()
	}
	pub fn primitive_unit(&self) -> Vec<(Unit, isize)> {
		primitive_unit(&self.unit)
	}
	pub fn contains_category(&self, category: UnitType) -> bool {
		self.unit.iter().any(|(u, _)| u.category() == category)
	}
	/// Always returns positive exponents, like `second^2` because we print
	/// `1 meter / second^2` rather than `1 meter * second^2`
	fn get_unit_string_abs_exponent(&self, plural: bool) -> String {
		let mut s = String::new();
		let mut units = self.unit.clone();
		sort_units(&mut units);
		let mut positives = units.iter().filter(|u| u.1 > 0).peekable();
		while let Some(unit) = positives.next() {
			if unit.1 <= 0 {
				continue;
			}
			if s.len() != 0 {
				s.push_str(" * ");
			}
			// only the last multiplication should be plural: `100 minute meters / volt hour`
			let is_last = positives.peek().is_none();
			match is_last && plural {
				false => s.push_str(unit.0.singular()),
				true => s.push_str(unit.0.plural()),
			};
			if unit.1.abs() >= 2 {
				s.push_str("^");
				s.push_str(&unit.1.abs().to_string());
			}
		}
		for unit in units {
			if unit.1 >= 0 {
				continue;
			}
			match s.len() {
				0 => s.push_str("/ "),
				_ => s.push_str(" / "),
			}
			s.push_str(unit.0.singular());
			if unit.1.abs() >= 2 {
				s.push_str("^");
				s.push_str(&unit.1.abs().to_string());
			}
		}
		s
	}
	pub fn singular(&self) -> String {
		self.get_unit_string_abs_exponent(false)
	}
	pub fn plural(&self) -> String {
		self.get_unit_string_abs_exponent(true)
	}
}
impl Display for Number {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let value = self.get_simplified_value();
		let word = match self.value == d!(1) {
			true => self.singular(),
			false => self.plural(),
		};
		let approx_str = match value.is_op_inexact() {
			true => "≈ ",
			false => "",
		};
		let formatted_value = add_thousands_separator(&value.to_string());
		let output = match word.as_str() {
			"" => format!("{approx_str}{formatted_value}"),
			_ => format!("{approx_str}{formatted_value} {word}"),
		};
		write!(f, "{output}")
	}
}
impl Debug for Number {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let unit_strings: Vec<_> = self
			.unit
			.iter()
			.map(|u| format!("{:?}^{}", u.0, u.1))
			.collect();
		write!(
			f,
			"Number({} {})",
			self.get_simplified_value(),
			unit_strings.join(" ")
		)
	}
}
impl PartialEq for Number {
	fn eq(&self, other: &Self) -> bool {
		self.value == other.value && self.primitive_unit() == other.primitive_unit()
	}
}

fn add_thousands_separator(s: &str) -> String {
	let (sign, s) = match s.strip_prefix('-') {
		Some(rest) => ("-", rest),
		None => ("", s),
	};
	let (int_part, rest) = s.split_once('.').map_or((s, ""), |(i, f)| (i, f));
	let len = int_part.len();
	let mut result = sign.to_string();
	for (i, c) in int_part.chars().enumerate() {
		if i > 0 && (len - i) % 3 == 0 {
			result.push(' ');
		}
		result.push(c);
	}
	if !rest.is_empty() {
		result.push('.');
		result.push_str(rest);
	}
	result
}

#[test]
fn test_thousands_separator() {
	assert_eq!(add_thousands_separator("-100000000"), "-100 000 000");
	assert_eq!(add_thousands_separator("1000"), "1 000");
	assert_eq!(add_thousands_separator("100"), "100");
	assert_eq!(add_thousands_separator("-1000"), "-1 000");
}

#[derive(Clone, Debug, PartialEq)]
/// Math operators like [`Multiply`](Operator::Multiply), parentheses, etc.
pub enum Operator {
	Plus,
	Minus,
	Multiply,
	Divide,
	Modulo,
	Caret,
	LeftParen,  // lexer only
	RightParen, // lexer only
}

#[derive(Clone, Debug, PartialEq)]
/// Unary operators like [`Percent`](UnaryOperator::Percent) and [`Factorial`](UnaryOperator::Factorial).
pub enum UnaryOperator {
	Percent,
	Factorial,
}

#[derive(Clone, Debug, PartialEq)]
/// A Text operator like [`To`](TextOperator::To) or [`Of`](TextOperator::Of).
pub enum TextOperator {
	To,
	Of,
	Per,
}

#[derive(Clone, Debug, PartialEq)]
/// A named number like [`Million`](NamedNumber::Million).
pub enum NamedNumber {
	Hundred,
	Thousand,
	Million,
	Billion,
	Trillion,
	Quadrillion,
	Quintillion,
	Sextillion,
	Septillion,
	Octillion,
	Nonillion,
	Decillion,
	Undecillion,
	Duodecillion,
	Tredecillion,
	Quattuordecillion,
	Quindecillion,
	Sexdecillion,
	Septendecillion,
	Octodecillion,
	Novemdecillion,
	Vigintillion,
	Centillion,
	Googol,
}

#[derive(Clone, Debug, PartialEq)]
/// A constant like [`Pi`](Constant::Pi) or [`E`](Constant::E).
pub enum Constant {
	Pi,
	E,
}

#[derive(Clone, Debug, PartialEq)]
/// Functions identifiers like [`Sqrt`](FunctionIdentifier::Sqrt), [`Sin`](FunctionIdentifier::Sin), [`Round`](FunctionIdentifier::Round), etc.
pub enum FunctionIdentifier {
	Sqrt,
	Cbrt,

	Log,
	Ln,
	Exp,

	Round,
	Ceil,
	Floor,
	Abs,

	Sin,
	Cos,
	Tan,
}

#[derive(Clone, Debug, PartialEq)]
/// A temporary enum used by the [`lexer`] to later determine what [`Token`] it is.
///
/// For example, when a symbol like `%` is found, the lexer turns it into a
/// the [`PercentChar`](LexerKeyword::PercentChar) variant
/// and then later it checks the surrounding [`Token`]s and,
/// dependingon them, turns it into a [`Percent`](UnaryOperator::Percent) or
/// [`Modulo`](Operator::Modulo) [`Token`].
pub enum LexerKeyword {
	PercentChar,
	In,
	Revolution,
}

#[derive(Clone, PartialEq)]
/// A token like a [`Number`](Token::Number), [`Operator`](Token::Operator), [`Unit`](Token::Unit) etc.
///
/// Strings can be divided up into these tokens by the [`lexer`], and then put into the [`parser`].
pub enum Token {
	Operator(Operator),
	UnaryOperator(UnaryOperator),
	Number(D128),
	FunctionIdentifier(FunctionIdentifier),
	Constant(Constant),
	/// Used by the parser only
	Paren,
	/// Used by the lexer only
	LexerKeyword(LexerKeyword),
	TextOperator(TextOperator),
	NamedNumber(NamedNumber),
	/// The `-` symbol, specifically when used as `-5` and not `5-5`. Used by the parser only
	Negative,
	Unit(Vec<(Unit, isize)>),
}
impl Token {
	fn unit(u: Unit) -> Token {
		Token::Unit(vec![(u, 1)])
	}
}
impl fmt::Debug for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Token::Operator(op) => write!(f, "Operator({:?})", op),
			Token::UnaryOperator(op) => write!(f, "UnaryOperator({:?})", op),
			Token::Number(num) => write!(f, "Number({num})"),
			Token::FunctionIdentifier(id) => write!(f, "FunctionIdentifier({:?})", id),
			Token::Constant(c) => write!(f, "Constant({:?})", c),
			Token::Paren => write!(f, "Paren"),
			Token::LexerKeyword(op) => write!(f, "LexerKeyword({:?})", op),
			Token::TextOperator(op) => write!(f, "TextOperator({:?})", op),
			Token::NamedNumber(num) => write!(f, "NamedNumber({:?})", num),
			Token::Negative => write!(f, "Negative"),
			Token::Unit(u) => write!(f, "Unit({:?})", u),
		}
	}
}

#[macro_export]
macro_rules! numtok {
	( $num:literal ) => {
		Token::Number(fastnum::dec128!($num))
	};
}

static SETTINGS: OnceLock<RwLock<Settings>> = OnceLock::new();

#[derive(Serialize, Deserialize)]
pub struct Settings {
	pub locale: String,
}

impl Settings {
	pub fn region(&self) -> String {
		let region = self.locale.split('-').last().unwrap().to_string();
		region
	}
	pub fn get() -> &'static RwLock<Settings> {
		SETTINGS.get_or_init(|| RwLock::new(Settings::default()))
	}
	pub fn read() -> RwLockReadGuard<'static, Settings> {
		Self::get().read().unwrap()
	}
	pub fn write() -> RwLockWriteGuard<'static, Settings> {
		Self::get().write().unwrap()
	}
	#[cfg(test)]
	pub fn reset_to_default() {
		*Self::write() = Settings::default();
	}
}
impl Default for Settings {
	fn default() -> Self {
		#[cfg(test)]
		{
			Settings {
				locale: "nb-NO".to_string(),
			}
		}
		#[cfg(not(test))]
		{
			#[cfg(target_arch = "wasm32")]
			{
				Settings {
					locale: "".to_string(),
				}
			}
			#[cfg(not(target_arch = "wasm32"))]
			Settings {
				locale: sys_locale::get_locale().unwrap_or("".to_string()),
			}
		}
	}
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn set_settings(value: JsValue) -> Result<(), JsValue> {
	let settings: Settings =
		serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
	*Settings::write() = settings;
	Ok(())
}

pub fn get_region() -> String {
	Settings::read().region()
}

/// Evaluates a string into a resulting [`Number`].
///
/// Example:
/// ```rust
/// use cpc::eval;
/// use cpc::units::Unit;
///
/// match eval("3m + 1cm", true, false) {
///     Ok(answer) => {
///         // answer: Number { value: 301, unit: Unit::Centimeter }
///         println!("Evaluated value: {} {:?}", answer.value, answer.unit)
///     },
///     Err(e) => {
///         println!("{e}")
///     }
/// }
/// ```
pub fn eval(input: &str, allow_trailing_operators: bool, verbose: bool) -> Result<Number, String> {
	let lex_start = Instant::now();

	match lexer::lex(input, allow_trailing_operators) {
		Ok(tokens) => {
			let lex_time = Instant::now().duration_since(lex_start).as_nanos() as f32;
			if verbose {
				println!("Lexed TokenVector: {:?}", tokens);
			}

			let parse_start = Instant::now();
			match parser::parse(&tokens) {
				Ok(mut ast) => {
					let parse_time = Instant::now().duration_since(parse_start).as_nanos() as f32;
					if verbose {
						println!("Parsed AstNode: {:#?}", ast);
					}

					let eval_start = Instant::now();
					match evaluator::evaluate(&mut ast) {
						Ok(answer) => {
							let eval_time =
								Instant::now().duration_since(eval_start).as_nanos() as f32;

							if verbose {
								println!("Evaluated value: {} {:?}", answer.value, answer.unit);
								println!("\u{23f1}  {:.3}ms lexing", lex_time / 1000.0 / 1000.0);
								println!("\u{23f1}  {:.3}ms parsing", parse_time / 1000.0 / 1000.0);
								println!(
									"\u{23f1}  {:.3}ms evaluation",
									eval_time / 1000.0 / 1000.0
								);
							}

							Ok(answer)
						}
						Err(e) => Err(format!("Eval error: {}", e)),
					}
				}
				Err(e) => Err(format!("Parsing error: {}", e)),
			}
		}
		Err(e) => Err(format!("Lexing error: {}", e)),
	}
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_eval(expression: &str) -> String {
	console_error_panic_hook::set_once();

	let result = eval(expression, true, false);
	match result {
		Ok(result) => result.to_string(),
		Err(e) => format!("Error: {e}"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn default_eval(input: &str) -> Number {
		eval(input, true, false).unwrap()
	}

	#[test]
	fn test_simplify() {
		assert_eq!(&default_eval("sin(pi)").to_string(), "0");
		assert_eq!(&default_eval("0.2/0.01").to_string(), "20");
	}

	#[test]
	fn test_evaluations() {
		assert_eq!(default_eval("-2(-3)"), Number::new_unitless(d!(6)));
		assert_eq!(default_eval("-2(3)"), Number::new_unitless(d!(-6)));
		assert_eq!(default_eval("(3)-2"), Number::new_unitless(d!(1)));
		assert_eq!(
			default_eval("-1km to m"),
			Number::with_basic_unit(d!(-1000), Unit::Meter)
		);
		assert_eq!(default_eval("2*-3*0.5"), Number::new_unitless(d!(-3)));
		assert_eq!(default_eval("-3^2"), Number::new_unitless(d!(-9)));
		assert_eq!(default_eval("-1+2"), Number::new_unitless(d!(1)));
	}
}
