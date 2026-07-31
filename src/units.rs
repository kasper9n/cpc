use crate::Number;
use crate::currency;
use fastnum::{D128, dec128 as d};
use std::cmp::Reverse;

#[derive(Clone, Copy, PartialEq, Debug)]
/// An enum of all possible unit types, like [`Length`], [`DigitalStorage`] etc.
pub enum UnitType {
	/// May be one of several units depending on context, such as `pound` or `kr`.
	Unknown,
	/// A unit of time, for example [`Hour`]
	Time,
	/// A unit of length, for example [`Mile`]
	Length,
	/// A unit of area, for example [`SquareKilometer`]
	Area,
	/// A unit of volume, for example [`Liter`] or [`Tablespoon`]
	Volume,
	/// A unit of mass, for example [`Gram`]
	Mass,
	/// A unit of digital storage, for example [`Kilobyte`]
	DigitalStorage,
	/// A unit of data rate transfer, for example [`KilobytesPerSecond`]
	DataTransferRate,
	/// A unit of computational work, for example [`KiloFLOP`]
	FlopCount,
	/// A unit of computational performance, for example [`KiloFLOPPerSecond`]
	FlopRate,
	/// A unit of energy, for example [`Joule`] or [`KilowattHour`]
	Energy,
	/// A unit of power, for example [`Watt`]
	Power,
	/// A unit of electrical current, for example [`Ampere`]
	ElectricCurrent,
	/// A unit of electric resistance, for example [`Ohm`]
	Resistance,
	/// A unit of voltage, for example [`Volt`]
	Voltage,
	/// A unit of pressure, for example [`Bar`]
	Force,
	/// A unit of force, for example [`Hertz`]
	Pressure,
	/// A unit of frequency, for example [`Hertz`]
	Frequency,
	/// A unit of x, for example [`KilometersPerHour`]
	Speed,
	/// A unit of temperature, for example [`Kelvin`]
	Temperature,
	/// A unit of currency, for example [`EUR`]
	Currency,
}
impl UnitType {
	fn primitive(&self) -> Vec<(Unit, isize)> {
		let units = match self {
			UnitType::Unknown => panic!("Unknown unit type for ambiguous unit"),
			Time => vec![(Second, 1)],
			Length => vec![(Meter, 1)],
			Area => vec![(Meter, 2)],
			Volume => vec![(Meter, 3)],
			Mass => vec![(Kilogram, 1)],
			DigitalStorage => vec![(Bit, 1)],
			DataTransferRate => vec![(Bit, 1), (Second, -1)],
			FlopCount => vec![(Flop, 1)],
			FlopRate => vec![(Flop, 1), (Second, -1)],
			Energy => vec![(Meter, 2), (Kilogram, 1), (Second, -2)],
			Power => vec![(Meter, 2), (Kilogram, 1), (Second, -3)],
			ElectricCurrent => vec![(Ampere, 1)],
			Resistance => vec![(Meter, 2), (Kilogram, 1), (Second, -3), (Ampere, -2)],
			Voltage => vec![(Meter, 2), (Kilogram, 1), (Second, -3), (Ampere, -1)],
			Force => vec![(Meter, 1), (Kilogram, 1), (Second, -2)],
			Pressure => vec![(Kilogram, 1), (Second, -2), (Meter, -1)],
			Frequency => vec![(Second, -1)],
			Speed => vec![(Meter, 1), (Second, -1)],
			Temperature => vec![(Kelvin, 1)],
			Currency => vec![(EUR, 1)],
		};
		#[cfg(debug_assertions)]
		{
			let u0 = units.clone();
			let mut u1 = units.clone();
			sort_units(&mut u1);
			assert!(u0 == u1)
		}
		units
	}
}
use UnitType::*;

#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord, Hash)]
pub struct Ambiguity {
	pub string: &'static str,
	pub candidates: &'static [Unit],
	pub fallback: &'static Unit,
}

/// Sort for display and comparison purposes.
pub fn sort_units(primitives: &mut Vec<(Unit, isize)>) {
	primitives.sort_by_key(|u| {
		(
			u.1 < 0,            // multiplications first
			Reverse(u.1.abs()), // largest first, like "sqm seconds"
			u.0,                // then sort by unit, to have a fully deterministic order
		)
	});
}

pub fn primitive_unit(unit: &[(Unit, isize)]) -> Vec<(Unit, isize)> {
	let mut primitives: Vec<(Unit, isize)> = Vec::new();
	for (unit, exponent) in unit {
		for (primitive, primitive_exponent) in unit.category().primitive() {
			let existing = primitives.iter_mut().find(|(u, _)| u == &primitive);
			match existing {
				Some(existing) => existing.1 += primitive_exponent * exponent,
				None => primitives.push((primitive, primitive_exponent * exponent)),
			}
		}
	}
	primitives.retain(|(_, exponent)| exponent != &0);
	sort_units(&mut primitives);
	primitives
}

fn reduce_unit(number: Number) -> Number {
	let mut new_unit: Vec<(Unit, isize)> = Vec::new();
	for (unit, exponent) in &number.unit {
		if unit.category() == UnitType::Temperature {
			new_unit.push((unit.clone(), *exponent));
			continue;
		}
		let existing = new_unit
			.iter_mut()
			.find(|(u, _)| u.category() == unit.category());
		if let Some(existing) = existing {
			existing.1 += exponent;
		} else {
			new_unit.push((unit.clone(), *exponent))
		}
	}
	convert(number.clone(), new_unit).unwrap_or(number)
}

// Macro for creating units. Not possible to extend/change the default units
// with this because the default units are imported into the lexer, parser
// and evaluator
macro_rules! create_units {
	( $( $variant:ident : $properties:expr ),*, ) => {
		#[derive(Clone, Copy, PartialEq, Debug, Eq, PartialOrd, Ord, Hash)]
		/// A Unit enum. Note that it can also be [`NoUnit`].
		pub enum Unit {
			Ambiguity(Ambiguity),
			$($variant),*
		}
		use Unit::*;

		impl Unit {
			pub fn category(&self) -> UnitType {
				match self {
					Unit::Ambiguity(_) => UnitType::Unknown,
					$(
						Unit::$variant => $properties.0
					),*
				}
			}
			pub fn weight(&self) -> D128 {
				match self {
					Unit::Ambiguity(ambiguity) => panic!("Ambiguous unit {}", ambiguity.string),
					$(
						Unit::$variant => $properties.1
					),*
				}
			}
			pub(crate) fn singular(&self) -> &str {
				match self {
					Unit::Ambiguity(ambiguity) => panic!("Ambiguous unit {}", ambiguity.string),
					$(
						Unit::$variant => $properties.2
					),*
				}
			}
			pub(crate) fn plural(&self) -> &str {
				match self {
					Unit::Ambiguity(ambiguity) => panic!("Ambiguous unit {}", ambiguity.string),
					$(
						Unit::$variant => $properties.3
					),*
				}
			}
		}
	}
}

