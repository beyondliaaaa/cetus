//! Unit registry and expression parser.
//!
//! Syntax accepted by [`UnitRegistry::parse`] (§5.1):
//!
//! ```text
//! expr    := term (('*' | '/') term)*
//! term    := atom ('^' exponent)?
//! atom    := IDENT | NUMBER | '(' expr ')'
//! exponent:= INT | '(' INT '/' INT ')'
//! ```
//!
//! Examples: `kW*h`, `W/m^2`, `kg*m/s^2`, `1/s`, `mol/(L*s)`, `"1"`.
//! Whitespace is not allowed inside an expression; the empty string and `"1"`
//! both mean the dimensionless unit.

use indexmap::IndexMap;

use crate::dim::{BaseDim, Dim};
use crate::rational::Rational;
use crate::unit::{Unit, UnitError};

/// A set of named units plus SI prefixes.
///
/// The default registry contains the SI base and derived units, common
/// non-SI units used in the content, and the custom base dimensions `person`,
/// `item`, `currency` and `bit` (K14).
#[derive(Debug, Clone)]
pub struct UnitRegistry {
    units: IndexMap<String, Unit>,
    prefixes: Vec<(&'static str, f64)>,
}

impl Default for UnitRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitRegistry {
    /// Builds the built-in registry.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            units: IndexMap::new(),
            prefixes: prefixes(),
        };

        let base = [
            ("m", BaseDim::Length),
            ("kg", BaseDim::Mass),
            ("s", BaseDim::Time),
            ("A", BaseDim::Current),
            ("K", BaseDim::Temperature),
            ("mol", BaseDim::Amount),
            ("cd", BaseDim::Luminous),
        ];
        for (symbol, dim) in base {
            registry
                .units
                .insert(symbol.to_owned(), Unit::linear(symbol, Dim::base(dim), 1.0));
        }

        registry.derive("Hz", "1/s", 1.0);
        registry.derive("N", "kg*m/s^2", 1.0);
        registry.derive("Pa", "N/m^2", 1.0);
        registry.derive("J", "N*m", 1.0);
        registry.derive("W", "J/s", 1.0);
        registry.derive("C", "A*s", 1.0);
        registry.derive("V", "W/A", 1.0);
        registry.derive("F", "C/V", 1.0);
        registry.derive("Ω", "V/A", 1.0);
        registry.derive("S", "1/Ω", 1.0);
        registry.derive("Wb", "V*s", 1.0);
        registry.derive("T", "Wb/m^2", 1.0);
        registry.derive("H", "Wb/A", 1.0);
        registry.derive("lx", "cd/m^2", 1.0);
        registry.derive("Bq", "1/s", 1.0);
        registry.derive("Gy", "J/kg", 1.0);
        registry.derive("kat", "mol/s", 1.0);

        let time = Dim::base(BaseDim::Time);
        registry
            .units
            .insert("min".to_owned(), Unit::linear("min", time.clone(), 60.0));
        registry
            .units
            .insert("h".to_owned(), Unit::linear("h", time.clone(), 3600.0));
        registry.units.insert(
            "day".to_owned(),
            Unit::linear("day", time.clone(), 86_400.0),
        );
        registry.units.insert(
            "week".to_owned(),
            Unit::linear("week", time.clone(), 7.0 * 86_400.0),
        );
        registry.units.insert(
            "month".to_owned(),
            Unit::linear("month", time.clone(), 30.4375 * 86_400.0),
        );
        registry.units.insert(
            "year".to_owned(),
            Unit::linear("year", time.clone(), 365.25 * 86_400.0),
        );

        let volume = Dim::base(BaseDim::Length).pow(Rational::from_int(3));
        registry
            .units
            .insert("L".to_owned(), Unit::linear("L", volume, 1e-3));

        let mass = Dim::base(BaseDim::Mass);
        registry
            .units
            .insert("g".to_owned(), Unit::linear("g", mass.clone(), 1e-3));
        registry
            .units
            .insert("t".to_owned(), Unit::linear("t", mass, 1000.0));

