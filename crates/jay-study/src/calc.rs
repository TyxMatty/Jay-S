//! A small calculator for the chat: the model picks the formula, this computes the numbers
//! (docs/ARQUITETURA.md, "Contas feitas pelo motor"). The input is model output, so the parser is
//! total and bounded: length and nesting are capped, no names exist beyond four functions, every
//! intermediate result must be finite, and nothing is ever executed.

const MAX_LEN: usize = 200;
const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalcError(pub &'static str);

type R = Result<f64, CalcError>;

/// Evaluates an arithmetic expression: numbers (one `,` or `.` each), `+ - * /` (also `× · ÷`), `^`
/// (right-associative), postfix `²`, unary signs, parentheses, `sqrt`/`raiz`/`√` and `ceil`/`teto`.
pub fn eval(expr: &str) -> R {
    if expr.chars().count() > MAX_LEN {
        return Err(CalcError("too long"));
    }
    let mut p = Parser { c: expr.chars().collect(), i: 0, depth: 0 };
    let v = p.expr()?;
    if p.peek().is_some() {
        return Err(CalcError("unexpected character"));
    }
    Ok(v)
}

/// The result as the student reads it: at most 4 decimals, no trailing zeros, decimal comma.
pub fn show(x: f64) -> String {
    show_with(x, false)
}

/// `show`, with a decimal point when `point` (for a student writing in English).
pub fn show_with(x: f64, point: bool) -> String {
    let s = format!("{x:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else if point { s.into() } else { s.replace('.', ",") }
}

fn fin(x: f64) -> R {
    if x.is_finite() { Ok(x) } else { Err(CalcError("not a finite number")) }
}

struct Parser {
    c: Vec<char>,
    i: usize,
    depth: usize,
}

impl Parser {
    fn peek(&mut self) -> Option<char> {
        while self.c.get(self.i).is_some_and(|c| c.is_whitespace()) {
            self.i += 1;
        }
        self.c.get(self.i).copied()
    }

    /// Every way the grammar can recurse (parentheses, signs, `^`, `√`) goes through here, so no
    /// input nests deeper than `MAX_DEPTH`.
    fn nested(&mut self, f: fn(&mut Self) -> R) -> R {
        self.depth += 1;
        let r = if self.depth > MAX_DEPTH { Err(CalcError("too deep")) } else { f(self) };
        self.depth -= 1;
        r
    }

    fn expr(&mut self) -> R {
        let mut v = self.term()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.i += 1;
            let r = self.term()?;
            v = fin(if op == '+' { v + r } else { v - r })?;
        }
        Ok(v)
    }

    fn term(&mut self) -> R {
        let mut v = self.unary()?;
        while let Some(op @ ('*' | '×' | '·' | '/' | '÷')) = self.peek() {
            self.i += 1;
            let r = self.unary()?;
            if matches!(op, '/' | '÷') && r == 0.0 {
                return Err(CalcError("division by zero"));
            }
            v = fin(if matches!(op, '/' | '÷') { v / r } else { v * r })?;
        }
        Ok(v)
    }

    fn unary(&mut self) -> R {
        match self.peek() {
            Some(s @ ('-' | '+')) => {
                self.i += 1;
                let v = self.nested(Self::unary)?;
                Ok(if s == '-' { -v } else { v })
            }
            _ => self.power(),
        }
    }

    /// `a ^ b ^ c` is `a ^ (b ^ c)`, and `-2 ^ 2` is `-(2 ^ 2)`.
    fn power(&mut self) -> R {
        let base = self.postfix()?;
        if self.peek() == Some('^') {
            self.i += 1;
            let e = self.nested(Self::unary)?;
            return fin(base.powf(e));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> R {
        let mut v = self.atom()?;
        while self.peek() == Some('²') {
            self.i += 1;
            v = fin(v * v)?;
        }
        Ok(v)
    }

    fn atom(&mut self) -> R {
        match self.peek() {
            Some(c) if c.is_ascii_digit() => self.number(),
            Some('(') => self.group(),
            Some('√') => {
                self.i += 1;
                sqrt(self.nested(Self::atom)?)
            }
            Some(c) if c.is_alphabetic() => {
                let start = self.i;
                while self.c.get(self.i).is_some_and(|c| c.is_alphabetic()) {
                    self.i += 1;
                }
                let name: String = self.c[start..self.i].iter().collect::<String>().to_lowercase();
                if self.peek() != Some('(') {
                    return Err(CalcError("unknown name"));
                }
                let v = self.group()?;
                match name.as_str() {
                    "sqrt" | "raiz" => sqrt(v),
                    "ceil" | "teto" => fin((v - 1e-9).ceil()), // 1e-9: 100.00000000000001 from float error must not become 101
                    _ => Err(CalcError("unknown name")),
                }
            }
            _ => Err(CalcError("expected a value")),
        }
    }

    fn group(&mut self) -> R {
        self.i += 1; // the "("
        let v = self.nested(Self::expr)?;
        if self.peek() != Some(')') {
            return Err(CalcError("missing )"));
        }
        self.i += 1;
        Ok(v)
    }

    /// Digits with at most one `,` or `.` followed by digits: "1.000,5" and "1,2,3" are errors, not guesses.
    fn number(&mut self) -> R {
        let mut s = String::new();
        let mut sep = false;
        while let Some(&c) = self.c.get(self.i) {
            if c.is_ascii_digit() {
                s.push(c);
            } else if (c == ',' || c == '.') && !sep && self.c.get(self.i + 1).is_some_and(|d| d.is_ascii_digit()) {
                sep = true;
                s.push('.');
            } else {
                break;
            }
            self.i += 1;
        }
        fin(s.parse::<f64>().map_err(|_| CalcError("bad number"))?)
    }
}

fn sqrt(v: f64) -> R {
    if v < 0.0 { Err(CalcError("square root of a negative")) } else { Ok(v.sqrt()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(e: &str) -> String {
        show(eval(e).unwrap_or_else(|x| panic!("{e}: {x:?}")))
    }

    #[test]
    fn the_sample_size_formulas() {
        assert_eq!(ok("1,96^2 * 0,10 * 0,90 / 0,02^2"), "864,36");
        assert_eq!(ok("teto(1,96^2 * 0,10 * 0,90 / 0,02^2)"), "865");
        assert_eq!(ok("(1,96 * 10 / (0,10 * 40))^2"), "24,01");
        assert_eq!(ok("ceil((1.96 * 10 / (0.10 * 40))²)"), "25");
        assert_eq!(ok("(1,96 * 10 / 4)^2"), "24,01");
        // Float noise on a whole number does not round up.
        assert_eq!(ok("teto(0,1 * 3 * 10)"), "3");
    }

    #[test]
    fn precedence_associativity_and_signs() {
        assert_eq!(ok("2 + 3 * 4"), "14");
        assert_eq!(ok("(2 + 3) * 4"), "20");
        assert_eq!(ok("2^3^2"), "512");
        assert_eq!(ok("-2^2"), "-4");
        assert_eq!(ok("2^-1"), "0,5");
        assert_eq!(ok("--3"), "3");
        assert_eq!(ok("+3 - -2"), "5");
        assert_eq!(ok("3²"), "9");
        assert_eq!(ok("2³".replace('³', "^3").as_str()), "8");
        assert_eq!(ok("10 ÷ 4 × 2 · 3"), "15");
        assert_eq!(ok("raiz(16) + sqrt(9) + √25 + √(4)"), "14");
        assert_eq!(ok("1,5 + 1.5"), "3");
        assert_eq!(ok("0,5 - 1"), "-0,5");
    }

    #[test]
    fn what_is_not_a_calculation_is_an_error() {
        for bad in [
            "", "   ", "1.000,5", "1,2,3", "1..2", "5.", ".5", "1,", "2 +", "(1", "1)", "1 2", "x", "pi", "abs(2)", "sqrt 4", "sqrt", "2 = 2", "1 + a", "rm -rf /", "</trecho>", "1/0", "1 / (2 - 2)", "÷2", "sqrt(-1)", "√-4", "9^9^9", "10^400",
            "0^-1", "(-8)^0,5", "1e5", "١٢", "ceil(", "teto()",
        ] {
            assert!(eval(bad).is_err(), "{bad:?} gave {:?}", eval(bad));
        }
        // An intermediate infinity never turns back into a number.
        assert!(eval("1 / 9^9^9").is_err());
        assert!(eval(&format!("{} + 1", "9".repeat(190))).is_ok());
        assert!(eval(&"9".repeat(201)).is_err());
    }

    #[test]
    fn length_and_depth_are_bounded() {
        assert!(eval(&"1+".repeat(100)).is_err());
        assert_eq!(eval(&format!("{}1", "1+".repeat(99))), Ok(100.0));
        assert!(eval(&format!("{}1{}", "(".repeat(30), ")".repeat(30))).is_ok());
        for e in [format!("{}1{}", "(".repeat(90), ")".repeat(90)), format!("{}1", "-".repeat(150)), format!("1{}", "^-1".repeat(60)), format!("{}1", "√".repeat(150)), format!("{}1", "sqrt(".repeat(35)), format!("{}2", "2^".repeat(99))] {
            assert_eq!(eval(&e), Err(CalcError("too deep")), "{e}");
        }
    }

    #[test]
    fn nothing_panics() {
        let alphabet: Vec<char> = "0123456789+-*/^()abxrsqtceilo ,.²√×·÷=é日🙂\u{0}\n".chars().collect();
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..20_000 {
            let len = (next() % 40) as usize;
            let s: String = (0..len).map(|_| alphabet[(next() % alphabet.len() as u64) as usize]).collect();
            let _ = eval(&s);
        }
        for s in ["(".repeat(10_000), "-".repeat(10_000), "2^".repeat(1000), ")".repeat(10_000), "√".repeat(10_000), "²".repeat(10_000), "9".repeat(10_000), "日本語".repeat(500), "🙂".repeat(300)] {
            let _ = eval(&s);
        }
        // Total, not just short: the same sweep cut at the limit.
        assert!(eval(&"2^".repeat(1000)).is_err());
    }

    #[test]
    fn results_are_shown_with_a_decimal_comma() {
        for (x, want) in [(864.36, "864,36"), (24.01, "24,01"), (865.0, "865"), (-0.5, "-0,5"), (0.0, "0"), (-0.00001, "0"), (1.0 / 3.0, "0,3333"), (2.00004, "2"), (1234567.5, "1234567,5"), (100.0, "100")] {
            assert_eq!(show(x), want);
        }
    }

    #[test]
    fn results_can_be_shown_with_a_decimal_point() {
        assert_eq!(show_with(eval("(1.96 * 10 / 4)^2").unwrap(), true), "24.01");
        for (x, want) in [(864.36, "864.36"), (865.0, "865"), (-0.5, "-0.5"), (-0.00001, "0"), (1.0 / 3.0, "0.3333")] {
            assert_eq!(show_with(x, true), want);
        }
    }
}
