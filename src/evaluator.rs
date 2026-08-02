use crate::Constant::*;
use crate::FunctionIdentifier::*;
use crate::Operator::*;
use crate::TextOperator::*;
use crate::UnaryOperator::*;
use crate::lookup::{lookup_factorial, lookup_named_number};
use crate::parser::AstNode;
use crate::units::Unit;
use crate::units::UnitType;
use crate::units::multiply_any;
use crate::units::to_ideal_unit;
use crate::units::{add, convert, divide, modulo, multiply, pow, subtract};
use crate::{Number, Token};
use fastnum::decimal::Context;
use fastnum::{D128, dec128 as d, decimal::RoundingMode};

/// Evaluate an [`AstNode`] into a [`Number`]
pub fn evaluate(ast: &mut AstNode) -> Result<Number, String> {
	resolve_ambiguities(ast, None);
	let answer = evaluate_node(&ast)?;
	Ok(answer)
}

/// Returns the factorial of a [`struct@d128`] up to `1000!` without doing any math
///
/// Factorials do not work with decimal numbers.
///
/// All return values of this function are hard-coded.
pub fn factorial(input: D128) -> D128 {
	lookup_factorial(input.try_into().unwrap())
}

/// Returns the sine of a [`struct@d128`]
pub fn sin(input: D128) -> D128 {
	let result = input.sin();
	// D128::PI is a finite-precision approximation of pi, so sin(pi) lands a few
	// ulp away from zero rather than exactly zero. Snap sub-precision residue to 0.
	if result.abs() < d!(1E-30) {
		D128::ZERO
	} else {
		result
	}
}

/// Returns the cosine of a [`struct@d128`]
pub fn cos(input: D128) -> D128 {
	input.cos()
}

/// Returns the tangent of a [`struct@d128`]
pub fn tan(input: D128) -> D128 {
	input.tan()
}

/// Turn `old` into `new` without updating the signal
fn replace_without_updating_signals(old: D128, new: D128) -> D128 {
	let new_without_signal = D128::parse_str(&new.to_string(), Context::default());
	old - old + new_without_signal
}

fn find_concrete_category(ast: &AstNode) -> Option<UnitType> {
	if let Token::Unit(units) = &ast.token {
		if let Some((u, _)) = units.iter().find(|(u, _)| !matches!(u, Unit::Ambiguity(_))) {
			return Some(u.category());
		}
	}
	ast.children.iter().find_map(find_concrete_category)
}

fn resolve_ambiguities(ast: &mut AstNode, hint: Option<UnitType>) {
	let mut child_hints = vec![hint; ast.children.len()];

	if let Token::TextOperator(To) = &ast.token {
		if ast.children.len() == 2 {
			let left_cat = find_concrete_category(&ast.children[0]);
			let right_cat = find_concrete_category(&ast.children[1]);
			child_hints[0] = right_cat.or(hint); // left side hinted by right
			child_hints[1] = left_cat.or(hint); // right side hinted by left
		}
	}

	for (child, h) in ast.children.iter_mut().zip(child_hints) {
		resolve_ambiguities(child, h);
	}

	if let Token::Unit(units) = &mut ast.token {
		for (unit, _) in units.iter_mut() {
			if let Unit::Ambiguity(amb) = unit {
				*unit = hint
					.and_then(|cat| amb.candidates.iter().find(|c| c.category() == cat).copied())
					.unwrap_or(*amb.fallback);
			}
		}
	}
}

fn evaluate_unit(ast: &AstNode) -> Result<Vec<(Unit, isize)>, String> {
	match &ast.token {
		Token::Unit(unit) => Ok(unit.to_vec()),
		Token::Operator(Divide) | Token::TextOperator(Per) => {
			let left = evaluate_unit(&ast.children[0])?;
			let mut right = evaluate_unit(&ast.children[1])?;
			for (_, exponent) in right.iter_mut() {
				*exponent = -*exponent;
			}
			Ok([left.as_slice(), right.as_slice()].concat())
		}
		Token::Operator(Multiply) => {
			let left = evaluate_unit(&ast.children[0])?;
			let right = evaluate_unit(&ast.children[1])?;
			Ok([left.as_slice(), right.as_slice()].concat())
		}
		_ => Err("Expected unit expression".to_string()),
	}
}

