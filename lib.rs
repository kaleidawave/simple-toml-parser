pub mod formatting;
pub mod utilities;

use utilities::KeyChain;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TOMLKeyContext {
    pub table_keys: u8,
    pub specifier_keys: u8,
    /// values can have joined keys
    pub value_keys: Vec<u8>,
    /// for comments. in array or inline-table
    pub in_value: bool,
    /// for comments
    pub comment_after_value: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TOMLParseOptions {
    pub yield_comments: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TOMLKey<'a> {
    Slice(&'a str),
    Index(usize),
}

impl TOMLKey<'_> {
    #[must_use]
    pub fn same_variant(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Slice(_), Self::Slice(_)) => true,
            (Self::Index(a), Self::Index(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Level {
    Table,
    #[default]
    Specifier,
    Value,
}

#[derive(Debug, PartialEq, Hash, Clone, Copy)]
pub enum RootTOMLValue<'a> {
    String(TOMLStringValue<'a>),
    Number(TOMLNumberValue<'a>),
    Boolean(bool),
    /// Not a value but for formamtting
    Comment(&'a str),
    /// For document preservation
    EmptyArray,
    EmptyInlineTable,
}

/// Last is literal
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct TOMLStringValue<'a> {
    on: &'a str,
    literal: bool,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct TOMLNumberValue<'a>(pub(crate) &'a str);

#[derive(Default, Debug)]
pub struct KeyState {
    pub in_string: Option<char>,
    // pub last_was_string: bool,
}

#[derive(Debug)]
pub enum TOMLParseErrorReason {
    ExpectedColon,
    ExpectedEndOfValue,
    ExpectedBracket,
    ExpectedBoolean,
    ExpectedValue,
    ExpectedEqual,
    ExpectedQuote,
    ExpectedEndOfMultilineComment,
    ExpectedKey,
    InvalidKey,
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

/// If you want to return early (not parse the whole input) use [`parse_toml_with_options`]
/// and that contains more information about keys
///
/// # Errors
/// Returns an error if it tries to parse invalid TOML input
pub fn parse_toml<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], RootTOMLValue<'a>),
) -> Result<(), TOMLParseError> {
    parse_toml_with_options(on, TOMLParseOptions::default(), |k, _m, v| {
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

#[derive(Debug)]
enum State {
    StartOfLine,
    // AwaitingEqual,
    InComment,
    InKey { state: KeyState, context: Context },
}

/// # Errors
/// Returns an error if it tries to parse invalid TOML input
///
/// # Panics
/// On unimplemented items
#[allow(clippy::too_many_lines)]
pub fn parse_toml_with_options<'a>(
    on: &'a str,
    options: TOMLParseOptions,
    mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b TOMLKeyContext, RootTOMLValue<'a>) -> bool,
) -> Result<(), TOMLParseError> {
    let mut key_chain = KeyChain::new();
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
                            // HMM ?
                            start = idx + 1;
                            Context::Table
                        };
                        state = State::InKey {
                            context,
                            state: KeyState::default(),
                        };
                    }
                    '#' => {
                        start = idx + 1;
                        state = State::InComment;
                    }
                    chr => {
                        if chr.is_alphabetic() {
                            start = idx;
                            state = State::InKey {
                                context: Context::CurrentLevel,
                                state: KeyState::default(),
                            };
                        } else if !chr.is_whitespace() {
                            return Err(TOMLParseError {
                                at: idx,
                                reason: TOMLParseErrorReason::ExpectedKey,
                            });
                        }
                    }
                }
            }
            State::InKey {
                context,
                state: ref mut key_state,
            } => {
                match (context, chr) {
                    (Context::ArrayOfTables | Context::Table, ']')
                        if key_state.in_string.is_none() =>
                    {
                        if let Context::ArrayOfTables = context {
                            let valid = chars.next().is_some_and(|(_, chr)| chr == ']');
                            assert!(valid, "TODO error");
                        }
                        let level = Level::Table;
                        let part = &on[start..idx];
                        if let Context::ArrayOfTables = context {
                            // TODO check prefix. if does not match then want zero
                            let idx = if let Some(TOMLKey::Index(idx)) = key_chain.keys.last() {
                                idx + 1
                            } else {
                                0
                            };
                            // Collect here
                            let () = key_chain.clear();
                            let result = parse_toml_key(part, level, &mut key_chain);
                            if let Err(()) = result {
                                return Err(TOMLParseError {
                                    at: start,
                                    reason: TOMLParseErrorReason::InvalidKey,
                                });
                            }
                            key_chain.push_table_key(TOMLKey::Index(idx));
                        } else {
                            key_chain.clear();
                            let result = parse_toml_key(part, level, &mut key_chain);
                            if let Err(()) = result {
                                return Err(TOMLParseError {
                                    at: start,
                                    reason: TOMLParseErrorReason::InvalidKey,
                                });
                            }
                        }
                        state = State::StartOfLine;
                        start = idx + 1;
                    }
                    (Context::CurrentLevel, '=') if key_state.in_string.is_none() => {
                        let part = &on[start..idx];
                        let level = Level::Specifier;
                        let result = parse_toml_key(part, level, &mut key_chain);
                        if let Err(()) = result {
                            return Err(TOMLParseError {
                                at: start,
                                reason: TOMLParseErrorReason::InvalidKey,
                            });
                        }

                        let result = value::parse_toml_with_options_chars(
                            on,
                            &mut chars,
                            &mut key_chain,
                            options,
                            &mut cb,
                        );

                        if let Ok(Some(_result)) = result {
                            return Ok(());
                        }

                        state = State::StartOfLine;
                        key_chain.pop_whole_slice_key();
                    }
                    (_ctx, '"' | '\'') => {
                        if let Some(delim) = key_state.in_string {
                            if delim == chr {
                                key_state.in_string = None;
                            }
                        } else {
                            key_state.in_string = Some(chr);
                        }
                    }
                    (_ctx, _chr) => {}
                }
            }
            State::InComment => {
                if let '\n' = chr {
                    if options.yield_comments {
                        let comment = on[start..idx].trim();
                        let result = cb(
                            &key_chain.keys,
                            &key_chain.context,
                            RootTOMLValue::Comment(comment),
                        );
                        if result {
                            return Ok(());
                        }
                    }
                    state = State::StartOfLine;
                }
            }
        }
    }

    match state {
        State::InComment => {
            if options.yield_comments {
                let comment = on[start..].trim();
                let _ = cb(
                    &key_chain.keys,
                    &key_chain.context,
                    RootTOMLValue::Comment(comment),
                );
            }
        }
        State::StartOfLine => {}
        State::InKey { .. } => panic!("TODO return error for leftover {state:?}"),
    }

    Ok(())
}

