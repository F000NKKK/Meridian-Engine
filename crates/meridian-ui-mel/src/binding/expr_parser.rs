//! [`ExprParser`]: recursive descent over an `x:Bind` expression.

use super::compare_operator::CompareOperator;
use super::expr::Expr;
use super::expr_error::{ExprError, ExprErrorKind};
use super::literal::Literal;

pub(crate) struct ExprParser<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> ExprParser<'a> {
    pub(crate) fn parse(text: &'a str) -> Result<Expr, ExprError> {
        let mut parser = Self { text, pos: 0 };
        let expr = parser.comparison()?;
        parser.skip_whitespace();
        match parser.peek() {
            None => Ok(expr),
            Some(ch) => Err(parser.error(ExprErrorKind::Unexpected(ch))),
        }
    }

    fn error(&self, kind: ExprErrorKind) -> ExprError {
        ExprError {
            kind,
            offset: self.pos,
        }
    }

    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn skip_whitespace(&mut self) {
        let rest = self.rest();
        self.pos += rest.len() - rest.trim_start().len();
    }

    /// Consumes `token` (after whitespace) if it is next.
    fn eat(&mut self, token: &str) -> bool {
        self.skip_whitespace();
        if self.rest().starts_with(token) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }

    fn comparison(&mut self) -> Result<Expr, ExprError> {
        let left = self.unary()?;
        let operator = if self.eat("==") {
            CompareOperator::Equal
        } else if self.eat("!=") {
            CompareOperator::NotEqual
        } else {
            return Ok(left);
        };
        let right = self.unary()?;
        Ok(Expr::Compare {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }

    fn unary(&mut self) -> Result<Expr, ExprError> {
        if self.eat("!") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        let mut expr = self.primary()?;
        loop {
            let optional = if self.eat("?.") {
                true
            } else if self.eat(".") {
                false
            } else {
                return Ok(expr);
            };
            let name = self.name()?;
            let arguments = self.arguments()?;
            expr = Expr::Access {
                target: Box::new(expr),
                name,
                arguments,
                optional,
            };
        }
    }

    fn primary(&mut self) -> Result<Expr, ExprError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(self.error(ExprErrorKind::UnexpectedEnd)),
            Some('(') => {
                self.pos += 1;
                let inner = self.comparison()?;
                self.expect(')')?;
                Ok(inner)
            }
            Some('\'') => self.text_literal(),
            Some(ch) if ch.is_ascii_digit() || ch == '-' => self.number(),
            Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => self.path(),
            Some(ch) => Err(self.error(ExprErrorKind::Unexpected(ch))),
        }
    }

    fn expect(&mut self, ch: char) -> Result<(), ExprError> {
        self.skip_whitespace();
        match self.peek() {
            Some(found) if found == ch => {
                self.pos += ch.len_utf8();
                Ok(())
            }
            Some(found) => Err(self.error(ExprErrorKind::Unexpected(found))),
            None => Err(self.error(ExprErrorKind::UnexpectedEnd)),
        }
    }

    /// An identifier: `[A-Za-z_][A-Za-z0-9_]*`.
    fn name(&mut self) -> Result<String, ExprError> {
        self.skip_whitespace();
        let rest = self.rest();
        let length = rest
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .unwrap_or(rest.len());
        if length == 0 || rest.starts_with(|ch: char| ch.is_ascii_digit()) {
            return Err(self.error(ExprErrorKind::ExpectedName));
        }
        self.pos += length;
        Ok(rest[..length].to_string())
    }

    /// `name`, `name(..)`, `true`/`false`, or a `::` path.
    fn path(&mut self) -> Result<Expr, ExprError> {
        let mut segments = vec![self.name()?];
        while self.rest().starts_with("::") {
            self.pos += 2;
            segments.push(self.name()?);
        }
        let arguments = self.arguments()?;
        if segments.len() > 1 {
            return Ok(Expr::Path {
                segments,
                arguments,
            });
        }
        let name = segments.remove(0);
        Ok(match (name.as_str(), &arguments) {
            ("true", None) => Expr::Literal(Literal::Bool(true)),
            ("false", None) => Expr::Literal(Literal::Bool(false)),
            _ => Expr::Member { name, arguments },
        })
    }

    /// `(a, b)` right after a name, if present.
    fn arguments(&mut self) -> Result<Option<Vec<Expr>>, ExprError> {
        if !self.rest().starts_with('(') {
            return Ok(None);
        }
        self.pos += 1;
        let mut arguments = Vec::new();
        if self.eat(")") {
            return Ok(Some(arguments));
        }
        loop {
            arguments.push(self.comparison()?);
            if self.eat(")") {
                return Ok(Some(arguments));
            }
            self.expect(',')?;
        }
    }

    fn text_literal(&mut self) -> Result<Expr, ExprError> {
        let start = self.pos;
        self.pos += 1;
        let Some(length) = self.rest().find('\'') else {
            self.pos = start;
            return Err(self.error(ExprErrorKind::UnterminatedText));
        };
        let text = self.rest()[..length].to_string();
        self.pos += length + 1;
        Ok(Expr::Literal(Literal::Text(text)))
    }

    fn number(&mut self) -> Result<Expr, ExprError> {
        let start = self.pos;
        let rest = self.rest();
        let length = rest
            .char_indices()
            .find(|&(index, ch)| !(ch.is_ascii_digit() || ch == '.' || (index == 0 && ch == '-')))
            .map_or(rest.len(), |(index, _)| index);
        let text = &rest[..length];
        self.pos += length;
        let literal = if text.parse::<i64>().is_ok() {
            Literal::Integer(text.to_string())
        } else if text.parse::<f64>().is_ok() {
            Literal::Float(text.to_string())
        } else {
            self.pos = start;
            return Err(self.error(ExprErrorKind::InvalidNumber(text.to_string())));
        };
        Ok(Expr::Literal(literal))
    }
}

#[cfg(test)]
mod tests;