macro_rules! inexact {
	($a:tt / $b:tt) => {
		d!($a) / d!($b)
	};
}
create_units!(
	Nanosecond:         (Time, d!(0.000000001), "nanosecond", "nanoseconds"),
	Microsecond:        (Time, d!(0.000001), "microsecond", "microseconds"),
	Millisecond:        (Time, d!(0.001), "millisecond", "milliseconds"),
	Second:             (Time, d!(1), "second", "seconds"),
	Minute:             (Time, d!(60), "minute", "minutes"),
	Hour:               (Time, d!(3600), "hour", "hours"),
	Day:                (Time, d!(86400), "day", "days"),
	Week:               (Time, d!(604800), "week", "weeks"),
	Month:              (Time, d!(2629746), "month", "months"),
	Quarter:            (Time, d!(7889238), "quarter", "quarters"),
	Year:               (Time, d!(31556952), "year", "years"),
	Decade:             (Time, d!(315569520), "decade", "decades"),
	Century:            (Time, d!(3155695200), "century", "centuries"),
	Millennium:         (Time, d!(31556952000), "millennium", "millennia"),

	Millimeter:         (Length, d!(0.001), "millimeter", "millimeters"),
	Centimeter:         (Length, d!(0.01), "centimeter", "centimeters"),
	Decimeter:          (Length, d!(0.1), "decimeter", "decimeters"),
	Meter:              (Length, d!(1), "meter", "meters"),
	Kilometer:          (Length, d!(1000), "kilometer", "kilometers"),
	Inch:               (Length, d!(0.0254), "inch", "inches"),
	Foot:               (Length, d!(0.3048), "foot", "feet"),
	Yard:               (Length, d!(0.9144), "yard", "yards"),
	Mile:               (Length, d!(1609.344), "mile", "miles"),
	// 1-dimensional only:
	Marathon:           (Length, d!(42195), "marathon", "marathons"),
	NauticalMile:       (Length, d!(1852), "nautical mile", "nautical miles"),
	LightYear:          (Length, d!(9460730472580800), "light year", "light years"),
	LightSecond:        (Length, d!(299792458), "light second", "light seconds"),

	SquareMillimeter:   (Area, d!(0.000001), "square millimeter", "square millimeters"),
	SquareCentimeter:   (Area, d!(0.0001), "square centimeter", "square centimeters"),
	SquareDecimeter:    (Area, d!(0.01), "square decimeter", "square decimeters"),
	SquareMeter:        (Area, d!(1), "square meter", "square meters"),
	SquareKilometer:    (Area, d!(1000000), "square kilometer", "square kilometers"),
	SquareInch:         (Area, d!(0.00064516), "square inch", "square inches"),
	SquareFoot:         (Area, d!(0.09290304), "square foot", "square feet"),
	SquareYard:         (Area, d!(0.83612736), "square yard", "square yards"),
	SquareMile:         (Area, d!(2589988.110336), "square mile", "square miles"),
	// 2-dimensional only
	Are:                (Area, d!(100), "are", "ares"),
	Decare:             (Area, d!(1000), "decare", "decares"),
	Hectare:            (Area, d!(10000), "hectare", "hectares"),
	Acre:               (Area, d!(4046.8564224), "acre", "acres"),

	CubicMillimeter:    (Volume, d!(0.000000001), "cubic millimeter", "cubic millimeters"),
	CubicCentimeter:    (Volume, d!(0.000001), "cubic centimeter", "cubic centimeters"),
	CubicDecimeter:     (Volume, d!(0.001), "cubic decimeter", "cubic decimeters"),
	CubicMeter:         (Volume, d!(1), "cubic meter", "cubic meters"),
	CubicKilometer:     (Volume, d!(1000000000), "cubic kilometer", "cubic kilometers"),
	CubicInch:          (Volume, d!(0.000016387064), "cubic inch", "cubic inches"),
	CubicFoot:          (Volume, d!(0.028316846592), "cubic foot", "cubic feet"),
	CubicYard:          (Volume, d!(0.764554857984), "cubic yard", "cubic yards"),
	CubicMile:          (Volume, d!(4168181825.440579584), "cubic mile", "cubic miles"),
	// 3-dimensional only
	Milliliter:         (Volume, d!(0.000001), "milliliter", "milliliters"),
	Centiliter:         (Volume, d!(0.00001), "centiliter", "centiliters"),
	Deciliter:          (Volume, d!(0.0001), "deciliter", "deciliters"),
	Liter:              (Volume, d!(0.001), "liter", "liters"),
	Teaspoon:           (Volume, d!(0.00000492892159375), "teaspoon", "teaspoons"),
	Tablespoon:         (Volume, d!(0.00001478676478125), "tablespoon", "tablespoons"),
	FluidOunce:         (Volume, d!(0.0000295735295625), "fluid ounce", "fluid ounces"),
	Cup:                (Volume, d!(0.0002365882365), "cup", "cups"),
	Pint:               (Volume, d!(0.000473176473), "pint", "pints"),
	Quart:              (Volume, d!(0.000946352946), "quart", "quarts"),
	Gallon:             (Volume, d!(0.003785411784), "gallon", "gallons"),
	OilBarrel:          (Volume, d!(0.158987294928), "oil barrel", "oil barrels"),

	Milligram:          (Mass, d!(0.000001), "milligram", "milligrams"),
	Gram:               (Mass, d!(0.001), "gram", "grams"),
	Hectogram:          (Mass, d!(0.1), "hectogram", "hectograms"),
	Kilogram:           (Mass, d!(1), "kilogram", "kilograms"),
	MetricTon:          (Mass, d!(1000), "metric ton", "metric tons"),
	Ounce:              (Mass, d!(0.028349523125), "ounce", "ounces"),
	Pound:              (Mass, d!(0.45359237), "pound", "pounds"),
	Stone:              (Mass, d!(6.35029318), "stone", "stones"),
	ShortTon:           (Mass, d!(907.18474), "short ton", "short tons"),
	LongTon:            (Mass, d!(1016.0469088), "long ton", "long tons"),

	Bit:                (DigitalStorage, d!(1), "bit", "bits"),
	Kilobit:            (DigitalStorage, d!(1000), "kilobit", "kilobits"),
	Megabit:            (DigitalStorage, d!(1000000), "megabit", "megabits"),
	Gigabit:            (DigitalStorage, d!(1000000000), "gigabit", "gigabits"),
	Terabit:            (DigitalStorage, d!(1000000000000), "terabit", "terabits"),
	Petabit:            (DigitalStorage, d!(1000000000000000), "petabit", "petabits"),
	Exabit:             (DigitalStorage, d!(1000000000000000000), "exabit", "exabits"),
	Zettabit:           (DigitalStorage, d!(1000000000000000000000), "zettabit", "zettabits"),
	Yottabit:           (DigitalStorage, d!(1000000000000000000000000), "yottabit", "yottabits"),
	Kibibit:            (DigitalStorage, d!(1024), "kibibit", "kibibits"),
	Mebibit:            (DigitalStorage, d!(1048576), "mebibit", "mebibits"),
	Gibibit:            (DigitalStorage, d!(1073741824), "gibibit", "gibibits"),
	Tebibit:            (DigitalStorage, d!(1099511627776), "tebibit", "tebibits"),
	Pebibit:            (DigitalStorage, d!(1125899906842624), "pebibit", "pebibits"),
	Exbibit:            (DigitalStorage, d!(1152921504606846976), "exbibit", "exbibits"),
	Zebibit:            (DigitalStorage, d!(1180591620717411303424), "zebibit", "zebibits"),
	Yobibit:            (DigitalStorage, d!(1208925819614629174706176), "yobibit", "yobibits"),
	Byte:               (DigitalStorage, d!(8), "byte", "bytes"),
	Kilobyte:           (DigitalStorage, d!(8000), "kilobyte", "kilobytes"),
	Megabyte:           (DigitalStorage, d!(8000000), "megabyte", "megabytes"),
	Gigabyte:           (DigitalStorage, d!(8000000000), "gigabyte", "gigabytes"),
	Terabyte:           (DigitalStorage, d!(8000000000000), "terabyte", "terabytes"),
	Petabyte:           (DigitalStorage, d!(8000000000000000), "petabyte", "petabytes"),
	Exabyte:            (DigitalStorage, d!(8000000000000000000), "exabyte", "exabytes"),
	Zettabyte:          (DigitalStorage, d!(8000000000000000000000), "zettabyte", "zettabytes"),
	Yottabyte:          (DigitalStorage, d!(8000000000000000000000000), "yottabyte", "yottabytes"),
	Kibibyte:           (DigitalStorage, d!(8192), "kibibyte", "kibibytes"),
	Mebibyte:           (DigitalStorage, d!(8388608), "mebibyte", "mebibytes"),
	Gibibyte:           (DigitalStorage, d!(8589934592), "gibibyte", "gibibytes"),
	Tebibyte:           (DigitalStorage, d!(8796093022208), "tebibyte", "tebibytes"),
	Pebibyte:           (DigitalStorage, d!(9007199254740992), "pebibyte", "pebibytes"),
	Exbibyte:           (DigitalStorage, d!(9223372036854775808), "exbibyte", "exbibytes"),
	Zebibyte:           (DigitalStorage, d!(9444732965739290427392), "zebibyte", "zebibytes"),
	Yobibyte:           (DigitalStorage, d!(9671406556917033397649408), "yobibyte", "yobibytes"),

	BitsPerSecond:        (DataTransferRate, d!(1), "bit per second", "bits per second"),
	KilobitsPerSecond:    (DataTransferRate, d!(1000), "kilobit per second", "kilobits per second"),
	MegabitsPerSecond:    (DataTransferRate, d!(1000000), "megabit per second", "megabits per second"),
	GigabitsPerSecond:    (DataTransferRate, d!(1000000000), "gigabit per second", "gigabits per second"),
	TerabitsPerSecond:    (DataTransferRate, d!(1000000000000), "terabit per second", "terabits per second"),
	PetabitsPerSecond:    (DataTransferRate, d!(1000000000000000), "petabit per second", "petabits per second"),
	ExabitsPerSecond:     (DataTransferRate, d!(1000000000000000000), "exabit per second", "exabits per second"),
	ZettabitsPerSecond:   (DataTransferRate, d!(1000000000000000000000), "zettabit per second", "zettabits per second"),
	YottabitsPerSecond:   (DataTransferRate, d!(1000000000000000000000000), "yottabit per second", "yottabits per second"),
	KibibitsPerSecond:    (DataTransferRate, d!(1024), "kibibit per second", "kibibits per second"),
	MebibitsPerSecond:    (DataTransferRate, d!(1048576), "mebibit per second", "mebibits per second"),
	GibibitsPerSecond:    (DataTransferRate, d!(1073741824), "gibibit per second", "gibibits per second"),
	TebibitsPerSecond:    (DataTransferRate, d!(1099511627776), "tebibit per second", "tebibits per second"),
	PebibitsPerSecond:    (DataTransferRate, d!(1125899906842624), "pebibit per second", "pebibits per second"),
	ExbibitsPerSecond:    (DataTransferRate, d!(1152921504606846976), "exbibit per second", "exbibits per second"),
	ZebibitsPerSecond:    (DataTransferRate, d!(1180591620717411303424), "zebibit per second", "zebibits per second"),
	YobibitsPerSecond:    (DataTransferRate, d!(1208925819614629174706176), "yobibit per second", "yobibits per second"),
	BytesPerSecond:       (DataTransferRate, d!(8), "byte per second", "bytes per second"),
	KilobytesPerSecond:   (DataTransferRate, d!(8000), "kilobyte per second", "kilobytes per second"),
	MegabytesPerSecond:   (DataTransferRate, d!(8000000), "megabyte per second", "megabytes per second"),
	GigabytesPerSecond:   (DataTransferRate, d!(8000000000), "gigabyte per second", "gigabytes per second"),
	TerabytesPerSecond:   (DataTransferRate, d!(8000000000000), "terabyte per second", "terabytes per second"),
	PetabytesPerSecond:   (DataTransferRate, d!(8000000000000000), "petabyte per second", "petabytes per second"),
	ExabytesPerSecond:    (DataTransferRate, d!(8000000000000000000), "exabyte per second", "exabytes per second"),
	ZettabytesPerSecond:  (DataTransferRate, d!(8000000000000000000000), "zettabyte per second", "zettabytes per second"),
	YottabytesPerSecond:  (DataTransferRate, d!(8000000000000000000000000), "yottabyte per second", "yottabytes per second"),
	KibibytesPerSecond:   (DataTransferRate, d!(8192), "kibibyte per second", "kibibytes per second"),
	MebibytesPerSecond:   (DataTransferRate, d!(8388608), "mebibyte per second", "mebibytes per second"),
	GibibytesPerSecond:   (DataTransferRate, d!(8589934592), "gibibyte per second", "gibibytes per second"),
	TebibytesPerSecond:   (DataTransferRate, d!(8796093022208), "tebibyte per second", "tebibytes per second"),
	PebibytesPerSecond:   (DataTransferRate, d!(9007199254740992), "pebibyte per second", "pebibytes per second"),
	ExbibytesPerSecond:   (DataTransferRate, d!(9223372036854775808), "exbibyte per second", "exbibytes per second"),
	ZebibytesPerSecond:   (DataTransferRate, d!(9444732965739290427392), "zebibyte per second", "zebibytes per second"),
	YobibytesPerSecond:   (DataTransferRate, d!(9671406556917033397649408), "yobibyte per second", "yobibytes per second"),

	Flop:                 (FlopCount, d!(1), "FLOP", "FLOP"),
	KiloFlop:             (FlopCount, d!(1000), "kiloFLOP", "kiloFLOP"),
	MegaFlop:             (FlopCount, d!(1000000), "megaFLOP", "megaFLOP"),
	GigaFlop:             (FlopCount, d!(1000000000), "gigaFLOP", "gigaFLOP"),
	TeraFlop:             (FlopCount, d!(1000000000000), "teraFLOP", "teraFLOP"),
	PetaFlop:             (FlopCount, d!(1000000000000000), "petaFLOP", "petaFLOP"),
	ExaFlop:              (FlopCount, d!(1000000000000000000), "exaFLOP", "exaFLOP"),
	ZettaFlop:            (FlopCount, d!(1000000000000000000000), "zettaFLOP", "zettaFLOP"),
	YottaFlop:            (FlopCount, d!(1000000000000000000000000), "yottaFLOP", "yottaFLOP"),
	RonnaFlop:            (FlopCount, d!(1000000000000000000000000000), "ronnaFLOP", "ronnaFLOP"),
	QuettaFlop:           (FlopCount, d!(1000000000000000000000000000000), "quettaFLOP", "quettaFLOP"),

	FlopPerSecond:        (FlopRate, d!(1), "FLOP per second", "FLOP per second"),
	KiloFlopPerSecond:    (FlopRate, d!(1000), "kiloFLOP per second", "kiloFLOP per second"),
	MegaFlopPerSecond:    (FlopRate, d!(1000000), "megaFLOP per second", "megaFLOP per second"),
	GigaFlopPerSecond:    (FlopRate, d!(1000000000), "gigaFLOP per second", "gigaFLOP per second"),
	TeraFlopPerSecond:    (FlopRate, d!(1000000000000), "teraFLOP per second", "teraFLOP per second"),
	PetaFlopPerSecond:    (FlopRate, d!(1000000000000000), "petaFLOP per second", "petaFLOP per second"),
	ExaFlopPerSecond:     (FlopRate, d!(1000000000000000000), "exaFLOP per second", "exaFLOP per second"),
	ZettaFlopPerSecond:   (FlopRate, d!(1000000000000000000000), "zettaFLOP per second", "zettaFLOP per second"),
	YottaFlopPerSecond:   (FlopRate, d!(1000000000000000000000000), "yottaFLOP per second", "yottaFLOP per second"),
	RonnaFlopPerSecond:   (FlopRate, d!(1000000000000000000000000000), "ronnaFLOP per second", "ronnaFLOP per second"),
	QuettaFlopPerSecond:  (FlopRate, d!(1000000000000000000000000000000), "quettaFLOP per second", "quettaFLOP per second"),

	Millijoule:         (Energy, d!(0.001), "millijoule", "millijoules"),
	Joule:              (Energy, d!(1), "joule", "joules"),
	NewtonMeter:        (Energy, d!(1), "newton meter", "newton meters"),
	Kilojoule:          (Energy, d!(1000), "kilojoule", "kilojoules"),
	Megajoule:          (Energy, d!(1000000), "megajoule", "megajoules"),
	Gigajoule:          (Energy, d!(1000000000), "gigajoule", "gigajoules"),
	Terajoule:          (Energy, d!(1000000000000), "terajoule", "terajoules"),
	Calorie:            (Energy, d!(4.1868), "calorie", "calories"),
	KiloCalorie:        (Energy, d!(4186.8), "kilocalorie", "kilocalories"),
	BritishThermalUnit: (Energy, d!(1055.05585262), "British thermal unit", "British thermal units"),
	WattHour:           (Energy, d!(3600), "watt-hour", "watt-hours"),
	KilowattHour:       (Energy, d!(3600000), "kilowatt-hour", "kilowatt-hours"),
	MegawattHour:       (Energy, d!(3600000000),	"megawatt-hour", "megawatt-hours"),
	GigawattHour:       (Energy, d!(3600000000000), "gigawatt-hour", "gigawatt-hours"),
	TerawattHour:       (Energy, d!(3600000000000000), "terawatt-hour", "terawatt-hours"),
	PetawattHour:       (Energy, d!(3600000000000000000), "petawatt-hour", "petawatt-hours"),

	Milliwatt:                    (Power, d!(0.001), "milliwatt", "milliwatts"),
	Watt:                         (Power, d!(1), "watt", "watts"),
	Kilowatt:                     (Power, d!(1000), "kilowatt", "kilowatts"),
	Megawatt:                     (Power, d!(1000000), "megawatt", "megawatts"),
	Gigawatt:                     (Power, d!(1000000000), "gigawatt", "gigawatts"),
	Terawatt:                     (Power, d!(1000000000000), "terawatt", "terawatts"),
	Petawatt:                     (Power, d!(1000000000000000), "petawatt", "petawatts"),
	BritishThermalUnitsPerMinute: (Power, inexact!(1055.05585262 / 60), "british thermal unit per minute", "british thermal units per minute"),
	BritishThermalUnitsPerHour:   (Power, inexact!(1055.05585262 / 3600), "british thermal unit per hour", "british thermal units per hour"),
	Horsepower:                   (Power, d!(745.69987158227022), "horsepower", "horsepower"),
	MetricHorsepower:             (Power, d!(735.49875), "metric horsepower", "metric horsepower"),

	Milliampere:                  (ElectricCurrent, d!(0.001), "milliampere", "milliamperes"),
	Ampere:                       (ElectricCurrent, d!(1), "ampere", "amperes"),
	Kiloampere:                   (ElectricCurrent, d!(1000), "kiloampere", "kiloamperes"),
	Abampere:                     (ElectricCurrent, d!(10), "abampere", "abamperes"),

	Milliohm:                     (Resistance, d!(0.001), "milliohm", "milliohms"),
	Ohm:                          (Resistance, d!(1), "ohm", "ohms"),
	Kiloohm:                      (Resistance, d!(1000), "kiloohm", "kiloohms"),

	Millivolt:                    (Voltage, d!(0.001), "millivolt", "millivolts"),
	Volt:                         (Voltage, d!(1), "volt", "volts"),
	Kilovolt:                     (Voltage, d!(1000), "kilovolt", "kilovolts"),

	Millinewton:                  (Force, d!(0.001), "millinewton", "millinewtons"),
	Newton:                       (Force, d!(1), "newton", "newtons"),
	Kilonewton:                   (Force, d!(1000), "kilonewton", "kilonewtons"),
	Meganewton:                   (Force, d!(1000000), "meganewton", "meganewtons"),
	Dyne:                         (Force, d!(0.00001), "dyne", "dynes"),
	PoundForce:                   (Force, d!(4.4482216152605), "pound-force", "pound-force"),

	Pascal:                       (Pressure, d!(1), "pascal", "pascals"),
	Kilopascal:                   (Pressure, d!(1000), "kilopascal", "kilopascals"),
	Atmosphere:                   (Pressure, d!(101325), "atmosphere", "atmospheres"),
	Millibar:                     (Pressure, d!(100), "millibar", "millibars"),
	Bar:                          (Pressure, d!(100000), "bar", "bars"),
	InchOfMercury:                (Pressure, d!(3386.389), "inch of mercury", "inches of mercury"),
	PoundsPerSquareInch:          (Pressure, inexact!(8896443230521/1290320000), "pound per square inch", "pounds per square inch"),
	Torr:                         (Pressure, inexact!(4053000 / 30400), "torr", "torr"),

	Hertz:                        (Frequency, d!(1), "hertz", "hertz"),
	Kilohertz:                    (Frequency, d!(1000), "kilohertz", "kilohertz"),
	Megahertz:                    (Frequency, d!(1000000), "megahertz", "megahertz"),
	Gigahertz:                    (Frequency, d!(1000000000), "gigahertz", "gigahertz"),
	Terahertz:                    (Frequency, d!(1000000000000), "terahertz", "terahertz"),
	Petahertz:                    (Frequency, d!(1000000000000000), "petahertz", "petahertz"),
	RevolutionsPerMinute:         (Frequency, d!(60), "revolution per minute", "revolutions per minute"),

	KilometersPerHour:  (Speed, inexact!(1 / 3.6), "kilometer per hour", "kilometers per hour"),
	MetersPerSecond:    (Speed, d!(1), "meter per second", "meters per second"),
	MilesPerHour:       (Speed, d!(0.44704), "mile per hour", "miles per hour"),
	FeetPerSecond:      (Speed, d!(0.3048), "foot per second", "feet per second"),
	Knot:               (Speed, inexact!(463 / 900), "knot", "knots"),

	Kelvin:             (Temperature, d!(0), "kelvin", "kelvin"),
	Celsius:            (Temperature, d!(0), "celsius", "celsius"),
	Fahrenheit:         (Temperature, d!(0), "fahrenheit", "fahrenheit"),

	// Currency weights are fetched on-demand
	AFN: (Currency, d!(0), "AFN", "AFN"),
	ALL: (Currency, d!(0), "ALL", "ALL"),
	AMD: (Currency, d!(0), "AMD", "AMD"),
	ANG: (Currency, d!(0), "ANG", "ANG"),
	AOA: (Currency, d!(0), "AOA", "AOA"),
	ARS: (Currency, d!(0), "ARS", "ARS"),
	AUD: (Currency, d!(0), "AUD", "AUD"),
	AWG: (Currency, d!(0), "AWG", "AWG"),
	AZN: (Currency, d!(0), "AZN", "AZN"),
	BAM: (Currency, d!(0), "BAM", "BAM"),
	BBD: (Currency, d!(0), "BBD", "BBD"),
	BDT: (Currency, d!(0), "BDT", "BDT"),
	BHD: (Currency, d!(0), "BHD", "BHD"),
	BIF: (Currency, d!(0), "BIF", "BIF"),
	BMD: (Currency, d!(0), "BMD", "BMD"),
	BND: (Currency, d!(0), "BND", "BND"),
	BOB: (Currency, d!(0), "BOB", "BOB"),
	BRL: (Currency, d!(0), "BRL", "BRL"),
	BSD: (Currency, d!(0), "BSD", "BSD"),
	BTN: (Currency, d!(0), "BTN", "BTN"),
	BWP: (Currency, d!(0), "BWP", "BWP"),
	BYN: (Currency, d!(0), "BYN", "BYN"),
	BZD: (Currency, d!(0), "BZD", "BZD"),
	CAD: (Currency, d!(0), "CAD", "CAD"),
	CDF: (Currency, d!(0), "CDF", "CDF"),
	CHF: (Currency, d!(0), "CHF", "CHF"),
	CLP: (Currency, d!(0), "CLP", "CLP"),
	CNH: (Currency, d!(0), "CNH", "CNH"),
	CNY: (Currency, d!(0), "CNY", "CNY"),
	COP: (Currency, d!(0), "COP", "COP"),
	CRC: (Currency, d!(0), "CRC", "CRC"),
	CUP: (Currency, d!(0), "CUP", "CUP"),
	CVE: (Currency, d!(0), "CVE", "CVE"),
	CZK: (Currency, d!(0), "CZK", "CZK"),
	DJF: (Currency, d!(0), "DJF", "DJF"),
	DKK: (Currency, d!(0), "DKK", "DKK"),
	DOP: (Currency, d!(0), "DOP", "DOP"),
	DZD: (Currency, d!(0), "DZD", "DZD"),
	EGP: (Currency, d!(0), "EGP", "EGP"),
	ERN: (Currency, d!(0), "ERN", "ERN"),
	ETB: (Currency, d!(0), "ETB", "ETB"),
	EUR: (Currency, d!(0), "EUR", "EUR"),
	FJD: (Currency, d!(0), "FJD", "FJD"),
	FKP: (Currency, d!(0), "FKP", "FKP"),
	GBP: (Currency, d!(0), "GBP", "GBP"),
	GEL: (Currency, d!(0), "GEL", "GEL"),
	GGP: (Currency, d!(0), "GGP", "GGP"),
	GHS: (Currency, d!(0), "GHS", "GHS"),
	GIP: (Currency, d!(0), "GIP", "GIP"),
	GMD: (Currency, d!(0), "GMD", "GMD"),
	GNF: (Currency, d!(0), "GNF", "GNF"),
	GTQ: (Currency, d!(0), "GTQ", "GTQ"),
	GYD: (Currency, d!(0), "GYD", "GYD"),
	HKD: (Currency, d!(0), "HKD", "HKD"),
	HNL: (Currency, d!(0), "HNL", "HNL"),
	HTG: (Currency, d!(0), "HTG", "HTG"),
	HUF: (Currency, d!(0), "HUF", "HUF"),
	IDR: (Currency, d!(0), "IDR", "IDR"),
	ILS: (Currency, d!(0), "ILS", "ILS"),
	IMP: (Currency, d!(0), "IMP", "IMP"),
	INR: (Currency, d!(0), "INR", "INR"),
	IQD: (Currency, d!(0), "IQD", "IQD"),
	IRR: (Currency, d!(0), "IRR", "IRR"),
	ISK: (Currency, d!(0), "ISK", "ISK"),
	JEP: (Currency, d!(0), "JEP", "JEP"),
	JMD: (Currency, d!(0), "JMD", "JMD"),
	JOD: (Currency, d!(0), "JOD", "JOD"),
	JPY: (Currency, d!(0), "JPY", "JPY"),
	KES: (Currency, d!(0), "KES", "KES"),
	KGS: (Currency, d!(0), "KGS", "KGS"),
	KHR: (Currency, d!(0), "KHR", "KHR"),
	KMF: (Currency, d!(0), "KMF", "KMF"),
	KPW: (Currency, d!(0), "KPW", "KPW"),
	KRW: (Currency, d!(0), "KRW", "KRW"),
	KWD: (Currency, d!(0), "KWD", "KWD"),
	KYD: (Currency, d!(0), "KYD", "KYD"),
	KZT: (Currency, d!(0), "KZT", "KZT"),
	LAK: (Currency, d!(0), "LAK", "LAK"),
	LBP: (Currency, d!(0), "LBP", "LBP"),
	LKR: (Currency, d!(0), "LKR", "LKR"),
	LRD: (Currency, d!(0), "LRD", "LRD"),
	LSL: (Currency, d!(0), "LSL", "LSL"),
	LYD: (Currency, d!(0), "LYD", "LYD"),
	MAD: (Currency, d!(0), "MAD", "MAD"),
	MDL: (Currency, d!(0), "MDL", "MDL"),
	MGA: (Currency, d!(0), "MGA", "MGA"),
	MKD: (Currency, d!(0), "MKD", "MKD"),
	MMK: (Currency, d!(0), "MMK", "MMK"),
	MNT: (Currency, d!(0), "MNT", "MNT"),
	MOP: (Currency, d!(0), "MOP", "MOP"),
	MRO: (Currency, d!(0), "MRO", "MRO"),
	MRU: (Currency, d!(0), "MRU", "MRU"),
	MUR: (Currency, d!(0), "MUR", "MUR"),
	MVR: (Currency, d!(0), "MVR", "MVR"),
	MWK: (Currency, d!(0), "MWK", "MWK"),
	MXN: (Currency, d!(0), "MXN", "MXN"),
	MYR: (Currency, d!(0), "MYR", "MYR"),
	MZN: (Currency, d!(0), "MZN", "MZN"),
	NAD: (Currency, d!(0), "NAD", "NAD"),
	NGN: (Currency, d!(0), "NGN", "NGN"),
	NIO: (Currency, d!(0), "NIO", "NIO"),
	NOK: (Currency, d!(0), "NOK", "NOK"),
	NPR: (Currency, d!(0), "NPR", "NPR"),
	NZD: (Currency, d!(0), "NZD", "NZD"),
	OMR: (Currency, d!(0), "OMR", "OMR"),
	PAB: (Currency, d!(0), "PAB", "PAB"),
	PEN: (Currency, d!(0), "PEN", "PEN"),
	PGK: (Currency, d!(0), "PGK", "PGK"),
	PHP: (Currency, d!(0), "PHP", "PHP"),
	PKR: (Currency, d!(0), "PKR", "PKR"),
	PLN: (Currency, d!(0), "PLN", "PLN"),
	PYG: (Currency, d!(0), "PYG", "PYG"),
	QAR: (Currency, d!(0), "QAR", "QAR"),
	RON: (Currency, d!(0), "RON", "RON"),
	RSD: (Currency, d!(0), "RSD", "RSD"),
	RUB: (Currency, d!(0), "RUB", "RUB"),
	RWF: (Currency, d!(0), "RWF", "RWF"),
	SAR: (Currency, d!(0), "SAR", "SAR"),
	SBD: (Currency, d!(0), "SBD", "SBD"),
	SCR: (Currency, d!(0), "SCR", "SCR"),
	SDG: (Currency, d!(0), "SDG", "SDG"),
	SEK: (Currency, d!(0), "SEK", "SEK"),
	SGD: (Currency, d!(0), "SGD", "SGD"),
	SHP: (Currency, d!(0), "SHP", "SHP"),
	SLE: (Currency, d!(0), "SLE", "SLE"),
	SOS: (Currency, d!(0), "SOS", "SOS"),
	SRD: (Currency, d!(0), "SRD", "SRD"),
	SSP: (Currency, d!(0), "SSP", "SSP"),
	STN: (Currency, d!(0), "STN", "STN"),
	SVC: (Currency, d!(0), "SVC", "SVC"),
	SYP: (Currency, d!(0), "SYP", "SYP"),
	SZL: (Currency, d!(0), "SZL", "SZL"),
	THB: (Currency, d!(0), "THB", "THB"),
	TJS: (Currency, d!(0), "TJS", "TJS"),
	TMT: (Currency, d!(0), "TMT", "TMT"),
	TND: (Currency, d!(0), "TND", "TND"),
	TOP: (Currency, d!(0), "TOP", "TOP"),
	TRY: (Currency, d!(0), "TRY", "TRY"),
	TTD: (Currency, d!(0), "TTD", "TTD"),
	TWD: (Currency, d!(0), "TWD", "TWD"),
	TZS: (Currency, d!(0), "TZS", "TZS"),
	UAH: (Currency, d!(0), "UAH", "UAH"),
	UGX: (Currency, d!(0), "UGX", "UGX"),
	USD: (Currency, d!(0), "USD", "USD"),
	UYU: (Currency, d!(0), "UYU", "UYU"),
	UZS: (Currency, d!(0), "UZS", "UZS"),
	VES: (Currency, d!(0), "VES", "VES"),
	VND: (Currency, d!(0), "VND", "VND"),
	VUV: (Currency, d!(0), "VUV", "VUV"),
	WST: (Currency, d!(0), "WST", "WST"),
	XAF: (Currency, d!(0), "XAF", "XAF"),
	XAG: (Currency, d!(0), "XAG", "XAG"),
	XAU: (Currency, d!(0), "XAU", "XAU"),
	XCD: (Currency, d!(0), "XCD", "XCD"),
	XCG: (Currency, d!(0), "XCG", "XCG"),
	XDR: (Currency, d!(0), "XDR", "XDR"),
	XOF: (Currency, d!(0), "XOF", "XOF"),
	XPD: (Currency, d!(0), "XPD", "XPD"),
	XPF: (Currency, d!(0), "XPF", "XPF"),
	XPT: (Currency, d!(0), "XPT", "XPT"),
	YER: (Currency, d!(0), "YER", "YER"),
	ZAR: (Currency, d!(0), "ZAR", "ZAR"),
	ZMW: (Currency, d!(0), "ZMW", "ZMW"),
	ZWG: (Currency, d!(0), "ZWG", "ZWG"),
);

