//! The command line of `kawarod` (S-046 AC5).

/// What the process was asked to be.
#[derive(Debug, PartialEq)]
pub enum Mode {
    /// The background service.
    Daemon,
    /// `open <files>`: ask a running service to show the hive for them, or
    /// become the service and do it.
    Open(Vec<String>),
    /// `settings`: ask a running service to open the Settings window, or
    /// become the service and open it (S-086 AC1).
    Settings,
}

/// Parses the arguments after the program name.
pub fn parse(args: &[String]) -> Result<Mode, String> {
    match args.split_first() {
        None => Ok(Mode::Daemon),
        Some((cmd, files)) if cmd == "open" => {
            if files.is_empty() {
                Err("kawarod: open needs at least one file".to_string())
            } else {
                Ok(Mode::Open(files.to_vec()))
            }
        }
        Some((cmd, more)) if cmd == "settings" => {
            if more.is_empty() {
                Ok(Mode::Settings)
            } else {
                Err("kawarod: settings takes no arguments".to_string())
            }
        }
        Some((other, _)) => Err(format!("kawarod: unknown argument `{other}`")),
    }
}
