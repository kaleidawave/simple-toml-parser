#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct TOMLKeyContext {
    pub table_keys: u8,
    pub specifier_keys: u8,
    // later keys get posted here
    pub object_keys: Vec<u8>,
}

impl TOMLKeyContext {
    #[must_use]
    pub fn split_keys<'a, 'b>(
        &'a self,
        on: &'b [TOMLKey<'a>],
    ) -> (
        &'b [TOMLKey<'a>],
        &'b [TOMLKey<'a>],
        Partition<'b, TOMLKey<'a>>,
    ) {
        (
            &on[..self.table_keys as usize],
            &on[self.table_keys as usize..][..self.specifier_keys as usize],
            Partition::new(
                &on[self.table_keys as usize..][self.specifier_keys as usize..],
                &self.object_keys,
            ),
        )
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Level {
    Table,
    #[default]
    Specifier,
    Object,
}

#[derive(Debug, PartialEq, Hash)]
pub enum RootTOMLValue<'a> {
    String(TOMLStringValue<'a>),
    Number(&'a str),
    Boolean(bool),
    Null,
}

/// Last is literal
#[derive(PartialEq, Eq, Hash)]
pub struct TOMLStringValue<'a> {
    on: &'a str,
    literal: bool,
}

impl<'a> std::fmt::Debug for TOMLStringValue<'a> {
    // Required method
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        if self.literal {
            write!(f, "'{value}'", value = self.on)
        } else {
            write!(f, "{value:?}", value = self.on)
        }
    }
}

impl<'a> TOMLStringValue<'a> {
    pub fn is_literal(&self) -> bool {
        self.literal
    }

    pub fn raw(&self) -> &'a str {
        self.on
    }

    pub fn value(&self) -> std::borrow::Cow<'a, str> {
        if self.literal {
            std::borrow::Cow::Borrowed(self.on)
        } else {
            let mut start = 0;
            let mut value = std::borrow::Cow::Borrowed("");
            for (idx, _matched) in self.on.match_indices('\\') {
                value += std::borrow::Cow::Borrowed(&self.on[start..idx]);
                match self.on[idx..].chars().nth(1) {
                    Some('\r' | '\n') => {
                        let after = &self.on[(idx + 1)..];
                        start = idx + 1 + (after.len() - after.trim_start().len());
                    }
                    Some('n') => {
                        value += std::borrow::Cow::Borrowed("\n");
                        start = idx + 2;
                    }
                    Some('r') => {
                        value += std::borrow::Cow::Borrowed("\r");
                        start = idx + 2;
                    }
                    Some('t') => {
                        value += std::borrow::Cow::Borrowed("\t");
                        start = idx + 2;
                    }
                    Some('"') => {
                        start = idx + 1;
                    }
                    Some('u') => {
                        let after = self.on[idx..][2..].split_once(|chr: char| !chr.is_digit(16));
                        if let Some((after, _)) = after {
                            assert!(after.len() <= 4);
                            let mut unicode_code = 0u32;
                            for byte in after.as_bytes() {
                                unicode_code <<= 4; // 16=2^4
                                match byte {
                                    b'0'..=b'9' => {
                                        unicode_code += u32::from(byte - b'0');
                                    }
                                    b'a'..=b'f' => {
                                        unicode_code += u32::from(byte - b'a') + 10;
                                    }
                                    b'A'..=b'F' => {
                                        unicode_code += u32::from(byte - b'A') + 10;
                                    }
                                    _ => unreachable!(),
                                }
                            }
                            if let Some(chr) = char::from_u32(unicode_code) {
                                value.to_mut().push(chr);
                            } else {
                                eprintln!("bad code {after}");
                            }
                            start = idx + 2 + after.len();
                        } else {
                            eprintln!("bad char");
                        }
                    }
                    chr => {
                        eprintln!("bad char {chr:?}");
                    }
                }
            }
            value += std::borrow::Cow::Borrowed(&self.on[start..]);
            value
        }
    }
}

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
    ExpectedTrueFalseNull,
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
    // Equal,
    InComment,
    InKey { state: KeyState, context: Context },
}