fn combined_weight(unit: &[(Unit, isize)]) -> D128 {
	unit.iter().fold(D128::from(1), |acc, (u, exp)| {
		acc * integer_power(u.weight(), *exp)
	})
}

fn integer_power(base: D128, exp: isize) -> D128 {
	let positive = (0..exp.unsigned_abs()).fold(D128::from(1), |acc, _| acc * base);
	if exp >= 0 {
		positive
	} else {
		D128::from(1) / positive
	}
}

fn contains_category(unit: &[(Unit, isize)], category: UnitType) -> bool {
	unit.iter().any(|(u, _)| u.category() == category)
}

/// Get the non-currency weight of a unit vector
fn non_currency_weight(unit: &[(Unit, isize)]) -> D128 {
	use UnitType::*;
	unit.iter().fold(D128::from(1), |acc, (u, exp)| {
		if u.category() == Currency {
			acc
		} else {
			acc * integer_power(u.weight(), *exp)
		}
	})
}

/// Convert a [`Number`] to a specified [`Unit`].
pub fn convert(number: Number, to_unit: Vec<(Unit, isize)>) -> Result<Number, String> {
	if number.unit == to_unit {
		return Ok(number);
	}
	if number.primitive_unit() != primitive_unit(&to_unit) {
		return Err(format!(
			"Cannot convert {} to {}",
			number,
			Number::with_unit(d!(0), to_unit).plural()
		));
	}
	let value = number.value;
	if number.primitive_unit() == Temperature.primitive() {
		if number.unit.len() != 1
			|| to_unit.len() != 1
			|| number.unit[0].1 != 1
			|| to_unit[0].1 != 1
		{
			return Err(format!(
				"Cannot convert {} to {}",
				number,
				Number::with_unit(d!(0), to_unit).plural()
			));
		}
		let ok = |new_value| Ok(Number::with_unit(new_value, to_unit.clone()));
		match (number.unit[0].0, to_unit[0].0) {
			(Kelvin, Kelvin) => ok(value),
			(Kelvin, Celsius) => ok(value - d!(273.15)),
			(Kelvin, Fahrenheit) => ok(value * d!(1.8) - d!(459.67)),
			(Celsius, Celsius) => ok(value),
			(Celsius, Kelvin) => ok(value + d!(273.15)),
			(Celsius, Fahrenheit) => ok(value * d!(1.8) + d!(32)),
			(Fahrenheit, Fahrenheit) => ok(value),
			(Fahrenheit, Kelvin) => ok((value + d!(459.67)) * d!(5) / d!(9)),
			(Fahrenheit, Celsius) => ok((value - d!(32)) / d!(1.8)),
			_ => Err(format!(
				"Error converting temperature {} to {}",
				number,
				Number::with_unit(d!(0), to_unit).plural()
			)),
		}
	} else if number.contains_category(Currency) && contains_category(&to_unit, Currency) {
		// Handle compound units with currency, like "EUR/liter"
		// Find the currency in both units
		let from_currency = number
			.unit
			.iter()
			.find(|(u, _)| u.category() == Currency)
			.map(|(u, _)| *u);
		let to_currency = to_unit
			.iter()
			.find(|(u, _)| u.category() == Currency)
			.map(|(u, _)| *u);

		if let (Some(from_curr), Some(to_curr)) = (from_currency, to_currency) {
			let rate = currency::get_exchange_rate(from_curr, to_curr)?;

			// Calculate the ratio of non-currency parts
			let source_non_currency = non_currency_weight(&number.unit);
			let target_non_currency = non_currency_weight(&to_unit);

			let value = number.value * rate * source_non_currency / target_non_currency;

			Ok(Number {
				value,
				unit: to_unit.to_vec(),
			})
		} else {
			Err("Currency conversion requires both units to have currency".to_string())
		}
	} else {
		let source_weight = combined_weight(&number.unit);
		let target_weight = combined_weight(&to_unit);

		Ok(Number {
			value: number.value * source_weight / target_weight,
			unit: to_unit.to_vec(),
		})
	}
}

