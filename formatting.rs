use super::{
    RootTOMLValue, TOMLKey, TOMLParseError, TOMLParseOptions, parse_toml_with_options as parse_toml,
};

use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug, Default, Copy, Clone)]
pub enum KeyIndentation {
    WholeDocument,
    Section,
    #[default]
    None,
}

/// We create a new type so that default works
#[derive(Debug, Copy, Clone)]
pub struct ValueIndentation(pub(crate) &'static str);

impl Default for ValueIndentation {
    fn default() -> Self {
        Self("\t")
    }
}

impl From<&'static str> for ValueIndentation {
    fn from(value: &'static str) -> Self {
        Self(value)
    }
}

impl AsRef<str> for ValueIndentation {
    fn as_ref(&self) -> &str {
        self.0
    }
}

#[derive(Default)]
pub struct FormatOptions {
    pub indentation: KeyIndentation,
    pub prefix_indent_keys: bool,
    // TODO pub prefix_indent_values: bool,
    pub value_indentation: ValueIndentation,
    pub use_crlf: bool,
}

#[derive(Debug)]
pub struct ValueFindings<'a>(pub(crate) HashMap<Vec<TOMLKey<'a>>, u8>);

impl ValueFindings<'_> {
    #[must_use]
    pub fn in_expanded_value(&self, chain: &[TOMLKey<'_>]) -> bool {
        const PROPERTY_COUNT_THRESHOLD: u8 = 3;

        let current_properties = self.0.get(chain).copied().unwrap_or_default();

        current_properties >= PROPERTY_COUNT_THRESHOLD
    }
}