#[derive(Debug)]
struct KeyChain<'a> {
    pub keys: Vec<TOMLKey<'a>>,
    pub context: TOMLKeyContext,
}

impl<'a> KeyChain<'a> {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            context: TOMLKeyContext::default(),
        }
    }

    pub fn push_table_key(&mut self, key: TOMLKey<'a>) {
        self.keys.push(key);
        self.context.table_keys += 1;
    }

    pub fn push_specifier_key(&mut self, key: TOMLKey<'a>) {
        self.keys.push(key);
        self.context.specifier_keys += 1;
    }

    pub fn push_object_key(&mut self, key: TOMLKey<'a>) {
        self.keys.push(key);
        // TODO length
        self.context.object_keys.push(1);
    }

    pub fn pop_whole_slice_key(&mut self) {
        // TODO temp
        let is_index = matches!(self.keys.last(), Some(TOMLKey::Index(_)));
        if is_index {
            let _ = self.keys.pop();
            if !self.context.object_keys.is_empty() {
                self.context.object_keys.pop();
            } else if self.context.specifier_keys > 0 {
                self.context.specifier_keys -= 1;
            } else if self.context.table_keys > 0 {
                self.context.table_keys -= 1;
            }
        } else {
            if let Some(item) = self.context.object_keys.pop() {
                (0..item).for_each(|_| {
                    self.keys.pop();
                });
            } else if self.context.specifier_keys > 0 {
                let offset = self.context.table_keys;
                let range = offset as usize..(self.context.specifier_keys + offset) as usize;
                self.keys.drain(range);
                self.context.specifier_keys = 0;
            } else if self.context.table_keys > 0 {
                self.keys.clear();
                self.context.table_keys = 0;
            }
        }
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.context.object_keys.clear();
        self.context.specifier_keys = 0;
        self.context.table_keys = 0;
    }

    pub fn in_object(&self) -> bool {
        !self.context.object_keys.is_empty()
    }
}

/// # Errors
/// Returns an error if it tries to parse invalid TOML input
///
/// # Panics
/// On unimplemented items
#[allow(clippy::too_many_lines)]
pub fn parse_with_exit_signal<'a>(
    on: &'a str,
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
                            if !valid {
                                panic!("TODO error")
                            }
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
                            let _ = key_chain.clear();
                            let result = parse_key(part, level, &mut key_chain);
                            if let Err(()) = result {
                                return Err(TOMLParseError {
                                    at: start,
                                    reason: TOMLParseErrorReason::InvalidKey,
                                });
                            }
                            key_chain.push_table_key(TOMLKey::Index(idx));
                        } else {
                            key_chain.clear();
                            let result = parse_key(part, level, &mut key_chain);
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
                        let result = parse_key(part, level, &mut key_chain);
                        if let Err(()) = result {
                            return Err(TOMLParseError {
                                at: start,
                                reason: TOMLParseErrorReason::InvalidKey,
                            });
                        }

                        let result = value::parse_with_exit_signal_chars(
                            on,
                            &mut chars,
                            &mut key_chain,
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
                    state = State::StartOfLine;
                }
            }
        }
    }

    Ok(())
}