/// If one of two provided [`Number`]s has a larger [`Unit`] than the other, convert
/// the large one to the unit of the small one.
pub fn convert_to_lowest(left: Number, right: Number) -> Result<(Number, Number), String> {
	assert!(left.primitive_unit() == right.primitive_unit());
	if combined_weight(&left.unit) == combined_weight(&right.unit) {
		Ok((left, right))
	} else if combined_weight(&left.unit) > combined_weight(&right.unit) {
		let left_converted = convert(left, right.unit.clone())?;
		Ok((left_converted, right))
	} else {
		let right_converted = convert(right, left.unit.clone())?;
		Ok((left, right_converted))
	}
}

/// Return the sum of two [`Number`]s
pub fn add(left: Number, right: Number) -> Result<Number, String> {
	if left.unit == right.unit {
		Ok(Number::with_unit(left.value + right.value, left.unit))
	} else if left.primitive_unit() == right.primitive_unit()
		&& !left.contains_category(Temperature)
	{
		let (left, right) = convert_to_lowest(left, right)?;
		Ok(Number::with_unit(left.value + right.value, left.unit))
	} else {
		Err(format!("Cannot add {} and {}", left, right))
	}
}

/// Subtract a [`Number`] from another [`Number`]
pub fn subtract(left: Number, right: Number) -> Result<Number, String> {
	if left.unit == right.unit {
		Ok(Number::with_unit(left.value - right.value, left.unit))
	} else if left.primitive_unit() == right.primitive_unit()
		&& !left.contains_category(Temperature)
	{
		let (left, right) = convert_to_lowest(left, right)?;
		Ok(Number::with_unit(left.value - right.value, left.unit))
	} else {
		Err(format!("Cannot subtract {} by {}", left, right))
	}
}