/// # Errors
///
/// Returns an error if the TOML document is invalid
///
/// # Panics
///
/// May panic on internal problems
#[allow(clippy::too_many_lines)]
pub fn format_toml<'a>(input: &'a str, options: &FormatOptions) -> Result<String, TOMLParseError> {
    enum IndentationResult<'a> {
        WholeDocument(usize),
        Section(std::collections::HashMap<Vec<TOMLKey<'a>>, usize>),
        None,
    }

    let parse_options = TOMLParseOptions {
        yield_comments: true,
    };

    // TODO under option?
    let mut value_property_counts: HashMap<Vec<TOMLKey<'a>>, u8> = HashMap::new();

    let mut calculate_value_properties =
        |keys: &[TOMLKey<'a>], context: &crate::TOMLKeyContext, value: crate::RootTOMLValue| {
            if context.in_value {
                let table_and_specifier_keys_len = context.table_keys + context.specifier_keys;
                let (_table_and_specifier_keys, this_value) =
                    keys.split_at(table_and_specifier_keys_len as usize);

                // Comments need to be lines
                if let crate::RootTOMLValue::Comment(_) = value {
                    // TODO
                    let keys = &keys[..keys.len().saturating_sub(1)];
                    // let keys = if context.comment_after_value {
                    // } else {
                    //     keys
                    // };
                    let entry = value_property_counts.entry(keys.to_vec()).or_default();
                    *entry += 3;
                } else {
                    // TODO to_vec?
                    let head = &keys[..keys.len() - 1];
                    let entry = value_property_counts.entry(head.to_vec()).or_default();
                    *entry = entry.saturating_add(1);

                    // Also set parents
                    for i in 1..this_value.len() {
                        let head_head = &keys[..keys.len() - 1 - i];
                        let entry = value_property_counts.entry(head_head.to_vec()).or_default();
                        *entry = entry.saturating_add(3);
                    }
                }
            }
        };

    // Find indentations
    let indentation: IndentationResult<'_> = match options.indentation {
        KeyIndentation::None => {
            parse_toml(input, parse_options, |keys, context, value| {
                calculate_value_properties(keys, context, value);
                false
            })?;
            IndentationResult::None
        }
        KeyIndentation::WholeDocument => {
            let mut indent = 0;
            parse_toml(input, parse_options, |keys, context, value| {
                calculate_value_properties(keys, context, value);
                let mut len = 0;
                for key in &keys[context.table_keys as usize..][..context.specifier_keys as usize] {
                    if len != 0 {
                        len += 1;
                    }
                    if let TOMLKey::Slice(key) = key {
                        len += key.len();
                    }
                }

                indent = std::cmp::max(indent, len);

                false
            })?;
            IndentationResult::WholeDocument(indent)
        }
        KeyIndentation::Section => {
            let mut indents = std::collections::HashMap::new();
            parse_toml(input, parse_options, |keys, context, value| {
                calculate_value_properties(keys, context, value);
                let mut len = 0;
                for key in &keys[context.table_keys as usize..][..context.specifier_keys as usize] {
                    if len != 0 {
                        len += 1;
                    }
                    if let TOMLKey::Slice(key) = key {
                        len += key.len();
                    }
                }

                let prefix = &keys[..context.table_keys as usize];
                if let Some(indent) = indents.get_mut(prefix) {
                    *indent = std::cmp::max(*indent, len);
                } else {
                    indents.insert(prefix.to_owned(), len);
                }

                false
            })?;
            IndentationResult::Section(indents)
        }
    };

    let value_property_counts = ValueFindings(value_property_counts);

    // eprintln!("{value_property_counts:?}");

    let new_line_sequence = if options.use_crlf { "\r\n" } else { "\n" };

    let indent: &str = options.value_indentation.0;

    let mut buf = String::new();

    let mut last_keys: Vec<TOMLKey<'a>> = Vec::new();
    let mut last_context: crate::TOMLKeyContext = crate::TOMLKeyContext::default();

    parse_toml(
        input,
        parse_options,
        |keys: &[TOMLKey<'a>], context, value| {
            let is_comment = matches!(value, RootTOMLValue::Comment(..));
            let is_value = !is_comment;

            let (last_table, last_specifier, last_value) = last_context.split_keys(&last_keys);
            let (this_table, this_specifier, this_value) = context.split_keys(keys);

            let last_value = last_value.raw();
            let this_value = this_value.raw();

            // TODO `EmptyArray` and `EmptyInlineTable` return here
            // cannot have comments in inline-tables
            if is_comment
                && this_value
                    .iter()
                    .any(|key| matches!(key, TOMLKey::Slice(_)))
            {
                return false;
            }

            let same_table = this_table == last_table;
            let same_specifier = this_specifier == last_specifier;

            let same_value_root = same_table && same_specifier;
            let value_difference = if same_value_root {
                split_prefix_difference(last_value, this_value)
            } else {
                Difference {
                    removed: last_value,
                    same: &[],
                    new: this_value,
                }
            };

            let Difference {
                mut removed,
                same,
                mut new,
            } = value_difference;

            let (incident_key, common) = if let ([existing, r2 @ ..], [new_key, n2 @ ..]) =
                (removed, new)
                && same_value_root
                && (matches!((existing, new_key), (TOMLKey::Slice(_), TOMLKey::Slice(_)))
                    || matches!((existing, new_key), (TOMLKey::Index(prev), TOMLKey::Index(this)) if prev + 1 == *this))
            {
                removed = r2;
                new = n2;
                (Some(new_key), 1)
            } else {
                (None, 0)
            };

            // eprintln!();
            // eprintln!("removed: {removed:?} -> new: {new:?}. v={value:?}, ic={incident_key:?}");

            if !removed.is_empty() || !context.in_value {
                push_removed(
                    removed,
                    common,
                    (&last_keys, &last_context),
                    (indent, new_line_sequence),
                    &value_property_counts,
                    &mut buf,
                );
            }

            if same_value_root && !last_value.is_empty() && !is_comment {
                buf.push(',');
            }

            if let Some(incident_key) = incident_key {
                let current_chain = &keys[..this_table.len() + this_specifier.len() + same.len()];
                // eprintln!("  current {current_chain:?}");

                let pretty = value_property_counts.in_expanded_value(current_chain)
                    && is_array_chain(this_value);
                if let TOMLKey::Slice(incident_key) = incident_key {
                    write!(&mut buf, " {incident_key} = ").unwrap();
                } else if pretty && !is_comment {
                    write!(&mut buf, "{new_line_sequence}").unwrap();
                    for _ in new.len()..this_value.len() {
                        write!(&mut buf, "{indent}").unwrap();
                    }
                }
            }

            {
                if !same_table {
                    let is_start = buf.is_empty();
                    if !(is_start || buf.ends_with('\n')) {
                        buf.push_str(new_line_sequence);
                    }
                    if !(is_start || last_line_is_comment(&buf)) {
                        buf.push_str(new_line_sequence);
                    }
                    let is_table = this_table
                        .iter()
                        .any(|key| matches!(key, TOMLKey::Index(_)));
                    buf.push_str(if is_table { "[[" } else { "[" });

                    let mut keys = this_table.iter();
                    if let Some(first) = keys.next() {
                        match first {
                            TOMLKey::Slice(key) => {
                                let wrapping = if is_alphanumeric_string(key) { "" } else { "'" };
                                write!(&mut buf, "{wrapping}{key}{wrapping}").unwrap();
                            }
                            TOMLKey::Index(_key) => {}
                        }
                        for item in keys {
                            match item {
                                TOMLKey::Slice(key) => {
                                    let wrapping =
                                        if is_alphanumeric_string(key) { "" } else { "'" };
                                    write!(&mut buf, ".{wrapping}{key}{wrapping}").unwrap();
                                }
                                TOMLKey::Index(_key) => {}
                            }
                        }
                    }
                    buf.push_str(if is_table { "]]" } else { "]" });
                }

                if !same_specifier {
                    if !is_comment {
                        if !buf.is_empty() {
                            buf.push_str(new_line_sequence);
                        }
                        if options.prefix_indent_keys {
                            buf.push_str(indent);
                        }
                    }

                    let buf_len = buf.len();
                    let mut iter = this_specifier.iter();
                    if let Some(first) = iter.next() {
                        match first {
                            TOMLKey::Slice(key) => {
                                let wrapping = if is_alphanumeric_string(key) { "" } else { "'" };
                                write!(&mut buf, "{wrapping}{key}{wrapping}").unwrap();
                            }
                            TOMLKey::Index(_key) => todo!("index key"),
                        }
                        for item in iter {
                            match item {
                                TOMLKey::Slice(key) => {
                                    let wrapping =
                                        if is_alphanumeric_string(key) { "" } else { "'" };
                                    write!(&mut buf, ".{wrapping}{key}{wrapping}").unwrap();
                                }
                                TOMLKey::Index(_key) => todo!("index key"),
                            }
                        }
                    } else {
                        // eprintln!("no specifiers... {keys:?} {context:?}");
                    }

                    match &indentation {
                        IndentationResult::WholeDocument(indent) => {
                            let key_width = buf.len() - buf_len;
                            buf.push_str(&"                    "[key_width..*indent]);
                        }
                        IndentationResult::Section(indents) => {
                            let key_width = buf.len() - buf_len;
                            let prefix = &keys[..context.table_keys as usize];
                            let indent = *indents.get(prefix).unwrap();
                            buf.push_str(&"                    "[key_width..indent]);
                        }
                        IndentationResult::None => {}
                    }

                    if !is_comment {
                        buf.push_str(" = ");
                    }
                }
            }

            for (level_root, key) in new.iter().enumerate() {
                let level = level_root + common;
                let current_chain = &keys[..this_table.len() + this_specifier.len() + level];
                // eprintln!("  new: {current_chain:?} {level:?}");
                let pretty = value_property_counts.in_expanded_value(current_chain)
                    && is_array_chain(this_value);

                match key {
                    TOMLKey::Slice(key) => {
                        // if pretty && buf.ends_with(',') {
                        //     write!(&mut buf, "{new_line_sequence}").unwrap();
                        //     for _ in 0..level {
                        //         write!(&mut buf, "{indent}").unwrap();
                        //     }
                        // }
                        write!(&mut buf, "{{").unwrap();

                        let wrapping = if is_alphanumeric_string(key) { "" } else { "'" };
                        if false {
                            // write!(&mut buf, "{new_line_sequence}").unwrap();
                            // for _ in 0..=level {
                            //     write!(&mut buf, "{indent}").unwrap();
                            // }
                            write!(&mut buf, "{wrapping}{key}{wrapping} = ").unwrap();
                        } else {
                            write!(&mut buf, " {wrapping}{key}{wrapping} = ").unwrap();
                        }
                    }
                    TOMLKey::Index(_) => {
                        // TODO is the best way to do this?
                        if !(buf.ends_with(',')
                            || buf.ends_with('[')
                            || buf.ends_with("= ")
                            || buf.ends_with(indent))
                        {
                            if pretty {
                                write!(&mut buf, "{new_line_sequence}").unwrap();
                                for _ in 1..this_value.len() {
                                    write!(&mut buf, "{indent}").unwrap();
                                }
                            } else {
                                buf.push(' ');
                            }
                        }
                        write!(&mut buf, "[").unwrap();
                        if pretty {
                            write!(&mut buf, "{new_line_sequence}").unwrap();
                            for _ in 0..=level {
                                write!(&mut buf, "{indent}").unwrap();
                            }
                        }
                    }
                }
            }

            // TODO is the best way to do this?
            if is_value
                && !(buf.ends_with(char::is_whitespace)
                    || buf.ends_with('[')
                    || buf.ends_with(indent))
            {
                buf.push(' ');
            }

            match value {
                RootTOMLValue::String(value) => {
                    if value.is_literal() {
                        buf.push('\'');
                        buf.push_str(value.raw());
                        buf.push('\'');
                    } else {
                        buf.push('"');
                        buf.push_str(&value.value());
                        buf.push('"');
                    }
                }
                RootTOMLValue::Number(value) => {
                    buf.push_str(value.raw());
                }
                RootTOMLValue::Boolean(true) => buf.push_str("true"),
                RootTOMLValue::Boolean(false) => buf.push_str("false"),
                RootTOMLValue::Comment(comment) => {
                    use crate::utilities::{CommentPosition, get_comment_kind};

                    let kind = get_comment_kind(input, comment);

                    if let CommentPosition::Standalone | CommentPosition::AnnotatingNext = kind {
                        // let pretty = value_property_counts.in_expanded_value(keys);
                        let should_add_second = !buf.is_empty()
                            && buf
                                .rsplit_once('\n')
                                .is_none_or(|(_, after)| !after.starts_with(['#', '[']))
                            && this_value.is_empty();
                        if should_add_second {
                            buf.push_str(new_line_sequence);
                        }
                        buf.push_str(new_line_sequence);
                        // if pretty {
                        for _ in 0..context.value_keys.len() {
                            write!(&mut buf, "{indent}").unwrap();
                        }
                        // }
                        buf.push_str("# ");
                    } else {
                        buf.push_str(" # ");
                    }
                    buf.push_str(comment);
                    if let CommentPosition::Standalone = kind {
                        buf.push_str(new_line_sequence);
                    }

                    // if !removed.is_empty() && context.in_value {
                    //     push_removed(
                    //         removed,
                    //         common,
                    //         (&last_keys, &last_context),
                    //         (indent, new_line_sequence),
                    //         &value_property_counts,
                    //         &mut buf,
                    //     );
                    // }
                }
                // TODO under cfg ?
                RootTOMLValue::EmptyArray => buf.push_str("[]"),
                // TODO under cfg ?
                RootTOMLValue::EmptyInlineTable => buf.push_str("{}"),
            }

            keys.clone_into(&mut last_keys);
            context.clone_into(&mut last_context);

            false
        },
    )?;

    let (_last_table, _last_specifier, last_value) = last_context.split_keys(&last_keys);
    let last_value = last_value.raw();
    let Difference { removed, .. } = split_prefix_difference(last_value, &[]);

    push_removed(
        removed,
        0,
        (&last_keys, &last_context),
        (indent, new_line_sequence),
        &value_property_counts,
        &mut buf,
    );

    if !buf.ends_with('\n') {
        write!(&mut buf, "{new_line_sequence}").unwrap();
    }

    Ok(buf)
}