        let pressure = &Dim::base(BaseDim::Mass)
            / &(&Dim::base(BaseDim::Length) * &Dim::base(BaseDim::Time).pow(Rational::from_int(2)));
        registry
            .units
            .insert("bar".to_owned(), Unit::linear("bar", pressure.clone(), 1e5));
        registry
            .units
            .insert("atm".to_owned(), Unit::linear("atm", pressure, 101_325.0));

        let energy = &(&Dim::base(BaseDim::Mass)
            * &Dim::base(BaseDim::Length).pow(Rational::from_int(2)))
            / &Dim::base(BaseDim::Time).pow(Rational::from_int(2));
        registry.units.insert(
            "eV".to_owned(),
            Unit::linear("eV", energy.clone(), 1.602_176_634e-19),
        );
        registry
            .units
            .insert("cal".to_owned(), Unit::linear("cal", energy, 4.184));

        let temperature = Dim::base(BaseDim::Temperature);
        registry.units.insert(
            "°C".to_owned(),
            Unit::affine("°C", temperature.clone(), 1.0, 273.15),
        );
        registry.units.insert(
            "°F".to_owned(),
            Unit::affine("°F", temperature, 5.0 / 9.0, 273.15 - 32.0 * 5.0 / 9.0),
        );

        let angle = Unit::linear("rad", Dim::dimensionless(), 1.0).with_angle(true);
        registry.units.insert("rad".to_owned(), angle);
        registry.units.insert(
            "deg".to_owned(),
            Unit::linear("deg", Dim::dimensionless(), core::f64::consts::PI / 180.0)
                .with_angle(true),
        );
        registry.units.insert(
            "%".to_owned(),
            Unit::linear("%", Dim::dimensionless(), 0.01),
        );
        registry.units.insert("1".to_owned(), Unit::one());

        for name in ["person", "item", "currency", "bit"] {
            registry
                .units
                .insert(name.to_owned(), Unit::linear(name, Dim::custom(name), 1.0));
        }

        registry
    }

    fn derive(&mut self, symbol: &str, expression: &str, factor: f64) {
        let parsed = self
            .parse(expression)
            .unwrap_or_else(|error| panic!("built-in unit {symbol}: {error}"));
        self.units.insert(
            symbol.to_owned(),
            Unit::linear(symbol, parsed.dim().clone(), parsed.factor() * factor),
        );
    }

    /// Registers a custom dimension name as a base unit (K14).
    ///
    /// Content repositories call this from `content/units/dimensions.yaml`.
    /// Re-registering an existing name replaces it.
    pub fn register_custom_dimension(&mut self, name: &str) {
        self.units
            .insert(name.to_owned(), Unit::linear(name, Dim::custom(name), 1.0));
    }

    /// Registers a named unit.
    pub fn register(&mut self, symbol: &str, unit: Unit) {
        self.units.insert(symbol.to_owned(), unit);
    }

    /// Looks up a unit by exact name, then by SI prefix + name.
    #[must_use]
    pub fn lookup(&self, symbol: &str) -> Option<Unit> {
        if let Some(unit) = self.units.get(symbol) {
            return Some(unit.clone());
        }
        for (prefix, factor) in &self.prefixes {
            let Some(rest) = symbol.strip_prefix(prefix) else {
                continue;
            };
            if rest.is_empty() {
                continue;
            }
            if let Some(base) = self.units.get(rest) {
                if base.is_affine() {
                    continue;
                }
                return Some(Unit::linear(
                    symbol,
                    base.dim().clone(),
                    base.factor() * factor,
                ));
            }
        }
        None
    }

    /// Parses a unit expression.
    ///
    /// # Errors
    /// Returns [`UnitError`] with a byte offset when the expression is malformed
    /// or references an unknown unit.
    pub fn parse(&self, expression: &str) -> Result<Unit, UnitError> {
        let trimmed = expression.trim();
        if trimmed.is_empty() {
            return Err(UnitError::EmptyExpression);
        }
        if trimmed == "1" || trimmed == "\"1\"" {
            return Ok(Unit::one());
        }
        let mut parser = Parser {
            input: trimmed,
            pos: 0,
            registry: self,
        };
        let unit = parser.parse_expr()?;
        parser.skip_ws();
        if parser.pos != parser.input.len() {
            return Err(UnitError::UnexpectedChar {
                ch: parser.peek().unwrap_or('\0'),
                offset: parser.pos,
            });
        }
        Ok(unit)
    }
}