/// Convert a [`Number`] to an ideal unit.
///
/// If you have 1,000,000 millimeters, this will return 1 kilometer.
///
/// This only affects units of `Length`, `Time`, `Area`, `Volume`,
/// `Energy`, `Power`, `ElectricCurrent`, `Resistance`, and `Voltage`.
/// Other units are passed through.
pub fn to_ideal_unit(number: Number) -> Number {
	let number = reduce_unit(number);
	let value = number.value * combined_weight(&number.unit);
	let primitive = number.primitive_unit();
	if primitive == Length.primitive() {
		if value >= 0.1 * LightYear.weight() {
			return Number::with_basic_unit(value / LightYear.weight(), LightYear);
		} else if value >= Kilometer.weight() {
			return Number::with_basic_unit(value / Kilometer.weight(), Kilometer);
		} else if value >= Meter.weight() {
			return Number::with_basic_unit(value / Meter.weight(), Meter);
		} else if value >= Centimeter.weight() {
			return Number::with_basic_unit(value / Centimeter.weight(), Centimeter);
		} else {
			return Number::with_basic_unit(value / Millimeter.weight(), Millimeter);
		}
	} else if primitive == Time.primitive() {
		if value >= Year.weight() {
			return Number::with_basic_unit(value / Year.weight(), Year);
		} else if value >= Day.weight() {
			return Number::with_basic_unit(value / Day.weight(), Day);
		} else if value >= Hour.weight() {
			return Number::with_basic_unit(value / Hour.weight(), Hour);
		} else if value >= Minute.weight() {
			return Number::with_basic_unit(value / Minute.weight(), Minute);
		} else if value >= Second.weight() {
			return Number::with_basic_unit(value / Second.weight(), Second);
		} else if value >= Millisecond.weight() {
			return Number::with_basic_unit(value / Millisecond.weight(), Millisecond);
		} else if value >= Microsecond.weight() {
			return Number::with_basic_unit(value / Microsecond.weight(), Microsecond);
		} else {
			return Number::with_basic_unit(value / Nanosecond.weight(), Nanosecond);
		}
	} else if primitive == Area.primitive() {
		if value >= SquareKilometer.weight() {
			return Number::with_basic_unit(value / SquareKilometer.weight(), SquareKilometer);
		} else if value >= Hectare.weight() {
			return Number::with_basic_unit(value / Hectare.weight(), Hectare);
		} else if value >= SquareMeter.weight() {
			return Number::with_basic_unit(value / SquareMeter.weight(), SquareMeter);
		} else if value >= SquareCentimeter.weight() {
			return Number::with_basic_unit(value / SquareCentimeter.weight(), SquareCentimeter);
		} else {
			return Number::with_basic_unit(value / SquareMillimeter.weight(), SquareMillimeter);
		}
	} else if primitive == Volume.primitive() {
		if value >= CubicKilometer.weight() {
			return Number::with_basic_unit(value / CubicKilometer.weight(), CubicKilometer);
		} else if value >= CubicMeter.weight() {
			return Number::with_basic_unit(value / CubicMeter.weight(), CubicMeter);
		} else if value >= Liter.weight() {
			return Number::with_basic_unit(value / Liter.weight(), Liter);
		} else if value >= Milliliter.weight() {
			return Number::with_basic_unit(value / Milliliter.weight(), Milliliter);
		} else {
			return Number::with_basic_unit(value / CubicMillimeter.weight(), CubicMillimeter);
		}
	} else if primitive == Energy.primitive() {
		let has_second = number.unit.iter().find(|unit| unit.0 == Second).is_some();
		if has_second {
			if value >= Terajoule.weight() {
				return Number::with_basic_unit(value / Terajoule.weight(), Terajoule);
			} else if value >= Gigajoule.weight() {
				return Number::with_basic_unit(value / Gigajoule.weight(), Gigajoule);
			} else if value >= Megajoule.weight() {
				return Number::with_basic_unit(value / Megajoule.weight(), Megajoule);
			} else if value >= Kilojoule.weight() {
				return Number::with_basic_unit(value / Kilojoule.weight(), Kilojoule);
			} else if value >= Joule.weight() {
				return Number::with_basic_unit(value / Joule.weight(), Joule);
			} else {
				return Number::with_basic_unit(value / Millijoule.weight(), Millijoule);
			}
		} else {
			if value >= PetawattHour.weight() {
				return Number::with_basic_unit(value / PetawattHour.weight(), PetawattHour);
			} else if value >= TerawattHour.weight() {
				return Number::with_basic_unit(value / TerawattHour.weight(), TerawattHour);
			} else if value >= GigawattHour.weight() {
				return Number::with_basic_unit(value / GigawattHour.weight(), GigawattHour);
			} else if value >= MegawattHour.weight() {
				return Number::with_basic_unit(value / MegawattHour.weight(), MegawattHour);
			} else if value >= KilowattHour.weight() {
				return Number::with_basic_unit(value / KilowattHour.weight(), KilowattHour);
			} else if value >= WattHour.weight() {
				return Number::with_basic_unit(value / WattHour.weight(), WattHour);
			} else if value >= Joule.weight() {
				return Number::with_basic_unit(value / Joule.weight(), Joule);
			} else {
				return Number::with_basic_unit(value / Millijoule.weight(), Millijoule);
			}
		}
	} else if primitive == Power.primitive() {
		if value >= Petawatt.weight() {
			return Number::with_basic_unit(value / Petawatt.weight(), Petawatt);
		} else if value >= Terawatt.weight() {
			return Number::with_basic_unit(value / Terawatt.weight(), Terawatt);
		} else if value >= Gigawatt.weight() {
			return Number::with_basic_unit(value / Gigawatt.weight(), Gigawatt);
		} else if value >= Megawatt.weight() {
			return Number::with_basic_unit(value / Megawatt.weight(), Megawatt);
		} else if value >= Kilowatt.weight() {
			return Number::with_basic_unit(value / Kilowatt.weight(), Kilowatt);
		} else if value >= Watt.weight() {
			return Number::with_basic_unit(value / Watt.weight(), Watt);
		} else {
			return Number::with_basic_unit(value / Milliwatt.weight(), Milliwatt);
		}
	} else if primitive == ElectricCurrent.primitive() {
		if value >= Kiloampere.weight() {
			return Number::with_basic_unit(value / Kiloampere.weight(), Kiloampere);
		} else if value >= Ampere.weight() {
			return Number::with_basic_unit(value / Ampere.weight(), Ampere);
		} else {
			return Number::with_basic_unit(value / Milliampere.weight(), Milliampere);
		}
	} else if primitive == Resistance.primitive() {
		if value >= Kiloohm.weight() {
			return Number::with_basic_unit(value / Kiloohm.weight(), Kiloohm);
		} else if value >= Ohm.weight() {
			return Number::with_basic_unit(value / Ohm.weight(), Ohm);
		} else {
			return Number::with_basic_unit(value / Milliohm.weight(), Milliohm);
		}
	} else if primitive == Voltage.primitive() {
		if value >= Kilovolt.weight() {
			return Number::with_basic_unit(value / Kilovolt.weight(), Kilovolt);
		} else if value >= Volt.weight() {
			return Number::with_basic_unit(value / Volt.weight(), Volt);
		} else {
			return Number::with_basic_unit(value / Millivolt.weight(), Millivolt);
		}
	} else if primitive == DigitalStorage.primitive() {
		let bits = &[
			Bit, Kilobit, Megabit, Gigabit, Terabit, Petabit, Exabit, Zettabit, Yottabit,
		];
		let bibits = &[
			Bit, Kibibit, Mebibit, Gibibit, Tebibit, Pebibit, Exbibit, Zebibit, Yobibit,
		];
		let bytes = &[
			Byte, Kilobyte, Megabyte, Gigabyte, Terabyte, Petabyte, Exabyte, Zettabyte, Yottabyte,
		];
		let bibytes = &[
			Byte, Kibibyte, Mebibyte, Gibibyte, Tebibyte, Pebibyte, Exbibyte, Zebibyte, Yobibyte,
		];
		for unit in &number.unit {
			if unit.0.category() == DigitalStorage || unit.0.category() == DataTransferRate {
				let weight = unit.0.weight();
				let candidates = if weight % 8192 == d!(0) {
					bibytes
				} else if weight % 8000 == d!(0) {
					bytes
				} else if weight % 1024 == d!(0) {
					bibits
				} else {
					bits
				};
				let unit = candidates
					.iter()
					.rev()
					.find(|&&u| value >= u.weight())
					.copied()
					.unwrap_or(candidates[0]);
				return Number::with_basic_unit(value / unit.weight(), unit);
			}
		}
	} else if primitive == FlopCount.primitive() {
		let candidates = [
			Flop, KiloFlop, MegaFlop, GigaFlop, TeraFlop, PetaFlop, ExaFlop, ZettaFlop, YottaFlop,
			RonnaFlop, QuettaFlop,
		];
		let unit = candidates
			.iter()
			.rev()
			.find(|&&u| value >= u.weight())
			.copied()
			.unwrap_or(candidates[0]);
		return Number::with_basic_unit(value / unit.weight(), unit);
	}

	number
}