fn parse_toml_key<'a>(part: &'a str, level: Level, key_chain: &mut KeyChain<'a>) -> Result<(), ()> {
    let mut last = 0;
    let mut in_string: Option<&str> = None;
    let mut found = false;

    for (idx, matched) in part.match_indices(&['"', '\'', '.']) {
        if let Some(expected) = in_string {
            if expected == matched {
                let key = &part[(last + 1)..idx];
                let key = TOMLKey::Slice(key);
                match level {
                    Level::Table => key_chain.push_table_key(key),
                    Level::Specifier => key_chain.push_specifier_key(key),
                    Level::Value => key_chain.push_value_key(key),
                }
                last = idx + 1;
                in_string = None;
                found = true;
            }
        } else if let "." = matched {
            if !part[..idx].ends_with(['"', '\'']) {
                let key = &part[last..idx].trim();
                let key = TOMLKey::Slice(key);
                match level {
                    Level::Table => key_chain.push_table_key(key),
                    Level::Specifier => key_chain.push_specifier_key(key),
                    Level::Value => key_chain.push_value_key(key),
                }
                found = true;
            }
            last = idx + 1;
        } else {
            assert!(part[..idx].trim_end().ends_with('.'));
            in_string = Some(matched);
        }
    }

    let key = part[last..].trim();
    if (!found && key.is_empty()) || in_string.is_some() {
        Err(())
    } else if !key.is_empty() {
        let key = TOMLKey::Slice(key);
        match level {
            Level::Table => key_chain.push_table_key(key),
            Level::Specifier => key_chain.push_specifier_key(key),
            Level::Value => key_chain.push_value_key(key),
        }
        Ok(())
    } else {
        // TODO assert something here...?
        Ok(())
    }
}

