# Changelog

## Next
- Add REPL to the `cpc` CLI
- Add support for numbers with thousands separators `_`, ` ` and ` ` (thin space)
- Format numbers with thousands separator

## 4.2.0 - 2026 Jul 1
- Add ambiguous unit resolution depending on context and user locale.
- Add ambiguous units like `pound`, `deg`, `$` and `kr`
- Add currency symbols like `€` and `kr`
- Add `as` and `into` keywords for unit conversion
- Fix some units not being simplified

## 4.1.0 - 2026 Jun 29
- Add currency units

## 4.0.0 - 2026 Jun 27
- Rewrite units to be exponent-based, making things like `km^5` and `sqkm^-1` possible
- Add generic unit multiplication and division, such as `1kg/1m` and `1m*1kg`
- Add generic unit combinations, such as `1kg/m`, `1km per year` and `m+km`
- Add generic unit simplification. For example, `1kg*1sqm/1s/1s/1s/1a` now becomes `1 volt`
- Add `≈` indicator when a result is approximate
- Add case-sensitive `Bps` units for bytes per second
- Use max precision for inexact unit conversions
- Fix `^` allowing numbers to be raised to the power of units

## 3.1.0 - 2026 Jun 20
- Add FLOP and FLOPS units
- Update `fastnum` dependency
- Fix conversion of `torr`, `btu/s` and `btu/h`
- Fix unit spellings
- Fix spelling of `decares`

## 3.0.0 - 2025 May 30
- Create a [website interface](https://cpc.kasper.space)
- Add wasm version
- Switch to the `fastnum` crate's d128
- Fix `exp` function

## 2.0.0 - 2025 May 30
- Remove the `degrees` keyword which referred to `celcius` by default
- Remove the `default_degrees` argument from `eval()` and `lex()`. Not necessary now that the `degrees` keyword is removed
- Fix trigonometry imprecision
- Fix unnecessary scientific notation

## 1.9.3 - 2023 sep 20
- Fix negative unary `-` always having higher precedence than `^`. This resulted in `-3^2` returning `9` instead of `-9`

## 1.9.2 - 2023 Jul 11
- Fix automatic light year unit not chosen for large distances (@gcomte)

## 1.9.1 - 2023 Mar 30
- Improve formatting of numbers
- Remove unnecessary dependencies (@jqnatividad)

## 1.9.0 - 2022 Dec 30
- Add `marathon` unit
- Add `aarch64` binaries

## 1.8.0 - 2021 Aug 16
- Add support for data transfer rate units (like mb/s)
- Add support for dividing length by speed (like 10 km / 100 kph)
- Fix implicit start/end parentheses

## 1.7.0 - 2021 Jul 14
- Add operator words `plus`, `minus` and `times`
- Add operator phrases `multiplied by` and `divided by`
- Add operator symbol `÷`
- Disallow named number followed by smaller named number (like 1 million thousand)
- Fix/improve parsing of multi-word units
- Fix light second parsed as light year
- Fix `Ω` lexing
- Fix lexing of rpm units

## 1.6.0 - 2021 Jul 3
- Add support for non-US "metre" and "litre" spellings
- Add help menu
- Add `--version` flag
- Freak out instead of ignoring unexpected arguments
- Print errors to STDERR
- Fix decimeter parsed as centimeter

## 1.5.1 - 2021 Jun 10
- Fix numbers unnecessarily displayed in E notation

## 1.5.0 - 2021 Apr 21
- Remove `TokenVector` type
- Rename `--debug` to `--verbose` and `-v`
- Allow CLI flags before input
- Fix panic when input contains only whitespace and/or commas

## 1.4.2 - 2021 Apr 8
- Fix d128 errors due to d128 error status not being cleared

## 1.4.1 - 2021 Apr 8
- Fix panic when input is empty string

## 1.4.0 - 2021 Feb 8
- Made cpc case insensitive
- Switch back to official `decimal` because [decimal#59](https://github.com/alkis/decimal/issues/59) is fixed.

## 1.3.2 - 2021 Feb 8
- Fix incorrect parsing of named numbers `Duodecillion` and greater

## 1.3.1 - 2021 Jan 14
- Fix spelling of `Celsius` (@joseluis)

## 1.3.0 - 2020 Nov 29
- Added unit of mass `Stone`
- Added keyword `pounds-force` (used for `PoundsPerSquareInch`)
- Fixed lexing of `Pound`

## 1.2.0 - 2020 Nov 26
- Added units of electric current
- Added units of voltage
- Added units of resistance
- Added support for `Voltage * ElectricCurrent`
- Added support for `Voltage / ElectricCurrent`
- Added support for `Voltage / Resistance`
- Added support for `Power / ElectricCurrent`
- Added support for `Power / Voltage`
- Added support for `Power * Time`
- Added support for `ElectricCurrent * Resistance`
- Added support for `Energy / Time`
- Fixed dividing a unit by `NoUnit` resulting in `NoUnit`
- Fixed interpreting of `µs`
- Fixed panics caused in Rust `1.48.0` by switching `decimal` dependency to `decimal_fixes_mirror`

## 1.1.0 - 2020 Nov 14
- Added units of frequency
- Added support using foot-inch syntax with addition, like `2"+6'4"`
- Unsupported foot-inch syntax like `(6)'4"` and `6'4!"` now cause errors
- Fixed README.md stating the performance is 1000x slower than it actually is
- Fixed trailing percentage signs being ignored when `allow_trailing_operators` is true
- Fixed error caused by consecutive percentage signs

## 1.0.2 - 2020 Oct 12
- Fix parsing of unit `Quarter` (@ethwu)
- Use division instead of multiplication when dividing numbers of the same unit `Quarter` (@ethwu)

## 1.0.1 - 2020 Aug 20
- Fixed the library not working
- Added documentation comments
- Added docs.rs documentation link
- Various fixes and improvements

## 1.0.0 - 2020 Aug 20
- Initial release
