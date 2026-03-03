use std::io::{self, Write};

pub fn interactive_config(search_root: &mut String, default_command: &mut String) -> Result<(), String> {
    clear_if_corrupt(search_root);
    clear_if_corrupt(default_command);

    println!("runin config");
    println!("────────────");

    if let Some(value) = prompt_value("Search root", search_root)? {
        *search_root = value;
    }
    if let Some(value) = prompt_value("Default command", default_command)? {
        *default_command = value;
    }

    Ok(())
}

fn prompt_value(label: &str, current: &str) -> Result<Option<String>, String> {
    println!("{label} [{current}]:");
    print!("> ");
    io::stdout()
        .flush()
        .map_err(|e| format!("Failed flushing stdout: {e}"))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed reading {label}: {e}"))?;

    Ok(normalize_input(input.trim()))
}

fn clear_if_corrupt(value: &mut String) {
    if value.chars().any(|ch| ch.is_control()) {
        value.clear();
    }
}

fn normalize_input(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }

    let mut cleaned = raw.to_string();
    clear_if_corrupt(&mut cleaned);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_if_corrupt, normalize_input};

    #[test]
    fn clear_if_corrupt_clears_when_control_chars_present() {
        let mut value = "qwen\x1b[A".to_string();
        clear_if_corrupt(&mut value);
        assert_eq!(value, "");
    }

    #[test]
    fn clear_if_corrupt_keeps_clean_value() {
        let mut value = "qwen --fast".to_string();
        clear_if_corrupt(&mut value);
        assert_eq!(value, "qwen --fast");
    }

    #[test]
    fn normalize_input_returns_none_for_empty() {
        assert_eq!(normalize_input(""), None);
    }

    #[test]
    fn normalize_input_returns_none_for_corrupt_value() {
        assert_eq!(normalize_input("\x1b[A"), None);
    }

    #[test]
    fn normalize_input_returns_clean_value() {
        assert_eq!(normalize_input("nvim ."), Some("nvim .".to_string()));
    }
}