/// Evaluate an [`AstNode`] into a [`Number`]
fn evaluate_node(ast_node: &AstNode) -> Result<Number, String> {
	let token = &ast_node.token;
	let children = &ast_node.children;
	match token {
		Token::Number(number) => Ok(Number::new_unitless(*number)),
		Token::Constant(constant) => match constant {
			Pi => Ok(Number::new_unitless(D128::PI)),
			E => Ok(Number::new_unitless(D128::E)),
		},
		Token::FunctionIdentifier(function) => {
			let child_node = children.get(0).ok_or("Function has no child[0]")?;
			let child_answer = evaluate_node(child_node)?;
			match function {
				Sqrt => {
					if child_answer.is_unitless() {
						let mut result = child_answer.value.sqrt();
						let result_with_old_signals =
							replace_without_updating_signals(child_answer.value, result);
						let result_squared = result_with_old_signals * result_with_old_signals;
						// if result^2 is exact and equals the original, we avoid the OP_INEXACT signal
						if !result_squared.is_op_inexact() && result_squared == child_answer.value {
							result = replace_without_updating_signals(child_answer.value, result);
						}
						Ok(Number::with_unit(result, child_answer.unit))
					} else {
						Err("sqrt() only accepts unitless numbers".to_string())
					}
				}
				Cbrt => {
					if child_answer.is_unitless() {
						let mut result = child_answer.value.cbrt();
						let result_with_old_signals =
							replace_without_updating_signals(child_answer.value, result);
						let result_squared = result_with_old_signals
							* result_with_old_signals
							* result_with_old_signals;
						// if result^2 is exact and equals the original, we avoid the OP_INEXACT signal
						if !result_squared.is_op_inexact() && result_squared == child_answer.value {
							result = replace_without_updating_signals(child_answer.value, result);
						}
						Ok(Number::with_unit(result, child_answer.unit))
					} else {
						Err("cbrt() only accepts unitless numbers".to_string())
					}
				}
				Log => {
					if child_answer.is_unitless() {
						let result = child_answer.value.log10();
						Ok(Number::with_unit(result, child_answer.unit))
					} else {
						Err("log() only accepts unitless numbers".to_string())
					}
				}
				Ln => {
					if child_answer.is_unitless() {
						let result = child_answer.value.ln();
						Ok(Number::with_unit(result, child_answer.unit))
					} else {
						Err("ln() only accepts unitless numbers".to_string())
					}
				}
				Exp => {
					if child_answer.is_unitless() {
						let result = child_answer.value.exp();
						Ok(Number::with_unit(result, child_answer.unit))
					} else {
						Err("exp() only accepts unitless numbers".to_string())
					}
				}
				Round => {
					// Round half away from zero (HalfUp) so round(2.5) == 3, not 2.
					let result = child_answer
						.value
						.with_rounding_mode(RoundingMode::HalfUp)
						.round(0);
					let result = replace_without_updating_signals(child_answer.value, result);
					Ok(Number::with_unit(result, child_answer.unit))
				}
				Ceil => {
					let result = child_answer.value.ceil();
					let result = replace_without_updating_signals(child_answer.value, result);
					Ok(Number::with_unit(result, child_answer.unit))
				}
				Floor => {
					let result = child_answer.value.floor();
					let result = replace_without_updating_signals(child_answer.value, result);
					Ok(Number::with_unit(result, child_answer.unit))
				}
				Abs => {
					let result = child_answer.value.abs();
					Ok(Number::with_unit(result, child_answer.unit))
				}
				Sin => {
					let result = sin(child_answer.value);
					Ok(Number::with_unit(result, child_answer.unit))
				}
				Cos => {
					let result = cos(child_answer.value);
					Ok(Number::with_unit(result, child_answer.unit))
				}
				Tan => {
					let result = tan(child_answer.value);
					Ok(Number::with_unit(result, child_answer.unit))
				}
			}
		}
		Token::Unit(unit) => {
			let child_answer = match children.get(0) {
				Some(node) => evaluate_node(node)?,
				None => Number::new_unitless(d!(1)),
			};
			Ok(Number::with_unit(child_answer.value, unit.clone()))
		}
		Token::Negative => {
			let child_node = children.get(0).ok_or("Negative has no child[0]")?;
			let child_answer = evaluate_node(child_node)?;
			Ok(Number::with_unit(-child_answer.value, child_answer.unit))
		}
		Token::Paren => {
			let child_node = children.get(0).ok_or("Paren has no child[0]")?;
			evaluate_node(child_node)
		}
		Token::UnaryOperator(operator) => {
			let child_node = children
				.get(0)
				.ok_or(format!("Token {:?} has no child[0]", token))?;
			let child_answer = evaluate_node(child_node)?;
			match operator {
				Percent => Ok(Number::with_unit(
					child_answer.value / d!(100),
					child_answer.unit,
				)),
				Factorial => {
					let result = factorial(child_answer.value);
					if result.is_nan() {
						return Err(
							"Can only perform factorial on integers from 0 to 1000".to_string()
						);
					}
					Ok(Number::with_unit(result, child_answer.unit))
				}
			}
		}
		Token::NamedNumber(named_number) => {
			let child_node = children
				.get(0)
				.ok_or(format!("Token {:?} has no child[0]", token))?;
			let named_number_value = lookup_named_number(named_number);
			if let Token::NamedNumber(child_nn) = &child_node.token {
				let child_nn_value = lookup_named_number(child_nn);
				if child_nn_value > named_number_value {
					return Err(format!("Unexpected smaller token {:?}", token));
				}
			}
			let child_answer = evaluate_node(child_node)?;
			let result = child_answer.value * named_number_value;
			Ok(Number::with_unit(result, child_answer.unit))
		}
		Token::TextOperator(operator) => {
			let left_child = children
				.get(0)
				.ok_or(format!("Token {:?} has no child[0]", token))?;
			let right_child = children
				.get(1)
				.ok_or(format!("Token {:?} has no child[1]", token))?;

			match operator {
				To => {
					let left = evaluate_node(left_child)?;
					let right = evaluate_unit(right_child)
						.map_err(|_| "Right side of To operator needs to be a unit".to_string())?;
					let result = convert(left, right)?;
					Ok(result)
				}
				Of => {
					let left = evaluate_node(left_child)?;
					let right = evaluate_node(right_child)?;
					if left.is_unitless() {
						Ok(Number::with_unit(left.value * right.value, right.unit))
					} else {
						Err("Left side of the Of operator must be NoUnit".to_string())
					}
				}
				Per => {
					let mut node = AstNode::new(Token::Operator(Divide));
					node.children = children.to_vec();
					Ok(evaluate_node(&node)?)
				}
			}
		}
		Token::Operator(operator) => {
			let left_child = children
				.get(0)
				.ok_or(format!("Token {:?} has no child[0]", token))?;
			let right_child = children
				.get(1)
				.ok_or(format!("Token {:?} has no child[1]", token))?;
			let left = evaluate_node(left_child)?;
			if matches!(operator, Multiply | Divide)
				&& let Ok(right) = evaluate_unit(right_child)
			{
				// multiply/divide by unit, for example `2km`, `2ft/s`, `2*m`
				let raw_result = match operator {
					Multiply => multiply_any(left, Number::with_unit(d!(1), right))?,
					Divide => divide(left, Number::with_unit(d!(1), right))?,
					_ => panic!(),
				};
				let ideal_result = to_ideal_unit(raw_result.clone());
				// If the ideal unit has a different category/exponent, then we use it.
				// For example, we want to keep `v*a` -> `watt`, but not `ft/s` -> `cm/s`.
				if raw_result.unit.len() != ideal_result.unit.len() {
					return Ok(ideal_result);
				}
				for (ideal, raw) in ideal_result.unit.iter().zip(raw_result.unit.iter()) {
					if ideal.0.category() != raw.0.category() || ideal.1 != raw.1 {
						return Ok(ideal_result);
					}
				}
				return Ok(raw_result);
			}

			let right = evaluate_node(right_child)?;
			match operator {
				Plus => Ok(add(left, right)?),
				Minus => Ok(subtract(left, right)?),
				Multiply => Ok(to_ideal_unit(multiply(left, right)?)),
				Divide => Ok(to_ideal_unit(divide(left, right)?)),
				Modulo => Ok(modulo(left, right)?),
				Caret => Ok(pow(left, right)?),
				_ => Err(format!("Unexpected operator {:?}", operator)),
			}
		}
		_ => Err(format!("Unexpected token {:?}", token)),
	}
}

