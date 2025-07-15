use owo_colors::OwoColorize;
use std::io::{self, IsTerminal};

/// Output formatting modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutputMode {
    /// Full color output with styles
    Colorized,
    /// Compact format with minimal colors
    Concise,
    /// Plain text output without colors
    NoColor,
}

/// Colorized output formatter
pub struct ColorizedFormatter {
    mode: OutputMode,
    is_terminal: bool,
}

impl Default for ColorizedFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorizedFormatter {
    pub fn new() -> Self {
        Self {
            mode: OutputMode::Colorized,
            is_terminal: io::stdout().is_terminal(),
        }
    }

    pub fn with_mode(mode: OutputMode) -> Self {
        Self {
            mode,
            is_terminal: io::stdout().is_terminal(),
        }
    }

    pub fn set_mode(&mut self, mode: OutputMode) {
        self.mode = mode;
    }

    pub fn get_mode(&self) -> OutputMode {
        self.mode
    }

    /// Format a window ID with appropriate coloring
    pub fn format_window_id(&self, id: u32) -> String {
        let id_str = format!("0x{:x}", id);
        
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                id_str.bright_blue().bold().to_string()
            }
            OutputMode::Concise if self.is_terminal => {
                id_str.blue().to_string()
            }
            _ => id_str,
        }
    }

    /// Format a window name with appropriate coloring
    pub fn format_window_name(&self, name: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                name.bright_cyan().to_string()
            }
            OutputMode::Concise if self.is_terminal => {
                name.cyan().to_string()
            }
            _ => name.to_string(),
        }
    }

    /// Format geometry information with appropriate coloring
    pub fn format_geometry(&self, geom_str: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                geom_str.bright_red().to_string()
            }
            OutputMode::Concise if self.is_terminal => {
                geom_str.red().to_string()
            }
            _ => geom_str.to_string(),
        }
    }

    /// Format attributes with appropriate coloring
    pub fn format_attributes(&self, attrs_str: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                attrs_str.bright_green().to_string()
            }
            OutputMode::Concise if self.is_terminal => {
                attrs_str.green().to_string()
            }
            _ => attrs_str.to_string(),
        }
    }

    /// Format a line with diff highlighting
    pub fn format_diff_line(&self, line: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                line.on_bright_white().black().to_string()
            }
            OutputMode::Concise if self.is_terminal => {
                line.on_white().black().to_string()
            }
            _ => format!(">>> {}", line),
        }
    }

    /// Format an error message
    pub fn format_error(&self, message: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}: {}", "Error".bright_red().bold(), message.red())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}: {}", "Error".red(), message)
            }
            _ => format!("Error: {}", message),
        }
    }

    /// Format a warning message
    pub fn format_warning(&self, message: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}: {}", "Warning".bright_yellow().bold(), message.yellow())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}: {}", "Warning".yellow(), message)
            }
            _ => format!("Warning: {}", message),
        }
    }

    /// Format an info message
    pub fn format_info(&self, message: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}: {}", "Info".bright_blue().bold(), message.blue())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}: {}", "Info".blue(), message)
            }
            _ => format!("Info: {}", message),
        }
    }

    /// Format a success message
    pub fn format_success(&self, message: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}: {}", "Success".bright_green().bold(), message.green())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}: {}", "Success".green(), message)
            }
            _ => format!("Success: {}", message),
        }
    }

    /// Format a header/title
    pub fn format_header(&self, title: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("=== {} ===", title.bright_magenta().bold())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("=== {} ===", title.magenta())
            }
            _ => format!("=== {} ===", title),
        }
    }

    /// Format a separator line
    pub fn format_separator(&self) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                "─".repeat(80).bright_black().to_string()
            }
            OutputMode::Concise if self.is_terminal => {
                "─".repeat(40).black().to_string()
            }
            _ => "-".repeat(40),
        }
    }

    /// Format a complete window entry
    pub fn format_window_entry(&self, index: usize, id: u32, name: &str, geom: &str, attrs: &str, is_diff: bool) -> String {
        let id_formatted = self.format_window_id(id);
        let name_formatted = self.format_window_name(name);
        let geom_formatted = self.format_geometry(geom);
        let attrs_formatted = self.format_attributes(attrs);
        
        let line = match self.mode {
            OutputMode::Concise => {
                format!("{}: {}({})", index, id_formatted, name_formatted)
            }
            _ => {
                format!("{}: {}({}) {} {}", index, id_formatted, name_formatted, geom_formatted, attrs_formatted)
            }
        };
        
        if is_diff {
            self.format_diff_line(&line)
        } else {
            line
        }
    }

    /// Format a statistics line
    pub fn format_stats(&self, label: &str, value: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}: {}", label.bright_white().bold(), value.bright_yellow())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}: {}", label.white(), value.yellow())
            }
            _ => format!("{}: {}", label, value),
        }
    }

    /// Format a key-value pair
    pub fn format_key_value(&self, key: &str, value: &str) -> String {
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}: {}", key.bright_white(), value.white())
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}: {}", key.white(), value)
            }
            _ => format!("{}: {}", key, value),
        }
    }

    /// Create a styled table header
    pub fn format_table_header(&self, headers: &[&str]) -> String {
        let header_line = headers.join(" | ");
        
        match self.mode {
            OutputMode::Colorized if self.is_terminal => {
                format!("{}\n{}", 
                    header_line.bright_white().bold().underline(),
                    "─".repeat(header_line.len()).bright_black()
                )
            }
            OutputMode::Concise if self.is_terminal => {
                format!("{}\n{}", 
                    header_line.white().bold(),
                    "-".repeat(header_line.len())
                )
            }
            _ => {
                format!("{}\n{}", header_line, "-".repeat(header_line.len()))
            }
        }
    }

    /// Check if output supports colors
    pub fn supports_color(&self) -> bool {
        self.is_terminal && self.mode != OutputMode::NoColor
    }

    /// Get terminal width for formatting
    pub fn get_terminal_width(&self) -> usize {
        if let Some((width, _)) = term_size::dimensions() {
            width
        } else {
            80 // Default width
        }
    }

    /// Truncate text to fit terminal width
    pub fn truncate_to_width(&self, text: &str, max_width: Option<usize>) -> String {
        let width = max_width.unwrap_or_else(|| self.get_terminal_width());
        if text.len() <= width {
            text.to_string()
        } else {
            format!("{}...", &text[..width.saturating_sub(3)])
        }
    }
}

