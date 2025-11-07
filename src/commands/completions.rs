//! Shell completion generation.
//!
//! Provides the implementation behind `confluence-dl completions`, emitting
//! tab-completion scripts for the supported shells.

use std::io;

use clap::{CommandFactory, ValueEnum};
use clap_complete::{Shell as CompletionShell, generate};

use crate::cli::Cli;

/// Supported shells for completion script generation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
  Bash,
  Zsh,
  Fish,
  Powershell,
  Elvish,
}

/// Generate shell completion scripts for the requested shell.
///
/// # Arguments
/// * `shell` - Target shell to emit completions for, as chosen by the user.
pub fn handle_completions_command(shell: Shell) {
  let mut cmd = Cli::command();
  let bin_name = cmd.get_name().to_string();

  let clap_shell = match shell {
    Shell::Bash => CompletionShell::Bash,
    Shell::Zsh => CompletionShell::Zsh,
    Shell::Fish => CompletionShell::Fish,
    Shell::Powershell => CompletionShell::PowerShell,
    Shell::Elvish => CompletionShell::Elvish,
  };

  generate(clap_shell, &mut cmd, bin_name, &mut io::stdout());
}
