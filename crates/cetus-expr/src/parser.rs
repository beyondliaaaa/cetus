//! Abstract syntax tree and Pratt parser.
//!
//! Precedence (loosest to tightest): `or`, `and`, comparisons, `+ -`, `* /`,
//! unary `-` / `not`, `^` (right associative), call/atom. `if c then a else b`
//! is a prefix form and binds loosest.

use crate::lexer::{LexError, Spanned, Token, lex};

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `^`
    Pow,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `and`
    And,
    /// `or`
    Or,
}

impl BinOp {
    /// The source spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Pow => "^",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::And => "and",
            Self::Or => "or",
        }
    }

    /// Whether this is a comparison operator.
    #[must_use]
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Lt | Self::Le | Self::Gt | Self::Ge | Self::Eq | Self::Ne
        )
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    /// `-`
    Neg,
    /// `not`
    Not,
}

/// A byte span in the expression source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start byte.
    pub start: usize,
    /// End byte.
    pub end: usize,
}

impl Span {
    /// Builds a span.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// An expression: a node plus its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// The node.
    pub kind: ExprKind,
    /// Byte span in the source.
    pub span: Span,
}

/// An expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// Numeric literal with an optional unit expression (`3.2[m/s]`).
    Number {
        /// The literal value.
        value: f64,
        /// The unit expression text, if any.
        unit: Option<String>,
    },
    /// Boolean literal.
    Bool(bool),
    /// Identifier: variable, parameter, stock, `time`, `dt`, or constant.
    Ident(String),
    /// Unary operation.
    Unary {
        /// Operator.
        op: UnOp,
        /// Operand.
        operand: Box<Expr>,
    },
    /// Binary operation.
    Binary {
        /// Operator.
        op: BinOp,
        /// Left operand.
        lhs: Box<Expr>,
        /// Right operand.
        rhs: Box<Expr>,
    },
    /// Conditional expression.
    If {
        /// Condition.
        cond: Box<Expr>,
        /// Value when the condition is true.
        then: Box<Expr>,
        /// Value otherwise.
        otherwise: Box<Expr>,
    },
    /// Function call.
    Call {
        /// Function name as written.
        name: String,
        /// Arguments.
        args: Vec<Expr>,
    },
}

impl Expr {
    fn new(kind: ExprKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// A parse error with a byte span.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message} (at byte {start})")]
pub struct ParseError {
    /// What went wrong.
    pub message: String,
    /// Start byte.
    pub start: usize,
    /// End byte.
    pub end: usize,
}

impl ParseError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            start: span.start,
            end: span.end,
        }
    }
}

/// Parses an expression.
///
/// # Errors
/// Returns [`ParseError`] on lexical or syntactic errors.
pub fn parse(source: &str) -> Result<Expr, ParseError> {
    let tokens = lex(source).map_err(|error| lex_error(&error))?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        source,
    };
    let expr = parser.parse_expr(0)?;
    if !matches!(parser.peek(), Token::Eof) {
        let span = parser.current_span();
        return Err(ParseError::new(
            format!("unexpected {}", parser.peek().describe()),
            span,
        ));
    }
    Ok(expr)
}

fn lex_error(error: &LexError) -> ParseError {
    let (message, start) = match &error {
        LexError::UnexpectedChar { ch, offset } => {
            (format!("unexpected character `{ch}`"), *offset)
        }
        LexError::BadNumber { text, offset } => (format!("bad number literal `{text}`"), *offset),
        LexError::UnterminatedUnit { offset } => {
            ("unterminated unit expression".to_owned(), *offset)
        }
        LexError::SingleEquals { offset } => (
            "unexpected `=`; use `==` for comparison".to_owned(),
            *offset,
        ),
        LexError::LoneBang { offset } => ("unexpected `!`; use `!=` or `not`".to_owned(), *offset),
    };
    ParseError {
        message,
        start,
        end: start + 1,
    }
}

struct Parser<'a> {
    tokens: Vec<Spanned>,
    pos: usize,
    source: &'a str,
}

