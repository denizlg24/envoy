use console::{StyledObject, style};
use indicatif::{ProgressBar, ProgressStyle};

pub const ICON_SUCCESS: &str = "✓";
pub const ICON_ERROR: &str = "✗";
pub const ICON_INFO: &str = "·";
pub const ICON_ARROW: &str = "→";
pub const ICON_WARN: &str = "!";
pub const ICON_BULLET: &str = "•";

pub fn success_prefix() -> StyledObject<&'static str> {
    style(ICON_SUCCESS).green().bold()
}

pub fn error_prefix() -> StyledObject<&'static str> {
    style(ICON_ERROR).red().bold()
}

pub fn info_prefix() -> StyledObject<&'static str> {
    style(ICON_INFO).cyan()
}

pub fn warn_prefix() -> StyledObject<&'static str> {
    style(ICON_WARN).yellow().bold()
}

pub fn arrow_prefix() -> StyledObject<&'static str> {
    style(ICON_ARROW).cyan().bold()
}

pub fn bullet_prefix() -> StyledObject<&'static str> {
    style(ICON_BULLET).dim()
}

pub fn print_success(message: &str) {
    println!("{} {}", success_prefix(), style(message).green());
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", error_prefix(), style(message).red());
}

pub fn print_info(message: &str) {
    println!("{} {}", info_prefix(), style(message).dim());
}

pub fn print_warn(message: &str) {
    println!("{} {}", warn_prefix(), style(message).yellow());
}

pub fn print_header(title: &str) {
    println!("\n{} {}", arrow_prefix(), style(title).bold());
}

pub fn print_kv(key: &str, value: &str) {
    println!("  {} {}", style(key).dim(), value);
}

pub fn print_kv_highlight(key: &str, value: &str) {
    println!("  {} {}", style(key).dim(), style(value).cyan());
}

pub fn print_kv_warn(key: &str, value: &str) {
    println!("  {} {}", style(key).dim(), style(value).yellow());
}

pub fn print_item(text: &str) {
    println!("  {} {}", bullet_prefix(), text);
}

pub fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message(message.to_string());
    spinner
}

pub fn create_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("━╸─"),
    );
    pb
}
