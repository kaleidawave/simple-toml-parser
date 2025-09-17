use super::{TOMLKey, TOMLKeyContext, TOMLNumberValue, TOMLStringValue};

impl<'a> TOMLNumberValue<'a> {
    #[must_use]
    pub fn raw(&self) -> &'a str {
        self.0
    }
}

impl std::fmt::Debug for TOMLNumberValue<'_> {
    // Required method
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{value}", value = self.0)
    }
}

impl std::fmt::Debug for TOMLStringValue<'_> {
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
    #[must_use]
    pub fn is_literal(&self) -> bool {
        self.literal
    }

    #[must_use]
    pub fn raw(&self) -> &'a str {
        self.on
    }

    /// # Panics
    ///
    /// (temporarily) panics for some bad values
    #[must_use]
    pub fn value(&self) -> std::borrow::Cow<'a, str> {
        if self.literal {
            std::borrow::Cow::Borrowed(self.on)
        } else {
            let mut start = 0;
            let mut value = std::borrow::Cow::Borrowed("");
            for (idx, _matched) in self.on.match_indices('\\') {
                value += &self.on[start..idx];
                match self.on[idx..].chars().nth(1) {
                    Some('\r' | '\n') => {
                        let after = &self.on[(idx + 1)..];
                        start = idx + 1 + (after.len() - after.trim_start().len());
                    }
                    Some('n') => {
                        value += "\n";
                        start = idx + 2;
                    }
                    Some('r') => {
                        value += "\r";
                        start = idx + 2;
                    }
                    Some('t') => {
                        value += "\t";
                        start = idx + 2;
                    }
                    Some('"') => {
                        start = idx + 1;
                    }
                    Some('u') => {
                        let after =
                            self.on[idx..][2..].split_once(|chr: char| !chr.is_ascii_hexdigit());
                        if let Some((after, _)) = after {
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
                                value += "\\u";
                                value += after;
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

#[derive(Debug)]
pub(crate) struct KeyChain<'a> {
    pub keys: Vec<TOMLKey<'a>>,
    pub context: TOMLKeyContext,
}

impl TOMLKeyContext {
    #[must_use]
    pub fn split_keys<'a, 'b, 'c>(
        &'c self,
        on: &'b [TOMLKey<'a>],
    ) -> (
        &'b [TOMLKey<'a>],
        &'b [TOMLKey<'a>],
        Partition<'b, 'c, TOMLKey<'a>>,
    ) {
        let table = &on[..self.table_keys as usize];
        let specifier = &on[self.table_keys as usize..][..self.specifier_keys as usize];
        let value = Partition::new(
            &on[self.table_keys as usize..][self.specifier_keys as usize..],
            &self.value_keys,
        );
        (table, specifier, value)
    }
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

    pub fn push_value_key(&mut self, key: TOMLKey<'a>) {
        self.keys.push(key);
        // TODO length
        self.context.value_keys.push(1);
    }

    pub fn pop_whole_slice_key(&mut self) {
        // TODO temp
        let is_index = matches!(self.keys.last(), Some(TOMLKey::Index(_)));
        if is_index {
            let _ = self.keys.pop();
            if !self.context.value_keys.is_empty() {
                self.context.value_keys.pop();
            } else if self.context.specifier_keys > 0 {
                self.context.specifier_keys -= 1;
            } else if self.context.table_keys > 0 {
                self.context.table_keys -= 1;
            }
        } else if let Some(item) = self.context.value_keys.pop() {
            (0..item).for_each(|_| {
                self.keys.pop();
            });
            if self.context.value_keys.is_empty() {
                self.context.in_value = false;
            }
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

    pub fn clear(&mut self) {
        self.keys.clear();
        self.context.value_keys.clear();
        self.context.specifier_keys = 0;
        self.context.table_keys = 0;
    }

    pub fn in_value(&self) -> bool {
        !self.context.value_keys.is_empty()
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

/// A structure for slices
#[allow(unused)]
pub struct Partition<'a, 'b, T> {
    on: &'a [T],
    points: &'b [u8],
}

impl<'a, 'b, T> Partition<'a, 'b, T> {
    pub fn new(on: &'a [T], points: &'b [u8]) -> Self {
        Self { on, points }
    }

    #[must_use]
    pub fn raw(&self) -> &'a [T] {
        self.on
    }

    #[must_use]
    pub fn iter(&self) -> PartitionIter<'_, 'a, 'b, T> {
        PartitionIter(self, 0, 0)
    }
}

impl<'a, 'b, 'c, T> IntoIterator for &'a Partition<'b, 'c, T> {
    type Item = &'a [T];
    type IntoIter = PartitionIter<'a, 'b, 'c, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct PartitionIter<'a, 'b, 'c, T>(
    pub(crate) &'a Partition<'b, 'c, T>,
    pub(crate) usize,
    pub(crate) usize,
);

// TODO partition iter
impl<'a, T> Iterator for PartitionIter<'a, '_, '_, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        let chunk_size: usize = *self.0.points.get(self.2)? as usize;
        self.1 = self.2;
        self.2 += chunk_size;
        Some(&self.0.on[self.1..][..chunk_size])
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CommentPosition {
    AtEnd,
    AnnotatingNext,
    Standalone,
}

/// # Panics
/// panics if the comment is not prefixed by '#' in the source
#[must_use]
pub fn get_comment_kind(source: &str, comment: &str) -> CommentPosition {
    let start = comment.as_ptr() as usize - source.as_ptr() as usize;
    let start = source[..start].rfind('#').unwrap();
    let before_line = source[..start].rsplit_once('\n').map(|(_, line)| line);
    // context.comment_after_value ||
    if before_line.is_none_or(|line| !line.trim().is_empty()) {
        CommentPosition::AtEnd
    } else if let Some((_, line)) = source[start..].split_once('\n')
        && let Some((between, _)) = line.split_once('\n')
        && between.trim().is_empty()
    {
        CommentPosition::Standalone
    } else {
        CommentPosition::AnnotatingNext
    }
}
