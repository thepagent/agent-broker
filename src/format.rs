/// Split text into chunks at line boundaries, each <= limit bytes (UTF-8 safe).
pub fn split_message(text: &str, limit: usize) -> Vec<String> {
    if text.len() <= limit {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in text.split('\n') {
        // +1 for the newline
        if !current.is_empty() && current.len() + line.len() + 1 > limit {
            chunks.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push('\n');
        }
        // If a single line exceeds limit, hard-split on char boundaries
        if line.len() > limit {
            for ch in line.chars() {
                if current.len() + ch.len_utf8() > limit {
                    chunks.push(current);
                    current = String::new();
                }
                current.push(ch);
            }
        } else {
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

/// Truncate a string to at most `limit` bytes on a char boundary.
pub fn truncate_utf8(s: &str, limit: usize) -> &str {
    if s.len() <= limit {
        return s;
    }
    let mut end = limit;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}