/// Multiply two [`Number`]s.
///
/// Units are converted accordingly.
///
/// Temperatures don't work
pub fn multiply(left: Number, right: Number) -> Result<Number, String> {
	if left.contains_category(Temperature) || right.contains_category(Temperature) {
		Err(format!("Cannot multiply {} and {}", left, right))
	} else {
		multiply_any(left, right)
	}
}

pub(crate) fn multiply_any(left: Number, right: Number) -> Result<Number, String> {
	let mut new_number = left;
	new_number.value *= right.value;
	for (r_unit, r_exp) in right.unit {
		let existing = new_number.unit.iter_mut().find(|(u, _)| u == &r_unit);
		match existing {
			Some(existing) => existing.1 += r_exp,
			None => new_number.unit.push((r_unit, r_exp)),
		}
	}
	Ok(new_number)
}

/// Divide a [`Number`] by another [`Number`].
///
/// Units are converted accordingly.
///
/// Temperatures don't work.
pub fn divide(left: Number, right: Number) -> Result<Number, String> {
	if left.contains_category(Temperature) || right.contains_category(Temperature) {
		Err(format!("Cannot divide {} by {}", left, right))
	} else {
		divide_any(left, right)
	}
}

pub fn divide_any(left: Number, right: Number) -> Result<Number, String> {
	let mut new_number = left;
	new_number.value = new_number.value / right.value;
	for (r_unit, r_exp) in right.unit {
		let existing = new_number.unit.iter_mut().find(|(u, _)| u == &r_unit);
		match existing {
			Some(existing) => existing.1 -= r_exp,
			None => new_number.unit.push((r_unit, -r_exp)),
		}
	}
	Ok(new_number)
}