fn parse_key<'a>(part: &'a str, level: Level, key_chain: &mut KeyChain<'a>) -> Result<(), ()> {
    let mut last = 0;
    let mut in_string: Option<&str> = None;
    // let mut joined = Joined::Alone;
    let mut found = false;

    for (idx, matched) in part.match_indices(&['"', '\'', '.']) {
        if let Some(expected) = in_string {
            if expected == matched {
                let key = &part[(last + 1)..idx];
                let key = TOMLKey::Slice(key);
                match level {
                    Level::Table => key_chain.push_table_key(key),
                    Level::Specifier => key_chain.push_specifier_key(key),
                    Level::Object => key_chain.push_object_key(key),
                }
                last = idx + 1;
                in_string = None;
                found = true;
            }
        } else {
            if let "." = matched {
                let key = &part[last..idx].trim();
                let key = TOMLKey::Slice(key);
                match level {
                    Level::Table => key_chain.push_table_key(key),
                    Level::Specifier => key_chain.push_specifier_key(key),
                    Level::Object => key_chain.push_object_key(key),
                }
                // joined = Joined::Dot;
                found = true;
                last = idx + 1;
            } else {
                assert!(part[..idx].trim_end().ends_with("."));
                in_string = Some(matched);
            }
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
            Level::Object => key_chain.push_object_key(key),
        }
        Ok(())
    } else {
        // TODO assert something here...?
        Ok(())
    }
}

pub mod value {
    use super::{
        KeyChain, KeyState, RootTOMLValue, TOMLKey, TOMLKeyContext, TOMLParseError,
        TOMLParseErrorReason, TOMLStringValue,
    };

    #[derive(Debug)]
    enum State {
        InKey {
            start: usize,
            state: KeyState,
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
            multiline: bool,
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
        mut cb: impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b TOMLKeyContext, RootTOMLValue<'a>) -> bool,
    ) -> Result<Option<usize>, TOMLParseError> {
        let mut chars = on.char_indices();
        let mut key_chain = KeyChain::new();
        parse_with_exit_signal_chars(on, &mut chars, &mut key_chain, &mut cb)
    }

