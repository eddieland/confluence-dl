//! Color utilities for terminal output
//!
//! This module provides consistent color handling across the application,
//! respecting user preferences and terminal capabilities.

use owo_colors::OwoColorize;

use crate::cli::ColorOption;

/// Color scheme for the application
///
/// This provides semantic color names that make the code more readable
/// and ensure consistent visual design across the application.
pub struct ColorScheme {
  enabled: bool,
}

impl ColorScheme {
  /// Create a new color scheme based on user preference and terminal
  /// capabilities
  pub fn new(color_option: ColorOption) -> Self {
    let enabled = match color_option {
      ColorOption::Always => true,
      ColorOption::Never => false,
      ColorOption::Auto => {
        // Check if stdout is a TTY
        use std::io::IsTerminal;
        std::io::stdout().is_terminal()
      }
    };

    Self { enabled }
  }

  /// Check if colors are enabled
  #[allow(dead_code)]
  pub fn is_enabled(&self) -> bool {
    self.enabled
  }

  // Semantic color methods for different message types

  /// Style for success messages (green)
  pub fn success<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.green())
    } else {
      text.to_string()
    }
  }

  /// Style for error messages (bright red)
  pub fn error<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.bright_red().bold())
    } else {
      text.to_string()
    }
  }

  /// Style for warning messages (yellow)
  pub fn warning<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.yellow())
    } else {
      text.to_string()
    }
  }

  /// Style for info messages (cyan)
  pub fn info<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.cyan())
    } else {
      text.to_string()
    }
  }

  /// Style for debug messages (bright black/gray)
  #[allow(dead_code)]
  pub fn debug<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.bright_black())
    } else {
      text.to_string()
    }
  }

  /// Style for emphasis/important text (bright white, bold)
  pub fn emphasis<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.bright_white().bold())
    } else {
      text.to_string()
    }
  }

  /// Style for URLs and links (blue, underlined)
  pub fn link<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.blue().underline())
    } else {
      text.to_string()
    }
  }

  /// Style for file paths (magenta)
  pub fn path<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.magenta())
    } else {
      text.to_string()
    }
  }

  /// Style for numbers and metrics (bright blue)
  pub fn number<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.bright_blue())
    } else {
      text.to_string()
    }
  }

  /// Style for commands and code (bright green, monospace feel via styling)
  pub fn code<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.bright_green())
    } else {
      text.to_string()
    }
  }

  /// Style for dimmed/secondary text (gray)
  pub fn dimmed<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.dimmed())
    } else {
      text.to_string()
    }
  }

  /// Style for progress indicators (bright cyan)
  pub fn progress<T: std::fmt::Display>(&self, text: T) -> String {
    if self.enabled {
      format!("{}", text.bright_cyan())
    } else {
      text.to_string()
    }
  }
}

// Best Practices for Color Usage:
//
// 1. **Semantic Naming**: Use method names that describe the purpose (success,
//    error) rather than the color itself (green, red). This makes the code more
//    maintainable.
//
// 2. **Consistency**: Always use the same color for the same type of message
//    across the entire application. For example, errors are always bright red.
//
// 3. **Accessibility**: Consider colorblind users:
//    - Never rely solely on color to convey information
//    - Use icons or prefixes (✓, ✗, ⚠) alongside colors
//    - Ensure sufficient contrast between text and background
//
// 4. **Respect User Preferences**: Always honor the --color flag:
//    - auto: Detect terminal capabilities
//    - always: Force colors (for piping to files that will be viewed later)
//    - never: No colors (for CI/CD, logs, accessibility)
//
// 5. **Progressive Enhancement**: The application should work perfectly without
//    colors. Colors are a visual enhancement, not a requirement.
//
// 6. **Terminal Compatibility**: Use standard ANSI colors that work across
//    terminals. owo-colors handles this well with its supports-colors
//    detection.
//
// 7. **Emotional Design**: Colors evoke emotions and set expectations:
//    - Green = Success, positive, "go ahead"
//    - Red = Error, danger, "stop"
//    - Yellow = Warning, caution, "be careful"
//    - Blue = Information, neutral, "here's something you should know"
//    - Cyan = Progress, ongoing activity
//    - Magenta = Files/paths, special entities
//
// 8. **Visual Hierarchy**: Use colors to guide the user's attention:
//    - Bold + bright colors for critical information
//    - Dimmed/gray for less important context
//    - Underline for clickable/actionable items
//
// 9. **Cultural Considerations**: Be aware that color meanings can vary by
//    culture, but the tech world has generally standardized on the conventions
//    used here.
//
// 10. **Testing**: Always test your CLI in different terminals and with
//     different color schemes (light/dark backgrounds) to ensure readability.

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_color_scheme_disabled() {
    let scheme = ColorScheme::new(ColorOption::Never);
    assert!(!scheme.is_enabled());
    assert_eq!(scheme.success("test"), "test");
    assert_eq!(scheme.error("test"), "test");
  }

  #[test]
  fn test_color_scheme_enabled() {
    let scheme = ColorScheme::new(ColorOption::Always);
    assert!(scheme.is_enabled());
    // With colors enabled, the output should contain ANSI codes
    // (we can't easily test the exact codes, but we can verify they're different)
    assert_ne!(scheme.success("test"), "test");
    assert_ne!(scheme.error("test"), "test");
  }

  #[test]
  fn test_all_semantic_colors() {
    let scheme = ColorScheme::new(ColorOption::Always);
    let text = "test";

    // Just verify all methods produce some output
    assert!(!scheme.success(text).is_empty());
    assert!(!scheme.error(text).is_empty());
    assert!(!scheme.warning(text).is_empty());
    assert!(!scheme.info(text).is_empty());
    assert!(!scheme.debug(text).is_empty());
    assert!(!scheme.emphasis(text).is_empty());
    assert!(!scheme.link(text).is_empty());
    assert!(!scheme.path(text).is_empty());
    assert!(!scheme.number(text).is_empty());
    assert!(!scheme.code(text).is_empty());
    assert!(!scheme.dimmed(text).is_empty());
    assert!(!scheme.progress(text).is_empty());
  }
}
