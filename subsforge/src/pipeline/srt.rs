use crate::error::{SubsForgeError, Result};

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub sequence: u32,
    pub timestamp: String, // "00:00:01,000 --> 00:00:04,000"
    pub text: String,
}

/// Parse SRT content into structured entries.
pub fn parse(content: &str) -> Result<Vec<SubtitleEntry>> {
    let mut entries = Vec::new();
    let mut lines = content.lines().peekable();

    while lines.peek().is_some() {
        // Skip blank lines
        while lines.peek().is_some_and(|l| l.trim().is_empty()) {
            lines.next();
        }

        // Sequence number
        let seq_line = match lines.next() {
            Some(l) if !l.trim().is_empty() => l.trim(),
            _ => break,
        };
        let sequence: u32 = seq_line.parse().map_err(|_| {
            SubsForgeError::SrtParse(format!("invalid sequence number: {seq_line}"))
        })?;

        // Timestamp line
        let timestamp = lines.next()
            .ok_or_else(|| SubsForgeError::SrtParse("unexpected end: expected timestamp".into()))?
            .trim()
            .to_string();

        if !timestamp.contains("-->") {
            return Err(SubsForgeError::SrtParse(format!("invalid timestamp: {timestamp}")));
        }

        // Text lines (until blank line or EOF)
        let mut text_lines = Vec::new();
        while lines.peek().is_some_and(|l| !l.trim().is_empty()) {
            text_lines.push(lines.next().unwrap().to_string());
        }

        entries.push(SubtitleEntry {
            sequence,
            timestamp,
            text: text_lines.join("\n"),
        });
    }

    if entries.is_empty() {
        return Err(SubsForgeError::SrtParse("no subtitle entries found".into()));
    }

    Ok(entries)
}

/// Serialize subtitle entries back to SRT format.
pub fn serialize(entries: &[SubtitleEntry]) -> String {
    entries.iter()
        .map(|e| format!("{}\n{}\n{}", e.sequence, e.timestamp, e.text))
        .collect::<Vec<_>>()
        .join("\n\n")
        + "\n"
}

/// Extract just the text lines from entries (for translation).
pub fn extract_texts(entries: &[SubtitleEntry]) -> Vec<String> {
    entries.iter().map(|e| e.text.clone()).collect()
}

/// Replace text in entries with translated text, preserving timestamps.
pub fn replace_texts(entries: &[SubtitleEntry], translations: &[String]) -> Result<Vec<SubtitleEntry>> {
    if entries.len() != translations.len() {
        return Err(SubsForgeError::SrtParse(format!(
            "entry count mismatch: {} entries, {} translations",
            entries.len(),
            translations.len()
        )));
    }

    Ok(entries.iter().zip(translations.iter()).map(|(entry, text)| {
        SubtitleEntry {
            sequence: entry.sequence,
            timestamp: entry.timestamp.clone(),
            text: text.clone(),
        }
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:04,000\nHello world\n\n2\n00:00:05,000 --> 00:00:08,000\nHow are you?\n";

    #[test]
    fn test_parse() {
        let entries = parse(SAMPLE_SRT).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[0].text, "Hello world");
        assert_eq!(entries[1].text, "How are you?");
    }

    #[test]
    fn test_roundtrip() {
        let entries = parse(SAMPLE_SRT).unwrap();
        let output = serialize(&entries);
        let reparsed = parse(&output).unwrap();
        assert_eq!(reparsed.len(), entries.len());
        assert_eq!(reparsed[0].text, entries[0].text);
    }

    #[test]
    fn test_replace_texts() {
        let entries = parse(SAMPLE_SRT).unwrap();
        let translated = replace_texts(&entries, &["Bonjour le monde".into(), "Comment allez-vous ?".into()]).unwrap();
        assert_eq!(translated[0].text, "Bonjour le monde");
        assert_eq!(translated[0].timestamp, entries[0].timestamp);
    }
}