impl Parser<'_> {
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        while let Some((op, left_bp, right_bp)) = self.precedence() {
            if left_bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr(right_bp)?;
            let span = Span::new(lhs.span.start, rhs.span.end);
            lhs = Expr::new(
                ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            );
        }
        Ok(lhs)
    }

    fn precedence(&self) -> Option<(BinOp, u8, u8)> {
        let (op, bp, right_assoc) = match self.peek() {
            Token::Or => (BinOp::Or, 1, false),
            Token::And => (BinOp::And, 2, false),
            Token::Lt => (BinOp::Lt, 3, false),
            Token::Le => (BinOp::Le, 3, false),
            Token::Gt => (BinOp::Gt, 3, false),
            Token::Ge => (BinOp::Ge, 3, false),
            Token::EqEq => (BinOp::Eq, 3, false),
            Token::NotEq => (BinOp::Ne, 3, false),
            Token::Plus => (BinOp::Add, 4, false),
            Token::Minus => (BinOp::Sub, 4, false),
            Token::Star => (BinOp::Mul, 5, false),
            Token::Slash => (BinOp::Div, 5, false),
            Token::Caret => (BinOp::Pow, 7, true),
            _ => return None,
        };
        let right_bp = if right_assoc { bp } else { bp + 1 };
        Some((op, bp, right_bp))
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.current_span();
        match self.peek().clone() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_expr(6)?;
                let span = Span::new(start_span.start, operand.span.end);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnOp::Neg,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_expr(6)?;
                let span = Span::new(start_span.start, operand.span.end);
                Ok(Expr::new(
                    ExprKind::Unary {
                        op: UnOp::Not,
                        operand: Box::new(operand),
                    },
                    span,
                ))
            }
            Token::If => {
                self.advance();
                let cond = self.parse_expr(0)?;
                self.expect(&Token::Then, "expected `then` after the condition")?;
                let then = self.parse_expr(0)?;
                self.expect(&Token::Else, "expected `else` after the true branch")?;
                let otherwise = self.parse_expr(0)?;
                let span = Span::new(start_span.start, otherwise.span.end);
                Ok(Expr::new(
                    ExprKind::If {
                        cond: Box::new(cond),
                        then: Box::new(then),
                        otherwise: Box::new(otherwise),
                    },
                    span,
                ))
            }
            _ => self.parse_atom(),
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.current_span();
        match self.peek().clone() {
            Token::Number(value) => {
                self.advance();
                let unit = if matches!(self.peek(), Token::LBracket) {
                    self.advance();
                    let unit_start = self.current_span().start;
                    let mut end = unit_start;
                    while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                        end = self.current_span().end;
                        self.advance();
                    }
                    if !matches!(self.peek(), Token::RBracket) {
                        return Err(ParseError::new("unterminated unit expression", start_span));
                    }
                    let text = &self.source[unit_start..end];
                    self.advance();
                    Some(text.trim().to_owned())
                } else {
                    None
                };
                let end = self.previous_end();
                Ok(Expr::new(
                    ExprKind::Number { value, unit },
                    Span::new(start_span.start, end),
                ))
            }
            Token::True => {
                self.advance();
                Ok(Expr::new(ExprKind::Bool(true), start_span))
            }
            Token::False => {
                self.advance();
                Ok(Expr::new(ExprKind::Bool(false), start_span))
            }
            Token::Ident(name) => {
                self.advance();
                if matches!(self.peek(), Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Token::RParen) {
                        loop {
                            args.push(self.parse_expr(0)?);
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(&Token::RParen, "expected `)` to close the call")?;
                    let span = Span::new(start_span.start, self.previous_end());
                    return Ok(Expr::new(ExprKind::Call { name, args }, span));
                }
                Ok(Expr::new(ExprKind::Ident(name), start_span))
            }
            Token::LParen => {
                self.advance();
                let inner = self.parse_expr(0)?;
                self.expect(&Token::RParen, "expected `)`")?;
                let span = Span::new(start_span.start, self.previous_end());
                Ok(Expr::new(inner.kind, span))
            }
            other => Err(ParseError::new(
                format!("expected an expression, found {}", other.describe()),
                start_span,
            )),
        }
    }

    fn expect(&mut self, token: &Token, message: &str) -> Result<(), ParseError> {
        if self.peek() == token {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::new(message, self.current_span()))
        }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map_or(&Token::Eof, |span| &span.token)
    }

    fn advance(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map_or(Span::new(0, 0), |span| Span::new(span.start, span.end))
    }

    fn previous_end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map_or(0, |span| span.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_precedence() {
        let expr = parse("a + b * c").expect("parse");
        let ExprKind::Binary { op, rhs, .. } = expr.kind else {
            panic!("expected binary");
        };
        assert_eq!(op, BinOp::Add);
        assert!(matches!(rhs.kind, ExprKind::Binary { op: BinOp::Mul, .. }));
    }

    #[test]
    fn power_is_right_associative() {
        let expr = parse("a ^ b ^ c").expect("parse");
        let ExprKind::Binary { op, rhs, .. } = expr.kind else {
            panic!("expected binary");
        };
        assert_eq!(op, BinOp::Pow);
        assert!(matches!(rhs.kind, ExprKind::Binary { op: BinOp::Pow, .. }));
    }

    #[test]
    fn parses_if_then_else() {
        let expr = parse("if x > 0 then x else -x").expect("parse");
        assert!(matches!(expr.kind, ExprKind::If { .. }));
    }

    #[test]
    fn parses_call() {
        let expr = parse("max(a, 0)").expect("parse");
        let ExprKind::Call { name, args } = expr.kind else {
            panic!("expected call");
        };
        assert_eq!(name, "max");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn parses_units() {
        let expr = parse("3.2[m/s] + 1").expect("parse");
        let ExprKind::Binary { lhs, .. } = expr.kind else {
            panic!("expected binary");
        };
        let ExprKind::Number { unit, .. } = lhs.kind else {
            panic!("expected number");
        };
        assert_eq!(unit.as_deref(), Some("m/s"));
    }

    #[test]
    fn rejects_dangling_operator() {
        assert!(parse("1 +").is_err());
    }

    #[test]
    fn spans_cover_the_whole_expression() {
        let expr = parse("a + b * c").expect("parse");
        assert_eq!(expr.span, Span::new(0, 9));
    }
}