fn prefixes() -> Vec<(&'static str, f64)> {
    vec![
        ("da", 1e1),
        ("Y", 1e24),
        ("Z", 1e21),
        ("E", 1e18),
        ("P", 1e15),
        ("T", 1e12),
        ("G", 1e9),
        ("M", 1e6),
        ("k", 1e3),
        ("h", 1e2),
        ("d", 1e-1),
        ("c", 1e-2),
        ("m", 1e-3),
        ("µ", 1e-6),
        ("u", 1e-6),
        ("n", 1e-9),
        ("p", 1e-12),
        ("f", 1e-15),
        ("a", 1e-18),
        ("z", 1e-21),
        ("y", 1e-24),
    ]
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    registry: &'a UnitRegistry,
}

impl Parser<'_> {
    fn parse_expr(&mut self) -> Result<Unit, UnitError> {
        let mut unit = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('*') => {
                    self.bump();
                    unit = unit.try_mul(&self.parse_term()?)?;
                }
                Some('/') => {
                    self.bump();
                    unit = unit.try_div(&self.parse_term()?)?;
                }
                _ => return Ok(unit),
            }
        }
    }

    fn parse_term(&mut self) -> Result<Unit, UnitError> {
        let unit = self.parse_atom()?;
        self.skip_ws();
        if self.peek() == Some('^') {
            self.bump();
            let exponent = self.parse_exponent()?;
            return unit.try_pow(exponent);
        }
        Ok(unit)
    }

    fn parse_atom(&mut self) -> Result<Unit, UnitError> {
        self.skip_ws();
        match self.peek() {
            None => Err(UnitError::UnexpectedEnd),
            Some('(') => {
                self.bump();
                let inner = self.parse_expr()?;
                self.skip_ws();
                if self.peek() != Some(')') {
                    return Err(UnitError::UnexpectedEnd);
                }
                self.bump();
                Ok(inner)
            }
            Some(ch) if ch.is_ascii_digit() || ch == '.' => self.parse_number(),
            Some(ch) if is_symbol_char(ch) => {
                let start = self.pos;
                while self.peek().is_some_and(is_symbol_char) {
                    self.bump();
                }
                let symbol = &self.input[start..self.pos];
                self.registry
                    .lookup(symbol)
                    .ok_or_else(|| UnitError::UnknownUnit {
                        name: symbol.to_owned(),
                    })
            }
            Some(ch) => Err(UnitError::UnexpectedChar {
                ch,
                offset: self.pos,
            }),
        }
    }

    fn parse_number(&mut self) -> Result<Unit, UnitError> {
        let start = self.pos;
        while self.peek().is_some_and(|ch| {
            ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' || ch == '+' || ch == '-'
        }) {
            let ch = self.peek().unwrap_or('\0');
            let next = self.input[self.pos + ch.len_utf8()..].chars().next();
            if (ch == 'e' || ch == 'E')
                && !next.is_some_and(|n| n.is_ascii_digit() || n == '+' || n == '-')
            {
                break;
            }
            self.pos += ch.len_utf8();
        }
        let text = &self.input[start..self.pos];
        let value: f64 = text.parse().map_err(|_| UnitError::BadExponent {
            text: text.to_owned(),
        })?;
        Ok(Unit::linear(text, Dim::dimensionless(), value))
    }

    fn parse_exponent(&mut self) -> Result<Rational, UnitError> {
        self.skip_ws();
        if self.peek() == Some('(') {
            self.bump();
            self.skip_ws();
            let start = self.pos;
            while self.peek().is_some_and(|ch| ch != '/' && ch != ')') {
                self.bump();
            }
            let num_text = &self.input[start..self.pos];
            self.skip_ws();
            let mut den_text = "1";
            if self.peek() == Some('/') {
                self.bump();
                self.skip_ws();
                let den_start = self.pos;
                while self.peek().is_some_and(|ch| ch != ')') {
                    self.bump();
                }
                den_text = &self.input[den_start..self.pos];
            }
            self.skip_ws();
            if self.peek() != Some(')') {
                return Err(UnitError::UnexpectedEnd);
            }
            self.bump();
            let num = num_text
                .trim()
                .parse::<i32>()
                .map_err(|_| UnitError::BadExponent {
                    text: num_text.to_owned(),
                })?;
            let den = den_text
                .trim()
                .parse::<i32>()
                .map_err(|_| UnitError::BadExponent {
                    text: den_text.to_owned(),
                })?;
            return Ok(Rational::new(num, den));
        }
        let start = self.pos;
        if self.peek() == Some('-') || self.peek() == Some('+') {
            self.bump();
        }
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump();
        }
        let text = &self.input[start..self.pos];
        text.parse::<i32>()
            .map(Rational::from_int)
            .map_err(|_| UnitError::BadExponent {
                text: text.to_owned(),
            })
    }

    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.pos += ch.len_utf8();
        }
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }
}

