use std::io::{self, Write};

pub fn interactive_config(
    search_root: &mut String,
    default_command: &mut String,
    include_root: &mut bool,
) -> Result<(), String> {
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
    if let Some(value) = prompt_include_root(*include_root)? {
        *include_root = value;
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

fn prompt_include_root(current: bool) -> Result<Option<bool>, String> {
    let current_label = if current { "y" } else { "n" };
    loop {
        println!("Include root [{current_label}]:");
        print!("> ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed flushing stdout: {e}"))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed reading Include root: {e}"))?;

        match normalize_include_root_input(input.trim()) {
            Ok(value) => return Ok(value),
            Err(()) => println!("Please enter y, n, or press Enter to keep current."),
        }
    }
}

fn normalize_include_root_input(raw: &str) -> Result<Option<bool>, ()> {
    if raw.is_empty() {
        return Ok(None);
    }

    match raw {
        "y" | "Y" => Ok(Some(true)),
        "n" | "N" => Ok(Some(false)),
        _ => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_if_corrupt, normalize_include_root_input, normalize_input};

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

    #[test]
    fn normalize_include_root_input_keeps_current_on_empty() {
        assert_eq!(normalize_include_root_input(""), Ok(None));
    }

    #[test]
    fn normalize_include_root_input_accepts_yes() {
        assert_eq!(normalize_include_root_input("y"), Ok(Some(true)));
        assert_eq!(normalize_include_root_input("Y"), Ok(Some(true)));
    }

    #[test]
    fn normalize_include_root_input_accepts_no() {
        assert_eq!(normalize_include_root_input("n"), Ok(Some(false)));
        assert_eq!(normalize_include_root_input("N"), Ok(Some(false)));
    }

    #[test]
    fn normalize_include_root_input_rejects_invalid() {
        assert_eq!(normalize_include_root_input("yes"), Err(()));
    }
}