/// Contains parsing for things after the `=`. Includes a function for parsing the RHS (standalone)
///
/// TODO date handling
pub mod value {
    use super::{
        KeyChain, KeyState, RootTOMLValue, TOMLKey, TOMLKeyContext, TOMLNumberValue,
        TOMLParseError, TOMLParseErrorReason, TOMLParseOptions, TOMLStringValue,
    };

    #[derive(Debug)]
    enum State {
        InKey {
            start: usize,
            state: KeyState,
        },
        AwaitingEqual,
        InInlineTable {
            yielded: bool,
        },
        InArray {
            yielded: bool,
        },
        InComment {
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
            multiline: bool,
        },
        NumberValue {
            start: usize,
        },
        Boolean {
            start: usize,
        },
        EndOfValue,
    }

    /// # Errors
    /// errors on invalid TOML syntax
    pub fn parse_toml_with_options<'a>(
        on: &'a str,
        options: TOMLParseOptions,
        mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b TOMLKeyContext, RootTOMLValue<'a>) -> bool,
    ) -> Result<Option<usize>, TOMLParseError> {
        let mut chars = on.char_indices();
        let mut key_chain = KeyChain::new();
        parse_toml_with_options_chars(on, &mut chars, &mut key_chain, options, &mut cb)
    }

    /// Returns `Some` if finished early
    #[allow(clippy::too_many_lines)]
    pub(crate) fn parse_toml_with_options_chars<'a>(
        on: &'a str,
        chars: &mut std::str::CharIndices<'a>,
        key_chain: &mut KeyChain<'a>,
        options: TOMLParseOptions,
        cb: &mut impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b TOMLKeyContext, RootTOMLValue<'a>) -> bool,
    ) -> Result<Option<usize>, TOMLParseError> {
        let mut state = State::ExpectingValue;

        let current_len = key_chain.keys.len();

        while let Some((idx, chr)) = chars.next() {
            match state {
                State::ExpectingValue | State::InArray { .. } => {
                    state = match chr {
                        '[' => {
                            key_chain.context.in_value = true;
                            key_chain.push_value_key(TOMLKey::Index(0));
                            State::InArray { yielded: false }
                        }
                        ']' => {
                            if let State::InArray { yielded } = state {
                                let () = key_chain.pop_whole_slice_key();
                                if !yielded {
                                    let _result = cb(
                                        &key_chain.keys,
                                        &key_chain.context,
                                        RootTOMLValue::EmptyArray,
                                    );
                                }
                                key_chain.context.in_value =
                                    !key_chain.context.value_keys.is_empty();
                                State::EndOfValue
                            } else {
                                panic!("err")
                            }
                        }
                        '{' => {
                            key_chain.context.in_value = true;
                            State::InInlineTable { yielded: false }
                        }
                        '}' => {
                            dbg!();
                            // hmm
                            key_chain.pop_whole_slice_key();
                            let in_array =
                                matches!(key_chain.keys.last(), Some(TOMLKey::Index(..)));

                            if in_array {
                                State::ExpectingValue
                            } else {
                                // Continues work...
                                // key_chain.pop_whole_slice_key();
                                State::InInlineTable { yielded: true }
                            }
                        }
                        '"' => {
                            let multiline = on[idx..].starts_with(r#"""""#);
                            let start = if multiline {
                                // advance_by
                                let _ = chars.next();
                                let _ = chars.next();
                                idx + 3
                            } else {
                                idx + 1
                            };
                            State::StringValue {
                                start,
                                literal: false,
                                escaped: false,
                                multiline,
                            }
                        }
                        '\'' => {
                            let multiline = on[idx..].starts_with("'''");
                            let start = if multiline {
                                // advance_by
                                let _ = chars.next();
                                let _ = chars.next();
                                idx + 3
                            } else {
                                idx + 1
                            };
                            State::StringValue {
                                start,
                                literal: true,
                                escaped: false,
                                multiline,
                            }
                        }
                        '/' | '#' => State::InComment {
                            last_was_asterisk: false,
                            start: idx + 1,
                            multiline: false,
                            hash: chr == '#',
                        },
                        '0'..='9' | '-' => State::NumberValue { start: idx },
                        't' | 'f' => State::Boolean { start: idx },
                        chr if chr.is_whitespace() => state,
                        _ => {
                            return Err(TOMLParseError {
                                at: idx,
                                reason: TOMLParseErrorReason::ExpectedValue,
                            });
                        }
                    }
                }
                // TODO parse_toml key function
                State::InKey {
                    ref mut start,
                    state: ref mut _key_state,
                } => {
                    let is_equal = chr == '=';
                    let is_whitespace = chr.is_whitespace();
                    // TODO more based on existing
                    if is_equal || is_whitespace {
                        let key = &on[*start..idx];
                        key_chain.push_value_key(TOMLKey::Slice(key));
                        state = if is_equal {
                            State::ExpectingValue
                        } else {
                            State::AwaitingEqual
                        };
                    } else if let '.' = chr {
                        key_chain.push_value_key(TOMLKey::Slice(&on[*start..idx]));
                        *start = idx + chr.len_utf8();
                        // key_state.joined = Joined::Dot;
                    } else {
                        // *escaped = chr == '\\';
                    }
                }
                State::StringValue {
                    start,
                    literal,
                    ref mut escaped,
                    multiline,
                } => {
                    if *escaped {
                        *escaped = false;
                    } else {
                        let (r#yield, idx) = if multiline {
                            let idx = idx + 1;
                            let r#yield = idx > start + 3
                                && on[..idx].ends_with(if literal { "'''" } else { r#"""""# })
                                && !on[idx..].starts_with(if literal { '\'' } else { '"' })
                                && !on[..idx - 3].ends_with('\\');

                            (r#yield, idx - 3)
                        } else {
                            ((literal && chr == '\'') || (!literal && chr == '"'), idx)
                        };
                        if r#yield {
                            let value = &on[start..idx];
                            let on = if multiline {
                                value
                                    .strip_prefix("\r\n")
                                    .or_else(|| value.strip_prefix('\n'))
                                    .unwrap_or(value)
                            } else {
                                value
                            };
                            let value = TOMLStringValue { on, literal };
                            let result = cb(
                                &key_chain.keys,
                                &key_chain.context,
                                RootTOMLValue::String(value),
                            );
                            if result {
                                return Ok(Some(idx + chr.len_utf8()));
                            }
                            state = State::EndOfValue;
                        } else if !literal {
                            *escaped = chr == '\\';
                        }
                    }
                }
                State::AwaitingEqual => {
                    if chr == '=' {
                        state = State::ExpectingValue;
                    } else if !chr.is_whitespace() {
                        return Err(TOMLParseError {
                            at: idx,
                            reason: TOMLParseErrorReason::ExpectedEqual,
                        });
                    }
                }
                // State::AwaitingCommaOrClose => {
                //     match chr {
                //         ',' => {
                //             state = State::ExpectingValue;
                //         }
                //         ']' => {
                //             state = State::ExpectingValue;
                //         }
                //         '}' => {
                //             state = State::ExpectingValue;
                //         }
                //         '#' | '/' => {
                //             State::InComment {
                //                 last_was_asterisk: false,
                //                 start: idx + 1,
                //                 multiline: false,
                //                 hash: chr == '#',
                //             }
                //         }
                //         chr => {
                //             if !chr.is_whitespace() {
                //                 return Err(TOMLParseError {
                //                     at: idx,
                //                     reason: TOMLParseErrorReason::ExpectedEqual,
                //                 });
                //             }
                //         }
                //     }
                // }
                State::EndOfValue => {
                    state = end_of_value(idx, chr, key_chain)?;

                    let r#return = key_chain.keys.len() == current_len
                        && !matches!(state, State::InInlineTable { .. } | State::InComment { .. })
                        && !key_chain.in_value();

                    if r#return {
                        return Ok(None);
                    }
                }
                // TODO I don't think multiline exists?
                State::InComment {
                    ref mut last_was_asterisk,
                    ref mut multiline,
                    hash,
                    start,
                } => {
                    if chr == '\n' && !*multiline {
                        if options.yield_comments {
                            let comment = on[start..idx].trim();
                            let result = cb(
                                &key_chain.keys,
                                &key_chain.context,
                                RootTOMLValue::Comment(comment),
                            );

                            if result {
                                return Ok(None);
                            }
                        }

                        if key_chain.context.comment_after_value {
                            key_chain.pop_whole_slice_key();
                            key_chain.context.comment_after_value = false;

                            let r#return = key_chain.keys.is_empty();

                            if r#return {
                                return Ok(None);
                            }
                        }

                        if let Some(TOMLKey::Index(..)) = key_chain.keys.last() {
                            state = State::InArray { yielded: true };
                        } else {
                            state = State::InInlineTable { yielded: true };
                        }
                    } else if chr == '*' && start + 1 == idx && !hash {
                        *multiline = true;
                    } else if *multiline {
                        if *last_was_asterisk && chr == '/' {
                            // TODO emit comment
                            if let Some(TOMLKey::Index(..)) = key_chain.keys.last() {
                                state = State::InArray { yielded: true };
                            } else {
                                state = State::InInlineTable { yielded: true };
                            }
                        } else {
                            *last_was_asterisk = chr == '*';
                        }
                    }
                }
                State::InInlineTable { yielded } => match chr {
                    '}' => {
                        if !yielded {
                            let _result = cb(
                                &key_chain.keys,
                                &key_chain.context,
                                RootTOMLValue::EmptyInlineTable,
                            );
                        }
                        state = State::EndOfValue;
                    }
                    '#' => {
                        state = State::InComment {
                            last_was_asterisk: false,
                            start: idx + 1,
                            multiline: false,
                            hash: chr == '#',
                        };
                    }
                    chr if chr.is_alphabetic() => {
                        state = State::InKey {
                            start: idx,
                            state: KeyState::default(),
                        }
                    }
                    chr => {
                        if !chr.is_whitespace() {
                            return Err(TOMLParseError {
                                at: idx,
                                reason: TOMLParseErrorReason::ExpectedKey,
                            });
                        }
                    }
                },
                State::NumberValue { start } => {
                    // TODO actual number handing
                    // TODO if date : etc
                    if chr.is_whitespace() || matches!(chr, '}' | ',' | ']' | '#') {
                        let value = RootTOMLValue::Number(TOMLNumberValue(&on[start..idx]));
                        let result = cb(&key_chain.keys, &key_chain.context, value);
                        if result {
                            return Ok(Some(idx));
                        }

                        // Because chr can be delimeter, we run `end_of_value` here
                        state = end_of_value(idx, chr, key_chain)?;

                        let r#return = key_chain.keys.len() == current_len
                            && !matches!(state, State::InInlineTable { .. })
                            && !key_chain.in_value()
                            && !on[idx..].trim_start().starts_with('#');
                        // && !utilities::rest_of_line(&on[idx..]).contains('#');

                        if r#return {
                            return Ok(None);
                        }
                    }
                }
                State::Boolean { start } => {
                    let diff = idx - start + 1;
                    if diff < 4 {
                        // ...
                    } else if diff == 4 {
                        match &on[start..(idx + chr.len_utf8())] {
                            "true" => {
                                let result = cb(
                                    &key_chain.keys,
                                    &key_chain.context,
                                    RootTOMLValue::Boolean(true),
                                );
                                if result {
                                    return Ok(Some(idx + chr.len_utf8()));
                                }
                                state = State::EndOfValue;
                            }
                            "fals" => {}
                            _ => {
                                return Err(TOMLParseError {
                                    at: idx,
                                    reason: TOMLParseErrorReason::ExpectedBoolean,
                                });
                            }
                        }
                    } else if let "false" = &on[start..(idx + chr.len_utf8())] {
                        let result = cb(
                            &key_chain.keys,
                            &key_chain.context,
                            RootTOMLValue::Boolean(false),
                        );
                        if result {
                            return Ok(Some(idx + chr.len_utf8()));
                        }
                        state = State::EndOfValue;
                    } else {
                        return Err(TOMLParseError {
                            at: idx,
                            reason: TOMLParseErrorReason::ExpectedBoolean,
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
                });
            }
            State::AwaitingEqual => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedEqual,
                });
            }
            State::InComment { multiline, .. } => {
                if multiline {
                    return Err(TOMLParseError {
                        at: on.len(),
                        reason: TOMLParseErrorReason::ExpectedEndOfMultilineComment,
                    });
                }
            }
            State::EndOfValue | State::ExpectingValue | State::InArray { .. } => {
                if !key_chain.keys.is_empty() {
                    return Err(TOMLParseError {
                        at: on.len(),
                        reason: TOMLParseErrorReason::ExpectedBracket,
                    });
                }
            }
            State::InInlineTable { .. } => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedBracket,
                });
            }
            State::NumberValue { start } => {
                // TODO actual number handing
                let result = cb(
                    &key_chain.keys,
                    &key_chain.context,
                    RootTOMLValue::Number(TOMLNumberValue(&on[start..])),
                );
                if result {
                    return Ok(Some(on.len()));
                }
            }
            State::Boolean { start: _ } => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedBoolean,
                });
            }
        }

        Ok(None)
    }

    /// Always pops from `key_chain`
    /// TODO should return new state
    fn end_of_value(
        idx: usize,
        chr: char,
        key_chain: &mut KeyChain<'_>,
    ) -> Result<State, TOMLParseError> {
        match chr {
            ',' => {
                if let Some(TOMLKey::Index(i)) = key_chain.keys.last_mut() {
                    *i += 1;
                    Ok(State::InArray { yielded: true })
                } else {
                    key_chain.pop_whole_slice_key();
                    Ok(State::InInlineTable { yielded: true })
                }
            }
            '}' => {
                let last = key_chain.keys.last();
                // TODO errors here if index
                assert!(matches!(last, Some(TOMLKey::Slice(..))));
                key_chain.pop_whole_slice_key();
                Ok(State::EndOfValue)
            }
            ']' => {
                let last = key_chain.keys.last();
                // TODO errors here if slice etc
                assert!(matches!(last, Some(TOMLKey::Index(..))));
                key_chain.pop_whole_slice_key();
                key_chain.context.in_value = !key_chain.context.value_keys.is_empty();
                Ok(State::EndOfValue)
            }
            '#' | '/' => {
                key_chain.context.comment_after_value = true;
                Ok(State::InComment {
                    last_was_asterisk: false,
                    start: idx + 1,
                    multiline: false,
                    hash: chr == '#',
                })
            }
            chr => {
                if chr.is_whitespace() {
                    Ok(State::EndOfValue)
                } else {
                    eprintln!("Error {chr:?}, {key_chain:?}");
                    Err(TOMLParseError {
                        at: idx,
                        reason: TOMLParseErrorReason::ExpectedEndOfValue,
                    })
                }
            }
        }
    }
}
