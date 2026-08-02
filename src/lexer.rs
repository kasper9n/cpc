use crate::Constant::*;
use crate::FunctionIdentifier::*;
use crate::LexerKeyword::*;
use crate::NamedNumber::*;
use crate::Operator::*;
use crate::TextOperator::*;
use crate::Token;
use crate::UnaryOperator::*;
use crate::currency::currency_code_to_unit;
use crate::get_region;
use crate::units::Ambiguity;
use crate::units::Unit::*;
use fastnum::D128;
use fastnum::decimal::Context;
use std::iter::Peekable;
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

fn is_word_char_str(input: &str) -> bool {
	match input {
		"A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O"
		| "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" => true,
		"a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o"
		| "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" => true,
		"Ω" | "Ω" | "µ" | "μ" | "ł" | "$" | "€" | "£" | "₹" | "₪" | "¥" | "₩" | "₱" | "฿" | "₺"
		| "₴" | "₫" | "đ" | "Đ" | "č" | "°" => true,
		_ => false,
	}
}

fn is_numeric_str(input: &str) -> bool {
	matches!(
		input,
		"." | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
	)
}

/// For example parse a hyphen with no whitespace before or after it
fn read_immediate_grapheme(infix: &str, lexer: &mut Lexer) -> bool {
	if let Some((_i, grapheme)) = lexer.graphemes.peek() {
		if *grapheme == infix {
			lexer.graphemes.next();
			return true;
		}
	}
	false
}

/// Read next characters as a word, otherwise return empty string.
/// Returns an empty string if there's leading whitespace.
fn read_immediate_word(lexer: &mut Lexer) -> String {
	let graphemes = &mut lexer.graphemes;

	let mut word = String::new();
	while let Some((_i, grapheme)) = graphemes.peek() {
		if is_word_char_str(grapheme) {
			word += graphemes.next().unwrap().1;
		} else {
			break;
		}
	}
	word
}

/// Read next as a word, otherwise return empty string.
/// Leading whitespace is ignored. A trailing digit may be included.
fn read_word(lexer: &mut Lexer) -> String {
	let graphemes = &mut lexer.graphemes;
	// skip whitespace
	while let Some((_i, grapheme)) = graphemes.peek() {
		if grapheme.trim_start().is_empty() {
			graphemes.next();
		} else {
			break;
		}
	}
	let mut word = "".to_string();
	while let Some((_i, grapheme)) = graphemes.peek() {
		if is_word_char_str(grapheme) {
			word += graphemes.next().unwrap().1;
		} else {
			break;
		}
	}
	if !word.is_empty() {
		match *graphemes.peek().map(|(_i, g)| g).unwrap_or(&"") {
			"2" | "²" => {
				word += "2";
				graphemes.next();
			}
			"3" | "³" => {
				word += "3";
				graphemes.next();
			}
			_ => {}
		}
	}
	word
}

fn lex_token(lexer: &mut Lexer) -> Result<(), String> {
	let (_, first_grapheme) = match lexer.graphemes.peek() {
		Some(c) => *c,
		None => return Ok(()),
	};
	let token = match first_grapheme.to_ascii_lowercase().as_str() {
		grapheme if grapheme.trim_start().is_empty() => {
			lexer.graphemes.next();
			return Ok(());
		}
		grapheme if is_word_char_str(grapheme) => {
			lex_word(read_word(lexer).as_str(), lexer)?;
			return Ok(());
		}
		grapheme if is_numeric_str(grapheme) => {
			let mut number_string = String::new();
			number_string.push_str(grapheme);
			lexer.graphemes.next();

			loop {
				match lexer.graphemes.peek() {
					Some((_, g)) if is_numeric_str(g) => {
						number_string.push_str(g);
						lexer.graphemes.next();
					}
					Some((_, g)) if *g == "_" || *g == " " || *g == " " => {
						// Only treat as a thousands separator if a digit follows.
						let mut lookahead = lexer.graphemes.clone();
						lookahead.next();
						match lookahead.peek() {
							Some((_, next)) if is_numeric_str(next) => {
								lexer.graphemes.next(); // consume separator, don't keep it
							}
							_ => break,
						}
					}
					_ => break,
				}
			}

			let token = match D128::from_str(&number_string, Context::default()) {
				Ok(number) => Token::Number(number),
				Err(_e) => {
					return Err(format!("Error lexing d128 number: {}", number_string));
				}
			};
			lexer.tokens.push(token);
			return Ok(());
		}
		"+" => Token::Operator(Plus),
		"-" => Token::Operator(Minus),
		"*" => Token::Operator(Multiply),
		"/" | "÷" => Token::Operator(Divide),
		"%" => Token::LexerKeyword(PercentChar),
		"^" => Token::Operator(Caret),
		"!" => Token::UnaryOperator(Factorial),
		"(" => {
			lexer.left_paren_count += 1;
			Token::Operator(LeftParen)
		}
		")" => {
			lexer.right_paren_count += 1;
			Token::Operator(RightParen)
		}
		"π" => Token::Constant(Pi),
		"'" => Token::unit(Foot),
		"\"" | "“" | "”" | "″" => {
			lexer.graphemes.next();
			lex_word(first_grapheme, lexer)?;
			return Ok(());
		}
		grapheme => {
			return Err(format!("Invalid character: {}", grapheme));
		}
	};
	lexer.graphemes.next();
	lexer.tokens.push(token);
	Ok(())
}

fn lex_word_if_non_empty(word: &str, lexer: &mut Lexer) -> Result<(), String> {
	match word {
		"" => Ok(()),
		_ => lex_word(word, lexer),
	}
}

