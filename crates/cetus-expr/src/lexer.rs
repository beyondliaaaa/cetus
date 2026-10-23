//! Lexer for the Cetus expression language.
//!
//! The language is deliberately small (§5.2). Tokens carry byte spans so that
//! diagnostics can point at the exact source location.

use core::fmt;

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Numeric literal.
    Number(f64),
    /// Identifier: a variable, parameter, stock, `time`, `dt`, or function name.
    Ident(String),
    /// A unit expression written between brackets, e.g. `[W/m^2]`.
    Unit(String),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `==`
    EqEq,
    /// `!=`
    NotEq,
    /// `and`
    And,
    /// `or`
    Or,
    /// `not`
    Not,
    /// `if`
    If,
    /// `then`
    Then,
    /// `else`
    Else,
    /// `true`
    True,
    /// `false`
    False,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// End of input.
    Eof,
}

impl Token {
    /// Human-readable description used in diagnostics.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Number(value) => format!("number `{value}`"),
            Self::Ident(name) => format!("identifier `{name}`"),
            Self::Unit(text) => format!("unit `{text}`"),
            Self::Plus => "`+`".to_owned(),
            Self::Minus => "`-`".to_owned(),
            Self::Star => "`*`".to_owned(),
            Self::Slash => "`/`".to_owned(),
            Self::Caret => "`^`".to_owned(),
            Self::Lt => "`<`".to_owned(),
            Self::Le => "`<=`".to_owned(),
            Self::Gt => "`>`".to_owned(),
            Self::Ge => "`>=`".to_owned(),
            Self::EqEq => "`==`".to_owned(),
            Self::NotEq => "`!=`".to_owned(),
            Self::And => "`and`".to_owned(),
            Self::Or => "`or`".to_owned(),
            Self::Not => "`not`".to_owned(),
            Self::If => "`if`".to_owned(),
            Self::Then => "`then`".to_owned(),
            Self::Else => "`else`".to_owned(),
            Self::True => "`true`".to_owned(),
            Self::False => "`false`".to_owned(),
            Self::LParen => "`(`".to_owned(),
            Self::RParen => "`)`".to_owned(),
            Self::Comma => "`,`".to_owned(),
            Self::LBracket => "`[`".to_owned(),
            Self::RBracket => "`]`".to_owned(),
            Self::Eof => "end of input".to_owned(),
        }
    }
}

/// A token with its byte span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    /// The token.
    pub token: Token,
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character.
    pub end: usize,
}

/// A lexer error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LexError {
    /// An unexpected character.
    #[error("unexpected character `{ch}` at byte {offset}")]
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Byte offset.
        offset: usize,
    },
    /// A number literal could not be parsed.
    #[error("bad number literal `{text}` at byte {offset}")]
    BadNumber {
        /// The offending text.
        text: String,
        /// Byte offset.
        offset: usize,
    },
    /// A unit bracket was not closed.
    #[error("unterminated unit expression at byte {offset}")]
    UnterminatedUnit {
        /// Byte offset of the opening bracket.
        offset: usize,
    },
    /// A single `=` was used instead of `==`.
    #[error("unexpected `=` at byte {offset}; use `==` for comparison")]
    SingleEquals {
        /// Byte offset.
        offset: usize,
    },
    /// A bare `!` was used instead of `!=` or `not`.
    #[error("unexpected `!` at byte {offset}; use `!=` or `not`")]
    LoneBang {
        /// Byte offset.
        offset: usize,
    },
}

/// Tokenises `source`.
///
/// # Errors
/// Returns a [`LexError`] with a byte offset on malformed input.
pub fn lex(source: &str) -> Result<Vec<Spanned>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((offset, ch)) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        let start = offset;
        let token = match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '^' => Token::Caret,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '<' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next();
                    Token::Le
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next();
                    Token::Ge
                } else {
                    Token::Gt
                }
            }
            '=' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next();
                    Token::EqEq
                } else {
                    return Err(LexError::SingleEquals { offset });
                }
            }
            '!' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next();
                    Token::NotEq
                } else {
                    return Err(LexError::LoneBang { offset });
                }
            }
            ch if ch.is_ascii_digit() || ch == '.' => {
                let mut end = offset + ch.len_utf8();
                let mut text = ch.to_string();
                while let Some((_, c)) = chars.peek().copied() {
                    let is_exponent_sign = matches!(c, '+' | '-')
                        && text
                            .chars()
                            .last()
                            .is_some_and(|last| last == 'e' || last == 'E');
                    if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || is_exponent_sign {
                        text.push(c);
                        end += c.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                let value: f64 = text.parse().map_err(|_| LexError::BadNumber {
                    text: text.clone(),
                    offset,
                })?;
                tokens.push(Spanned {
                    token: Token::Number(value),
                    start,
                    end,
                });
                continue;
            }
            ch if is_ident_start(ch) => {
                let mut text = ch.to_string();
                let mut end = offset + ch.len_utf8();
                while let Some((_, c)) = chars.peek().copied() {
                    if is_ident_continue(c) {
                        text.push(c);
                        end += c.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Spanned {
                    token: keyword_or_ident(text),
                    start,
                    end,
                });
                continue;
            }
            other => return Err(LexError::UnexpectedChar { ch: other, offset }),
        };
        let end = start + token_width(&token, source, start);
        tokens.push(Spanned { token, start, end });
    }

    tokens.push(Spanned {
        token: Token::Eof,
        start: source.len(),
        end: source.len(),
    });
    Ok(tokens)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '$'
}

fn keyword_or_ident(text: String) -> Token {
    match text.as_str() {
        "and" => Token::And,
        "or" => Token::Or,
        "not" => Token::Not,
        "if" => Token::If,
        "then" => Token::Then,
        "else" => Token::Else,
        "true" => Token::True,
        "false" => Token::False,
        _ => Token::Ident(text),
    }
}

fn token_width(token: &Token, source: &str, start: usize) -> usize {
    match token {
        Token::Le | Token::Ge | Token::EqEq | Token::NotEq => 2,
        _ => source[start..].chars().next().map_or(0, char::len_utf8),
    }
}

impl fmt::Display for Spanned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.token.describe(),
            self.start,
            self.end
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> Vec<Token> {
        lex(source)
            .expect("lex")
            .into_iter()
            .map(|span| span.token)
            .collect()
    }

    #[test]
    fn lexes_arithmetic() {
        assert_eq!(
            tokens("2 * pi * time / 3"),
            vec![
                Token::Number(2.0),
                Token::Star,
                Token::Ident("pi".to_owned()),
                Token::Star,
                Token::Ident("time".to_owned()),
                Token::Slash,
                Token::Number(3.0),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_comparison_and_logic() {
        assert_eq!(
            tokens("a <= b and not c != d"),
            vec![
                Token::Ident("a".to_owned()),
                Token::Le,
                Token::Ident("b".to_owned()),
                Token::And,
                Token::Not,
                Token::Ident("c".to_owned()),
                Token::NotEq,
                Token::Ident("d".to_owned()),
                Token::Eof
            ]
        );
    }

    #[test]
    fn lexes_numbers_with_exponent() {
        assert_eq!(tokens("1.5e-3"), vec![Token::Number(1.5e-3), Token::Eof]);
    }

    #[test]
    fn rejects_single_equals() {
        let error = lex("a = b").expect_err("single =");
        assert!(matches!(error, LexError::SingleEquals { .. }));
    }
}
