//! A small query language over a log, for `cairns ls` and the MCP list tool.
//!
//! Borrowed in shape from tsoding's `tatr`: words a shell passes through
//! unquoted - square brackets to group, not parentheses; `lt`, `gt` and the rest
//! to compare, not `<` and `>`, which a shell takes as redirection. Text is
//! `has word`, not `~word`, which a shell expands to a home directory. zsh
//! still globs a bare `[`; quote the brackets there.
//!
//! ```text
//! :battle and open and not superseded
//! took gt 2h
//! date ge 2026-10-01 and '[' :ui or :render ']'
//! has dungeon and clocked
//! ```

use crate::date::Date;
use crate::log::{Log, LogEntry};

/// A parsed query.
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    Any,
    Area(String),
    /// `has word`: in the title, summary or prose, any case.
    Text(String),
    Flag(Flag),
    Compare(Field, Op, Literal),
    Not(Box<Query>),
    And(Box<Query>, Box<Query>),
    Or(Box<Query>, Box<Query>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// Something it asked is still open here.
    Open,
    /// It asked something, and none of it is open here.
    Closed,
    /// A later entry corrected it.
    Superseded,
    /// It records how long it took.
    Clocked,
    /// A reference page cites it.
    Documented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Number,
    Date,
    Took,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Literal {
    Number(u32),
    Date(Date),
    Minutes(u32),
}

/// Parse a query from its words, as the shell hands them over. No words at all
/// is `any`.
pub fn parse(words: &[String]) -> Result<Query, String> {
    // Brackets may be written against a word - `[:ui` - so they are split off.
    let mut tokens = Vec::new();
    for word in words {
        let mut word = word.as_str();
        while let Some(rest) = word.strip_prefix('[') {
            tokens.push("[".to_string());
            word = rest;
        }
        let mut closing = 0;
        while let Some(rest) = word.strip_suffix(']') {
            closing += 1;
            word = rest;
        }
        if !word.is_empty() {
            tokens.push(word.to_string());
        }
        tokens.extend(std::iter::repeat_n("]".to_string(), closing));
    }
    if tokens.is_empty() {
        return Ok(Query::Any);
    }
    let mut parser = Parser { tokens, at: 0 };
    let query = parser.or()?;
    match parser.peek() {
        None => Ok(query),
        Some(extra) => Err(format!("unexpected {extra:?} after a complete query")),
    }
}

struct Parser {
    tokens: Vec<String>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.at).map(String::as_str)
    }

    fn next(&mut self) -> Option<String> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn or(&mut self) -> Result<Query, String> {
        let mut left = self.and()?;
        while self.peek() == Some("or") {
            self.at += 1;
            left = Query::Or(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Query, String> {
        let mut left = self.unary()?;
        while self.peek() == Some("and") {
            self.at += 1;
            left = Query::And(Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Query, String> {
        if self.peek() == Some("not") {
            self.at += 1;
            return Ok(Query::Not(Box::new(self.unary()?)));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Query, String> {
        let token = self
            .next()
            .ok_or("the query ends where something was expected")?;
        let field = match token.as_str() {
            "[" => {
                let inner = self.or()?;
                return match self.next().as_deref() {
                    Some("]") => Ok(inner),
                    _ => Err("a `[` is never closed".into()),
                };
            }
            "any" => return Ok(Query::Any),
            "open" => return Ok(Query::Flag(Flag::Open)),
            "closed" => return Ok(Query::Flag(Flag::Closed)),
            "superseded" => return Ok(Query::Flag(Flag::Superseded)),
            "clocked" => return Ok(Query::Flag(Flag::Clocked)),
            "documented" => return Ok(Query::Flag(Flag::Documented)),
            "has" => {
                let word = self.next().ok_or("`has` needs a word to look for")?;
                return Ok(Query::Text(word.to_lowercase()));
            }
            "number" => Field::Number,
            "date" => Field::Date,
            "took" => Field::Took,
            other => {
                if let Some(area) = other.strip_prefix(':').filter(|a| !a.is_empty()) {
                    return Ok(Query::Area(area.to_string()));
                }
                // A bare number is the entry with that number.
                if let Ok(number) = other.parse::<u32>() {
                    return Ok(Query::Compare(
                        Field::Number,
                        Op::Eq,
                        Literal::Number(number),
                    ));
                }
                return Err(format!(
                    "{other:?} is not part of a query - :area, has word, open, closed, superseded, \
                     clocked, documented, any, or number, date, took with lt le gt ge eq ne"
                ));
            }
        };
        let op = match self.next().as_deref() {
            Some("lt") => Op::Lt,
            Some("le") => Op::Le,
            Some("gt") => Op::Gt,
            Some("ge") => Op::Ge,
            Some("eq") => Op::Eq,
            Some("ne") => Op::Ne,
            other => {
                return Err(format!(
                    "{token} needs a comparison - lt, le, gt, ge, eq or ne - not {:?}",
                    other.unwrap_or("the end")
                ));
            }
        };
        let value = self
            .next()
            .ok_or(format!("{token} compared with nothing"))?;
        let literal = match field {
            Field::Number => Literal::Number(
                value
                    .parse()
                    .map_err(|_| format!("{value:?} is not an entry number"))?,
            ),
            Field::Date => Literal::Date(
                value
                    .parse()
                    .map_err(|_| format!("{value:?} is not a date, YYYY-MM-DD"))?,
            ),
            Field::Took => Literal::Minutes(
                crate::entry::parse_duration(&value.replace("h", "h ").replace("h  ", "h "))
                    .map_err(|_| format!("{value:?} is not a duration - 45m, 2h, 1h30m"))?,
            ),
        };
        Ok(Query::Compare(field, op, literal))
    }
}

impl Query {
    /// Whether an entry of `log` matches.
    pub fn matches(&self, log: &Log, entry: &LogEntry) -> bool {
        match self {
            Query::Any => true,
            Query::Area(area) => entry.areas.iter().any(|a| a == area),
            Query::Text(text) => {
                entry.title.to_lowercase().contains(text)
                    || entry
                        .summary
                        .as_deref()
                        .is_some_and(|s| s.to_lowercase().contains(text))
                    || entry.body.to_lowercase().contains(text)
            }
            Query::Flag(Flag::Open) => log.open_questions.iter().any(|q| q.entry == entry.number),
            Query::Flag(Flag::Closed) => {
                entry.still_unknown.is_some()
                    && !log.open_questions.iter().any(|q| q.entry == entry.number)
            }
            Query::Flag(Flag::Superseded) => !entry.superseded_by.is_empty(),
            Query::Flag(Flag::Clocked) => entry.took_minutes.is_some(),
            Query::Flag(Flag::Documented) => !entry.documented_by.is_empty(),
            Query::Compare(field, op, literal) => {
                let ordering = match (field, literal) {
                    (Field::Number, Literal::Number(n)) => entry.number.cmp(n),
                    (Field::Date, Literal::Date(d)) => entry.date.cmp(d),
                    // An entry that recorded no time has no time to compare.
                    (Field::Took, Literal::Minutes(m)) => match entry.took_minutes {
                        Some(took) => took.cmp(m),
                        None => return false,
                    },
                    _ => return false,
                };
                use std::cmp::Ordering::*;
                match op {
                    Op::Lt => ordering == Less,
                    Op::Le => ordering != Greater,
                    Op::Gt => ordering == Greater,
                    Op::Ge => ordering != Less,
                    Op::Eq => ordering == Equal,
                    Op::Ne => ordering != Equal,
                }
            }
            Query::Not(inner) => !inner.matches(log, entry),
            Query::And(a, b) => a.matches(log, entry) && b.matches(log, entry),
            Query::Or(a, b) => a.matches(log, entry) || b.matches(log, entry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(text: &str) -> Result<Query, String> {
        parse(
            &text
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn a_query_reads_the_way_tatr_does() {
        assert_eq!(q(""), Ok(Query::Any));
        assert_eq!(
            q(":battle and not open"),
            Ok(Query::And(
                Box::new(Query::Area("battle".into())),
                Box::new(Query::Not(Box::new(Query::Flag(Flag::Open))))
            ))
        );
        // `and` binds tighter than `or`; brackets regroup, written against words.
        assert!(matches!(q(":a or :b and :c"), Ok(Query::Or(_, _))));
        assert!(matches!(q("[:a or :b] and :c"), Ok(Query::And(_, _))));
        assert_eq!(
            q("took gt 1h30m"),
            Ok(Query::Compare(Field::Took, Op::Gt, Literal::Minutes(90)))
        );
        assert!(q("date ge 2026-10-01").is_ok());
        assert_eq!(q("has Dungeon"), Ok(Query::Text("dungeon".into())));
        assert_eq!(
            q("50"),
            Ok(Query::Compare(Field::Number, Op::Eq, Literal::Number(50)))
        );
    }

    #[test]
    fn a_bad_query_says_what_was_wrong() {
        assert!(q("took 2h").unwrap_err().contains("needs a comparison"));
        assert!(q("[ :a").unwrap_err().contains("never closed"));
        assert!(q("bogus").unwrap_err().contains("not part of a query"));
        assert!(q("date gt yesterday").unwrap_err().contains("not a date"));
        assert!(q(":a :b").unwrap_err().contains("unexpected"));
    }
}
