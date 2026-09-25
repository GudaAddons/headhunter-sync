//! A small reader for the Lua that WoW writes into SavedVariables files:
//! `Name = { ["key"] = value, [1] = value, value, ... }` with strings, numbers,
//! booleans and nil. Tables become JSON: a table with only positional values is an
//! array, anything else an object (numeric keys as strings).

use serde_json::{Map, Number, Value};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum LuaError {
    #[error("`{0} =` was not found in the file")]
    MissingVariable(String),
    #[error("unexpected {found} at line {line}")]
    Unexpected { found: String, line: usize },
}

/// The value assigned to `variable` at the top level of a SavedVariables file.
pub fn read_variable(source: &str, variable: &str) -> Result<Value, LuaError> {
    let mut parser = Parser::new(source);
    loop {
        parser.skip_space();
        if parser.at_end() {
            return Err(LuaError::MissingVariable(variable.to_string()));
        }
        let name = parser.identifier();
        parser.skip_space();
        if name.is_empty() || !parser.eat('=') {
            return Err(parser.unexpected());
        }
        let value = parser.value()?;
        if name == variable {
            return Ok(value);
        }
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    _source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self { chars: source.chars().collect(), pos: 0, _source: source }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn line(&self) -> usize {
        self.chars[..self.pos.min(self.chars.len())].iter().filter(|c| **c == '\n').count() + 1
    }

    fn unexpected(&self) -> LuaError {
        let found = match self.peek() {
            Some(c) => format!("'{c}'"),
            None => "end of file".to_string(),
        };
        LuaError::Unexpected { found, line: self.line() }
    }

    /// Whitespace and `--` comments.
    fn skip_space(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.pos += 1;
            }
            if self.peek() == Some('-') && self.chars.get(self.pos + 1) == Some(&'-') {
                while self.peek().is_some_and(|c| c != '\n') {
                    self.pos += 1;
                }
                continue;
            }
            break;
        }
    }

    fn identifier(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            self.pos += 1;
        }
        self.chars[start..self.pos].iter().collect()
    }

    fn value(&mut self) -> Result<Value, LuaError> {
        self.skip_space();
        match self.peek() {
            Some('{') => self.table(),
            Some('"') | Some('\'') => self.string().map(Value::String),
            Some(c) if c == '-' || c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if c.is_alphabetic() => match self.identifier().as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                "nil" => Ok(Value::Null),
                _ => Err(self.unexpected()),
            },
            _ => Err(self.unexpected()),
        }
    }

    fn table(&mut self) -> Result<Value, LuaError> {
        self.eat('{');
        let mut positional = Vec::new();
        let mut keyed = Map::new();
        loop {
            self.skip_space();
            if self.eat('}') {
                break;
            }
            if self.eat('[') {
                let key = self.value()?;
                self.skip_space();
                if !self.eat(']') {
                    return Err(self.unexpected());
                }
                self.skip_space();
                if !self.eat('=') {
                    return Err(self.unexpected());
                }
                let value = self.value()?;
                keyed.insert(key_text(&key), value);
            } else if self.peek().is_some_and(|c| c.is_alphabetic() || c == '_') && self.is_name_assignment() {
                let key = self.identifier();
                self.skip_space();
                self.eat('=');
                let value = self.value()?;
                keyed.insert(key, value);
            } else {
                positional.push(self.value()?);
            }
            self.skip_space();
            if !self.eat(',') && !self.eat(';') {
                self.skip_space();
                if !self.eat('}') {
                    return Err(self.unexpected());
                }
                break;
            }
        }

        if keyed.is_empty() {
            return Ok(Value::Array(positional));
        }
        for (i, value) in positional.into_iter().enumerate() {
            keyed.insert((i + 1).to_string(), value);
        }
        Ok(Value::Object(keyed))
    }

    /// `name =` (a field) rather than `true`, `nil` or a bare value
    fn is_name_assignment(&self) -> bool {
        let mut i = self.pos;
        while self.chars.get(i).is_some_and(|c| c.is_alphanumeric() || *c == '_') {
            i += 1;
        }
        while self.chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        self.chars.get(i) == Some(&'=') && self.chars.get(i + 1) != Some(&'=')
    }

    fn string(&mut self) -> Result<String, LuaError> {
        let quote = self.peek().unwrap_or('"');
        self.pos += 1;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else { return Err(self.unexpected()) };
            self.pos += 1;
            if c == quote {
                return Ok(out);
            }
            if c != '\\' {
                out.push(c);
                continue;
            }
            let Some(escaped) = self.peek() else { return Err(self.unexpected()) };
            self.pos += 1;
            match escaped {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '\n' => out.push('\n'),
                d if d.is_ascii_digit() => {
                    let mut code = d.to_digit(10).unwrap_or(0);
                    for _ in 0..2 {
                        match self.peek().and_then(|c| c.to_digit(10)) {
                            Some(digit) => {
                                code = code * 10 + digit;
                                self.pos += 1;
                            }
                            None => break,
                        }
                    }
                    out.push(char::from_u32(code).unwrap_or('?'));
                }
                other => out.push(other),
            }
        }
    }

    fn number(&mut self) -> Result<Value, LuaError> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_hexdigit() || matches!(c, '-' | '+' | '.' | 'x' | 'X')) {
            self.pos += 1;
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if let Ok(int) = text.parse::<i64>() {
            return Ok(Value::Number(int.into()));
        }
        match text.parse::<f64>().ok().and_then(Number::from_f64) {
            Some(number) => Ok(Value::Number(number)),
            None => {
                self.pos = start;
                Err(self.unexpected())
            }
        }
    }
}