fn is_symbol_char(ch: char) -> bool {
    ch.is_alphabetic() || matches!(ch, '°' | '%' | 'µ' | 'Ω' | '_') || ch == '\u{00B5}'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> UnitRegistry {
        UnitRegistry::new()
    }

    #[test]
    fn parses_simple_units() {
        let registry = registry();
        let kw = registry.parse("kW").expect("kW");
        assert_eq!(kw.factor(), 1000.0);
        let kwh = registry.parse("kW*h").expect("kW*h");
        assert_eq!(kwh.factor(), 3.6e6);
        let watt_per_square_metre = registry.parse("W/m^2").expect("W/m^2");
        assert_eq!(
            watt_per_square_metre.dim(),
            &(&Dim::base(BaseDim::Mass) / &Dim::base(BaseDim::Time).pow(Rational::from_int(3)))
        );
    }

    #[test]
    fn parses_inverse_and_parentheses() {
        let registry = registry();
        let per_second = registry.parse("1/s").expect("1/s");
        assert_eq!(
            per_second.dim(),
            &Dim::base(BaseDim::Time).pow(Rational::from_int(-1))
        );
        let molar_per_second = registry.parse("mol/(L*s)").expect("mol/(L*s)");
        assert!(!molar_per_second.dim().is_dimensionless());
    }

    #[test]
    fn parses_fractional_exponents() {
        let registry = registry();
        let root = registry.parse("m^(1/2)").expect("m^(1/2)");
        assert_eq!(
            root.dim(),
            &Dim::base(BaseDim::Length).pow(Rational::new(1, 2))
        );
    }

    #[test]
    fn rejects_unknown_units_with_offset() {
        let registry = registry();
        let error = registry.parse("furlong/s").expect_err("unknown");
        assert!(matches!(error, UnitError::UnknownUnit { .. }));
    }

    #[test]
    fn rejects_affine_in_compound() {
        let registry = registry();
        assert!(registry.parse("°C/s").is_err());
    }

    #[test]
    fn angle_label_is_kept() {
        let registry = registry();
        assert!(registry.parse("rad").expect("rad").is_angle());
        assert!(registry.parse("deg").expect("deg").is_angle());
        assert!(!registry.parse("1").expect("1").is_angle());
    }

    #[test]
    fn custom_dimension_registration() {
        let mut registry = registry();
        registry.register_custom_dimension("worshipper");
        let unit = registry.parse("worshipper/day").expect("custom unit");
        assert_eq!(
            unit.dim().custom_exponent("worshipper"),
            Some(Rational::ONE)
        );
    }
}
