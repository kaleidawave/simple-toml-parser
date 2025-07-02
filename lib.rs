#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Joined {
    Alone,
    Dot,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Level {
    Table,
    Specifier,
    InObject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TOMLKey<'a> {
    Slice(&'a str),
    Index(usize),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TOMLKeyMetadata(pub Joined, pub Level);

impl TOMLKeyMetadata {
    const UNKNOWN: Self = TOMLKeyMetadata(Joined::Alone, Level::Table);
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
    ExpectedEqual,
    ExpectedQuote,
    ExpectedEndOfMultilineComment,
    ExpectedKey,
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
/// and that contains more information about keys
///
/// # Errors
/// Returns an error if it tries to parse invalid TOML input
pub fn parse<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], RootTOMLValue<'a>),
) -> Result<(), TOMLParseError> {
    parse_with_exit_signal(on, |k, _m, v| {
        cb(k, v);
        false
    })
}

#[derive(Debug, Clone, Copy)]
enum Context {
    CurrentLevel,
    Table,
    ArrayOfTables,
}

enum State {
    StartOfLine,
    Equal,
    InComment,
    InKey {
        in_string: bool,
        joined: Joined,
        context: Context,
    },
}

/// # Errors
/// Returns an error if it tries to parse invalid TOML input
///
/// # Panics
/// On unimplemented items
#[allow(clippy::too_many_lines)]
pub fn parse_with_exit_signal<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b [TOMLKeyMetadata], RootTOMLValue<'a>) -> bool,
) -> Result<(), TOMLParseError> {
    let mut key_chain = Vec::new();
    let mut key_chain_metadata = Vec::new();
    let mut chars = on.char_indices();
    let mut start = 0;

    let mut state = State::StartOfLine;

    while let Some((idx, chr)) = chars.next() {
        match state {
            State::StartOfLine => {
                match chr {
                    '[' => {
                        let context = if on[idx..].starts_with("[[") {
                            start = idx + 2;
                            let _ = chars.next();
                            Context::ArrayOfTables
                        } else {
                            // TODO needed somewhere here
                            let _ = key_chain.drain(..);
                            key_chain_metadata.drain(..);
                            start = idx + 1;
                            Context::Table
                        };
                        state = State::InKey {
                            context,
                            in_string: false,
                            joined: Joined::Alone,
                        };
                    }
                    '#' => {
                        state = State::InComment;
                    }
                    chr => {
                        if !chr.is_whitespace() {
                            start = idx;
                            state = State::InKey {
                                context: Context::CurrentLevel,
                                in_string: chr == '"',
                                joined: Joined::Alone,
                            };
                        }
                    }
                }
            }
            State::InKey {
                context,
                ref mut in_string,
                ref mut joined,
            } => {
                if let '"' = chr {
                    if *in_string {
                        let key = &on[start..idx];
                        key_chain.push(TOMLKey::Slice(key));
                        key_chain_metadata.push(TOMLKeyMetadata(Joined::Alone, Level::Table));
                    }
                    start += 1;
                    *in_string = !*in_string;
                }
                if *in_string {
                    continue;
                }

                if let '.' = chr {
                    let key = &on[start..idx];
                    key_chain.push(TOMLKey::Slice(key));
                    key_chain_metadata.push(TOMLKeyMetadata(*joined, Level::Table));
                    *joined = Joined::Dot;
                    start = idx + chr.len_utf8();
                }

                match (context, chr) {
                    (Context::ArrayOfTables | Context::Table, ']') => {
                        if let Context::ArrayOfTables = context {
                            // TODO
                            dbg!(chars.next());
                            // assert!(.unwrap().1 == ']');
                        }
                        let key = &on[start..idx];
                        if let Context::ArrayOfTables = context {
                            let idx = if let Some(TOMLKey::Index(idx)) = key_chain.last() {
                                idx + 1
                            } else {
                                0
                            };
                            key_chain.push(TOMLKey::Slice(key));
                            key_chain_metadata.push(TOMLKeyMetadata::UNKNOWN);
                            key_chain.push(TOMLKey::Index(idx));
                            key_chain_metadata.push(TOMLKeyMetadata::UNKNOWN);
                        } else {
                            key_chain.push(TOMLKey::Slice(key));
                            key_chain_metadata.push(TOMLKeyMetadata::UNKNOWN);
                        }
                        state = State::StartOfLine;
                        start = idx + 1;
                    }
                    (Context::CurrentLevel, chr) => {
                        let is_equal = chr == '=';
                        let is_whitespace = chr.is_whitespace();
                        if is_equal || is_whitespace {
                            let key = &on[start..idx];
                            if key.is_empty() {
                                todo!("key is empty");
                            }
                            key_chain.push(TOMLKey::Slice(key));
                            key_chain_metadata.push(TOMLKeyMetadata::UNKNOWN);

                            if is_equal {
                                let _ = value::parse_with_exit_signal_chars(
                                    on,
                                    &mut chars,
                                    &mut key_chain,
                                    &mut key_chain_metadata,
                                    &mut cb,
                                );
                                state = State::StartOfLine;
                                {
                                    let mut popped = key_chain.pop();
                                    let mut metadata = key_chain_metadata.pop();
                                    while let (
                                        Some(TOMLKey::Slice(_)),
                                        Some(TOMLKeyMetadata(Joined::Dot, _)),
                                    ) = (popped, metadata)
                                    {
                                        popped = key_chain.pop();
                                        metadata = key_chain_metadata.pop();
                                    }
                                }
                            } else {
                                state = State::Equal;
                            }
                        }
                    }
                    (_ctx, _chr) => {}
                }
            }
            State::Equal => {
                if let '=' = chr {
                    let _ = value::parse_with_exit_signal_chars(
                        on,
                        &mut chars,
                        &mut key_chain,
                        &mut key_chain_metadata,
                        &mut cb,
                    );
                    state = State::StartOfLine;
                    {
                        let mut popped = key_chain.pop();
                        let mut metadata = key_chain_metadata.pop();
                        while let (Some(TOMLKey::Slice(_)), Some(TOMLKeyMetadata(Joined::Dot, _))) =
                            (popped, metadata)
                        {
                            popped = key_chain.pop();
                            metadata = key_chain_metadata.pop();
                        }
                    }
                } else if !chr.is_whitespace() {
                    todo!("{chr:?} {key_chain:?}");
                }
            }
            State::InComment => {
                if let '\n' = chr {
                    state = State::StartOfLine;
                }
            }
        }
        {
            // if let '\n' = chr {
            //     let line = &on[start..idx];

            //     if line.starts_with("[[") {
            //         let _ = key_chain.drain(..);
            //         assert!(line.ends_with("]]"), "TODO error");

            //         // TODO strings parsing here
            //         for part in line[1..(line.len() - "]]".len())].split('.') {

            //         }

            //     } else if line.starts_with('[') {
            //         let _ = key_chain.drain(..);
            //         assert!(line.ends_with(']'), "TODO error");
            //         // TODO strings parsing here
            //         for part in line[1..(line.len() - "]".len())].split('.') {

            //         }
            //     } else if !(line.starts_with('#') || line.is_empty()) {
            //         if let Some((key, value)) = line.split_once('=') {
            //             let length = key_chain.len();
            //             for part in key.trim().split('.') {
            //                 key_chain.push(TOMLKey::Slice(part));
            //             }

            //             let value = value.trim();
            //             let value = if let "true" | "false" = value {
            //                 RootTOMLValue::Boolean(value == "true")
            //             } else if value.starts_with(['"', '\'']) {
            //                 let first = value.chars().next().unwrap();
            //                 assert!(value.ends_with(first), "TODO unclosed string");
            //                 RootTOMLValue::String(&value[1..(value.len() - 1)])
            //             } else if value.starts_with(char::is_numeric) {
            //                 RootTOMLValue::Number(value)
            //             } else {
            //                 eprintln!("TODO JSON like parsing of {value}");
            //                 start = idx + chr.len_utf8();
            //                 continue;
            //             };

            //             let _result = cb(key_chain, value);

            //             let _ = key_chain.drain(length..);
            //         } else {
            //             panic!("bad key {line}")
            //         }
            //     }
            //     start = idx + chr.len_utf8();
            // }
        }
    }

    Ok(())
}