/// Modulo a [`Number`] by another [`Number`].
///
/// `left` and `right` need to have the same [`UnitType`], and the result will have that same [`UnitType`].
///
/// Temperatures don't work.
pub fn modulo(left: Number, right: Number) -> Result<Number, String> {
	if left.contains_category(Temperature) || right.contains_category(Temperature) {
		Err(format!("Cannot modulo {} by {}", left, right))
	} else if left.primitive_unit() == right.primitive_unit() {
		// 5 km % 3 m
		let (left, right) = convert_to_lowest(left, right)?;
		Ok(Number::with_unit(left.value % right.value, left.unit))
	} else {
		Err(format!("Cannot modulo {} by {}", left, right))
	}
}

/// Returns a [`Number`] to the power of another [`Number`]
///
/// - If you take [`Length`] to the power of [`NoType`], the result has a unit of [`Area`].
/// - If you take [`Length`] to the power of [`Length`], the result has a unit of [`Area`]
/// - If you take [`Length`] to the power of [`Area`], the result has a unit of [`Volume`]
/// - etc.
pub fn pow(left: Number, right: Number) -> Result<Number, String> {
	// I tried converting `right` to use powi, but somehow that was slower
	if left.contains_category(Temperature) || right.has_unit() {
		Err(format!("Cannot raise {} to the power of {}", left, right))
	} else if left.is_unitless() {
		let result = left.value.pow(right.value);
		let new_number = Number::new_unitless(result);
		Ok(new_number)
	} else {
		let exp: isize = match (right.value.try_into(), right.value.is_integral()) {
			(Ok(exp), true) => exp,
			_ => {
				return Err(format!(
					"Cannot raise {} to the power of {}. Numbers with units can only be raised to integer powers",
					left, right
				));
			}
		};
		let result = left.value.pow(right.value);
		let mut new_number = Number::with_unit(result, left.unit);
		for (_, unit_exp) in new_number.unit.iter_mut() {
			*unit_exp *= exp;
		}
		let new_number = to_ideal_unit(new_number);
		Ok(new_number)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use fastnum::decimal::Context;

	macro_rules! assert_float_eq {
		( $actual:expr, $expected:literal ) => {
			assert!(
				($actual - $expected).abs() < f64::EPSILON,
				"assertion `left == right` failed\n  left: {:?}\n right: {:?}",
				$actual,
				$expected
			);
		};
	}

	#[test]
	fn test_convert() {
		pub fn convert_test(value: f64, unit: Unit, to_unit: Unit) -> f64 {
			use std::str::FromStr;

			let value_string = &value.to_string();
			let value_d128 = D128::from_str(value_string, Context::default()).unwrap();
			let number = Number::with_basic_unit(value_d128, unit);

			let result = convert(number, vec![(to_unit, 1)]);
			let string_result = &result.unwrap().value.to_string();
			let float_result = f64::from_str(string_result).unwrap();

			return float_result;
		}

		assert_float_eq!(convert_test(1000.0, Nanosecond, Microsecond), 1.0);
		assert_float_eq!(convert_test(1000.0, Nanosecond, Microsecond), 1.0);
		assert_float_eq!(convert_test(1000.0, Microsecond, Millisecond), 1.0);
		assert_float_eq!(convert_test(1000.0, Millisecond, Second), 1.0);
		assert_float_eq!(convert_test(60.0, Second, Minute), 1.0);
		assert_float_eq!(convert_test(60.0, Minute, Hour), 1.0);
		assert_float_eq!(convert_test(24.0, Hour, Day), 1.0);
		assert_float_eq!(convert_test(7.0, Day, Week), 1.0);
		assert_float_eq!(convert_test(30.436875, Day, Month), 1.0);
		assert_float_eq!(convert_test(3.0, Month, Quarter), 1.0);
		assert_float_eq!(convert_test(4.0, Quarter, Year), 1.0);
		assert_float_eq!(convert_test(10.0, Year, Decade), 1.0);
		assert_float_eq!(convert_test(10.0, Decade, Century), 1.0);
		assert_float_eq!(convert_test(10.0, Century, Millennium), 1.0);

		assert_float_eq!(convert_test(10.0, Millimeter, Centimeter), 1.0);
		assert_float_eq!(convert_test(10.0, Centimeter, Decimeter), 1.0);
		assert_float_eq!(convert_test(10.0, Decimeter, Meter), 1.0);
		assert_float_eq!(convert_test(1000.0, Meter, Kilometer), 1.0);
		assert_float_eq!(convert_test(2.54, Centimeter, Inch), 1.0);
		assert_float_eq!(convert_test(12.0, Inch, Foot), 1.0);
		assert_float_eq!(convert_test(3.0, Foot, Yard), 1.0);
		assert_float_eq!(convert_test(1760.0, Yard, Mile), 1.0);
		assert_float_eq!(convert_test(42195.0, Meter, Marathon), 1.0);
		assert_float_eq!(convert_test(1852.0, Meter, NauticalMile), 1.0);
		assert_float_eq!(convert_test(9460730472580800.0, Meter, LightYear), 1.0);
		assert_float_eq!(convert_test(299792458.0, Meter, LightSecond), 1.0);

		assert_float_eq!(convert_test(100.0, SquareMillimeter, SquareCentimeter), 1.0);
		assert_float_eq!(convert_test(100.0, SquareCentimeter, SquareDecimeter), 1.0);
		assert_float_eq!(convert_test(100.0, SquareDecimeter, SquareMeter), 1.0);
		assert_float_eq!(convert_test(1000000.0, SquareMeter, SquareKilometer), 1.0);
		assert_float_eq!(convert_test(645.16, SquareMillimeter, SquareInch), 1.0);
		assert_float_eq!(convert_test(144.0, SquareInch, SquareFoot), 1.0);
		assert_float_eq!(convert_test(9.0, SquareFoot, SquareYard), 1.0);
		assert_float_eq!(convert_test(3097600.0, SquareYard, SquareMile), 1.0);
		assert_float_eq!(convert_test(100.0, SquareMeter, Are), 1.0);
		assert_float_eq!(convert_test(10.0, Are, Decare), 1.0);
		assert_float_eq!(convert_test(10.0, Decare, Hectare), 1.0);
		assert_float_eq!(convert_test(640.0, Acre, SquareMile), 1.0);

		assert_float_eq!(convert_test(1000.0, CubicMillimeter, CubicCentimeter), 1.0);
		assert_float_eq!(convert_test(1000.0, CubicCentimeter, CubicDecimeter), 1.0);
		assert_float_eq!(convert_test(1000.0, CubicDecimeter, CubicMeter), 1.0);
		assert_float_eq!(convert_test(1000000000.0, CubicMeter, CubicKilometer), 1.0);
		assert_float_eq!(convert_test(1728.0, CubicInch, CubicFoot), 1.0);
		assert_float_eq!(convert_test(27.0, CubicFoot, CubicYard), 1.0);
		assert_float_eq!(convert_test(5451776000.0, CubicYard, CubicMile), 1.0);
		assert_float_eq!(convert_test(1.0, Milliliter, CubicCentimeter), 1.0);
		assert_float_eq!(convert_test(10.0, Milliliter, Centiliter), 1.0);
		assert_float_eq!(convert_test(10.0, Centiliter, Deciliter), 1.0);
		assert_float_eq!(convert_test(10.0, Deciliter, Liter), 1.0);
		assert_float_eq!(convert_test(4.92892159375, Milliliter, Teaspoon), 1.0);
		assert_float_eq!(convert_test(3.0, Teaspoon, Tablespoon), 1.0);
		assert_float_eq!(convert_test(2.0, Tablespoon, FluidOunce), 1.0);
		assert_float_eq!(convert_test(8.0, FluidOunce, Cup), 1.0);
		assert_float_eq!(convert_test(2.0, Cup, Pint), 1.0);
		assert_float_eq!(convert_test(2.0, Pint, Quart), 1.0);
		assert_float_eq!(convert_test(4.0, Quart, Gallon), 1.0);
		assert_float_eq!(convert_test(42.0, Gallon, OilBarrel), 1.0);

		assert_float_eq!(convert_test(1000.0, Milligram, Gram), 1.0);
		assert_float_eq!(convert_test(100.0, Gram, Hectogram), 1.0);
		assert_float_eq!(convert_test(1000.0, Gram, Kilogram), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilogram, MetricTon), 1.0);
		assert_float_eq!(convert_test(0.45359237, Kilogram, Pound), 1.0);
		assert_float_eq!(convert_test(16.0, Ounce, Pound), 1.0);
		assert_float_eq!(convert_test(14.0, Pound, Stone), 1.0);
		assert_float_eq!(convert_test(2000.0, Pound, ShortTon), 1.0);
		assert_float_eq!(convert_test(2240.0, Pound, LongTon), 1.0);

		assert_float_eq!(convert_test(1000.0, Bit, Kilobit), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilobit, Megabit), 1.0);
		assert_float_eq!(convert_test(1000.0, Megabit, Gigabit), 1.0);
		assert_float_eq!(convert_test(1000.0, Gigabit, Terabit), 1.0);
		assert_float_eq!(convert_test(1000.0, Terabit, Petabit), 1.0);
		assert_float_eq!(convert_test(1000.0, Petabit, Exabit), 1.0);
		assert_float_eq!(convert_test(1000.0, Exabit, Zettabit), 1.0);
		assert_float_eq!(convert_test(1000.0, Zettabit, Yottabit), 1.0);
		assert_float_eq!(convert_test(1024.0, Bit, Kibibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Kibibit, Mebibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Mebibit, Gibibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Gibibit, Tebibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Tebibit, Pebibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Pebibit, Exbibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Exbibit, Zebibit), 1.0);
		assert_float_eq!(convert_test(1024.0, Zebibit, Yobibit), 1.0);
		assert_float_eq!(convert_test(8.0, Bit, Byte), 1.0);
		assert_float_eq!(convert_test(1000.0, Byte, Kilobyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilobyte, Megabyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Megabyte, Gigabyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Gigabyte, Terabyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Terabyte, Petabyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Petabyte, Exabyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Exabyte, Zettabyte), 1.0);
		assert_float_eq!(convert_test(1000.0, Zettabyte, Yottabyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Kibibyte, Mebibyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Mebibyte, Gibibyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Gibibyte, Tebibyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Tebibyte, Pebibyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Pebibyte, Exbibyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Exbibyte, Zebibyte), 1.0);
		assert_float_eq!(convert_test(1024.0, Zebibyte, Yobibyte), 1.0);

		assert_float_eq!(convert_test(1000.0, BitsPerSecond, KilobitsPerSecond), 1.0);
		assert_float_eq!(
			convert_test(1000.0, KilobitsPerSecond, MegabitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, MegabitsPerSecond, GigabitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, GigabitsPerSecond, TerabitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, TerabitsPerSecond, PetabitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, PetabitsPerSecond, ExabitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, ExabitsPerSecond, ZettabitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, ZettabitsPerSecond, YottabitsPerSecond),
			1.0
		);
		assert_float_eq!(convert_test(1024.0, BitsPerSecond, KibibitsPerSecond), 1.0);
		assert_float_eq!(
			convert_test(1024.0, KibibitsPerSecond, MebibitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, MebibitsPerSecond, GibibitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, GibibitsPerSecond, TebibitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, TebibitsPerSecond, PebibitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, PebibitsPerSecond, ExbibitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, ExbibitsPerSecond, ZebibitsPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, ZebibitsPerSecond, YobibitsPerSecond),
			1.0
		);
		assert_float_eq!(convert_test(8.0, BitsPerSecond, BytesPerSecond), 1.0);
		assert_float_eq!(
			convert_test(1000.0, BytesPerSecond, KilobytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, KilobytesPerSecond, MegabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, MegabytesPerSecond, GigabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, GigabytesPerSecond, TerabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, TerabytesPerSecond, PetabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, PetabytesPerSecond, ExabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, ExabytesPerSecond, ZettabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, ZettabytesPerSecond, YottabytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, KibibytesPerSecond, MebibytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, MebibytesPerSecond, GibibytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, GibibytesPerSecond, TebibytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, TebibytesPerSecond, PebibytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, PebibytesPerSecond, ExbibytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, ExbibytesPerSecond, ZebibytesPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1024.0, ZebibytesPerSecond, YobibytesPerSecond),
			1.0
		);

		assert_float_eq!(convert_test(1000.0, Flop, KiloFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, KiloFlop, MegaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, MegaFlop, GigaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, GigaFlop, TeraFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, TeraFlop, PetaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, PetaFlop, ExaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, ExaFlop, ZettaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, ZettaFlop, YottaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, YottaFlop, RonnaFlop), 1.0);
		assert_float_eq!(convert_test(1000.0, RonnaFlop, QuettaFlop), 1.0);

		assert_float_eq!(convert_test(1000.0, FlopPerSecond, KiloFlopPerSecond), 1.0);
		assert_float_eq!(
			convert_test(1000.0, KiloFlopPerSecond, MegaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, MegaFlopPerSecond, GigaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, GigaFlopPerSecond, TeraFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, TeraFlopPerSecond, PetaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, PetaFlopPerSecond, ExaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, ExaFlopPerSecond, ZettaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, ZettaFlopPerSecond, YottaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, YottaFlopPerSecond, RonnaFlopPerSecond),
			1.0
		);
		assert_float_eq!(
			convert_test(1000.0, RonnaFlopPerSecond, QuettaFlopPerSecond),
			1.0
		);

		assert_float_eq!(convert_test(1000.0, Millijoule, Joule), 1.0);
		assert_float_eq!(convert_test(1000.0, Joule, Kilojoule), 1.0);
		assert_float_eq!(convert_test(1.0, NewtonMeter, Joule), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilojoule, Megajoule), 1.0);
		assert_float_eq!(convert_test(1000.0, Megajoule, Gigajoule), 1.0);
		assert_float_eq!(convert_test(1000.0, Gigajoule, Terajoule), 1.0);
		assert_float_eq!(convert_test(4.1868, Joule, Calorie), 1.0);
		assert_float_eq!(convert_test(1000.0, Calorie, KiloCalorie), 1.0);
		assert_float_eq!(convert_test(1055.05585262, Joule, BritishThermalUnit), 1.0);
		assert_float_eq!(convert_test(3600.0, Joule, WattHour), 1.0);
		assert_float_eq!(convert_test(1000.0, WattHour, KilowattHour), 1.0);
		assert_float_eq!(convert_test(1000.0, KilowattHour, MegawattHour), 1.0);
		assert_float_eq!(convert_test(1000.0, MegawattHour, GigawattHour), 1.0);
		assert_float_eq!(convert_test(1000.0, GigawattHour, TerawattHour), 1.0);
		assert_float_eq!(convert_test(1000.0, TerawattHour, PetawattHour), 1.0);

		assert_float_eq!(convert_test(1000.0, Milliwatt, Watt), 1.0);
		assert_float_eq!(convert_test(1000.0, Watt, Kilowatt), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilowatt, Megawatt), 1.0);
		assert_float_eq!(convert_test(1000.0, Megawatt, Gigawatt), 1.0);
		assert_float_eq!(convert_test(1000.0, Gigawatt, Terawatt), 1.0);
		assert_float_eq!(convert_test(1000.0, Terawatt, Petawatt), 1.0);
		assert_float_eq!(
			convert_test(
				17.5842642103333333333333333333333333334,
				Watt,
				BritishThermalUnitsPerMinute
			),
			1.0
		);
		assert_float_eq!(
			convert_test(
				60.0,
				BritishThermalUnitsPerHour,
				BritishThermalUnitsPerMinute
			),
			1.0
		);
		assert_float_eq!(convert_test(745.6998715822702, Watt, Horsepower), 1.0);
		assert_float_eq!(convert_test(735.49875, Watt, MetricHorsepower), 1.0);

		assert_float_eq!(convert_test(1000.0, Milliampere, Ampere), 1.0);
		assert_float_eq!(convert_test(1000.0, Ampere, Kiloampere), 1.0);
		assert_float_eq!(convert_test(10.0, Ampere, Abampere), 1.0);

		assert_float_eq!(convert_test(1000.0, Milliohm, Ohm), 1.0);
		assert_float_eq!(convert_test(1000.0, Ohm, Kiloohm), 1.0);

		assert_float_eq!(convert_test(1000.0, Millivolt, Volt), 1.0);
		assert_float_eq!(convert_test(1000.0, Volt, Kilovolt), 1.0);

		assert_float_eq!(convert_test(1000.0, Millinewton, Newton), 1.0);
		assert_float_eq!(convert_test(1000.0, Newton, Kilonewton), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilonewton, Meganewton), 1.0);
		assert_float_eq!(convert_test(100000.0, Dyne, Newton), 1.0);
		assert_float_eq!(convert_test(4.4482216152605, Newton, PoundForce), 1.0);

		assert_float_eq!(convert_test(1000.0, Pascal, Kilopascal), 1.0);
		assert_float_eq!(convert_test(101325.0, Pascal, Atmosphere), 1.0);
		assert_float_eq!(convert_test(100.0, Pascal, Millibar), 1.0);
		assert_float_eq!(convert_test(1000.0, Millibar, Bar), 1.0);
		assert_float_eq!(convert_test(3386.389, Pascal, InchOfMercury), 1.0);
		assert_float_eq!(
			convert_test(6894.757293168361, Pascal, PoundsPerSquareInch),
			1.0
		);
		assert_float_eq!(
			convert_test(133.322368421052631578947368421052631579, Pascal, Torr),
			1.0
		);

		assert_float_eq!(convert_test(1000.0, Hertz, Kilohertz), 1.0);
		assert_float_eq!(convert_test(1000.0, Kilohertz, Megahertz), 1.0);
		assert_float_eq!(convert_test(1000.0, Megahertz, Gigahertz), 1.0);
		assert_float_eq!(convert_test(1000.0, Gigahertz, Terahertz), 1.0);
		assert_float_eq!(convert_test(1000.0, Terahertz, Petahertz), 1.0);
		assert_float_eq!(convert_test(60.0, Hertz, RevolutionsPerMinute), 1.0);

		// assert_float_eq!(convert_test(3.6, KilometersPerHour, MetersPerSecond), 1.0);
		assert_float_eq!(convert_test(0.3048, MetersPerSecond, FeetPerSecond), 1.0);
		// assert_float_eq!(convert_test(1.609344, KilometersPerHour, MilesPerHour), 1.0);
		// assert_float_eq!(convert_test(1.852, KilometersPerHour, Knot), 1.0);

		assert_float_eq!(convert_test(274.15, Kelvin, Celsius), 1.0);
		assert_float_eq!(convert_test(300.0, Kelvin, Fahrenheit), 80.33);
		assert_float_eq!(convert_test(-272.15, Celsius, Kelvin), 1.0);
		assert_float_eq!(convert_test(-15.0, Celsius, Fahrenheit), 5.0);
		assert_float_eq!(convert_test(80.33, Fahrenheit, Kelvin), 300.0);
		assert_float_eq!(convert_test(5.0, Fahrenheit, Celsius), -15.0);
	}
}
