use console::style;

#[allow(dead_code)]
pub fn print_header(title: &str) {
    println!("{}", style(format!("🚀 {title}")).blue().bold());
    println!("{}", "─".repeat(50));
}

#[allow(dead_code)]
pub fn print_success(message: &str) {
    println!("{} {}", style("✅").green(), message);
}

#[allow(dead_code)]
pub fn print_error(message: &str) {
    eprintln!("{} {}", style("❌").red(), message);
}

#[allow(dead_code)]
pub fn print_warning(message: &str) {
    println!("{} {}", style("⚠️").yellow(), message);
}

#[allow(dead_code)]
pub fn print_info(message: &str) {
    println!("{} {}", style("ℹ️").blue(), message);
}

#[allow(dead_code)]
pub fn format_repository_name(name: &str) -> String {
    style(name).cyan().bold().to_string()
}

#[allow(dead_code)]
pub fn format_path(path: &str) -> String {
    style(path).dim().to_string()
}

#[allow(dead_code)]
pub fn format_branch(branch: &str) -> String {
    style(branch).yellow().to_string()
}

#[allow(dead_code)]
pub fn format_status_indicator(clean: bool) -> String {
    if clean {
        style("✓").green().to_string()
    } else {
        style("●").red().to_string()
    }
}

pub fn print_table_header(columns: &[&str]) {
    let header = columns
        .iter()
        .map(|col| style(col).bold().underlined().to_string())
        .collect::<Vec<_>>()
        .join("  ");

    println!("{header}");
}

pub fn print_separator() {
    println!("{}", "─".repeat(50));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_functions() {
        let repo_name = format_repository_name("test-repo");
        assert!(repo_name.contains("test-repo"));

        let path = format_path("/path/to/repo");
        assert!(path.contains("/path/to/repo"));

        let branch = format_branch("main");
        assert!(branch.contains("main"));

        let clean_status = format_status_indicator(true);
        assert!(clean_status.contains("✓"));

        let dirty_status = format_status_indicator(false);
        assert!(dirty_status.contains("●"));
    }
}
