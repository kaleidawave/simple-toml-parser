#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TOMLKey<'a> {
    Slice(&'a str),
    Index(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RootTOMLValue<'a> {
    String(&'a str),
    Number(&'a str),
    Boolean(bool),
    Null,
}

#[derive(Debug)]
pub enum TOMLParseErrorReason {
    ExpectedColon,
    ExpectedEndOfValue,
    ExpectedBracket,
    ExpectedTrueFalseNull,
    ExpectedValue,
}

#[derive(Debug)]
pub struct TOMLParseError {
    pub at: usize,
    pub reason: TOMLParseErrorReason,
}

impl std::error::Error for TOMLParseError {}

impl std::fmt::Display for TOMLParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!(
            "TOMLParseError: {:?} at {:?}",
            self.reason, self.at
        ))
    }
}

/// If you want to return early (not parse the whole input) use [`parse_with_exit_signal`]
///
/// # Errors
/// Returns an error if it tries to parse invalid TOML input
pub fn parse<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], RootTOMLValue<'a>),
) -> Result<(), TOMLParseError> {
    parse_with_exit_signal(on, |k, v| {
        cb(k, v);
        false
    })
}

/// # Errors
/// Returns an error if it tries to parse invalid TOML input
/// # Panics
/// On unimplemented items
#[allow(clippy::too_many_lines)]
pub fn parse_with_exit_signal<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], RootTOMLValue<'a>) -> bool,
) -> Result<(), TOMLParseError> {
    let mut key_chain = Vec::new();
    let lines = on.lines();

    for line in lines {
        if line.starts_with("[[") {
            let _ = key_chain.drain(..);
            assert!(line.ends_with("]]"), "TODO error");

            let idx = if let Some(TOMLKey::Index(idx)) = key_chain.last() {
                idx + 1
            } else {
                0
            };
            // TODO strings parsing here
            for part in line[1..(line.len() - "]]".len())].split('.') {
                key_chain.push(TOMLKey::Slice(part));
            }
            key_chain.push(TOMLKey::Index(idx));
        } else if line.starts_with('[') {
            let _ = key_chain.drain(..);
            assert!(line.ends_with(']'), "TODO error");
            // TODO strings parsing here
            for part in line[1..(line.len() - "]".len())].split('.') {
                key_chain.push(TOMLKey::Slice(part));
            }
        } else {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let length = key_chain.len();
                for part in key.trim().split('.') {
                    key_chain.push(TOMLKey::Slice(part));
                }

                let value = value.trim();
                let value = if let "true" | "false" = value {
                    RootTOMLValue::Boolean(value == "true")
                } else if value.starts_with(['"', '\'']) {
                    assert!(
                        value.ends_with(value.chars().next().unwrap()),
                        "TODO unclosed string"
                    );
                    RootTOMLValue::String(&value[1..(value.len() - 1)])
                } else if value.starts_with(char::is_numeric) {
                    RootTOMLValue::Number(value)
                } else {
                    eprintln!("here {value}");
                    continue;
                };

                let _result = cb(&key_chain, value);

                let _ = key_chain.drain(length..);
            } else {
                panic!("bad key")
            }
        }
    }

    Ok(())
}