    /// TODO: `allow_comments` fix
    /// Returns Some if finished early
    #[allow(clippy::too_many_lines)]
    pub(crate) fn parse_with_exit_signal_chars<'a>(
        on: &'a str,
        chars: &mut std::str::CharIndices<'a>,
        key_chain: &mut KeyChain<'a>,
        cb: &mut impl for<'b> FnMut(&'b [TOMLKey<'a>], &'b TOMLKeyContext, RootTOMLValue<'a>) -> bool,
    ) -> Result<Option<usize>, TOMLParseError> {
        // Temp fix
        struct Options {
            pub allow_comments: bool,
        }

        let options = Options {
            allow_comments: false,
        };

        let mut state = State::ExpectingValue;

        let current_len = key_chain.keys.len();

        while let Some((idx, chr)) = chars.next() {
            match state {
                State::ExpectingValue => {
                    state = match chr {
                        '[' => {
                            key_chain.push_object_key(TOMLKey::Index(0));
                            State::ExpectingValue
                        }
                        ']' => {
                            // TODO check
                            let _ = key_chain.pop_whole_slice_key();
                            State::EndOfValue
                        }
                        '{' => State::InObject,
                        '}' => {
                            let in_array =
                                matches!(key_chain.keys.last(), Some(TOMLKey::Index(..)));
                            let _context = key_chain.pop_whole_slice_key();
                            if in_array {
                                State::ExpectingValue
                            } else {
                                // Continues work...
                                // key_chain.pop_whole_slice_key();
                                State::InObject
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
                    state: ref mut _key_state,
                } => {
                    let is_equal = chr == '=';
                    let is_whitespace = chr.is_whitespace();
                    // TODO more based on existing
                    if is_equal || is_whitespace {
                        let key = &on[*start..idx];
                        key_chain.push_object_key(TOMLKey::Slice(key));
                        state = if is_equal {
                            State::ExpectingValue
                        } else {
                            State::Equal
                        };
                    } else if let '.' = chr {
                        key_chain.push_object_key(TOMLKey::Slice(&on[*start..idx]));
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
                            // dbg!(start, idx, multiline, on.len());
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
                    end_of_value(idx, chr, &mut state, key_chain, options.allow_comments)?;

                    // dbg!(key_chain.keys.len(), current_len, &state);
                    let r#return = key_chain.keys.len() == current_len
                        && !matches!(state, State::InObject)
                        && !key_chain.in_object();

                    if r#return {
                        return Ok(None);
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
                        if let Some(TOMLKey::Index(..)) = key_chain.keys.last() {
                            state = State::ExpectingValue;
                        } else {
                            state = State::InObject;
                        }
                    } else if chr == '*' && start + 1 == idx && !hash {
                        *multiline = true;
                    } else if *multiline {
                        if *last_was_asterisk && chr == '/' {
                            if let Some(TOMLKey::Index(..)) = key_chain.keys.last() {
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
                            start: idx,
                            state: KeyState::default(),
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
                        let result = cb(
                            &key_chain.keys,
                            &key_chain.context,
                            RootTOMLValue::Number(&on[start..idx]),
                        );
                        if result {
                            return Ok(Some(idx));
                        }
                        state = State::EndOfValue;
                        end_of_value(idx, chr, &mut state, key_chain, options.allow_comments)?;

                        let r#return = key_chain.keys.len() == current_len
                            && !matches!(state, State::InObject)
                            && !key_chain.in_object();

                        if r#return {
                            return Ok(None);
                        }
                    }
                }
                State::TrueFalseNull { start } => {
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
                            "null" => {
                                let result =
                                    cb(&key_chain.keys, &key_chain.context, RootTOMLValue::Null);
                                if result {
                                    return Ok(Some(idx + chr.len_utf8()));
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
                if !key_chain.keys.is_empty() {
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
                let result = cb(
                    &key_chain.keys,
                    &key_chain.context,
                    RootTOMLValue::Number(&on[start..]),
                );
                if result {
                    return Ok(Some(on.len()));
                }
            }
            State::TrueFalseNull { start: _ } => {
                return Err(TOMLParseError {
                    at: on.len(),
                    reason: TOMLParseErrorReason::ExpectedTrueFalseNull,
                })
            }
        }

        Ok(None)
    }

    /// Always pops from key_chain **unless** we are in an array.
    fn end_of_value(
        idx: usize,
        chr: char,
        state: &mut State,
        key_chain: &mut KeyChain<'_>,
        allow_comments: bool,
    ) -> Result<(), TOMLParseError> {
        if let ',' = chr {
            if let Some(TOMLKey::Index(i)) = key_chain.keys.last_mut() {
                *i += 1;
                *state = State::ExpectingValue;
            } else {
                *state = State::InObject;
                key_chain.pop_whole_slice_key();
            }
            Ok(())
        } else if let '}' = chr {
            let last = key_chain.keys.last();
            // TODO errors here if index
            assert!(matches!(last, Some(TOMLKey::Slice(..))));
            key_chain.pop_whole_slice_key();
            Ok(())
        } else if let ']' = chr {
            let last = key_chain.keys.last();
            // TODO errors here if slice etc
            assert!(matches!(last, Some(TOMLKey::Index(..))));
            key_chain.pop_whole_slice_key();
            Ok(())
        } else if let (true, c @ ('/' | '#')) = (allow_comments, chr) {
            // TODO
            key_chain.pop_whole_slice_key();
            *state = State::Comment {
                last_was_asterisk: false,
                start: idx,
                multiline: false,
                hash: c == '#',
            };
            Ok(())
        } else if !chr.is_whitespace() {
            eprintln!("Error {chr:?}, {key_chain:?}");
            Err(TOMLParseError {
                at: idx,
                reason: TOMLParseErrorReason::ExpectedEndOfValue,
            })
        } else {
            Ok(())
        }
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

pub struct Partition<'a, T> {
    on: &'a [T],
    points: &'a [u8],
}

impl<'a, T> Partition<'a, T> {
    pub fn new(on: &'a [T], points: &'a [u8]) -> Self {
        Self { on, points }
    }
}

impl<'a, T> Iterator for Partition<'a, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        let split = self.points.first()?;
        self.points = &self.points[1..];
        let (items, rest) = self.on.split_at(*split as usize);
        self.on = rest;
        Some(items)
    }
}
