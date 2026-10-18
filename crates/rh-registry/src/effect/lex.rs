//! The lexer of §2: identifiers, keywords, numbers, operators, whitespace and comments.
//!
//! The token classes are tried in a fixed order at each position (whitespace or a comment, a
//! number, an identifier, an operator), as the reference's regular expression does. A numeric token
//! runs over every following letter, digit, `_` and `.`, and is `E_FLOAT_LITERAL` unless it is an
//! `Int` or a `Decimal` (`1e3`, `1.`, `.5`, `0x10`, `1_000`). Whitespace is the four ASCII
//! characters space, tab, LF and CR; any other character outside a comment is `E_SYNTAX` (§2,
//! F-1). A comment runs to the next LF whatever it holds.
//!
//! REF: `specs/registry/effect_language_v0.md` §2, §3.2, §10.

use super::{ErrorCode, Rejection, Span};

/// The keywords of §2.
pub(crate) const KEYWORDS: [&str; 15] = [
    "and", "or", "not", "if", "then", "else", "when", "for", "in", "where", "true", "false",
    "none", "div", "mod",
];

/// A token's class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokKind {
    Num,
    Ident,
    Op,
    Kw,
    Eof,
}

/// A token: its class, its text and where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    pub kind: TokKind,
    pub text: String,
    pub span: Span,
}

/// Whitespace: exactly space, tab, line feed and carriage return (§2; F-1, 2026-10-18). Every
/// other character outside a comment is `E_SYNTAX`.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

fn is_num_tail(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

fn is_int(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_decimal(s: &str) -> bool {
    s.split_once('.')
        .is_some_and(|(a, b)| is_int(a) && is_int(b))
}

/// Tokenises a program; the last token is `Eof`.
#[allow(
    clippy::too_many_lines,
    reason = "one scanner loop, in the reference's order of token classes"
)]
pub(crate) fn lex(text: &str) -> Result<Vec<Token>, Rejection> {
    let mut tokens = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let end_of = |i: usize| chars.get(i).map_or(text.len(), |&(b, _)| b);
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        if is_space(c) {
            while i < chars.len() && is_space(chars[i].1) {
                i += 1;
            }
            continue;
        }
        if c == '#' {
            while i < chars.len() && chars[i].1 != '\n' {
                i += 1;
            }
            continue;
        }
        let next_digit = chars.get(i + 1).is_some_and(|&(_, d)| d.is_ascii_digit());
        if c.is_ascii_digit() || (c == '.' && next_digit) {
            let mut j = i + 1;
            if c == '.' {
                j += 1;
            }
            while j < chars.len() && is_num_tail(chars[j].1) {
                j += 1;
            }
            let span = Span {
                start,
                end: end_of(j),
            };
            let word = &text[span.start..span.end];
            if !(is_int(word) || is_decimal(word)) {
                return Err(Rejection::new(
                    ErrorCode::FloatLiteral,
                    span,
                    format!("`{word}` is not an Int or a Decimal"),
                ));
            }
            tokens.push(Token {
                kind: TokKind::Num,
                text: word.to_owned(),
                span,
            });
            i = j;
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].1.is_ascii_alphanumeric() || chars[j].1 == '_') {
                j += 1;
            }
            let span = Span {
                start,
                end: end_of(j),
            };
            let word = &text[span.start..span.end];
            let kind = if KEYWORDS.contains(&word) {
                TokKind::Kw
            } else {
                TokKind::Ident
            };
            tokens.push(Token {
                kind,
                text: word.to_owned(),
                span,
            });
            i = j;
            continue;
        }
        let pair = chars.get(i + 1).map(|&(_, d)| d);
        let two = matches!((c, pair), ('=' | '!' | '<' | '>', Some('=')));
        if two || "-+*/<>().,:;[]".contains(c) {
            let j = if two { i + 2 } else { i + 1 };
            let span = Span {
                start,
                end: end_of(j),
            };
            tokens.push(Token {
                kind: TokKind::Op,
                text: text[span.start..span.end].to_owned(),
                span,
            });
            i = j;
            continue;
        }
        let span = Span {
            start,
            end: end_of(i + 1),
        };
        return Err(Rejection::new(
            ErrorCode::Syntax,
            span,
            format!("unexpected character {c:?}"),
        ));
    }
    tokens.push(Token {
        kind: TokKind::Eof,
        text: String::new(),
        span: Span {
            start: text.len(),
            end: text.len(),
        },
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::{TokKind, lex};
    use crate::effect::ErrorCode;

    fn code(text: &str) -> Option<ErrorCode> {
        lex(text).err().map(|r| r.code)
    }

    #[test]
    fn numbers_are_int_or_decimal_only() {
        for bad in [
            "1e3", "1.", ".5", "0x10", "1_000", "1.5f", "1.5.2", "1..2", "x.5",
        ] {
            assert_eq!(code(bad), Some(ErrorCode::FloatLiteral), "{bad}");
        }
        let toks = lex("12 3.25").unwrap();
        assert_eq!(
            toks.iter().map(|t| t.kind).collect::<Vec<_>>(),
            [TokKind::Num, TokKind::Num, TokKind::Eof]
        );
    }

    #[test]
    fn only_ascii_whitespace_separates_and_other_characters_are_syntax_errors() {
        assert!(lex("a \tand\r\nb").is_ok());
        for c in ['\u{a0}', '\u{85}', '\u{2003}', '\u{3000}', '\u{b}', '\u{c}'] {
            assert_eq!(
                code(&format!("a{c}and b")),
                Some(ErrorCode::Syntax),
                "{c:?}"
            );
        }
        assert!(lex("a # \u{a0}\u{3000}\u{b} anything\nand b").is_ok());
        assert_eq!(code("a \u{e9} b"), Some(ErrorCode::Syntax));
        assert_eq!(code("a = b"), Some(ErrorCode::Syntax));
        assert_eq!(code("!a"), Some(ErrorCode::Syntax));
        assert!(lex("# only a comment\n").is_ok());
        assert!(lex("").is_ok());
    }
}
