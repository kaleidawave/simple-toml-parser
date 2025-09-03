use super::{
    CommentPosition, RootTOMLValue, TOMLKey, TOMLParseError, TOMLParseOptions,
    parse_with_options as parse_toml,
};

#[derive(Debug, Default, Copy, Clone)]
pub enum Indentation {
    WholeDocument,
    Section,
    #[default]
    None,
}

#[derive(Default)]
pub struct FormatOptions {
    pub indentation: Indentation,
    pub prefix_indent_keys: bool,
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

    let mut buf = String::new();

    let mut last_table: Vec<TOMLKey<'a>> = Vec::new();
    let mut last_specifier: Vec<TOMLKey<'a>> = Vec::new();
    let mut last_object: Vec<TOMLKey<'a>> = Vec::new();

    let parse_options = TOMLParseOptions {
        yield_comments: true,
        ..Default::default()
    };

    // Find indentations
    let indentation: IndentationResult<'_> = match options.indentation {
        Indentation::None => IndentationResult::None,
        Indentation::WholeDocument => {
            let mut indent = 0;
            parse_toml(input, parse_options, |keys, context, _value| {
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
        Indentation::Section => {
            let mut indents = std::collections::HashMap::new();
            parse_toml(input, parse_options, |keys, context, _value| {
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

    parse_toml(
        input,
        parse_options,
        |keys: &[TOMLKey<'a>], context, value| {
            if let RootTOMLValue::Comment(comment, kind) = value {
                // dbg!(comment, kind);
                if let CommentPosition::Standalone | CommentPosition::AnnotatingNext = kind {
                    if !buf.is_empty() {
                        buf.push('\n');
                    }
                    buf.push_str("\n# ");
                } else {
                    buf.push_str(" # ");
                }
                buf.push_str(comment);
                // buf.push('\n');
                if let CommentPosition::Standalone = kind {
                    buf.push('\n');
                }
                return false;
            }

            let (this_table, this_specifier, this_object) = context.split_keys(keys);

            let same_table = this_table == last_table;
            let same_specifier = this_specifier == last_specifier;

            let this_object = this_object.raw();

            // , TOMLKey::same_variant
            let same_object_root = same_table && same_specifier;
            let object_difference = if same_object_root {
                split_prefix_difference(&last_object, this_object)
            } else {
                Difference {
                    removed: &last_object,
                    same: &[],
                    new: this_object,
                }
            };

            let Difference {
                removed,
                same: _,
                new,
            } = object_difference;

            eprintln!("removed: {removed:?} -> new: {new:?}");

            for (idx, key) in removed.iter().enumerate() {
                let respective_new = new.get(idx);
                match key {
                    TOMLKey::Slice(_key) => {
                        if !same_object_root || respective_new.is_none() {
                            buf.push_str(" }");
                        }
                    }
                    TOMLKey::Index(removed_index) => {
                        let is_end_of_array = if let Some(TOMLKey::Index(index)) = respective_new {
                            // maybe a bit over the top
                            removed_index + 1 != *index
                        } else {
                            true
                        };
                        if is_end_of_array {
                            buf.push(']');
                        }
                    }
                }
            }

            if same_table
                && same_specifier
                && !last_object.is_empty()
                && !context.object_keys.is_empty()
            {
                buf.push_str(", ");
            }

            // // TODO ...?
            // if !(same_table && same_specifier) && !last_object.is_empty() {
            //     buf.push('\n');
            // }

            if !same_table {
                if !buf.is_empty() {
                    buf.push_str("\n\n");
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
                    buf.push('\n');
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

            for (idx, key) in new.iter().enumerate() {
                let respective_removed = removed.get(idx);
                match key {
                    TOMLKey::Slice(key) => {
                        if !same_object_root || respective_removed.is_none() {
                            buf.push_str("{ ");
                        }
                        buf.push_str(key);
                        buf.push_str(" = ");
                    }
                    TOMLKey::Index(new_index) => {
                        let is_start_of_array =
                            if let Some(TOMLKey::Index(removed_index)) = respective_removed {
                                // maybe a bit over the top
                                removed_index + 1 != *new_index
                            } else {
                                true
                            };
                        if is_start_of_array {
                            buf.push('[');
                        }
                    }
                }
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

            // if context.object_keys.is_empty() {
            //     buf.push('\n');
            // }

            this_table.clone_into(&mut last_table);
            this_specifier.clone_into(&mut last_specifier);
            this_object.clone_into(&mut last_object);

            false
        },
    )?;

    let Difference { removed, .. } = split_prefix_difference(&last_object, &[]);

    for key in removed {
        match key {
            TOMLKey::Slice(_key) => buf.push_str(" }"),
            TOMLKey::Index(_key) => buf.push(']'),
        }
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
    let mut idx = 0;
    for (l, t) in last.iter().zip(this) {
        if l == t {
            idx += 1;
        }
    }

    let removed = &last[idx..];
    let (same, new) = this.split_at(idx);
    Difference { removed, same, new }
}
// fn split_prefix_difference<'a, T>(
//     last: &'a [T],
//     this: &'a [T],
//     equality: impl for<'b> Fn(&'b T, &'b T) -> bool,
// ) -> Difference<'a, T> {
//     let mut idx = 0;
//     for (l, t) in last.iter().zip(this) {
//         if equality(l, t) {
//             idx += 1;
//         }
//     }

//     let removed = &last[idx..];
//     let (same, new) = this.split_at(idx);
//     Difference { removed, same, new }
// }