#[cfg(test)]
mod tests {
	use crate::{Settings, eval};
	use serial_test::serial;
	use std::str::FromStr;

	#[track_caller]
	fn eval_test(input: &str, expected: &str) {
		let result = eval(input, true, false).unwrap();
		assert_eq!(result.to_string(), expected);
	}

	#[track_caller]
	fn results_eq(a: &str, b: &str) {
		let result_a = crate::eval(a, true, false).unwrap();
		let result_b = crate::eval(b, true, false).unwrap();
		assert_eq!(result_a, result_b, "{a} != {b}");
		assert_eq!(
			result_a.value.op_signals(),
			result_b.value.op_signals(),
			"{a} != {b}",
		);
	}

	#[test]
	fn test_evaluations() {
		eval_test("-2(-3)", "6");
		eval_test("-2(3)", "-6");
		eval_test("(3)-2", "1");
		eval_test("-1km to m", "-1 000 meters");
		eval_test("2*-3*0.5", "-3");
		eval_test("-3^2", "-9");
		eval_test("e^2", "≈ 7.3890560989306502272304274605750078132");
		eval_test("e^2.5", "≈ 12.1824939607034734380701759511679661832");
		eval_test("-1+2", "1");
	}

	#[test]
	fn test_functions() {
		eval_test("cbrt(125)", "5");
		eval_test("cbrt(2)", "≈ 1.25992104989487316476721060727822835057");

		eval_test("sqrt(25)", "5");
		eval_test("sqrt(2)", "≈ 1.41421356237309504880168872420969807857");

		eval_test("log(100)", "2");
		eval_test("log(2)", "≈ 0.301029995663981195213738894724493026768");

		eval_test("ln(1)", "0");
		eval_test("ln(2)", "≈ 0.69314718055994530941723212145817656808");
		eval_test("ln(e)", "≈ 1");
		eval_test("ln(e^2)", "≈ 2");

		eval_test("exp(1)", "≈ 2.71828182845904523536028747135266249776");

		eval_test("round(1.4)", "1");
		eval_test("round(1.6)", "2");
		eval_test("round(1.5)", "2");
		eval_test("round(2.5)", "3");

		eval_test("ceil(1.5)", "2");
		eval_test("ceil(-1.5)", "-1");

		eval_test("floor(1.5)", "1");
		eval_test("floor(-1.5)", "-2");

		eval_test("abs(-3)", "3");

		eval_test("sin(2)", "≈ 0.9092974268256816953960198659117448427");
		eval_test("sin(-2)", "≈ -0.9092974268256816953960198659117448427");

		eval_test("cos(2)", "≈ -0.41614683654714238699756822950076218977");
		eval_test("cos(-2)", "≈ -0.41614683654714238699756822950076218977");
		eval_test("tan(2)", "≈ -2.18503986326151899164330610231368254343");
	}

