use std::iter::Peekable;
use std::str::CharIndices;

#[derive(PartialEq, Eq, Debug)]
/// Something with a position in some source code. This is useful for proper error reporting
pub struct Spanned<T>
where
    T: PartialEq + Eq,
{
    pub start_idx: usize, // inclusive
    pub end_idx: usize,   // exclusive
    pub line_nr: u32,     // the line of the start_idx
    pub content: T,
}

impl<T> Spanned<T>
where
    T: PartialEq + Eq,
{
    pub fn new(start_idx: usize, end_idx: usize, line_nr: u32, content: T) -> Self {
        Self {
            start_idx,
            end_idx,
            line_nr,
            content,
        }
    }

    /// Keeps the same location data as the old Spanned, but swaps out the inner value
    pub fn with_new_content<O>(&self, o: O) -> Spanned<O>
    where
        O: PartialEq + Eq,
    {
        Spanned::new(self.start_idx, self.end_idx, self.line_nr, o)
    }
}

/// A generic base Lexer similar to something like javas StreamTokenizer
pub struct StringLexer<'src> {
    source: &'src str,
    line_nr: u32,
    chars: Peekable<CharIndices<'src>>,
}

impl<'src> StringLexer<'src> {
    pub fn new(source: &'src str) -> Self {
        StringLexer {
            source,
            line_nr: 1,
            chars: source.char_indices().peekable(),
        }
    }

    pub fn current_char(&mut self) -> Option<Spanned<char>> {
        self.chars
            .peek()
            .map(|&(i, c)| Spanned::new(i, i + 1, self.line_nr, c))
    }

    pub fn current_eq(&mut self, test: char) -> bool {
        self.chars.peek().map_or(false, |&(_, c)| c == test)
    }

    pub fn advance(&mut self) -> Option<Spanned<char>> {
        let (i, c) = self.chars.next()?;
        if c == '\n' {
            self.line_nr += 1;
        }

        Some(Spanned::new(i, i + 1, self.line_nr, c))
    }

    pub fn take_chars_while<P>(&mut self, mut predicate: P) -> Option<Spanned<&'src str>>
    where
        P: FnMut(char) -> bool,
    {
        let Spanned { start_idx, .. } = self.current_char()?;
        let start_line = self.line_nr;

        let mut end_idx: usize = start_idx;
        loop {
            if let Some(&(i, c)) = self.chars.peek() {
                end_idx = i;
                if !predicate(c) {
                    break;
                }
                self.advance()?;
            } else {
                end_idx += 1;
                break;
            }
        }

        self.source
            .get(start_idx..end_idx)
            .map(|s| Spanned::new(start_idx, end_idx, start_line, s))
    }

    pub fn take_until_substr(&mut self, substr: &str) -> Option<Spanned<&'src str>> {
        let start_line = self.line_nr;
        let len = substr.len();

        let original_start: usize = *self.chars.peek().map(|(i, _)| i)?;
        let mut start = original_start;
        while let Some(s) = self.source.get(start..start + len) {
            if s == substr {
                return self
                    .source
                    .get(original_start..start)
                    .map(|s| Spanned::new(original_start, start, start_line, s));
            }
            start += 1;
            self.advance()?;
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_lexer_take_while() {
        let mut iter = StringLexer::new("hello world");
        assert_eq!(
            Some(Spanned::new(0, 0, 1, "")),
            iter.take_chars_while(char::is_whitespace)
        );

        assert_eq!(
            Some(Spanned::new(0, 0, 1, "")),
            iter.take_chars_while(|c| c != 'h')
        );

        assert_eq!(
            Some(Spanned::new(0, 5, 1, "hello")),
            iter.take_chars_while(|c| !c.is_whitespace())
        );

        iter.advance();

        assert_eq!(
            Some(Spanned::new(6, 11, 1, "world")),
            iter.take_chars_while(|c| !c.is_whitespace())
        );
    }

    #[test]
    fn test_lexer_take_until_substr_should_return_text_until_substr_if_found() {
        let mut iter = StringLexer::new("hello world");
        assert_eq!(
            Some(Spanned::new(0, 6, 1, "hello ")),
            iter.take_until_substr("world")
        );

        assert_eq!(
            Some(Spanned::new(6, 11, 1, "world")),
            iter.take_chars_while(|_| true)
        );
    }

    #[test]
    fn test_lexer_take_until_substr_should_return_empty_str_if_substr_is_at_beginning() {
        let mut iter = StringLexer::new("hello world");
        assert_eq!(
            Some(Spanned::new(0, 0, 1, "")),
            iter.take_until_substr("hello")
        );
    }

    #[test]
    fn test_lexer_take_until_substr_should_return_none_if_not_found() {
        let mut iter = StringLexer::new("hello world");
        assert_eq!(None, iter.take_until_substr("test"));
    }
}
