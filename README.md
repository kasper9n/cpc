# cpc

calculation + conversion

cpc calculates complex strings of math, with support for units and conversion. 128-bit decimal floating points are used for high accuracy.

For example `1tonne * 1sqm / 2second^3 / 5ampere` results in `100 volts`.

[![Crates.io](https://img.shields.io/crates/v/cpc.svg)](https://crates.io/crates/cpc)
[![Documentation](https://docs.rs/cpc/badge.svg)](https://docs.rs/cpc)

[List of all supported units](https://docs.rs/cpc/latest/cpc/units/enum.Unit.html)

## [Web Interface](https://cpc.kasper.space)

Try it out at [cpc.kasper.space](https://cpc.kasper.space)

## CLI Installation
Install using `cargo`:
```
cargo install cpc
```

To install it manually, grab the appropriate binary from the [GitHub Releases page](https://github.com/kasper9n/cpc/releases) and place it wherever you normally place binaries on your OS.

## CLI Usage
```
cpc '2h/3 to min'
```

## Examples
```
3 + 4 * 2

6'3" to cm

1tonne * 1sqm / 2s^3 / 5a

(7 % 4)km to light years

10m/2s * 5 trillion s

1 lightyear * 0.001mm in km2

1m/s + 1mi/5h in kilometers per h

round(sqrt(2)^4)! liters

10% of abs(sin(pi)) horsepower to watts
```

## Supported unit types
- Normal numbers
- Area
- Currency
- Digital storage (bytes etc)
- Electric current
- Energy
- FLOPS
- Frequency
- Length
- Mass
- Power
- Pressure
- Resistance
- Speed
- Temperature
- Time
- Voltage
- Volume

## API Installation
Add `cpc` as a dependency in `Cargo.toml`.

## API Usage

```rust
use cpc::eval;
use cpc::units::Unit;

match eval("3m + 1cm", true, false) {
    Ok(answer) => {
        // answer: Number { value: 301, unit: Unit::Centimeter }
        println!("Evaluated value: {} {:?}", answer.value, answer.unit)
    },
    Err(e) => {
        println!("{e}")
    }
}
```

## Accuracy
Inexact results are always indicated with `≈`. 128-bit Decimal Floating Point (d128) numbers are used for high accuracy, and prevents most floating-point errors.

## Dev Instructions

### Get started
Install [Rust](https://www.rust-lang.org).

Run cpc with a CLI argument as input:
```
cargo run -- '100ms to s'
```

Run in verbose mode, which shows some extra logs:
```
cargo run -- '100ms to s' --verbose
```

Run tests:
```
cargo test
```

Build:
```
cargo build
```

### Adding a unit

Nice resources for adding units:
- https://wolframalpha.com
- https://github.com/ryantenney/gnu-units/blob/master/units.dat
- https://support.google.com/websearch/answer/3284611 (unit list)
- https://translatorscafe.com/unit-converter (unit conversion)
- https://calculateme.com (unit conversion)
- https://wikipedia.org

#### 1. Add the unit
In `src/units.rs`, units are specified like this:
```rs
pub enum UnitType {
  Time,
  // etc
}

// ...

create_units!(
  Nanosecond:         (Time, d!(1), "nanosecond", "nanoseconds"),
  Microsecond:        (Time, d!(1000), "microsecond", "microseconds"),
  // etc
)
```

The number associated with a unit is it's "weight". For example, if a second's weight is `1`, then a minute's weight is `60`.

#### 2. Add a test for the unit
Make sure to also add a test for each unit. The tests look like this:
```rs
assert_eq!(convert_test(1000.0, Meter, Kilometer), 1.0);
```
Basically, 1000 Meter == 1 Kilometer.

#### 3. Add the unit to the lexer
Text is turned into tokens (some of which are units) in `lexer.rs`. Here's one example:
```rs
// ...
match string {
  "h" | "hr" | "hrs" | "hour" | "hours" => tokens.push(Token::Unit(Hour)),
  // etc
}
// ...
```

### Potential Improvements
- Fractional numbers (to make `1/3*2*3` accurate)
- E notation, like 2E+10
- Unit types
  - Timezones
  - Binary/octal/decimal/hexadecimal/base32/base64
  - Fuel consumption
  - Color codes
  - Force
  - Roman numerals
  - Angles
  - Flow rate

### Releasing a new version

1. Update `CHANGELOG.md`
2. Bump the version number in `Cargo.toml`
3. Run `cargo test`
4. Create a git tag in format `v#.#.#`
5. Add release notes to the generated GitHub release and publish it
6. Run `cargo publish`
