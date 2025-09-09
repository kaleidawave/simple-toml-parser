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
    // TODO pub prefix_indent_objects: bool,
    pub value_indentation: ValueIndentation,
    pub crlf: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CommentPosition {
    AtEnd,
    AnnotatingNext,
    Standalone,
}

#[derive(Debug)]
pub struct ObjectFindings<'a>(pub(crate) HashMap<Vec<TOMLKey<'a>>, u8>);

impl ObjectFindings<'_> {
    #[must_use]
    pub fn in_expanded_object(&self, chain: &[TOMLKey<'_>]) -> bool {
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
        ..Default::default()
    };

    // TODO under option?
    let mut object_property_counts: HashMap<Vec<TOMLKey<'a>>, u8> = HashMap::new();

    let mut calculate_object_properties =
        |keys: &[TOMLKey<'a>], context: &crate::TOMLKeyContext, value: crate::RootTOMLValue| {
            let table_and_specifier_keys_len = context.table_keys + context.specifier_keys;
            let (_table_and_specifier_keys, this_object) =
                keys.split_at(table_and_specifier_keys_len as usize);

            // Comments need to be lines
            if let crate::RootTOMLValue::Comment(_) = value {
                let entry = object_property_counts.entry(keys.to_vec()).or_default();
                *entry += 3;
            } else if !this_object.is_empty() {
                // TODO to_vec?
                let head = &keys[..keys.len() - 1];
                let entry = object_property_counts.entry(head.to_vec()).or_default();
                *entry = entry.saturating_add(1);

                // Also set parents
                for i in 1..this_object.len() {
                    let head_head = &keys[..keys.len() - 1 - i];
                    let entry = object_property_counts
                        .entry(head_head.to_vec())
                        .or_default();
                    *entry = entry.saturating_add(3);
                }
            }
        };

    // Find indentations
    let indentation: IndentationResult<'_> = match options.indentation {
        KeyIndentation::None => {
            parse_toml(input, parse_options, |keys, context, value| {
                calculate_object_properties(keys, context, value);
                false
            })?;
            IndentationResult::None
        }
        KeyIndentation::WholeDocument => {
            let mut indent = 0;
            parse_toml(input, parse_options, |keys, context, value| {
                calculate_object_properties(keys, context, value);
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
                calculate_object_properties(keys, context, value);
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

    // eprintln!("{object_property_counts:?}");

    let object_property_counts = ObjectFindings(object_property_counts);

    let new_line_sequence = if options.crlf { "\r\n" } else { "\n" };

    let indent: &str = options.value_indentation.0;

    let mut buf = String::new();

    let mut last_keys: Vec<TOMLKey<'a>> = Vec::new();
    let mut last_context: crate::TOMLKeyContext = crate::TOMLKeyContext::default();

    parse_toml(
        input,
        parse_options,
        |keys: &[TOMLKey<'a>], context, value| {
            if let RootTOMLValue::Comment(comment) = value {
                let start = comment.as_ptr() as usize - input.as_ptr() as usize;
                let start = input[..start].rfind('#').unwrap();
                let kind = {
                    let before_line = input[..start].rsplit_once('\n').map(|(_, line)| line);
                    if before_line.is_none_or(|line| !line.trim().is_empty()) {
                        CommentPosition::AtEnd
                    } else if let Some((_, line)) = input[start..].split_once('\n')
                        && let Some((between, _)) = line.split_once('\n')
                        && between.trim().is_empty()
                    {
                        CommentPosition::Standalone
                    } else {
                        CommentPosition::AnnotatingNext
                    }
                };

                if let CommentPosition::Standalone | CommentPosition::AnnotatingNext = kind {
                    if !buf.is_empty() {
                        buf.push_str(new_line_sequence);
                    }
                    buf.push_str(new_line_sequence);
                    buf.push_str("# ");
                } else {
                    buf.push_str(" # ");
                }
                buf.push_str(comment);
                if let CommentPosition::Standalone = kind {
                    buf.push_str(new_line_sequence);
                }
                return false;
            }

            let (last_table, last_specifier, last_object) = last_context.split_keys(&last_keys);
            let (this_table, this_specifier, this_object) = context.split_keys(keys);

            let same_table = this_table == last_table;
            let same_specifier = this_specifier == last_specifier;

            let last_object = last_object.raw();
            let this_object = this_object.raw();

            let same_object_root = same_table && same_specifier;
            let object_difference = if same_object_root {
                split_prefix_difference(last_object, this_object)
            } else {
                Difference {
                    removed: last_object,
                    same: &[],
                    new: this_object,
                }
            };

            let Difference {
                mut removed,
                same,
                mut new,
            } = object_difference;

            let (incident_key, common) = if let ([existing, r2 @ ..], [new_key, n2 @ ..]) =
                (removed, new)
                && same_object_root
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

            if !removed.is_empty() {
                push_removed(
                    removed,
                    common,
                    (&last_keys, &last_context),
                    (indent, new_line_sequence),
                    &object_property_counts,
                    &mut buf,
                );
            }

            if same_object_root && !last_object.is_empty() {
                buf.push(',');
            }

            if let Some(incident_key) = incident_key {
                let current_chain = &keys[..this_table.len() + this_specifier.len() + same.len()];
                // eprintln!("  current {current_chain:?}");
                let pretty = object_property_counts.in_expanded_object(current_chain);
                if pretty {
                    write!(&mut buf, "{new_line_sequence}").unwrap();
                    for _ in 0..current_chain.len() {
                        write!(&mut buf, "{indent}").unwrap();
                    }
                    if let TOMLKey::Slice(incident_key) = incident_key {
                        write!(&mut buf, "{incident_key} = ").unwrap();
                    }
                } else if let TOMLKey::Slice(incident_key) = incident_key {
                    write!(&mut buf, " {incident_key} = ").unwrap();
                }
            }

            {
                if !same_table {
                    if !buf.is_empty() {
                        buf.push_str(new_line_sequence);
                        buf.push_str(new_line_sequence);
                    }
                    let is_table = this_table
                        .iter()
                        .any(|key| matches!(key, TOMLKey::Index(_)));
                    buf.push_str(if is_table { "[[" } else { "[" });

                    let mut keys = this_table.iter();
                    if let Some(first) = keys.next() {
                        match first {
                            TOMLKey::Slice(key) => buf.push_str(key),
                            TOMLKey::Index(_key) => {}
                        }
                        for item in keys {
                            match item {
                                TOMLKey::Slice(key) => {
                                    buf.push('.');
                                    buf.push_str(key);
                                }
                                TOMLKey::Index(_key) => {}
                            }
                        }
                    }
                    buf.push_str(if is_table { "]]" } else { "]" });
                }

                if !same_specifier {
                    if !buf.is_empty() {
                        buf.push_str(new_line_sequence);
                    }
                    if options.prefix_indent_keys {
                        buf.push_str("  ");
                    }

                    let mut idx = 0;
                    let mut iter = this_specifier.iter();
                    if let Some(first) = iter.next() {
                        match first {
                            TOMLKey::Slice(key) => {
                                idx += key.len();
                                buf.push_str(key);
                            }
                            TOMLKey::Index(_key) => todo!("index key"),
                        }
                        for item in iter {
                            idx += 1;
                            buf.push('.');
                            match item {
                                TOMLKey::Slice(key) => {
                                    idx += key.len();
                                    buf.push_str(key);
                                }
                                TOMLKey::Index(_key) => todo!("index key"),
                            }
                        }
                    } else {
                        dbg!("no specifiers...", keys);
                    }

                    match &indentation {
                        IndentationResult::WholeDocument(indent) => {
                            buf.push_str(&"                    "[idx..*indent]);
                        }
                        IndentationResult::Section(indents) => {
                            let prefix = &keys[..context.table_keys as usize];
                            let indent = *indents.get(prefix).unwrap();
                            buf.push_str(&"                    "[idx..indent]);
                        }
                        IndentationResult::None => {}
                    }

                    buf.push_str(" = ");
                }
            }

            for (level_root, key) in new.iter().enumerate() {
                let level = level_root + common;
                let current_chain = &keys[..this_table.len() + this_specifier.len() + level];
                // eprintln!("  new: {current_chain:?} {level:?}");
                let pretty = object_property_counts.in_expanded_object(current_chain);

                match key {
                    TOMLKey::Slice(key) => {
                        if pretty && buf.ends_with(',') {
                            write!(&mut buf, "{new_line_sequence}").unwrap();
                            for _ in 0..level {
                                write!(&mut buf, "{indent}").unwrap();
                            }
                        }
                        write!(&mut buf, "{{").unwrap();
                        if pretty {
                            write!(&mut buf, "{new_line_sequence}").unwrap();
                            for _ in 0..=level {
                                write!(&mut buf, "{indent}").unwrap();
                            }
                            write!(&mut buf, "{key} = ").unwrap();
                        } else {
                            write!(&mut buf, " {key} = ").unwrap();
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
                                for _ in this_object {
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
            if !(buf.ends_with(char::is_whitespace) || buf.ends_with('[') || buf.ends_with(indent))
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
                RootTOMLValue::Comment(..) => unreachable!("handled earlier"),
                RootTOMLValue::Null => buf.push_str("null"),
            }

            keys.clone_into(&mut last_keys);
            context.clone_into(&mut last_context);

            false
        },
    )?;

    let (_last_table, _last_specifier, last_object) = last_context.split_keys(&last_keys);
    let last_object = last_object.raw();
    let Difference { removed, .. } = split_prefix_difference(last_object, &[]);

    push_removed(
        removed,
        0,
        (&last_keys, &last_context),
        (indent, new_line_sequence),
        &object_property_counts,
        &mut buf,
    );

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
    object_property_counts: &ObjectFindings,
    buf: &mut String,
) {
    for (level, key) in removed.iter().enumerate().rev() {
        let level = level + common;
        let current_chain = &last_keys
            [..last_context.table_keys as usize + last_context.specifier_keys as usize + level];
        // eprintln!("  removed {current_chain:?}");
        let pretty = object_property_counts.in_expanded_object(current_chain);

        match key {
            TOMLKey::Slice(_key) => {
                if pretty {
                    write!(buf, "{new_line_sequence}").unwrap();
                    // for _ in 0..=level {
                    for _ in 0..level {
                        write!(buf, "{indent}").unwrap();
                    }
                }

                if pretty {
                    buf.push('}');
                } else {
                    buf.push_str(" }");
                }
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