/// Convenience macros for formatted output
#[macro_export]
macro_rules! format_error {
    ($formatter:expr, $($arg:tt)*) => {
        $formatter.format_error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! format_warning {
    ($formatter:expr, $($arg:tt)*) => {
        $formatter.format_warning(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! format_info {
    ($formatter:expr, $($arg:tt)*) => {
        $formatter.format_info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! format_success {
    ($formatter:expr, $($arg:tt)*) => {
        $formatter.format_success(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_modes() {
        let mut formatter = ColorizedFormatter::new();
        
        // Test colorized mode
        formatter.set_mode(OutputMode::Colorized);
        assert_eq!(formatter.get_mode(), OutputMode::Colorized);
        
        // Test concise mode
        formatter.set_mode(OutputMode::Concise);
        assert_eq!(formatter.get_mode(), OutputMode::Concise);
        
        // Test no color mode
        formatter.set_mode(OutputMode::NoColor);
        assert_eq!(formatter.get_mode(), OutputMode::NoColor);
    }

    #[test]
    fn test_formatting_functions() {
        let formatter = ColorizedFormatter::with_mode(OutputMode::NoColor);
        
        // Test basic formatting functions
        assert_eq!(formatter.format_window_id(0x123), "0x123");
        assert_eq!(formatter.format_window_name("test"), "test");
        assert_eq!(formatter.format_geometry("100x200+50+75"), "100x200+50+75");
        assert_eq!(formatter.format_attributes("OR Viewable"), "OR Viewable");
        
        // Test message formatting
        assert_eq!(formatter.format_error("test error"), "Error: test error");
        assert_eq!(formatter.format_warning("test warning"), "Warning: test warning");
        assert_eq!(formatter.format_info("test info"), "Info: test info");
        assert_eq!(formatter.format_success("test success"), "Success: test success");
    }

    #[test]
    fn test_window_entry_formatting() {
        let formatter = ColorizedFormatter::with_mode(OutputMode::NoColor);
        
        let entry = formatter.format_window_entry(
            0, 
            0x123, 
            "test-window", 
            "100x200+50+75", 
            "OR Viewable", 
            false
        );
        
        assert_eq!(entry, "0: 0x123(test-window) 100x200+50+75 OR Viewable");
    }

    #[test]
    fn test_concise_formatting() {
        let formatter = ColorizedFormatter::with_mode(OutputMode::Concise);
        
        let entry = formatter.format_window_entry(
            0, 
            0x123, 
            "test-window", 
            "100x200+50+75", 
            "OR Viewable", 
            false
        );
        
        // In concise mode, only id and name should be shown
        assert!(entry.contains("0x123"));
        assert!(entry.contains("test-window"));
        assert!(!entry.contains("100x200+50+75"));
    }

    #[test]
    fn test_diff_formatting() {
        let formatter = ColorizedFormatter::with_mode(OutputMode::NoColor);
        
        let normal_entry = formatter.format_window_entry(
            0, 0x123, "test", "100x200+50+75", "Viewable", false
        );
        
        let diff_entry = formatter.format_window_entry(
            0, 0x123, "test", "100x200+50+75", "Viewable", true
        );
        
        assert_ne!(normal_entry, diff_entry);
        assert!(diff_entry.starts_with(">>>"));
    }

    #[test]
    fn test_table_formatting() {
        let formatter = ColorizedFormatter::with_mode(OutputMode::NoColor);
        
        let headers = vec!["ID", "Name", "Geometry"];
        let table_header = formatter.format_table_header(&headers);
        
        assert!(table_header.contains("ID | Name | Geometry"));
        assert!(table_header.contains("---"));
    }

    #[test]
    fn test_text_truncation() {
        let formatter = ColorizedFormatter::with_mode(OutputMode::NoColor);
        
        let long_text = "This is a very long text that should be truncated";
        let truncated = formatter.truncate_to_width(long_text, Some(20));
        
        assert_eq!(truncated.len(), 20);
        assert!(truncated.ends_with("..."));
    }
}
