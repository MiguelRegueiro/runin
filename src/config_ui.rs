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

    let value = input.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        let mut cleaned = value.to_string();
        clear_if_corrupt(&mut cleaned);
        Ok(Some(cleaned))
    }
}

fn clear_if_corrupt(value: &mut String) {
    if value.chars().any(|ch| ch.is_control()) {
        value.clear();
    }
}