fn key_text(key: &Value) -> String {
    match key {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `Name = <value>` as Lua source, for the data file the game loads as addon code.
/// Arrays become sequences, objects keyed tables (sorted keys, so the same data gives
/// the same text); null object fields are left out.
pub fn write_variable(name: &str, value: &Value) -> String {
    let mut out = format!("{name} = ");
    write_value(value, 0, &mut out);
    out.push('\n');
    out
}

fn write_value(value: &Value, depth: usize, out: &mut String) {
    match value {
        Value::Null => out.push_str("nil"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(s, out),
        Value::Array(list) => {
            if list.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for item in list {
                indent(depth + 1, out);
                write_value(item, depth + 1, out);
                out.push_str(",\n");
            }
            indent(depth, out);
            out.push('}');
        }
        Value::Object(map) => {
            let fields: Vec<(&String, &Value)> = map.iter().filter(|(_, v)| !v.is_null()).collect();
            if fields.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (key, item) in fields {
                indent(depth + 1, out);
                if is_identifier(key) {
                    out.push_str(key);
                } else {
                    out.push('[');
                    write_string(key, out);
                    out.push(']');
                }
                out.push_str(" = ");
                write_value(item, depth + 1, out);
                out.push_str(",\n");
            }
            indent(depth, out);
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 || c as u32 == 127 => out.push_str(&format!("\\{:03}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

const LUA_KEYWORDS: [&str; 22] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not",
    "or", "repeat", "return", "then", "true", "until", "while",
];

fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !LUA_KEYWORDS.contains(&key)
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_a_saved_variables_file() {
        let source = "\r\nHeadHunter_DB = {\r\n[\"meta\"] = {\r\n[\"loadCount\"] = 14,\r\n[\"region\"] = \"eu\",\r\n},\r\n[\"zones\"] = {\r\n[1424] = {\r\n[\"name\"] = \"Hillsbrad Foothills\",\r\n},\r\n},\r\n[\"deaths\"] = {\r\n{\r\n[\"t\"] = 1790266509,\r\n[\"x\"] = 0.412,\r\n[\"shared\"] = true,\r\n},\r\n},\r\n[\"empty\"] = {\r\n},\r\n}\r\n";
        let db = read_variable(source, "HeadHunter_DB").unwrap();
        assert_eq!(db["meta"]["loadCount"], json!(14));
        assert_eq!(db["meta"]["region"], json!("eu"));
        assert_eq!(db["zones"]["1424"]["name"], json!("Hillsbrad Foothills"));
        assert_eq!(db["deaths"][0]["t"], json!(1790266509));
        assert_eq!(db["deaths"][0]["x"], json!(0.412));
        assert_eq!(db["deaths"][0]["shared"], json!(true));
        assert_eq!(db["empty"], json!([]));
    }

    #[test]
    fn reads_escapes_negative_numbers_and_other_variables() {
        let source = "Other = { 1, 2 }\nHeadHunter_DB = { [\"name\"] = \"Say \\\"hi\\\"\\\\ \\233\", [\"level\"] = -1, [\"gone\"] = nil, plain = false }";
        let db = read_variable(source, "HeadHunter_DB").unwrap();
        assert_eq!(db["name"], json!("Say \"hi\"\\ é"));
        assert_eq!(db["level"], json!(-1));
        assert_eq!(db["gone"], json!(null));
        assert_eq!(db["plain"], json!(false));
    }

    #[test]
    fn writes_lua_that_reads_back_the_same() {
        let data = json!({
            "worlds": { "era|eu|Firemaw": { "wanted": [{ "name": "Dusk Blade", "kills": 12.5, "badges": ["coward"] }] } },
            "characters": [{ "name": "Say \"hi\"\\\n\ttab", "level": -1, "dead": true, "gone": null }],
            "end": "a keyword key",
            "empty": [],
            "none": {},
        });
        let source = write_variable("HeadHunter_SiteData", &data);
        let mut expected = data.clone();
        expected["characters"][0].as_object_mut().unwrap().remove("gone");
        expected["none"] = json!([]);
        assert_eq!(read_variable(&source, "HeadHunter_SiteData").unwrap(), expected, "round trip through the reader");
        assert!(source.contains("[\"end\"] ="), "a Lua keyword is written as a string key");
        assert!(source.contains("[\"era|eu|Firemaw\"] ="), "a key that is not a name is written as a string key");
    }

    #[test]
    fn writes_the_same_text_for_the_same_data() {
        let a = json!({ "b": 1, "a": [1, 2], "c": { "y": 2, "x": 1 } });
        let b = json!({ "c": { "x": 1, "y": 2 }, "a": [1, 2], "b": 1 });
        assert_eq!(write_variable("V", &a), write_variable("V", &b), "keys are sorted");
    }

    #[test]
    fn says_what_is_wrong() {
        assert_eq!(read_variable("Nothing = {}", "HeadHunter_DB"), Err(LuaError::MissingVariable("HeadHunter_DB".into())));
        assert!(matches!(read_variable("HeadHunter_DB = { [\"a\"] = }", "HeadHunter_DB"), Err(LuaError::Unexpected { .. })));
    }
}