fn lex_word(word: &str, lexer: &mut Lexer) -> Result<(), String> {
	let token = match word.to_ascii_lowercase().as_str() {
		"to" | "as" | "into" => Token::TextOperator(To),
		"of" => Token::TextOperator(Of),

		"hundred" => Token::NamedNumber(Hundred),
		"thousand" => Token::NamedNumber(Thousand),
		"mil" | "mill" | "million" => Token::NamedNumber(Million),
		"bil" | "bill" | "billion" => Token::NamedNumber(Billion),
		"tri" | "tril" | "trillion" => Token::NamedNumber(Trillion),
		"quadrillion" => Token::NamedNumber(Quadrillion),
		"quintillion" => Token::NamedNumber(Quintillion),
		"sextillion" => Token::NamedNumber(Sextillion),
		"septillion" => Token::NamedNumber(Septillion),
		"octillion" => Token::NamedNumber(Octillion),
		"nonillion" => Token::NamedNumber(Nonillion),
		"decillion" => Token::NamedNumber(Decillion),
		"undecillion" => Token::NamedNumber(Undecillion),
		"duodecillion" => Token::NamedNumber(Duodecillion),
		"tredecillion" => Token::NamedNumber(Tredecillion),
		"quattuordecillion" => Token::NamedNumber(Quattuordecillion),
		"quindecillion" => Token::NamedNumber(Quindecillion),
		"sexdecillion" => Token::NamedNumber(Sexdecillion),
		"septendecillion" => Token::NamedNumber(Septendecillion),
		"octodecillion" => Token::NamedNumber(Octodecillion),
		"novemdecillion" => Token::NamedNumber(Novemdecillion),
		"vigintillion" => Token::NamedNumber(Vigintillion),
		"centillion" => Token::NamedNumber(Centillion),
		"googol" => Token::NamedNumber(Googol),

		"pi" => Token::Constant(Pi),
		"e" => Token::Constant(E),

		"plus" => Token::Operator(Plus),
		"minus" => Token::Operator(Minus),
		"times" => Token::Operator(Multiply),
		"multiplied" => match read_word(lexer).as_str() {
			"by" => Token::Operator(Multiply),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"divided" => match read_word(lexer).as_str() {
			"by" => Token::Operator(Divide),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"mod" => Token::Operator(Modulo),

		"sqrt" => Token::FunctionIdentifier(Sqrt),
		"cbrt" => Token::FunctionIdentifier(Cbrt),

		"log" => Token::FunctionIdentifier(Log),
		"ln" => Token::FunctionIdentifier(Ln),
		"exp" => Token::FunctionIdentifier(Exp),

		"round" | "rint" => Token::FunctionIdentifier(Round),
		"ceil" => Token::FunctionIdentifier(Ceil),
		"floor" => Token::FunctionIdentifier(Floor),
		"abs" | "fabs" => Token::FunctionIdentifier(Abs),

		"sin" => Token::FunctionIdentifier(Sin),
		"cos" => Token::FunctionIdentifier(Cos),
		"tan" => Token::FunctionIdentifier(Tan),

		"per" => Token::TextOperator(Per),

		"ns" | "nanosec" | "nanosecs" | "nanosecond" | "nanoseconds" => Token::unit(Nanosecond),
		// µ and μ are two different characters
		"µs" | "μs" | "microsec" | "microsecs" | "microsecond" | "microseconds" => {
			Token::unit(Microsecond)
		}
		"ms" | "millisec" | "millisecs" | "millisecond" | "milliseconds" => {
			Token::unit(Millisecond)
		}
		"s" | "sec" | "secs" | "second" | "seconds" => Token::unit(Second),
		"min" | "mins" | "minute" | "minutes" => Token::unit(Minute),
		"h" | "hr" | "hrs" | "hour" | "hours" => Token::unit(Hour),
		"day" | "days" => Token::unit(Day),
		"wk" | "wks" | "week" | "weeks" => Token::unit(Week),
		"mo" | "mos" | "month" | "months" => Token::unit(Month),
		"q" | "quarter" | "quarters" => Token::unit(Quarter),
		"yr" | "yrs" | "year" | "years" => Token::unit(Year),
		"decade" | "decades" => Token::unit(Decade),
		"century" | "centuries" => Token::unit(Century),
		"millenium" | "millenia" | "milleniums" => Token::unit(Millennium),

		"mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => {
			Token::unit(Millimeter)
		}
		"cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => {
			Token::unit(Centimeter)
		}
		"dm" | "decimeter" | "decimeters" | "decimetre" | "decimetres" => Token::unit(Decimeter),
		"m" | "meter" | "meters" | "metre" | "metres" => Token::unit(Meter),
		"km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => Token::unit(Kilometer),
		"\"" | "“" | "”" | "″" | "in" | "inch" | "inches" => {
			match read_word(lexer).as_str() {
				"of" => match read_word(lexer).as_str() {
					"hg" | "mercury" => Token::unit(InchOfMercury),
					other => {
						lexer.tokens.push(Token::unit(Inch));
						lexer.tokens.push(Token::TextOperator(Of));
						lex_word_if_non_empty(other, lexer)?;
						return Ok(());
					}
				},
				"hg" if matches!(word, "\"" | "“" | "”" | "″") => Token::unit(InchOfMercury),
				other => {
					if word.to_ascii_lowercase() == "in" {
						lexer.tokens.push(Token::LexerKeyword(In));
					} else {
						lexer.tokens.push(Token::unit(Inch));
					}
					lex_word_if_non_empty(other, lexer)?;
					return Ok(());
				}
			}
		}
		"ft" | "foot" | "feet" => Token::unit(Foot),
		"yd" | "yard" | "yards" => Token::unit(Yard),
		"mi" | "mile" | "miles" => Token::unit(Mile),
		"marathon" | "marathons" => Token::unit(Marathon),
		"nmi" => Token::unit(NauticalMile),
		"nautical" => match read_word(lexer).as_str() {
			"mile" | "miles" => Token::unit(NauticalMile),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"ly" | "lightyear" | "lightyears" => Token::unit(LightYear),
		"lightsec" | "lightsecs" | "lightsecond" | "lightseconds" => Token::unit(LightSecond),
		"light" => match read_word(lexer).as_str() {
			"yr" | "yrs" | "year" | "years" => Token::unit(LightYear),
			"sec" | "secs" | "second" | "seconds" => Token::unit(LightSecond),
			string => return Err(format!("Invalid string: {}", string)),
		},

		"sqmm" | "mm2" | "millimeter2" | "millimeters2" | "millimetre2" | "millimetres2" => {
			Token::unit(SquareMillimeter)
		}
		"sqcm" | "cm2" | "centimeter2" | "centimeters2" | "centimetre2" | "centimetres2" => {
			Token::unit(SquareCentimeter)
		}
		"sqdm" | "dm2" | "decimeter2" | "decimeters2" | "decimetre2" | "decimetres2" => {
			Token::unit(SquareDecimeter)
		}
		"sqm" | "m2" | "meter2" | "meters2" | "metre2" | "metres2" => Token::unit(SquareMeter),
		"sqkm" | "km2" | "kilometer2" | "kilometers2" | "kilometre2" | "kilometres2" => {
			Token::unit(SquareKilometer)
		}
		"sqin" | "in2" | "inch2" | "inches2" => Token::unit(SquareInch),
		"sqft" | "ft2" | "foot2" | "feet2" => Token::unit(SquareFoot),
		"sqyd" | "yd2" | "yard2" | "yards2" => Token::unit(SquareYard),
		"sqmi" | "mi2" | "mile2" | "miles2" => Token::unit(SquareMile),
		"sq" | "square" => match read_word(lexer).as_str() {
			"mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => {
				Token::unit(SquareMillimeter)
			}
			"cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => {
				Token::unit(SquareCentimeter)
			}
			"dm" | "decimeter" | "decimeters" | "decimetre" | "decimetres" => {
				Token::unit(SquareDecimeter)
			}
			"m" | "meter" | "meters" | "metre" | "metres" => Token::unit(SquareMeter),
			"km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => {
				Token::unit(SquareKilometer)
			}
			"in" | "inch" | "inches" => Token::unit(SquareInch),
			"ft" | "foot" | "feet" => Token::unit(SquareFoot),
			"yd" | "yard" | "yards" => Token::unit(SquareYard),
			"mi" | "mile" | "miles" => Token::unit(SquareMile),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"are" | "ares" => Token::unit(Are),
		"decare" | "decares" => Token::unit(Decare),
		"ha" | "hectare" | "hectares" => Token::unit(Hectare),
		"acre" | "acres" => Token::unit(Acre),

		"mm3" | "millimeter3" | "millimeters3" | "millimetre3" | "millimetres3" => {
			Token::unit(CubicMillimeter)
		}
		"cm3" | "centimeter3" | "centimeters3" | "centimetre3" | "centimetres3" => {
			Token::unit(CubicCentimeter)
		}
		"dm3" | "decimeter3" | "decimeters3" | "decimetre3" | "decimetres3" => {
			Token::unit(CubicDecimeter)
		}
		"m3" | "meter3" | "meters3" | "metre3" | "metres3" => Token::unit(CubicMeter),
		"km3" | "kilometer3" | "kilometers3" | "kilometre3" | "kilometres3" => {
			Token::unit(CubicKilometer)
		}
		"inc3" | "inch3" | "inches3" => Token::unit(CubicInch),
		"ft3" | "foot3" | "feet3" => Token::unit(CubicFoot),
		"yd3" | "yard3" | "yards3" => Token::unit(CubicYard),
		"mi3" | "mile3" | "miles3" => Token::unit(CubicMile),
		"cubic" => match read_word(lexer).as_str() {
			"mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => {
				Token::unit(CubicMillimeter)
			}
			"cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => {
				Token::unit(CubicCentimeter)
			}
			"dm" | "decimeter" | "decimeters" | "decimetre" | "decimetres" => {
				Token::unit(CubicDecimeter)
			}
			"m" | "meter" | "meters" | "metre" | "metres" => Token::unit(CubicMeter),
			"km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => {
				Token::unit(CubicKilometer)
			}
			"in" | "inch" | "inches" => Token::unit(CubicInch),
			"ft" | "foot" | "feet" => Token::unit(CubicFoot),
			"yd" | "yard" | "yards" => Token::unit(CubicYard),
			"mi" | "mile" | "miles" => Token::unit(CubicMile),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => {
			Token::unit(Milliliter)
		}
		"cl" | "centiliter" | "centiliters" | "centilitre" | "centilitres" => {
			Token::unit(Centiliter)
		}
		"dl" | "deciliter" | "deciliters" | "decilitre" | "decilitres" => Token::unit(Deciliter),
		"l" | "liter" | "liters" | "litre" | "litres" => Token::unit(Liter),
		"ts" | "tsp" | "tspn" | "tspns" | "teaspoon" | "teaspoons" => Token::unit(Teaspoon),
		"tbs" | "tbsp" | "tablespoon" | "tablespoons" => Token::unit(Tablespoon),
		"floz" => Token::unit(FluidOunce),
		"fl" | "fluid" => match read_word(lexer).as_str() {
			"oz" | "ounce" | "ounces" => Token::unit(FluidOunce),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"cup" | "cups" => Token::unit(Cup),
		"pt" | "pint" | "pints" => Token::unit(Pint),
		"qt" | "quart" | "quarts" => Token::unit(Quart),
		"gal" | "gallon" | "gallons" => Token::unit(Gallon),
		"bbl" => Token::unit(OilBarrel),
		"oil" => match read_word(lexer).as_str() {
			"barrel" | "barrels" => Token::unit(OilBarrel),
			string => return Err(format!("Invalid string: {}", string)),
		},

		"metric" => match read_word(lexer).as_str() {
			"ton" | "tons" | "tonne" | "tonnes" => Token::unit(MetricTon),
			"hp" | "hps" | "horsepower" | "horsepowers" => Token::unit(MetricHorsepower),
			string => return Err(format!("Invalid string: {}", string)),
		},

		"mg" | "milligram" | "milligrams" => Token::unit(Milligram),
		"g" | "gram" | "grams" => Token::unit(Gram),
		// note: "hg (inches of mercury) is parsed earlier
		"hg" | "hectogram" | "hectograms" => Token::unit(Hectogram),
		"kg" | "kilo" | "kilos" | "kilogram" | "kilograms" => Token::unit(Kilogram),
		"t" | "tonne" | "tonnes" => Token::unit(MetricTon),
		"oz" | "ounces" => Token::unit(Ounce),
		"lb" | "lbs" => Token::unit(Pound),
		"pound" | "pounds" => match read_immediate_grapheme("-", lexer) {
			true => match lexer.read_immediate_word().as_str() {
				"force" => Token::unit(PoundForce),
				other => {
					lexer.tokens.push(Token::unit(Pound));
					lexer.tokens.push(Token::Operator(Minus));
					lex_word_if_non_empty(other, lexer)?;
					return Ok(());
				}
			},
			false => Token::unit(Ambiguity(Ambiguity {
				string: "pound",
				candidates: &[Pound, GBP],
				fallback: match get_region().as_str() {
					"GB" => &GBP,
					_ => &Pound,
				},
			})),
		},
		"stone" | "stones" => Token::unit(Stone),
		"st" | "ton" | "tons" => Token::unit(ShortTon),
		"short" => match read_word(lexer).as_str() {
			"ton" | "tons" | "tonne" | "tonnes" => Token::unit(ShortTon),
			string => return Err(format!("Invalid string: {}", string)),
		},
		"lt" => Token::unit(LongTon),
		"long" => match read_word(lexer).as_str() {
			"ton" | "tons" | "tonne" | "tonnes" => Token::unit(LongTon),
			string => return Err(format!("Invalid string: {}", string)),
		},

		"bit" | "bits" => Token::unit(Bit),
		"kbit" | "kilobit" | "kilobits" => Token::unit(Kilobit),
		"mbit" | "megabit" | "megabits" => Token::unit(Megabit),
		"gbit" | "gigabit" | "gigabits" => Token::unit(Gigabit),
		"tbit" | "terabit" | "terabits" => Token::unit(Terabit),
		"pbit" | "petabit" | "petabits" => Token::unit(Petabit),
		"ebit" | "exabit" | "exabits" => Token::unit(Exabit),
		"zbit" | "zettabit" | "zettabits" => Token::unit(Zettabit),
		"ybit" | "yottabit" | "yottabits" => Token::unit(Yottabit),
		"kibit" | "kibibit" | "kibibits" => Token::unit(Kibibit),
		"mibit" | "mebibit" | "mebibits" => Token::unit(Mebibit),
		"gibit" | "gibibit" | "gibibits" => Token::unit(Gibibit),
		"tibit" | "tebibit" | "tebibits" => Token::unit(Tebibit),
		"pibit" | "pebibit" | "pebibits" => Token::unit(Pebibit),
		"eibit" | "exbibit" | "exbibits" => Token::unit(Exbibit),
		"zibit" | "zebibit" | "zebibits" => Token::unit(Zebibit),
		"yibit" | "yobibit" | "yobibits" => Token::unit(Yobibit),
		"byte" | "bytes" => Token::unit(Byte),
		"kb" | "kilobyte" | "kilobytes" => Token::unit(Kilobyte),
		"mb" | "megabyte" | "megabytes" => Token::unit(Megabyte),
		"gb" | "gigabyte" | "gigabytes" => Token::unit(Gigabyte),
		"tb" | "terabyte" | "terabytes" => Token::unit(Terabyte),
		"pb" | "petabyte" | "petabytes" => Token::unit(Petabyte),
		"eb" | "exabyte" | "exabytes" => Token::unit(Exabyte),
		"zb" | "zettabyte" | "zettabytes" => Token::unit(Zettabyte),
		"yb" | "yottabyte" | "yottabytes" => Token::unit(Yottabyte),
		"kib" | "kibibyte" | "kibibytes" => Token::unit(Kibibyte),
		"mib" | "mebibyte" | "mebibytes" => Token::unit(Mebibyte),
		"gib" | "gibibyte" | "gibibytes" => Token::unit(Gibibyte),
		"tib" | "tebibyte" | "tebibytes" => Token::unit(Tebibyte),
		"pib" | "pebibyte" | "pebibytes" => Token::unit(Pebibyte),
		"eib" | "exbibyte" | "exbibytes" => Token::unit(Exbibyte),
		"zib" | "zebibyte" | "zebibytes" => Token::unit(Zebibyte),
		"yib" | "yobibyte" | "yobibytes" => Token::unit(Yobibyte),

		"bps" if word.as_bytes()[0] == b'B' => Token::unit(BytesPerSecond),
		"kbps" if word.as_bytes()[1] == b'B' => Token::unit(KilobytesPerSecond),
		"mbps" if word.as_bytes()[1] == b'B' => Token::unit(MegabytesPerSecond),
		"gbps" if word.as_bytes()[1] == b'B' => Token::unit(GigabytesPerSecond),
		"tbps" if word.as_bytes()[1] == b'B' => Token::unit(TerabytesPerSecond),
		"pbps" if word.as_bytes()[1] == b'B' => Token::unit(PetabytesPerSecond),
		"ebps" if word.as_bytes()[1] == b'B' => Token::unit(ExabytesPerSecond),
		"zbps" if word.as_bytes()[1] == b'B' => Token::unit(ZettabytesPerSecond),
		"ybps" if word.as_bytes()[1] == b'B' => Token::unit(YottabytesPerSecond),

		"bps" => Token::unit(BitsPerSecond),
		"kbps" => Token::unit(KilobitsPerSecond),
		"mbps" => Token::unit(MegabitsPerSecond),
		"gbps" => Token::unit(GigabitsPerSecond),
		"tbps" => Token::unit(TerabitsPerSecond),
		"pbps" => Token::unit(PetabitsPerSecond),
		"ebps" => Token::unit(ExabitsPerSecond),
		"zbps" => Token::unit(ZettabitsPerSecond),
		"ybps" => Token::unit(YottabitsPerSecond),

		"flop" => Token::unit(Flop),
		"kflop" | "kiloflop" => Token::unit(KiloFlop),
		"mflop" | "megaflop" => Token::unit(MegaFlop),
		"gflop" | "gigaflop" => Token::unit(GigaFlop),
		"tflop" | "teraflop" => Token::unit(TeraFlop),
		"pflop" | "petaflop" => Token::unit(PetaFlop),
		"eflop" | "exaflop" => Token::unit(ExaFlop),
		"zflop" | "zettaflop" => Token::unit(ZettaFlop),
		"yflop" | "yottaflop" => Token::unit(YottaFlop),
		"rflop" | "ronnaflop" => Token::unit(RonnaFlop),
		"qflop" | "quettaflop" => Token::unit(QuettaFlop),

		"flops" => Token::unit(FlopPerSecond),
		"kflops" | "kiloflops" => Token::unit(KiloFlopPerSecond),
		"mflops" | "megaflops" => Token::unit(MegaFlopPerSecond),
		"gflops" | "gigaflops" => Token::unit(GigaFlopPerSecond),
		"tflops" | "teraflops" => Token::unit(TeraFlopPerSecond),
		"pflops" | "petaflops" => Token::unit(PetaFlopPerSecond),
		"eflops" | "exaflops" => Token::unit(ExaFlopPerSecond),
		"zflops" | "zettaflops" => Token::unit(ZettaFlopPerSecond),
		"yflops" | "yottaflops" => Token::unit(YottaFlopPerSecond),
		"rflops" | "ronnaflops" => Token::unit(RonnaFlopPerSecond),
		"qflops" | "quettaflops" => Token::unit(QuettaFlopPerSecond),

		"millijoule" | "millijoules" => Token::unit(Millijoule),
		"j" | "joule" | "joules" => Token::unit(Joule),
		"nm" => Token::unit(NewtonMeter),
		"kj" | "kilojoule" | "kilojoules" => Token::unit(Kilojoule),
		"mj" | "megajoule" | "megajoules" => Token::unit(Megajoule),
		"gj" | "gigajoule" | "gigajoules" => Token::unit(Gigajoule),
		"tj" | "terajoule" | "terajoules" => Token::unit(Terajoule),
		"cal" | "calorie" | "calories" => Token::unit(Calorie),
		"kcal" | "kilocalorie" | "kilocalories" => Token::unit(KiloCalorie),
		"btu" => Token::unit(BritishThermalUnit),
		"british" => match read_word(lexer).as_str() {
			"thermal" => match read_word(lexer).as_str() {
				"unit" | "units" => Token::unit(BritishThermalUnit),
				string => return Err(format!("Invalid string: {}", string)),
			},
			string => return Err(format!("Invalid string: {}", string)),
		},
		"wh" => Token::unit(WattHour),
		"kwh" => Token::unit(KilowattHour),
		"mwh" => Token::unit(MegawattHour),
		"gwh" => Token::unit(GigawattHour),
		"twh" => Token::unit(TerawattHour),
		"pwh" => Token::unit(PetawattHour),

		"milliwatt" | "milliwatts" => Token::unit(Milliwatt),
		"w" | "watts" => Token::unit(Watt),
		"kw" | "kilowatts" => Token::unit(Kilowatt),
		"mw" | "megawatts" => Token::unit(Megawatt),
		"gw" | "gigawatts" => Token::unit(Gigawatt),
		"tw" | "terawatts" => Token::unit(Terawatt),
		"pw" | "petawatts" => Token::unit(Petawatt),
		"hp" | "hps" | "horsepower" | "horsepowers" => Token::unit(Horsepower),
		"mhp" | "hpm" => Token::unit(MetricHorsepower),

		"watt" => match read_word(lexer).as_str() {
			"hr" | "hrs" | "hour" | "hours" => Token::unit(WattHour),
			other => {
				lexer.tokens.push(Token::unit(Watt));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},
		"kilowatt" => match read_word(lexer).as_str() {
			"hr" | "hrs" | "hour" | "hours" => Token::unit(KilowattHour),
			other => {
				lexer.tokens.push(Token::unit(Kilowatt));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},
		"megawatt" => match read_word(lexer).as_str() {
			"hr" | "hrs" | "hour" | "hours" => Token::unit(MegawattHour),
			other => {
				lexer.tokens.push(Token::unit(Megawatt));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},
		"gigawatt" => match read_word(lexer).as_str() {
			"hr" | "hrs" | "hour" | "hours" => Token::unit(GigawattHour),
			other => {
				lexer.tokens.push(Token::unit(Gigawatt));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},
		"terawatt" => match read_word(lexer).as_str() {
			"hr" | "hrs" | "hour" | "hours" => Token::unit(TerawattHour),
			other => {
				lexer.tokens.push(Token::unit(Terawatt));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},
		"petawatt" => match &*read_word(lexer) {
			"hr" | "hrs" | "hour" | "hours" => Token::unit(PetawattHour),
			other => {
				lexer.tokens.push(Token::unit(Petawatt));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},

		"ma" | "milliamp" | "milliamps" | "milliampere" | "milliamperes" => {
			Token::unit(Milliampere)
		}
		"a" | "amp" | "amps" | "ampere" | "amperes" => Token::unit(Ampere),
		"ka" | "kiloamp" | "kiloamps" | "kiloampere" | "kiloamperes" => Token::unit(Kiloampere),
		"bi" | "biot" | "biots" | "aba" | "abampere" | "abamperes" => Token::unit(Abampere),

		"mΩ" | "mΩ" | "milliohm" | "milliohms" => Token::unit(Milliohm),
		"Ω" | "Ω" | "ohm" | "ohms" => Token::unit(Ohm),
		"kΩ" | "kΩ" | "kiloohm" | "kiloohms" => Token::unit(Kiloohm),

		"mv" | "millivolt" | "millivolts" => Token::unit(Millivolt),
		"v" | "volt" | "volts" => Token::unit(Volt),
		"kv" | "kilovolt" | "kilovolts" => Token::unit(Kilovolt),

		"mn" | "millinewton" | "millinewtons" => Token::unit(Millinewton),
		"n" | "newtons" => Token::unit(Newton),
		"newton" => match read_immediate_grapheme("-", lexer) {
			true => match lexer.read_immediate_word().as_str() {
				"meter" | "meters" | "metre" | "metres" => Token::unit(NewtonMeter),
				other => {
					lexer.tokens.push(Token::unit(Newton));
					lexer.tokens.push(Token::Operator(Minus));
					lex_word_if_non_empty(other, lexer)?;
					return Ok(());
				}
			},
			false => match lexer.read_word().as_str() {
				"meter" | "meters" | "metre" | "metres" => Token::unit(NewtonMeter),
				other => {
					lexer.tokens.push(Token::unit(Newton));
					lex_word_if_non_empty(other, lexer)?;
					return Ok(());
				}
			},
		},
		"kn" if word.as_bytes()[1] == b'N' => Token::unit(Kilonewton),
		"kilonewton" | "kilonewtons" => Token::unit(Kilonewton),
		"meganewton" | "meganewtons" => Token::unit(Meganewton),
		"dyn" | "dyne" | "dynes" => Token::unit(Dyne),
		"lbf" => Token::unit(PoundForce),

		"pa" | "pascal" | "pascals" => Token::unit(Pascal),
		"kpa" | "kilopascal" | "kilopascals" => Token::unit(Kilopascal),
		"atm" | "atms" | "atmosphere" | "atmospheres" => Token::unit(Atmosphere),
		"mbar" | "mbars" | "millibar" | "millibars" => Token::unit(Millibar),
		"bar" | "bars" => Token::unit(Bar),
		"inhg" => Token::unit(InchOfMercury),
		"psi" => Token::unit(PoundsPerSquareInch),
		"torr" | "torrs" => Token::unit(Torr),

		"arcmin" | "arcminute" | "arcminutes" => Token::unit(Arcminute),
		"arcsec" | "arcsecond" | "arcseconds" => Token::unit(Arcsecond),
		"grad" | "gon" | "gradians" | "gons" => Token::unit(Gradian),
		"rad" | "radian" | "radians" => Token::unit(Radian),
		"mrad" | "milliradian" | "milliradians" => Token::unit(Milliradian),
		"turn" | "turns" => Token::unit(Turn),
		"r" | "rev" | "revolution" | "revolutions" => Token::unit(Revolution),

		"hz" | "hertz" => Token::unit(Hertz),
		"khz" | "kilohertz" => Token::unit(Kilohertz),
		"mhz" | "megahertz" => Token::unit(Megahertz),
		"ghz" | "gigahertz" => Token::unit(Gigahertz),
		"thz" | "terahertz" => Token::unit(Terahertz),
		"phz" | "petahertz" => Token::unit(Petahertz),
		"rpm" => Token::unit(RevolutionsPerMinute),

		"kph" | "kmh" => Token::unit(KilometersPerHour),
		"mps" => Token::unit(MetersPerSecond),
		"mph" => Token::unit(MilesPerHour),
		"fps" => Token::unit(FeetPerSecond),
		"kn" | "kt" | "knot" | "knots" => Token::unit(Knot),

		"°" | "deg" | "degs" | "degree" | "degrees" => match read_word(lexer).as_str() {
			"k" | "kelvin" | "kelvins" => Token::unit(Kelvin),
			"c" | "celsius" | "celcius" => Token::unit(Celsius),
			"f" | "fahrenheit" | "fahrenheits" | "farenheit" | "farenheits" => {
				Token::unit(Fahrenheit)
			}
			other => {
				let candidates = match get_region().as_str() {
					"BS" | "BZ" | "KY" | "PR" | "PW" | "US" => &[Degree, Fahrenheit],
					_ => &[Degree, Celsius],
				};
				let unit = Ambiguity(Ambiguity {
					string: "degree",
					candidates,
					fallback: &candidates[0],
				});
				lexer.tokens.push(Token::unit(unit));
				lex_word_if_non_empty(other, lexer)?;
				return Ok(());
			}
		},
		"k" | "°k" | "kelvin" | "kelvins" => Token::unit(Kelvin),
		"c" | "°c" | "celsius" | "celcius" => Token::unit(Celsius),
		"f" | "°f" | "fahrenheit" | "fahrenheits" | "farenheit" | "farenheits" => {
			Token::unit(Fahrenheit)
		}

		"AU$" => Token::unit(AUD),
		"R$" => Token::unit(BRL),
		"CA$" => Token::unit(CAD),
		"€" | "euro" | "euros" => Token::unit(EUR),
		"£" => Token::unit(GBP),
		"HK$" => Token::unit(HKD),
		"₹" | "Rs" => Token::unit(INR),
		"₪" => Token::unit(ILS),
		"CN¥" => Token::unit(CNY),
		"JP¥" => Token::unit(JPY),
		"¥" => match get_region().as_str() {
			"JP" => Token::unit(JPY),
			"CN" => Token::unit(CNY),
			region => return Err(format!("\"¥\" is ambiguous in your region \"{region}\"")),
		},
		"₩" => match get_region().as_str() {
			"KP" => Token::unit(KPW),
			_ => Token::unit(KRW),
		},
		"MX$" => Token::unit(MXN),
		"NZ$" => Token::unit(NZD),
		"₱" => Token::unit(PHP),
		"zl" | "zł" => Token::unit(PLN),
		"S$" => Token::unit(SGD),
		"฿" => Token::unit(THB),
		"tl" | "₺" => Token::unit(TRY),
		"₴" => Token::unit(UAH),
		"US$" => Token::unit(USD),
		"$" => match get_region().as_str() {
			"CA" => Token::unit(CAD),
			"AU" | "TV" | "KI" => Token::unit(AUD),
			"NZ" => Token::unit(NZD),
			"SG" => Token::unit(SGD),
			"BS" => Token::unit(BSD),
			"BB" => Token::unit(BBD),
			"BZ" => Token::unit(BZD),
			"BM" => Token::unit(BMD),
			"BN" => Token::unit(BND),
			"KY" => Token::unit(KYD),
			"HK" => Token::unit(HKD),
			"TW" => Token::unit(TWD),
			"JM" => Token::unit(JMD),
			"GY" => Token::unit(GYD),
			"SR" => Token::unit(SRD),
			"TT" => Token::unit(TTD),
			"SB" => Token::unit(SBD),
			"FJ" => Token::unit(FJD),
			"NA" => Token::unit(NAD),
			"LR" => Token::unit(LRD),
			"AG" | "DM" | "GD" | "KN" | "LC" | "VC" | "MS" | "AI" => Token::unit(XCD),
			"MX" => Token::unit(MXN),
			"AR" => Token::unit(ARS),
			"CL" => Token::unit(CLP),
			"CO" => Token::unit(COP),
			"CU" => Token::unit(CUP),
			"DO" => Token::unit(DOP),
			"UY" => Token::unit(UYU),
			"MO" => Token::unit(MOP),
			_ => Token::unit(USD),
		},
		"₫" | "đ" | "Đ" => Token::unit(VND),
		"kč" | "kc" => Token::unit(CZK),
		"kr" => match get_region().as_str() {
			"NO" => Token::unit(NOK),
			"SE" => Token::unit(SEK),
			"DK" => Token::unit(DKK),
			"IS" => Token::unit(ISK),
			region => return Err(format!("\"kr\" is ambiguous in your region \"{region}\"")),
		},

		string if let Ok(unit) = currency_code_to_unit(string) => Token::unit(unit),
		string => {
			return Err(format!("Invalid string: {}", string));
		}
	};
	lexer.tokens.push(token);
	Ok(())
}

struct Lexer<'a> {
	left_paren_count: u16,
	right_paren_count: u16,
	graphemes: Peekable<GraphemeIndices<'a>>,
	tokens: Vec<Token>,
}
impl<'a> Lexer<'a> {
	fn read_word(&mut self) -> String {
		read_word(self)
	}
	fn read_immediate_word(&mut self) -> String {
		read_immediate_word(self)
	}
	fn lex_token(&mut self) -> Result<(), String> {
		lex_token(self)
	}
}

/// Lex an input string and returns [`Token`]s
pub fn lex(input: &str, remove_trailing_operator: bool) -> Result<Vec<Token>, String> {
	let mut input = input.replace(',', "");

	if remove_trailing_operator {
		match &input.chars().last().unwrap_or('x') {
			'+' | '-' | '*' | '/' | '^' | '(' => {
				input.pop();
			}
			_ => {}
		}
	}

	let mut lexer = Lexer {
		left_paren_count: 0,
		right_paren_count: 0,
		graphemes: UnicodeSegmentation::grapheme_indices(input.as_str(), true).peekable(),
		tokens: Vec::new(),
	};

	while let Some(_) = lexer.graphemes.peek() {
		lexer.lex_token()?;
	}
	let tokens = &mut lexer.tokens;
	// auto insert missing parentheses in first and last position
	if lexer.left_paren_count > lexer.right_paren_count {
		let missing_right_parens = lexer.left_paren_count - lexer.right_paren_count;
		for _ in 0..missing_right_parens {
			tokens.push(Token::Operator(RightParen));
		}
	} else if lexer.left_paren_count < lexer.right_paren_count {
		let missing_left_parens = lexer.right_paren_count - lexer.left_paren_count;
		for _ in 0..missing_left_parens {
			tokens.insert(0, Token::Operator(LeftParen));
		}
	}

	if tokens.is_empty() {
		return Err("Input was empty".to_string());
	}

	let mut token_index = 0;
	loop {
		match tokens[token_index] {
			// decide if % is percent or modulo
			Token::LexerKeyword(PercentChar) => {
				match tokens.get(token_index + 1) {
					Some(Token::TextOperator(Of)) => {
						// "10% of 1km" should be percentage
						tokens[token_index] = Token::UnaryOperator(Percent);
					}
					Some(Token::Operator(operator)) => {
						match operator {
							LeftParen => {
								// "10%(2)" should be modulo
								tokens[token_index] = Token::Operator(Modulo);
							}
							_ => {
								// "10%*2" should be a percentage
								tokens[token_index] = Token::UnaryOperator(Percent);
							}
						}
					}
					Some(Token::UnaryOperator(_)) => {
						// "10%!" should be a percentage
						tokens[token_index] = Token::UnaryOperator(Percent);
					}
					Some(Token::LexerKeyword(PercentChar)) => {
						// "10%%" should be a percentage
						tokens[token_index] = Token::UnaryOperator(Percent);
					}
					None => {
						// percent if there's no element afterwards
						tokens[token_index] = Token::UnaryOperator(Percent);
					}
					_ => {
						// everything else should be modulo, for example if the % is
						// before a number, function or constants
						tokens[token_index] = Token::Operator(Modulo);
					}
				}
			}
			_ => {}
		}
		if token_index == tokens.len() - 1 {
			break;
		} else {
			token_index += 1;
		}
	}

	Ok(lexer.tokens)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{numtok, units::Ambiguity};
	use regex::Regex;

	#[test]
	fn test_lex() {
		let strip_operator_spacing = Regex::new(r" ([+\-*/]) ").unwrap();
		let strip_afterdigit_spacing = Regex::new(r"(\d) ").unwrap();
		let nonplural_data_units = Regex::new(r"(bit|byte)s").unwrap();

		#[track_caller]
		fn run_lex(
			input: &str,
			expected_tokens: Vec<Token>,
			strip_operator_spacing: &Regex,
			strip_afterdigit_spacing: &Regex,
		) {
			let tokens = match lex(input, false) {
				Ok(tokens) => tokens,
				Err(e) => {
					panic!("lex error: {}\nrun_lex input: {}", e, input);
				}
			};
			let info_msg = format!(
				"run_lex assertion failed.\n input: {}\n  left: {:?}\n right: {:?}",
				input, expected_tokens, tokens
			);
			assert_eq!(tokens, expected_tokens);

			// Prove we can handle multiple spaces wherever we handle a single space
			let input_extra_spaces = input.replace(" ", "   ");
			let tokens_extra_spaces = lex(&input_extra_spaces, false).unwrap();
			assert_eq!(tokens_extra_spaces, expected_tokens, "{info_msg}");

			// Prove we don't need spaces around operators
			let input_stripped_spaces = strip_operator_spacing.replace_all(input, "$1");
			let tokens_stripped_spaces = lex(&input_stripped_spaces, false).unwrap();
			assert_eq!(tokens_stripped_spaces, expected_tokens, "{info_msg}");

			// Prove we don't need a space after a digit
			let input_afterdigit_stripped_spaces =
				strip_afterdigit_spacing.replace_all(input, "$1");
			let tokens_afterdigit_stripped_spaces =
				lex(&input_afterdigit_stripped_spaces, false).unwrap();
			assert_eq!(
				tokens_afterdigit_stripped_spaces, expected_tokens,
				"{info_msg}"
			);
		}

		let run_datarate_lex = |input: &str, expected_tokens: Vec<Token>| {
			run_lex(
				input,
				(*expected_tokens).to_vec(),
				&strip_operator_spacing,
				&strip_afterdigit_spacing,
			);

			// Prove plural and non-plural data units behave identically
			let input_nonplural_units = nonplural_data_units.replace_all(input, "$1");
			let tokens_nonplural_units = lex(&input_nonplural_units, false).unwrap();
			let info_msg = format!(
				"run_datarate_lex input: {}\n  left: {:?}\n right: {:?}",
				input, expected_tokens, tokens_nonplural_units
			);
			assert!(tokens_nonplural_units == expected_tokens, "{info_msg}");
		};

		run_lex(
			"88 kilometres * 2",
			vec![
				numtok!(88),
				Token::unit(Kilometer),
				Token::Operator(Multiply),
				numtok!(2),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0.5 marathon",
			vec![numtok!(0.5), Token::unit(Marathon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"100 nmi",
			vec![numtok!(100), Token::unit(NauticalMile)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"101 nautical miles",
			vec![numtok!(101), Token::unit(NauticalMile)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 lightyears",
			vec![numtok!(2), Token::unit(LightYear)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 light year",
			vec![numtok!(1), Token::unit(LightYear)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"10 lightsec",
			vec![numtok!(10), Token::unit(LightSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"12 light secs",
			vec![numtok!(12), Token::unit(LightSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"33.3 square meters",
			vec![numtok!(33.3), Token::unit(SquareMeter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"54 m2",
			vec![numtok!(54), Token::unit(SquareMeter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"87 sq miles",
			vec![numtok!(87), Token::unit(SquareMile)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"500 feet2",
			vec![numtok!(500), Token::unit(SquareFoot)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"500 feet²",
			vec![numtok!(500), Token::unit(SquareFoot)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 cubic metres",
			vec![numtok!(4), Token::unit(CubicMeter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"34 cubic feet + 23 cubic yards",
			vec![
				numtok!(34),
				Token::unit(CubicFoot),
				Token::Operator(Plus),
				numtok!(23),
				Token::unit(CubicYard),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"66 inches3 + 65 millimetre³",
			vec![
				numtok!(66),
				Token::unit(CubicInch),
				Token::Operator(Plus),
				numtok!(65),
				Token::unit(CubicMillimeter),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"66 inches³ + 65 millimetre3",
			vec![
				numtok!(66),
				Token::unit(CubicInch),
				Token::Operator(Plus),
				numtok!(65),
				Token::unit(CubicMillimeter),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"42 millilitres",
			vec![numtok!(42), Token::unit(Milliliter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 tbs",
			vec![numtok!(3), Token::unit(Tablespoon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 floz",
			vec![numtok!(6), Token::unit(FluidOunce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 fl oz",
			vec![numtok!(6), Token::unit(FluidOunce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 fluid ounces",
			vec![numtok!(6), Token::unit(FluidOunce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 oil barrels",
			vec![numtok!(3), Token::unit(OilBarrel)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"67 kg",
			vec![numtok!(67), Token::unit(Kilogram)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"34 oz",
			vec![numtok!(34), Token::unit(Ounce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"34 ounces",
			vec![numtok!(34), Token::unit(Ounce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"210 lb",
			vec![numtok!(210), Token::unit(Pound)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"210 lbs",
			vec![numtok!(210), Token::unit(Pound)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"210 pound",
			vec![
				numtok!(210),
				Token::unit(Ambiguity(Ambiguity {
					candidates: &[Pound, GBP],
					string: "pound",
					fallback: &Pound,
				})),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"210 pounds",
			vec![
				numtok!(210),
				Token::unit(Ambiguity(Ambiguity {
					candidates: &[Pound, GBP],
					string: "pound",
					fallback: &Pound,
				})),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"210 pounds-force",
			vec![numtok!(210), Token::unit(PoundForce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 ton",
			vec![numtok!(3), Token::unit(ShortTon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 short tons",
			vec![numtok!(3), Token::unit(ShortTon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 lt",
			vec![numtok!(4), Token::unit(LongTon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 long tonnes",
			vec![numtok!(4), Token::unit(LongTon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_datarate_lex("1 bit", vec![numtok!(1), Token::unit(Bit)]);
		run_datarate_lex("8 bits", vec![numtok!(8), Token::unit(Bit)]);
		run_datarate_lex("63 kilobits", vec![numtok!(63), Token::unit(Kilobit)]);
		run_datarate_lex("32 megabits", vec![numtok!(32), Token::unit(Megabit)]);
		run_datarate_lex("3.5 gigabits", vec![numtok!(3.5), Token::unit(Gigabit)]);
		run_datarate_lex("2.1 terabits", vec![numtok!(2.1), Token::unit(Terabit)]);
		run_datarate_lex("1.08 petabits", vec![numtok!(1.08), Token::unit(Petabit)]);
		run_datarate_lex("0.73 exabits", vec![numtok!(0.73), Token::unit(Exabit)]);
		run_datarate_lex("0.49 zettabits", vec![numtok!(0.49), Token::unit(Zettabit)]);
		run_datarate_lex("0.23 yottabits", vec![numtok!(0.23), Token::unit(Yottabit)]);
		run_datarate_lex("63 kibibits", vec![numtok!(63), Token::unit(Kibibit)]);
		run_datarate_lex("32 mebibits", vec![numtok!(32), Token::unit(Mebibit)]);
		run_datarate_lex("3.5 gibibits", vec![numtok!(3.5), Token::unit(Gibibit)]);
		run_datarate_lex("2.1 tebibits", vec![numtok!(2.1), Token::unit(Tebibit)]);
		run_datarate_lex("1.08 pebibits", vec![numtok!(1.08), Token::unit(Pebibit)]);
		run_datarate_lex("0.73 exbibits", vec![numtok!(0.73), Token::unit(Exbibit)]);
		run_datarate_lex("0.49 zebibits", vec![numtok!(0.49), Token::unit(Zebibit)]);
		run_datarate_lex("0.23 yobibits", vec![numtok!(0.23), Token::unit(Yobibit)]);
		run_datarate_lex("1 byte", vec![numtok!(1), Token::unit(Byte)]);
		run_datarate_lex("3 bytes", vec![numtok!(3), Token::unit(Byte)]);
		run_datarate_lex("63 kilobytes", vec![numtok!(63), Token::unit(Kilobyte)]);
		run_datarate_lex("32 megabytes", vec![numtok!(32), Token::unit(Megabyte)]);
		run_datarate_lex("3.5 gigabytes", vec![numtok!(3.5), Token::unit(Gigabyte)]);
		run_datarate_lex("2.1 terabytes", vec![numtok!(2.1), Token::unit(Terabyte)]);
		run_datarate_lex("1.08 petabytes", vec![numtok!(1.08), Token::unit(Petabyte)]);
		run_datarate_lex("0.73 exabytes", vec![numtok!(0.73), Token::unit(Exabyte)]);
		run_datarate_lex(
			"0.49 zettabytes",
			vec![numtok!(0.49), Token::unit(Zettabyte)],
		);
		run_datarate_lex(
			"0.23 yottabytes",
			vec![numtok!(0.23), Token::unit(Yottabyte)],
		);
		run_datarate_lex("63 kibibytes", vec![numtok!(63), Token::unit(Kibibyte)]);
		run_datarate_lex("32 mebibytes", vec![numtok!(32), Token::unit(Mebibyte)]);
		run_datarate_lex("3.5 gibibytes", vec![numtok!(3.5), Token::unit(Gibibyte)]);
		run_datarate_lex("2.1 tebibytes", vec![numtok!(2.1), Token::unit(Tebibyte)]);
		run_datarate_lex("1.08 pebibytes", vec![numtok!(1.08), Token::unit(Pebibyte)]);
		run_datarate_lex("0.73 exbibytes", vec![numtok!(0.73), Token::unit(Exbibyte)]);
		run_datarate_lex("0.49 zebibytes", vec![numtok!(0.49), Token::unit(Zebibyte)]);
		run_datarate_lex("0.23 yobibytes", vec![numtok!(0.23), Token::unit(Yobibyte)]);
		run_lex(
			"432 Bps",
			vec![numtok!(432), Token::unit(BytesPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"56 kBps",
			vec![numtok!(56), Token::unit(KilobytesPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"432 bps",
			vec![numtok!(432), Token::unit(BitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"56 kbps",
			vec![numtok!(56), Token::unit(KilobitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"12 mbps",
			vec![numtok!(12), Token::unit(MegabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4.2 gbps",
			vec![numtok!(4.2), Token::unit(GigabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2.2 tbps",
			vec![numtok!(2.2), Token::unit(TerabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1.7 pbps",
			vec![numtok!(1.7), Token::unit(PetabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0.99 ebps",
			vec![numtok!(0.99), Token::unit(ExabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0.64 zbps",
			vec![numtok!(0.64), Token::unit(ZettabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0.278 ybps",
			vec![numtok!(0.278), Token::unit(YottabitsPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_datarate_lex(
			"4 bits per second",
			vec![
				numtok!(4),
				Token::unit(Bit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"5 kilobits per second",
			vec![
				numtok!(5),
				Token::unit(Kilobit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"6 megabits per second",
			vec![
				numtok!(6),
				Token::unit(Megabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"7 gigabits per second",
			vec![
				numtok!(7),
				Token::unit(Gigabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"8 terabits per second",
			vec![
				numtok!(8),
				Token::unit(Terabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"9 petabits per second",
			vec![
				numtok!(9),
				Token::unit(Petabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"10 exabits per second",
			vec![
				numtok!(10),
				Token::unit(Exabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"11 zettabits per second",
			vec![
				numtok!(11),
				Token::unit(Zettabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"12 yottabits per second",
			vec![
				numtok!(12),
				Token::unit(Yottabit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"13 kibibits per second",
			vec![
				numtok!(13),
				Token::unit(Kibibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"14 mebibits per second",
			vec![
				numtok!(14),
				Token::unit(Mebibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"15 gibibits per second",
			vec![
				numtok!(15),
				Token::unit(Gibibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"16 tebibits per second",
			vec![
				numtok!(16),
				Token::unit(Tebibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"17 pebibits per second",
			vec![
				numtok!(17),
				Token::unit(Pebibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"18 exbibits per second",
			vec![
				numtok!(18),
				Token::unit(Exbibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"19 zebibits per second",
			vec![
				numtok!(19),
				Token::unit(Zebibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"20 yobibits per second",
			vec![
				numtok!(20),
				Token::unit(Yobibit),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"4 bytes per second",
			vec![
				numtok!(4),
				Token::unit(Byte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"5 kilobytes per second",
			vec![
				numtok!(5),
				Token::unit(Kilobyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"6 megabytes per second",
			vec![
				numtok!(6),
				Token::unit(Megabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"7 gigabytes per second",
			vec![
				numtok!(7),
				Token::unit(Gigabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"8 terabytes per second",
			vec![
				numtok!(8),
				Token::unit(Terabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"9 petabytes per second",
			vec![
				numtok!(9),
				Token::unit(Petabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"10 exabytes per second",
			vec![
				numtok!(10),
				Token::unit(Exabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"11 zettabytes per second",
			vec![
				numtok!(11),
				Token::unit(Zettabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"12 yottabytes per second",
			vec![
				numtok!(12),
				Token::unit(Yottabyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"13 kibibytes per second",
			vec![
				numtok!(13),
				Token::unit(Kibibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"14 mebibytes per second",
			vec![
				numtok!(14),
				Token::unit(Mebibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"15 gibibytes per second",
			vec![
				numtok!(15),
				Token::unit(Gibibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"16 tebibytes per second",
			vec![
				numtok!(16),
				Token::unit(Tebibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"17 pebibytes per second",
			vec![
				numtok!(17),
				Token::unit(Pebibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"18 exbibytes per second",
			vec![
				numtok!(18),
				Token::unit(Exbibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"19 zebibytes per second",
			vec![
				numtok!(19),
				Token::unit(Zebibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_datarate_lex(
			"20 yobibytes per second",
			vec![
				numtok!(20),
				Token::unit(Yobibyte),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
		);
		run_lex(
			"1 flop",
			vec![numtok!(1), Token::unit(Flop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kflop",
			vec![numtok!(2), Token::unit(KiloFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 mflop",
			vec![numtok!(3), Token::unit(MegaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gflop",
			vec![numtok!(4), Token::unit(GigaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"5 tflop",
			vec![numtok!(5), Token::unit(TeraFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 pflop",
			vec![numtok!(6), Token::unit(PetaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"7 eflop",
			vec![numtok!(7), Token::unit(ExaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"8 zflop",
			vec![numtok!(8), Token::unit(ZettaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"9 yflop",
			vec![numtok!(9), Token::unit(YottaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"10 rflop",
			vec![numtok!(10), Token::unit(RonnaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"11 qflop",
			vec![numtok!(11), Token::unit(QuettaFlop)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 flop/s",
			vec![
				numtok!(1),
				Token::unit(Flop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kflop/s",
			vec![
				numtok!(2),
				Token::unit(KiloFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 mflop/s",
			vec![
				numtok!(3),
				Token::unit(MegaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gflop/s",
			vec![
				numtok!(4),
				Token::unit(GigaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"5 tflop/s",
			vec![
				numtok!(5),
				Token::unit(TeraFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 pflop/s",
			vec![
				numtok!(6),
				Token::unit(PetaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"7 eflop/s",
			vec![
				numtok!(7),
				Token::unit(ExaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"8 zflop/s",
			vec![
				numtok!(8),
				Token::unit(ZettaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"9 yflop/s",
			vec![
				numtok!(9),
				Token::unit(YottaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"10 rflop/s",
			vec![
				numtok!(10),
				Token::unit(RonnaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"11 qflop/s",
			vec![
				numtok!(11),
				Token::unit(QuettaFlop),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 flop per second",
			vec![
				numtok!(1),
				Token::unit(Flop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kflop per second",
			vec![
				numtok!(2),
				Token::unit(KiloFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 mflop per second",
			vec![
				numtok!(3),
				Token::unit(MegaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gflop per second",
			vec![
				numtok!(4),
				Token::unit(GigaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"5 tflop per second",
			vec![
				numtok!(5),
				Token::unit(TeraFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 pflop per second",
			vec![
				numtok!(6),
				Token::unit(PetaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"7 eflop per second",
			vec![
				numtok!(7),
				Token::unit(ExaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"8 zflop per second",
			vec![
				numtok!(8),
				Token::unit(ZettaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"9 yflop per second",
			vec![
				numtok!(9),
				Token::unit(YottaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"10 rflop per second",
			vec![
				numtok!(10),
				Token::unit(RonnaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"11 qflop per second",
			vec![
				numtok!(11),
				Token::unit(QuettaFlop),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 flops",
			vec![numtok!(1), Token::unit(FlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kflops",
			vec![numtok!(2), Token::unit(KiloFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3 mflops",
			vec![numtok!(3), Token::unit(MegaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gflops",
			vec![numtok!(4), Token::unit(GigaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"5 tflops",
			vec![numtok!(5), Token::unit(TeraFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 pflops",
			vec![numtok!(6), Token::unit(PetaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"7 eflops",
			vec![numtok!(7), Token::unit(ExaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"8 zflops",
			vec![numtok!(8), Token::unit(ZettaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"9 yflops",
			vec![numtok!(9), Token::unit(YottaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"10 rflops",
			vec![numtok!(10), Token::unit(RonnaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"11 qflops",
			vec![numtok!(11), Token::unit(QuettaFlopPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"234 wh",
			vec![numtok!(234), Token::unit(WattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 w",
			vec![numtok!(1), Token::unit(Watt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 watt",
			vec![numtok!(1), Token::unit(Watt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 watts",
			vec![numtok!(1), Token::unit(Watt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 watt hour",
			vec![numtok!(1), Token::unit(WattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0 watt + 1 watts",
			vec![
				numtok!(0),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(1),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0 watt * 1",
			vec![
				numtok!(0),
				Token::unit(Watt),
				Token::Operator(Multiply),
				numtok!(1),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 watts + 3 watts",
			vec![
				numtok!(2),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 watts * 3",
			vec![
				numtok!(2),
				Token::unit(Watt),
				Token::Operator(Multiply),
				numtok!(3),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 watt plus 5 watts",
			vec![
				numtok!(4),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 watt times 5",
			vec![
				numtok!(4),
				Token::unit(Watt),
				Token::Operator(Multiply),
				numtok!(5),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 watts plus 7 watts",
			vec![
				numtok!(6),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 watts times 7",
			vec![
				numtok!(6),
				Token::unit(Watt),
				Token::Operator(Multiply),
				numtok!(7),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2.3 kwh",
			vec![numtok!(2.3), Token::unit(KilowattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 kw",
			vec![numtok!(1), Token::unit(Kilowatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 kilowatt",
			vec![numtok!(1), Token::unit(Kilowatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 kilowatts",
			vec![numtok!(1), Token::unit(Kilowatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 kilowatt hour",
			vec![numtok!(1), Token::unit(KilowattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatt + 3 watt",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatt * 4",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Multiply),
				numtok!(4),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatt times 4",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Multiply),
				numtok!(4),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatt + 3 watts",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatts + 3 watt",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatts + 3 watts",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatt plus 3 watt",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatt plus 3 watts",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatts plus 3 watt",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 kilowatts plus 3 watts",
			vec![
				numtok!(2),
				Token::unit(Kilowatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6.6 watts + 4 kilowatts",
			vec![
				numtok!(6.6),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(4),
				Token::unit(Kilowatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6.6 watts plus 4 kilowatts",
			vec![
				numtok!(6.6),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(4),
				Token::unit(Kilowatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2.3 mwh",
			vec![numtok!(2.3), Token::unit(MegawattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 mw",
			vec![numtok!(1), Token::unit(Megawatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 megawatt",
			vec![numtok!(1), Token::unit(Megawatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 megawatt hour",
			vec![numtok!(1), Token::unit(MegawattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatt + 3 watt",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatt * 6",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Multiply),
				numtok!(6),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatt times 6",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Multiply),
				numtok!(6),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatt + 3 watts",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatts + 3 watt",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatts + 3 watts",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatt plus 3 watt",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatt plus 3 watts",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatts plus 3 watt",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 megawatts plus 3 watts",
			vec![
				numtok!(2),
				Token::unit(Megawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6.6 watts + 4 megawatts",
			vec![
				numtok!(6.6),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(4),
				Token::unit(Megawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6.6 watts plus 4 megawatts",
			vec![
				numtok!(6.6),
				Token::unit(Watt),
				Token::Operator(Plus),
				numtok!(4),
				Token::unit(Megawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"234 gwh",
			vec![numtok!(234), Token::unit(GigawattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 gw",
			vec![numtok!(1), Token::unit(Gigawatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 gigawatt",
			vec![numtok!(1), Token::unit(Gigawatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 gigawatts",
			vec![numtok!(1), Token::unit(Gigawatt)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 gigawatt hour",
			vec![numtok!(1), Token::unit(GigawattHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0 gigawatt + 1 gigawatts",
			vec![
				numtok!(0),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(1),
				Token::unit(Gigawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"0 gigawatt * 1",
			vec![
				numtok!(0),
				Token::unit(Gigawatt),
				Token::Operator(Multiply),
				numtok!(1),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 gigawatts + 3 gigawatts",
			vec![
				numtok!(2),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(3),
				Token::unit(Gigawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2 gigawatts * 3",
			vec![
				numtok!(2),
				Token::unit(Gigawatt),
				Token::Operator(Multiply),
				numtok!(3),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt plus 5 watt",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt plus 5 megawatt",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Megawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt plus 5 gigawatt",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Gigawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt plus 5 watts",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt plus 5 megawatts",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Megawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt plus 5 gigawatts",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(5),
				Token::unit(Gigawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 gigawatt times 5",
			vec![
				numtok!(4),
				Token::unit(Gigawatt),
				Token::Operator(Multiply),
				numtok!(5),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts plus 7 watt",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts plus 7 megawatt",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Megawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts plus 7 gigawatt",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Gigawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts plus 7 watts",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Watt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts plus 7 megawatts",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Megawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts plus 7 gigawatts",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Plus),
				numtok!(7),
				Token::unit(Gigawatt),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 gigawatts times 7",
			vec![
				numtok!(6),
				Token::unit(Gigawatt),
				Token::Operator(Multiply),
				numtok!(7),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"88 mw * 3",
			vec![
				numtok!(88),
				Token::unit(Megawatt),
				Token::Operator(Multiply),
				numtok!(3),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"88 mw times 3",
			vec![
				numtok!(88),
				Token::unit(Megawatt),
				Token::Operator(Multiply),
				numtok!(3),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"999 kb",
			vec![numtok!(999), Token::unit(Kilobyte)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"200 gb - 100 mb",
			vec![
				numtok!(200),
				Token::unit(Gigabyte),
				Token::Operator(Minus),
				numtok!(100),
				Token::unit(Megabyte),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"999 kib",
			vec![numtok!(999), Token::unit(Kibibyte)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"200 gib - 100 mib",
			vec![
				numtok!(200),
				Token::unit(Gibibyte),
				Token::Operator(Minus),
				numtok!(100),
				Token::unit(Mebibyte),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"45 btu",
			vec![numtok!(45), Token::unit(BritishThermalUnit)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"45.5 british thermal unit",
			vec![numtok!(45.5), Token::unit(BritishThermalUnit)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"46 british thermal units",
			vec![numtok!(46), Token::unit(BritishThermalUnit)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"5432 newton metres",
			vec![numtok!(5432), Token::unit(NewtonMeter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"2345 newton-meters",
			vec![numtok!(2345), Token::unit(NewtonMeter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"20 lbf",
			vec![numtok!(20), Token::unit(PoundForce)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"60 hz",
			vec![numtok!(60), Token::unit(Hertz)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1100 rpm",
			vec![numtok!(1100), Token::unit(RevolutionsPerMinute)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1150 revolutions per minute",
			vec![
				numtok!(1150),
				Token::unit(Revolution),
				Token::TextOperator(Per),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1 revolution per min",
			vec![
				numtok!(1),
				Token::unit(Revolution),
				Token::TextOperator(Per),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"4 revolution / mins",
			vec![
				numtok!(4),
				Token::unit(Revolution),
				Token::Operator(Divide),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1250 r / min",
			vec![
				numtok!(1250),
				Token::unit(Revolution),
				Token::Operator(Divide),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1300 rev / min",
			vec![
				numtok!(1300),
				Token::unit(Revolution),
				Token::Operator(Divide),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1350 rev / minute",
			vec![
				numtok!(1350),
				Token::unit(Revolution),
				Token::Operator(Divide),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1250 r per min",
			vec![
				numtok!(1250),
				Token::unit(Revolution),
				Token::TextOperator(Per),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1300 rev per min",
			vec![
				numtok!(1300),
				Token::unit(Revolution),
				Token::TextOperator(Per),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"1350 rev per minute",
			vec![
				numtok!(1350),
				Token::unit(Revolution),
				Token::TextOperator(Per),
				Token::unit(Minute),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"100 kph",
			vec![numtok!(100), Token::unit(KilometersPerHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"100 kmh",
			vec![numtok!(100), Token::unit(KilometersPerHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"100 kilometers per hour",
			vec![
				numtok!(100),
				Token::unit(Kilometer),
				Token::TextOperator(Per),
				Token::unit(Hour),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"100 kilometre / hrs",
			vec![
				numtok!(100),
				Token::unit(Kilometer),
				Token::Operator(Divide),
				Token::unit(Hour),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3.6 mps",
			vec![numtok!(3.6), Token::unit(MetersPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3.6 meters per second",
			vec![
				numtok!(3.6),
				Token::unit(Meter),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"3.6 metre / secs",
			vec![
				numtok!(3.6),
				Token::unit(Meter),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"60 mph",
			vec![numtok!(60), Token::unit(MilesPerHour)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"60 miles per hour",
			vec![
				numtok!(60),
				Token::unit(Mile),
				Token::TextOperator(Per),
				Token::unit(Hour),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"60 mile / hr",
			vec![
				numtok!(60),
				Token::unit(Mile),
				Token::Operator(Divide),
				Token::unit(Hour),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 fps",
			vec![numtok!(35), Token::unit(FeetPerSecond)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 ft / sec",
			vec![
				numtok!(35),
				Token::unit(Foot),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 ft per seconds",
			vec![
				numtok!(35),
				Token::unit(Foot),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 foot / secs",
			vec![
				numtok!(35),
				Token::unit(Foot),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 foot per seconds",
			vec![
				numtok!(35),
				Token::unit(Foot),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 feet / sec",
			vec![
				numtok!(35),
				Token::unit(Foot),
				Token::Operator(Divide),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"35 feet per second",
			vec![
				numtok!(35),
				Token::unit(Foot),
				Token::TextOperator(Per),
				Token::unit(Second),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"30 pa",
			vec![numtok!(30), Token::unit(Pascal)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"23 celsius + 4 celsius",
			vec![
				numtok!(23),
				Token::unit(Celsius),
				Token::Operator(Plus),
				numtok!(4),
				Token::unit(Celsius),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"54 f - 1.5 fahrenheit",
			vec![
				numtok!(54),
				Token::unit(Fahrenheit),
				Token::Operator(Minus),
				numtok!(1.5),
				Token::unit(Fahrenheit),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"50 metric tonnes",
			vec![numtok!(50), Token::unit(MetricTon)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"77 metric hps",
			vec![numtok!(77), Token::unit(MetricHorsepower)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);

		run_lex(
			"100 + 99",
			vec![numtok!(100), Token::Operator(Plus), numtok!(99)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"100 plus 99",
			vec![numtok!(100), Token::Operator(Plus), numtok!(99)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"12 - 4",
			vec![numtok!(12), Token::Operator(Minus), numtok!(4)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"12 minus 4",
			vec![numtok!(12), Token::Operator(Minus), numtok!(4)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"50.5 * 2",
			vec![numtok!(50.5), Token::Operator(Multiply), numtok!(2)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"50.5 times 2",
			vec![numtok!(50.5), Token::Operator(Multiply), numtok!(2)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"50.5 multiplied by 2",
			vec![numtok!(50.5), Token::Operator(Multiply), numtok!(2)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 / 3",
			vec![numtok!(6), Token::Operator(Divide), numtok!(3)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"50 / 10",
			vec![numtok!(50), Token::Operator(Divide), numtok!(10)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"52 ÷ 12",
			vec![numtok!(52), Token::Operator(Divide), numtok!(12)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"6 divided by 3",
			vec![numtok!(6), Token::Operator(Divide), numtok!(3)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"7 mod 5",
			vec![numtok!(7), Token::Operator(Modulo), numtok!(5)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);

		run_lex(
			"(2 + 3) * 4",
			vec![
				Token::Operator(LeftParen),
				numtok!(2),
				Token::Operator(Plus),
				numtok!(3),
				Token::Operator(RightParen),
				Token::Operator(Multiply),
				numtok!(4),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"52 weeks * (12 hrs + 12 hours)",
			vec![
				numtok!(52),
				Token::unit(Week),
				Token::Operator(Multiply),
				Token::Operator(LeftParen),
				numtok!(12),
				Token::unit(Hour),
				Token::Operator(Plus),
				numtok!(12),
				Token::unit(Hour),
				Token::Operator(RightParen),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"12 cm+",
			vec![numtok!(12), Token::unit(Centimeter), Token::Operator(Plus)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);

		run_lex(
			"5 π m",
			vec![numtok!(5), Token::Constant(Pi), Token::unit(Meter)],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
		run_lex(
			"5 Ω + 2 mΩ",
			vec![
				numtok!(5),
				Token::unit(Ohm),
				Token::Operator(Plus),
				numtok!(2),
				Token::unit(Milliohm),
			],
			&strip_operator_spacing,
			&strip_afterdigit_spacing,
		);
	}
}
