use std::io::Write;

pub fn prompt_input(prompt: &str) -> String {
    print!("{}: ", prompt);
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub fn prompt_optional(prompt: &str) -> Option<String> {
    let input = prompt_input(&format!("{} (optional, press Enter to skip)", prompt));
    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}

pub fn prompt_bool(prompt: &str) -> bool {
    loop {
        let input = prompt_input(&format!("{} (y/n)", prompt));
        match input.to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("Please enter 'y' or 'n'"),
        }
    }
}

pub fn prompt_display_type(value_is_number: bool) -> Option<String> {
    if value_is_number {
        println!("\nDisplay type options for numbers:");
        println!("  1. number - Regular number display");
        println!("  2. boost_number - Number with + prefix");
        println!("  3. boost_percentage - Percentage with + prefix");
        println!("  4. date - Unix timestamp as date");
        println!("  5. (none) - No special display");

        loop {
            let input = prompt_input("Choose display type (1-5)");
            match input.as_str() {
                "1" => return Some("number".to_string()),
                "2" => return Some("boost_number".to_string()),
                "3" => return Some("boost_percentage".to_string()),
                "4" => return Some("date".to_string()),
                "5" | "" => return None,
                _ => println!("Please enter a number between 1-5 or press Enter for none"),
            }
        }
    } else {
        println!("\nDisplay type options for text:");
        println!("  1. (none) - Default text display");
        println!("  2. custom - Enter custom display type");

        loop {
            let input = prompt_input("Choose display type (1-2)");
            match input.as_str() {
                "1" | "" => return None,
                "2" => {
                    let custom = prompt_input("Enter custom display type");
                    return if custom.is_empty() {
                        None
                    } else {
                        Some(custom)
                    };
                }
                _ => println!("Please enter 1, 2, or press Enter for none"),
            }
        }
    }
}