struct Difference<'a, T> {
    pub removed: &'a [T],
    #[allow(unused)]
    pub same: &'a [T],
    pub new: &'a [T],
}

fn split_prefix_difference<'a, T: std::cmp::PartialEq>(
    last: &'a [T],
    this: &'a [T],
) -> Difference<'a, T> {
    let common = this
        .iter()
        .zip(last.iter())
        .take_while(|(k, l)| k == l)
        .count();

    let removed = &last[common..];
    let (same, new) = this.split_at(common);
    Difference { removed, same, new }
}

fn push_removed(
    removed: &[TOMLKey<'_>],
    common: usize,
    (last_keys, last_context): (&[TOMLKey<'_>], &crate::TOMLKeyContext),
    (indent, new_line_sequence): (&str, &str),
    value_property_counts: &ValueFindings,
    buf: &mut String,
) {
    for (level, key) in removed.iter().enumerate().rev() {
        let level = level + common;
        let last_chain = &last_keys
            [..last_context.table_keys as usize + last_context.specifier_keys as usize + level];
        // eprintln!("  removed {last_chain:?} {removed:?}");
        let last_value =
            &last_keys[last_context.table_keys as usize + last_context.specifier_keys as usize..];
        let pretty =
            value_property_counts.in_expanded_value(last_chain) && is_array_chain(last_value);

        match key {
            TOMLKey::Slice(_key) => {
                buf.push_str(" }");
            }
            TOMLKey::Index(_key) => {
                if pretty {
                    write!(buf, "{new_line_sequence}").unwrap();
                    for _ in 0..level {
                        write!(buf, "{indent}").unwrap();
                    }
                }

                buf.push(']');
            }
        }
    }
}

fn is_alphanumeric_string(key: &str) -> bool {
    key.chars()
        .all(|chr: char| chr.is_alphanumeric() || matches!(chr, '-' | '_'))
}

fn is_array_chain(this_value: &[TOMLKey<'_>]) -> bool {
    this_value[..this_value.len().saturating_sub(1)]
        .iter()
        .all(|key| matches!(key, TOMLKey::Index(_)))
}

fn last_line_is_comment(buf: &str) -> bool {
    if let Some((_, after)) = buf.rsplit_once('\n')
        && after.starts_with('#')
    {
        true
    } else {
        false
    }
}
