/// Split text into chunks at line boundaries, each <= limit **characters**.
pub fn split_message(text: &str, limit: usize) -> Vec<String> {
    if text.chars().count() <= limit {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;

    for line in text.split('\n') {
        let line_chars = line.chars().count();
        // +1 for the newline
        if !current.is_empty() && current_chars + line_chars + 1 > limit {
            chunks.push(current);
            current = String::new();
            current_chars = 0;
        }
        if !current.is_empty() {
            current.push('\n');
            current_chars += 1;
        }
        // If a single line exceeds limit, hard-split it at char boundaries
        if line_chars > limit {
            let mut chars = line.chars();
            loop {
                let chunk: String = chars.by_ref().take(limit - current_chars).collect();
                if chunk.is_empty() {
                    break;
                }
                current.push_str(&chunk);
                current_chars += chunk.chars().count();
                if current_chars >= limit {
                    chunks.push(current);
                    current = String::new();
                    current_chars = 0;
                }
            }
        } else {
            current.push_str(line);
            current_chars += line_chars;
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}