pub mod value {
    use super::{
        Joined, Level, RootTOMLValue, TOMLKey, TOMLKeyMetadata, TOMLParseError,
        TOMLParseErrorReason,
    };

    enum State {
        InKey {
            escaped: bool,
            start: usize,
            last_was_dot: bool,
        },
        Equal,
        InObject,
        Comment {
            start: usize,
            multiline: bool,
            last_was_asterisk: bool,
            hash: bool,
        },
        ExpectingValue,
        StringValue {
            start: usize,
            literal: bool,
            escaped: bool,
        },
        NumberValue {
            start: usize,
        },
        TrueFalseNull {
            start: usize,
        },
        EndOfValue,
    }

    /// # Errors
    /// errors on invalid TOML syntax
    pub fn parse_with_exit_signal<'a>(
        on: &'a str,
        mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b [TOMLKeyMetadata], RootTOMLValue<'a>) -> bool,
    ) -> Result<usize, TOMLParseError> {
        let mut chars = on.char_indices();
        let mut key_chain = Vec::new();
        let mut key_chain_metadata = Vec::new();
        parse_with_exit_signal_chars(
            on,
            &mut chars,
            &mut key_chain,
            &mut key_chain_metadata,
            &mut cb,
        )
    }

    /// TODO: `allow_comments` fix
    #[allow(clippy::too_many_lines)]
    pub(crate) fn parse_with_exit_signal_chars<'a>(
        on: &'a str,
        chars: &mut std::str::CharIndices<'a>,
        key_chain: &mut Vec<TOMLKey<'a>>,
        key_chain_metadata: &mut Vec<TOMLKeyMetadata>,
        cb: &mut impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b [TOMLKeyMetadata], RootTOMLValue<'a>) -> bool,
    ) -> Result<usize, TOMLParseError> {
        // Temp fix
        struct Options {
            pub allow_comments: bool,
        }

        let options = Options {
            allow_comments: false,
        };

        let mut state = State::ExpectingValue;

        let current_len = key_chain.len();

        for (idx, chr) in chars {
            match state {
                State::ExpectingValue => {
                    state = match chr {
                        '[' => {
                            key_chain.push(TOMLKey::Index(0));
                            key_chain_metadata.push(TOMLKeyMetadata(Joined::Dot, Level::InObject));
                            State::ExpectingValue
                        }
                        ']' => {
                            // TODO check
                            let _ = key_chain.pop();
                            key_chain_metadata.pop();
                            State::EndOfValue
                        }
                        '{' => State::InObject,
                        '}' => {
                            let popped = key_chain.pop();
                            let mut metadata = key_chain_metadata.pop();
                            if let Some(TOMLKey::Index(..)) = popped {
                                State::ExpectingValue
                            } else {
                                while let Some(TOMLKeyMetadata(Joined::Dot, _)) = metadata {
                                    key_chain.pop();
                                    metadata = key_chain_metadata.pop();
                                }
                                State::InObject
                            }
                        }
                        '"' | '\'' => State::StringValue {
                            start: idx + 1,
                            literal: chr == '\'',
                            escaped: false,
                        },
                        c @ ('/' | '#') if options.allow_comments => State::Comment {
                            last_was_asterisk: false,
                            start: idx,
                            multiline: false,
                            hash: c == '#',
                        },
                        '0'..='9' | '-' => State::NumberValue { start: idx },
                        't' | 'f' | 'n' => State::TrueFalseNull { start: idx },
                        chr if chr.is_whitespace() => state,
                        _ => {
                            return Err(TOMLParseError {
                                at: idx,
                                reason: TOMLParseErrorReason::ExpectedValue,
                            });
                        }
                    }
                }
                // TODO parse key function
                State::InKey {
                    ref mut start,
                    ref mut escaped,
                    ref mut last_was_dot,
                } => {
                    // TODO error if slice empty
                    let joined = if *last_was_dot {
                        Joined::Dot
                    } else {
                        Joined::Alone
                    };
                    let is_equal = chr == '=';
                    let is_whitespace = chr.is_whitespace();
                    if is_equal || is_whitespace {
                        let key = &on[*start..idx];
                        key_chain.push(TOMLKey::Slice(key));
                        key_chain_metadata.push(TOMLKeyMetadata(joined, Level::InObject));
                        state = if is_equal {
                            State::ExpectingValue
                        } else {
                            State::Equal
                        };
                    } else if let '.' = chr {
                        key_chain.push(TOMLKey::Slice(&on[*start..idx]));
                        key_chain_metadata.push(TOMLKeyMetadata(joined, Level::InObject));
                        *start = idx + chr.len_utf8();
                        *last_was_dot = true;
                    } else {
                        *escaped = chr == '\\';
                    }
                    // if !*escaped && chr == '"' {
                    //     key_chain.push(TOMLKey::Slice(&on[start..idx]));
                    //     state = State::Equal;
                    // } else {
                    //     *escaped = chr == '\\';
                    // }
                }
                State::StringValue {
                    start,
                    literal,
                    ref mut escaped,
                } => {
                    // TODO WIP
                    if !*escaped && !literal && chr == '"' {
                        let res = cb(
                            key_chain,
                            key_chain_metadata,
                            RootTOMLValue::String(&on[start..idx]),
                        );
                        if res {
                            return Ok(idx + chr.len_utf8());
                        }
                        state = State::EndOfValue;
                    } else if !*escaped && literal && chr == '\'' {
                        let res = cb(
                            key_chain,
                            key_chain_metadata,
                            RootTOMLValue::String(&on[start..idx]),
                        );
                        if res {
                            return Ok(idx + chr.len_utf8());
                        }
                        state = State::EndOfValue;
                    } else if *escaped {
                        *escaped = false;
                    } else {
                        *escaped = chr == '\\';
                    }
                }
                State::Equal => {
                    if chr == '=' {
                        state = State::ExpectingValue;
                    } else if !chr.is_whitespace() {
                        return Err(TOMLParseError {
                            at: idx,
                            reason: TOMLParseErrorReason::ExpectedEqual,
                        });
                    }
                }
                State::EndOfValue => {
                    end_of_value(
                        idx,
                        chr,
                        &mut state,
                        key_chain,
                        key_chain_metadata,
                        options.allow_comments,
                    )?;

                    if key_chain.len() == current_len {
                        return Ok(idx + chr.len_utf8());
                    }
                }
                // TODO I don't think this exists
                State::Comment {
                    ref mut last_was_asterisk,
                    ref mut multiline,
                    hash,
                    start,
                } => {
                    if chr == '\n' && !*multiline {
                        if let Some(TOMLKey::Index(..)) = key_chain.last() {
                            state = State::ExpectingValue;
                        } else {
                            state = State::InObject;
                        }
                    } else if chr == '*' && start + 1 == idx && !hash {
                        *multiline = true;
                    } else if *multiline {
                        if *last_was_asterisk && chr == '/' {
                            if let Some(TOMLKey::Index(..)) = key_chain.last() {
                                state = State::ExpectingValue;
                            } else {
                                state = State::InObject;
                            }
                        } else {
                            *last_was_asterisk = chr == '*';
                        }
                    }
                }
                State::InObject => {
                    if chr == '}' {
                        state = State::EndOfValue;
                    } else if let (true, c @ ('/' | '#')) = (options.allow_comments, chr) {
                        state = State::Comment {
                            last_was_asterisk: false,
                            start: idx,
                            multiline: false,
                            hash: c == '#',
                        };
                    } else if chr.is_alphabetic() {
                        state = State::InKey {
                            escaped: false,
                            start: idx,
                            last_was_dot: false,
                        }
                    } else if !chr.is_whitespace() {
                        return Err(TOMLParseError {
                            at: idx,
                            reason: TOMLParseErrorReason::ExpectedKey,
                        });
                    }
                }
                State::NumberValue { start } => {
                    // TODO actual number handing
                    if chr.is_whitespace() || matches!(chr, '}' | ',' | ']') {
                        let res = cb(
                            key_chain,
                            key_chain_metadata,
                            RootTOMLValue::Number(&on[start..idx]),
                        );
                        if res {
                            return Ok(idx);
                        }
                        state = State::EndOfValue;
                        end_of_value(
                            idx,
                            chr,
                            &mut state,
                            key_chain,
                            key_chain_metadata,
                            options.allow_comments,
                        )?;
                    }
                }
                State::TrueFalseNull { start } => {
                    let diff = idx - start + 1;
                    if diff < 4 {
                        // ...
                    } else if diff == 4 {
                        match &on[start..(idx + chr.len_utf8())] {
                            "true" => {
                                let res =
                                    cb(key_chain, key_chain_metadata, RootTOMLValue::Boolean(true));
                                if res {
                                    return Ok(idx + chr.len_utf8());
                                }
                                state = State::EndOfValue;
                            }
                            "null" => {
                                let res = cb(key_chain, key_chain_metadata, RootTOMLValue::Null);
                                if res {
                                    return Ok(idx + chr.len_utf8());
                                }
                                state = State::EndOfValue;
                            }
                            "fals" => {}
                            _ => {
                                return Err(TOMLParseError {
                                    at: idx,
                                    reason: TOMLParseErrorReason::ExpectedTrueFalseNull,
                                })
                            }
                        }
                    } else if let "false" = &on[start..(idx + chr.len_utf8())] {
                        let res = cb(key_chain, key_chain_metadata, RootTOMLValue::Boolean(false));
                        if res {
                            return Ok(idx + chr.len_utf8());
                        }
                        state = State::EndOfValue;
                    } else {
                        return Err(TOMLParseError {
                            at: idx,
                            reason: TOMLParseErrorReason::ExpectedTrueFalseNull,
                        });
                    }
                }
            }
        }

        match state {
            State::InKey { .. } | State::StringValue { .. } => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedQuote,
                })
            }
            State::Equal => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedEqual,
                });
            }
            State::Comment { multiline, .. } => {
                if multiline {
                    return Err(TOMLParseError {
                        at: on.len(),
                        reason: TOMLParseErrorReason::ExpectedEndOfMultilineComment,
                    });
                }
            }
            State::EndOfValue | State::ExpectingValue => {
                if !key_chain.is_empty() {
                    return Err(TOMLParseError {
                        at: on.len(),
                        reason: TOMLParseErrorReason::ExpectedBracket,
                    });
                }
            }
            State::InObject => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedBracket,
                });
            }
            State::NumberValue { start } => {
                // TODO actual number handing
                let _result = cb(
                    key_chain,
                    key_chain_metadata,
                    RootTOMLValue::Number(&on[start..]),
                );
            }
            State::TrueFalseNull { start: _ } => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedTrueFalseNull,
                })
            }
        }

        Ok(on.len())
    }

    // TODO always pops from key_chain **unless** we are in an array.
    // TODO there are complications using this in an iterator when we yielding numbers
    fn end_of_value(
        idx: usize,
        chr: char,
        state: &mut State,
        key_chain: &mut Vec<TOMLKey<'_>>,
        key_chain_metadata: &mut Vec<TOMLKeyMetadata>,
        allow_comments: bool,
    ) -> Result<(), TOMLParseError> {
        if let ',' = chr {
            if let Some(TOMLKey::Index(i)) = key_chain.last_mut() {
                *i += 1;
                *state = State::ExpectingValue;
                return Ok(());
            }
            *state = State::InObject;
        } else if let ('}', Some(TOMLKey::Slice(..))) = (chr, key_chain.last()) {
            // TODO errors here if index
        } else if let (']', Some(TOMLKey::Index(..))) = (chr, key_chain.last()) {
            // TODO errors here if slice etc
        } else if let (true, c @ ('/' | '#')) = (allow_comments, chr) {
            *state = State::Comment {
                last_was_asterisk: false,
                start: idx,
                multiline: false,
                hash: c == '#',
            };
        } else if !chr.is_whitespace() {
            dbg!(chr, key_chain);
            return Err(TOMLParseError {
                at: idx,
                reason: TOMLParseErrorReason::ExpectedEndOfValue,
            });
        }

        if !chr.is_whitespace() {
            let mut popped = key_chain.pop();
            let mut metadata = key_chain_metadata.pop();
            while let (Some(TOMLKey::Slice(_)), Some(TOMLKeyMetadata(Joined::Dot, _))) =
                (popped, metadata)
            {
                popped = key_chain.pop();
                metadata = key_chain_metadata.pop();
            }
        }

        Ok(())
    }
}

#[must_use]
pub fn matches(expecting: &[&str], chain: &[TOMLKey<'_>]) -> bool {
    expecting.len() == chain.len()
        && expecting
            .iter()
            .zip(chain.iter())
            .all(|(lhs, rhs)| match rhs {
                TOMLKey::Slice(rhs) => lhs == rhs,
                // TODO
                TOMLKey::Index(_) => false,
            })
}
