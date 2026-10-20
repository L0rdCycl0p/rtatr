use crate::md::strip_markdown;
use crate::task::Task;

/// Tatr Query Language expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Tag(String),
    Prop { key: String, value: Option<String> },
    Text(String),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
    Not(Box<Query>),
    True,
}

impl Query {
    pub fn matches(&self, task: &Task) -> bool {
        match self {
            Query::True => true,
            Query::Tag(t) => task.tags.contains(t),
            Query::Prop { key, value } => match (task.props.get(key), value) {
                (Some(_), None) => true,
                (Some(actual), Some(expected)) => actual == expected,
                (None, _) => false,
            },
            Query::Text(needle) => {
                let hay = strip_markdown(&task.body).to_lowercase();
                hay.contains(&needle.to_lowercase())
            }
            Query::And(a, b) => a.matches(task) && b.matches(task),
            Query::Or(a, b) => a.matches(task) || b.matches(task),
            Query::Not(a) => !a.matches(task),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Tag(String),
    Ident(String),
    Str(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
    Eq,
    Eof,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            '=' => { tokens.push(Token::Eq); i += 1; }
            ':' => {
                let start = i + 1;
                i += 1;
                while i < chars.len() && is_ident_char(chars[i]) { i += 1; }
                let tag: String = chars[start..i].iter().collect();
                if tag.is_empty() {
                    return Err("empty tag after ':'".into());
                }
                tokens.push(Token::Tag(tag));
            }
            '"' => {
                let start = i + 1;
                i += 1;
                while i < chars.len() && chars[i] != '"' { i += 1; }
                if i >= chars.len() {
                    return Err("unterminated string".into());
                }
                tokens.push(Token::Str(chars[start..i].iter().collect()));
                i += 1;
            }
            _ if is_ident_char(c) => {
                let start = i;
                while i < chars.len() && is_ident_char(chars[i]) { i += 1; }
                let word: String = chars[start..i].iter().collect();
                match word.as_str() {
                    "and" => tokens.push(Token::And),
                    "or" => tokens.push(Token::Or),
                    "not" => tokens.push(Token::Not),
                    _ => tokens.push(Token::Ident(word)),
                }
            }
            other => return Err(format!("unexpected character: {other:?}")),
        }
    }
    tokens.push(Token::Eof);
    Ok(tokens)
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.'
}

struct Parser { tokens: Vec<Token>, pos: usize }

impl Parser {
    fn peek(&self) -> &Token { &self.tokens[self.pos] }
    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn parse_or(&mut self) -> Result<Query, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::Or) {
            self.bump();
            let right = self.parse_and()?;
            left = Query::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Query, String> {
        let mut left = self.parse_unary()?;
        while matches!(self.peek(), Token::And) {
            self.bump();
            let right = self.parse_unary()?;
            left = Query::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Query, String> {
        if matches!(self.peek(), Token::Not) {
            self.bump();
            Ok(Query::Not(Box::new(self.parse_unary()?)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Query, String> {
        match self.bump() {
            Token::Tag(t) => Ok(Query::Tag(t)),
            Token::Str(s) => Ok(Query::Text(s)),
            Token::LParen => {
                let q = self.parse_or()?;
                match self.bump() {
                    Token::RParen => Ok(q),
                    other => Err(format!("expected ')', got {other:?}")),
                }
            }
            Token::Ident(name) => {
                if matches!(self.peek(), Token::Eq) {
                    self.bump();
                    let value = match self.bump() {
                        Token::Str(s) | Token::Ident(s) | Token::Tag(s) => Some(s),
                        Token::Eof => None,
                        other => return Err(format!("expected value, got {other:?}")),
                    };
                    Ok(Query::Prop { key: name, value })
                } else {
                    Ok(Query::Text(name))
                }
            }
            Token::Eof => Ok(Query::True),
            other => Err(format!("unexpected token: {other:?}")),
        }
    }
}

pub fn parse(input: &str) -> Result<Query, String> {
    if input.trim().is_empty() {
        return Ok(Query::True);
    }
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, pos: 0 };
    let q = parser.parse_or()?;
    if !matches!(parser.peek(), Token::Eof) {
        return Err(format!("unexpected trailing token: {:?}", parser.peek()));
    }
    Ok(q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    fn task(tags: &[&str], body: &str) -> Task {
        Task {
            huid: "20240101-120000".parse().unwrap(),
            tags: tags.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>(),
            props: BTreeMap::new(),
            body: body.to_string(),
            path: PathBuf::new(),
        }
    }

    #[test]
    fn tag_and_not() {
        let q = parse(":a and not :b").unwrap();
        assert!(q.matches(&task(&["a"], "")));
        assert!(!q.matches(&task(&["a", "b"], "")));
    }

    #[test]
    fn precedence() {
        let q = parse(":a and :b or :c").unwrap();
        assert!(q.matches(&task(&["c"], "")));
        assert!(q.matches(&task(&["a", "b"], "")));
        assert!(!q.matches(&task(&["a"], "")));
    }

    #[test]
    fn text_search() {
        let q = parse("\"login bug\"").unwrap();
        assert!(q.matches(&task(&[], "the login bug is back")));
        assert!(!q.matches(&task(&[], "unrelated")));
    }

    #[test]
    fn parens() {
        let q = parse(":a and (:b or :c)").unwrap();
        assert!(q.matches(&task(&["a", "c"], "")));
        assert!(!q.matches(&task(&["a"], "")));
    }
}