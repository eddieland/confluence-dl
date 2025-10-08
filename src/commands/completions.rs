use std::io;

use clap::CommandFactory;
use clap_complete::{Shell as CompletionShell, generate};

use crate::cli::{Cli, Shell};

/// Handle completions command
pub(crate) fn handle_completions_command(shell: Shell) {
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