	#[test]
	fn test_currency() {
		use crate::currency::{CurrencyRate, set_currency_cache};
		use serde_json::Number;

		set_currency_cache(vec![CurrencyRate {
			date: "2000-01-01".to_string(),
			base: "EUR".to_string(),
			quote: "NOK".to_string(),
			rate: Number::from_str("11.2839").unwrap(),
		}])
		.unwrap();

		eval_test("1 EUR to NOK", "11.2839 NOK");
		eval_test("11.2839 NOK to EUR", "≈ 0.99999952902 EUR");
		eval_test("1 NOK to EUR", "≈ 0.0886218 EUR");
		eval_test("1 EUR/liter to NOK/liter", "11.2839 NOK / liter");
		eval_test(
			"1 EUR/gallon to NOK/liter",
			"≈ 2.98089102160411090430525272544562882356 NOK / liter",
		);
	}

	#[test]
	#[serial]
	fn test_ambiguous_evals() {
		Settings::write().locale = "en-GB".to_string();
		results_eq("1pound", "1gbp");
		Settings::write().locale = "nb-NO".to_string();
		results_eq("1pound", "1lbs");
	}

	#[test]
	fn test_unit_evals() {
		results_eq("100kg*sqm / 2s^2", "50j");
		results_eq("3.6km/1h", "3.6 kph");
		results_eq("0.3048 m/s to ft/s", "1 ft/s");
		eval_test("1.609344 km/1h to mph", "≈ 1 mile per hour");
		eval_test("1.852 kph to knots", "≈ 1 knot");
		results_eq("120 seconds to minutes", "2 minutes");
		results_eq("100 cm to m", "1 m");
		results_eq("1 km2 to m2", "1000000 m2");
		results_eq("1 liter to ml", "1000 ml");
		results_eq("1 kg to g", "1000 g");
		results_eq("1 KB to bytes", "1000 bytes");
		results_eq("1 MBps to KBps", "1000 KBps");
		results_eq("1 KFLOP to FLOP", "1000 FLOP");
		results_eq("1 KFLOPs to FLOPs", "1000 FLOPs");
		results_eq("1 kWh to Wh", "1000 Wh");
		results_eq("1 kW to W", "1000 W");
		results_eq("1000 mA to A", "1 A");
		results_eq("1000 mΩ to Ω", "1 Ω");
		results_eq("1000 mV to V", "1 V");
		results_eq("1000 millinewtons to newtons", "1 N");
		results_eq("1000 n to kilonewtons", "1 kN");
		results_eq("1 kilonewton to newtons", "1000 N");
		results_eq("1 meganewton to newtons", "1000000 N");
		results_eq("1 newton to dynes", "100000 dyne");
		eval_test("4.4482216152605 newtons to lbf", "1 pound-force");
		eval_test("1 lbf to newtons", "4.4482216152605 newtons");
		results_eq("1 bar to Pa", "100000 Pa");
		results_eq("1 r per min", "1 revolution per minute");
		eval_test(
			"60 revolutions per minute to revolutions per second",
			"≈ 1 revolution / second",
		);
		results_eq("1 kHz to Hz", "1000 Hz");
		results_eq("1 in of mercury", "1 inch of mercury");
		results_eq("1 \" of mercury", "1 inch of mercury");
		results_eq("1 \" of hg", "1 inch of mercury");
		results_eq("1\"hg", "1 inch of mercury");
		eval_test(
			"1 km/h to m/s",
			"≈ 0.277777777777777777777777777777777777778 meters / second",
		);
		results_eq("0 C to K", "273.15 K");
		results_eq("8 megabytes per second * 1 minute", "480mb");
		results_eq("8 megaFLOP per second * 1 minute", "480megaFLOP");
	}
}
