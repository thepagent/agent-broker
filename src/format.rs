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
            for chunk in line.as_bytes().chunks(limit) {
                if !current.is_empty() {
                    chunks.push(current);
                }
                current = String::from_utf8_lossy(chunk).to_string();
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
