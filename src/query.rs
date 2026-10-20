use crate::md::strip_markdown;
use crate::task::{Prop, Task};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

impl CmpOp {
    fn eval(self, a: i64, b: i64) -> bool {
        match self {
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
            CmpOp::Eq => a == b,
            CmpOp::Ne => a != b,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntExpr {
    Priority,
    Lit(i64),
}

impl IntExpr {
    fn eval(&self, task: &Task) -> i64 {
        match self {
            IntExpr::Priority => task.priority as i64,
            IntExpr::Lit(n) => *n,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    True,
    Tag(String),
    Prop { key: String, value: Option<String> },
    Text(String),
    Tagged,
    HuidMatch(String),
    Cmp(CmpOp, IntExpr, IntExpr),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
    Not(Box<Query>),
}

fn prop_matches(task: &Task, key: &str, expected: Option<&str>) -> bool {
    // STATUS/PRIORITY/TAGS live in dedicated Task fields, everything else in `props`.
    let actual: Option<String> = match key {
        "STATUS" => Some(task.status.clone()),
        "PRIORITY" => Some(task.priority.to_string()),
        "TAGS" => Some(task.tags.join(",")),
        _ => task.props.iter().find_map(|p| match p {
            Prop::Other(k, v) if k == key => Some(v.clone()),
            _ => None,
        }),
    };
    match (actual, expected) {
        (Some(_), None) => true,
        (Some(a), Some(e)) => a == e,
        (None, _) => false,
    }
}
pub fn print_qop(q: &Query, indent: &str) {
    match q {
        Query::True => println!("{indent}QOP_ANY"),
        Query::Tag(t) => println!("{indent}QOP_TAG {t}"),
        Query::Tagged => println!("{indent}QOP_TAGGED"),
        Query::HuidMatch(h) => println!("{indent}QOP_ID {h}"),
        Query::Text(t) => println!("{indent}QOP_TEXT {t}"),
        Query::Prop { key, value: Some(v) } => println!("{indent}QOP_PROP {key}={v}"),
        Query::Prop { key, value: None } => println!("{indent}QOP_PROP {key}"),
        Query::Not(inner) => {
            print_qop(inner, indent);
            println!("{indent}QOP_NOT");
        }
        Query::And(a, b) => {
            print_qop(a, indent);
            print_qop(b, indent);
            println!("{indent}QOP_AND");
        }
        Query::Or(a, b) => {
            print_qop(a, indent);
            print_qop(b, indent);
            println!("{indent}QOP_OR");
        }
        Query::Cmp(op, a, b) => {
            print_intexpr(a, indent);
            print_intexpr(b, indent);
            let name = match op {
                CmpOp::Lt => "QOP_LT",
                CmpOp::Le => "QOP_LTE",
                CmpOp::Gt => "QOP_GT",
                CmpOp::Ge => "QOP_GTE",
                CmpOp::Eq => "QOP_EQ",
                CmpOp::Ne => "QOP_NEQ",
            };
            println!("{indent}{name}");
        }
    }
}

fn print_intexpr(e: &IntExpr, indent: &str) {
    match e {
        IntExpr::Priority => println!("{indent}QOP_PRIORITY"),
        IntExpr::Lit(n) => println!("{indent}QOP_INTEGER {n}"),
    }
}
impl Query {
    pub fn matches(&self, task: &Task) -> bool {
        match self {
            Query::True => true,
            Query::Tag(t) => task.tags.iter().any(|x| x == t),
            Query::Prop { key, value } => prop_matches(task, key, value.as_deref()),
            Query::Text(needle) => {
                let hay = strip_markdown(&task.body).to_lowercase();
                hay.contains(&needle.to_lowercase())
            }
            Query::Tagged => !task.tags.is_empty(),
            Query::HuidMatch(h) => task.huid.to_string() == *h,
            Query::Cmp(op, a, b) => op.eval(a.eval(task), b.eval(task)),
            Query::And(a, b) => a.matches(task) && b.matches(task),
            Query::Or(a, b) => a.matches(task) || b.matches(task),
            Query::Not(a) => !a.matches(task),
        }
    }
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Tag(String),
    Ident(String),
    Str(String),
    And,
    Or,
    Not,
    LBracket,
    RBracket,
    Eq,
    Eof,
}

fn is_bracket(c: char) -> bool {
    c == '[' || c == ']'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '+'
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
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            '(' | ')' => {
                return Err(format!(
                    "unexpected character: {c:?}; use '[' and ']' for grouping (upstream tatr syntax)"
                ));
            }
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            ':' => {
                // Upstream: a tag token is `:` followed by everything until
                // whitespace or `[]`. No character restrictions.
                let start = i + 1;
                i += 1;
                while i < chars.len() && !chars[i].is_whitespace() && !is_bracket(chars[i]) {
                    i += 1;
                }
                let tag: String = chars[start..i].iter().collect();
                if tag.is_empty() {
                    return Err("empty tag after ':'".into());
                }
                tokens.push(Token::Tag(tag));
            }
            '"' => {
                let start = i + 1;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                if i >= chars.len() {
                    return Err("unterminated string".into());
                }
                tokens.push(Token::Str(chars[start..i].iter().collect()));
                i += 1;
            }
            '.' => {
                // Deprecated `.tag` syntax
                let start = i + 1;
                i += 1;
                while i < chars.len() && !chars[i].is_whitespace() && !is_bracket(chars[i]) {
                    i += 1;
                }
                let tag: String = chars[start..i].iter().collect();
                if tag.is_empty() {
                    return Err("empty tag after '.'".into());
                }
                eprintln!(
                    "WARNING: Using `.` to refer to tags is deprecated and will be removed in the future. Use `:` instead."
                );
                tokens.push(Token::Tag(tag));
            }
            _ if is_ident_char(c) => {
                let start = i;
                while i < chars.len() && is_ident_char(chars[i]) {
                    i += 1;
                }
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

pub fn tokenize_for_debug(input: &str) -> Result<Vec<String>, String> {
    let tokens = tokenize(input)?;
    Ok(tokens
        .into_iter()
        .filter_map(|t| match t {
            Token::Tag(s) => Some(format!(":{}", s)),
            Token::Ident(s) => Some(s),
            Token::Str(s) => Some(format!("\"{}\"", s)),
            Token::And => Some("and".into()),
            Token::Or => Some("or".into()),
            Token::Not => Some("not".into()),
            Token::LBracket => Some("[".into()),
            Token::RBracket => Some("]".into()),
            Token::Eq => Some("=".into()),
            Token::Eof => None,
        })
        .collect())
}
/// Syntactic check matching upstream `is_valid_huid`:
/// `YYYYMMDD-HHMMSS` optionally followed by `-[A-Za-z0-9-]*`.
/// Purely lexical — does not validate the calendar date.
fn is_huid_syntax(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 15 {
        return false;
    }
    for i in bytes.iter().take(8) {
        if !i.is_ascii_digit() {
            return false;
        }
    }
    if bytes[8] != b'-' {
        return false;
    }
    for i in bytes.iter().take(15).skip(9) {
        if !i.is_ascii_digit() {
            return false;
        }
    }
    if bytes.len() == 15 {
        return true;
    }
    if bytes[15] != b'-' {
        return false;
    }
    for &c in &bytes[16..] {
        if !(c.is_ascii_alphanumeric() || c == b'-') {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Parser
//
// Grammar, matching upstream `compile_query_*`:
//
//   expr    = or
//   or      = and ('or'  and)*
//   and     = cmp ('and' cmp)*
//   cmp     = val ('lt'|'le'|'gt'|'ge'|'eq'|'ne' val)*
//   val     = 'not' val | primary
//   primary = tag | '[' expr ']' | any | tagged | priority
//           | HUID | integer | quoted | identifier | key=value
//
// `any`, `tagged`, HUID, and `not` produce booleans.
// `priority` and integer literals produce integers.
// `cmp` requires integer operands and produces a boolean.
// `and`/`or` require boolean operands.
// ---------------------------------------------------------------------------

enum Val {
    Bool(Query),
    Int(IntExpr),
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn peek_cmp_op(&self) -> Option<CmpOp> {
        match self.peek() {
            Token::Ident(s) => match s.as_str() {
                "lt" => Some(CmpOp::Lt),
                "le" => Some(CmpOp::Le),
                "gt" => Some(CmpOp::Gt),
                "ge" => Some(CmpOp::Ge),
                "eq" => Some(CmpOp::Eq),
                "ne" => Some(CmpOp::Ne),
                _ => None,
            },
            _ => None,
        }
    }

    fn parse_or(&mut self) -> Result<Query, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::Or) {
            self.bump();
            if matches!(self.peek(), Token::Eof) {
                return Err("unexpected end of query after 'or'".into());
            }
            let right = self.parse_and()?;
            left = Query::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Query, String> {
        let mut left = self.parse_compare()?;
        while matches!(self.peek(), Token::And) {
            self.bump();
            if matches!(self.peek(), Token::Eof) {
                return Err("unexpected end of query after 'and'".into());
            }
            let right = self.parse_compare()?;
            left = Query::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_compare(&mut self) -> Result<Query, String> {
        let mut left = self.parse_primary_val()?;
        while let Some(op) = self.peek_cmp_op() {
            self.bump();
            let right = self.parse_primary_val()?;
            left = match (left, right) {
                (Val::Int(a), Val::Int(b)) => Val::Bool(Query::Cmp(op, a, b)),
                _ => return Err("comparison requires integer operands".into()),
            };
        }
        match left {
            Val::Bool(q) => Ok(q),
            Val::Int(_) => Err("integer expression cannot be used as boolean".into()),
        }
    }

    fn parse_primary_val(&mut self) -> Result<Val, String> {
        if matches!(self.peek(), Token::Not) {
            self.bump();
            let inner = self.parse_primary_val()?;
            return match inner {
                Val::Bool(q) => Ok(Val::Bool(Query::Not(Box::new(q)))),
                Val::Int(_) => Err("'not' requires boolean operand".into()),
            };
        }
        match self.bump() {
            Token::Tag(t) => Ok(Val::Bool(Query::Tag(t))),
            Token::Str(s) => Ok(Val::Bool(Query::Text(s))),
            Token::LBracket => {
                let q = self.parse_or()?;
                match self.bump() {
                    Token::RBracket => Ok(Val::Bool(q)),
                    other => Err(format!("expected ']', got {other:?}")),
                }
            }
            Token::Ident(name) => {
                // Upstream keywords first.
                match name.as_str() {
                    "any" => return Ok(Val::Bool(Query::True)),
                    "tagged" => return Ok(Val::Bool(Query::Tagged)),
                    "priority" => return Ok(Val::Int(IntExpr::Priority)),
                    _ => {}
                }
                // Upstream HUID primary.
                if is_huid_syntax(&name) {
                    return Ok(Val::Bool(Query::HuidMatch(name)));
                }
                // Upstream integer primary.
                if let Ok(n) = name.parse::<i64>() {
                    return Ok(Val::Int(IntExpr::Lit(n)));
                }
                // rtatr extension: `key=value` / `key`.
                if matches!(self.peek(), Token::Eq) {
                    self.bump();
                    let value = match self.bump() {
                        Token::Str(s) | Token::Ident(s) | Token::Tag(s) => Some(s),
                        Token::Eof => None,
                        other => return Err(format!("expected value, got {other:?}")),
                    };
                    return Ok(Val::Bool(Query::Prop { key: name, value }));
                }
                // rtatr extension: bare-word text search.
                Ok(Val::Bool(Query::Text(name)))
            }
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
