use std::io::{self, Write};

pub fn interactive_config(
    search_root: &mut String,
    default_command: &mut String,
    include_root: &mut bool,
    include_hidden: &mut bool,
) -> Result<(), String> {
    clear_if_corrupt(search_root);
    clear_if_corrupt(default_command);

    println!("{}", style("runin setup", Style::Title));
    println!();

    if let Some(value) = prompt_value("Search root", search_root)? {
        *search_root = value;
    }

    if let Some(value) = prompt_value("Default command", default_command)? {
        *default_command = value;
    }

    if let Some(value) = prompt_include_root(*include_root)? {
        *include_root = value;
    }
    if let Some(value) = prompt_include_hidden(*include_hidden)? {
        *include_hidden = value;
    }

    Ok(())
}

fn prompt_value(label: &str, current: &str) -> Result<Option<String>, String> {
    println!(
        "{} {}:",
        style(label, Style::Label),
        style(&format!("[{current}]"), Style::Muted)
    );
    print!("{}", style("> ", Style::Prompt));
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
    prompt_toggle("Include root", current)
}

fn prompt_include_hidden(current: bool) -> Result<Option<bool>, String> {
    prompt_toggle("Include hidden paths", current)
}

fn prompt_toggle(label: &str, current: bool) -> Result<Option<bool>, String> {
    let current_label = if current { "y" } else { "n" };
    loop {
        println!(
            "{} {} {}:",
            style(label, Style::Label),
            style("(y/n)", Style::Muted),
            style(&format!("[{current_label}]"), Style::Muted)
        );
        print!("{}", style("> ", Style::Prompt));
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed flushing stdout: {e}"))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed reading {label}: {e}"))?;

        match normalize_toggle_input(input.trim()) {
            Ok(value) => return Ok(value),
            Err(()) => println!(
                "{}",
                style(
                    "Please enter y, n, or press Enter to keep current.",
                    Style::Error
                )
            ),
        }
    }
}

fn normalize_toggle_input(raw: &str) -> Result<Option<bool>, ()> {
    if raw.is_empty() {
        return Ok(None);
    }

    match raw {
        "y" | "Y" => Ok(Some(true)),
        "n" | "N" => Ok(Some(false)),
        _ => Err(()),
    }
}

#[derive(Clone, Copy)]
enum Style {
    Title,
    Label,
    Muted,
    Prompt,
    Error,
}

fn style(text: &str, style: Style) -> String {
    let code = match style {
        Style::Title => "1;36",
        Style::Label => "1",
        Style::Muted => "2",
        Style::Prompt => "1;34",
        Style::Error => "1;31",
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

#[cfg(test)]
mod tests {
    use super::{clear_if_corrupt, normalize_input, normalize_toggle_input};

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
    fn normalize_toggle_input_keeps_current_on_empty() {
        assert_eq!(normalize_toggle_input(""), Ok(None));
    }

    #[test]
    fn normalize_toggle_input_accepts_yes() {
        assert_eq!(normalize_toggle_input("y"), Ok(Some(true)));
        assert_eq!(normalize_toggle_input("Y"), Ok(Some(true)));
    }

    #[test]
    fn normalize_toggle_input_accepts_no() {
        assert_eq!(normalize_toggle_input("n"), Ok(Some(false)));
        assert_eq!(normalize_toggle_input("N"), Ok(Some(false)));
    }

    #[test]
    fn normalize_toggle_input_rejects_invalid() {
        assert_eq!(normalize_toggle_input("yes"), Err(()));
    }

    #[test]
    fn normalize_toggle_input_accepts_yes_no() {
        assert_eq!(normalize_toggle_input("y"), Ok(Some(true)));
        assert_eq!(normalize_toggle_input("N"), Ok(Some(false)));
    }
}
