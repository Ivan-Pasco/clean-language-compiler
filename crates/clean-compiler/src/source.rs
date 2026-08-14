//! Byte-offset spans and the line map that converts them to the 1-based
//! line/character positions diagnostics carry (Platform 13 §2).

use clean_compiler_types::{Position, Span};

/// Half-open byte range into one source file's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteSpan {
    pub start: u32,
    pub end: u32,
}

impl ByteSpan {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: ByteSpan) -> ByteSpan {
        ByteSpan {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Maps byte offsets to 1-based (line, column) positions, columns counted in
/// characters (Platform 13 §2). Built once per source file.
#[derive(Debug)]
pub struct LineMap {
    /// Byte offset at which each line starts.
    line_starts: Vec<u32>,
    content_len: u32,
}

impl LineMap {
    pub fn new(content: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (offset, byte) in content.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset as u32 + 1);
            }
        }
        Self {
            line_starts,
            content_len: content.len() as u32,
        }
    }

    /// 1-based line and character column for a byte offset. The content is
    /// needed to count characters (not bytes) within the line.
    pub fn position(&self, content: &str, offset: u32) -> Position {
        let offset = offset.min(self.content_len);
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };
        let line_start = self.line_starts[line_index] as usize;
        let column = content[line_start..offset as usize].chars().count() as u32 + 1;
        Position {
            line: line_index as u32 + 1,
            column,
        }
    }

    /// Converts a byte span into the diagnostic span shape.
    pub fn span(&self, content: &str, file: &str, span: ByteSpan) -> Span {
        Span {
            file: file.to_string(),
            start: self.position(content, span.start),
            end: self.position(content, span.end),
        }
    }
}
