/// Split text into chunks at line boundaries, each <= limit chars.
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
        // If a single line exceeds limit, hard-split it
        if line.len() > limit {
            let mut pos = 0;
            while pos < line.len() {
                let end = line.floor_char_boundary((pos + limit).min(line.len()));
                if !current.is_empty() {
                    chunks.push(current);
                }
                current = line[pos..end].to_string();
                pos = end;
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
