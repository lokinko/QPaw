use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CodexDevStatus {
    pub cli_installed: bool,
    pub cli_version: Option<String>,
    pub auth_file_found: bool,
    pub auth_file_path: Option<String>,
    pub app_server_available: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CodexDevProbe {
    codex_home: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    user_profile: Option<PathBuf>,
    auth_file_exists: bool,
    cli_version_output: Option<String>,
    app_server_help_ok: bool,
}

pub fn codex_dev_status() -> CodexDevStatus {
    let auth_file_path = codex_auth_file_path(
        env_path("CODEX_HOME"),
        env_path("HOME"),
        env_path("USERPROFILE"),
    );
    let cli_version_output = command_stdout("codex", &["--version"]);
    let app_server_help_ok = command_succeeds("codex", &["app-server", "--help"]);

    probe_codex_dev_status(CodexDevProbe {
        codex_home: env_path("CODEX_HOME"),
        home_dir: env_path("HOME"),
        user_profile: env_path("USERPROFILE"),
        auth_file_exists: auth_file_path
            .as_ref()
            .map(|path| path.is_file())
            .unwrap_or(false),
        cli_version_output,
        app_server_help_ok,
    })
}

pub(crate) fn probe_codex_dev_status(probe: CodexDevProbe) -> CodexDevStatus {
    let auth_file_path = codex_auth_file_path(probe.codex_home, probe.home_dir, probe.user_profile);

    CodexDevStatus {
        cli_installed: probe.cli_version_output.is_some(),
        cli_version: probe
            .cli_version_output
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        auth_file_found: probe.auth_file_exists,
        auth_file_path: auth_file_path.map(|path| path.to_string_lossy().to_string()),
        app_server_available: probe.app_server_help_ok,
    }
}

fn codex_auth_file_path(
    codex_home: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    user_profile: Option<PathBuf>,
) -> Option<PathBuf> {
    codex_home
        .or_else(|| home_dir.map(|path| path.join(".codex")))
        .or_else(|| user_profile.map(|path| path.join(".codex")))
        .map(|path| path.join("auth.json"))
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{probe_codex_dev_status, CodexDevProbe};

    #[test]
    fn uses_codex_home_auth_file_without_reading_secret_contents() {
        let root = PathBuf::from(r"C:\Users\dev\.codex-test");
        let status = probe_codex_dev_status(CodexDevProbe {
            codex_home: Some(root.clone()),
            home_dir: Some(PathBuf::from(r"C:\Users\dev")),
            user_profile: None,
            auth_file_exists: true,
            cli_version_output: Some("codex-cli 0.130.0".to_string()),
            app_server_help_ok: true,
        });

        assert!(status.cli_installed);
        assert_eq!(status.cli_version.as_deref(), Some("codex-cli 0.130.0"));
        assert!(status.auth_file_found);
        assert_eq!(
            status.auth_file_path.as_deref(),
            Some(r"C:\Users\dev\.codex-test\auth.json")
        );
        assert!(status.app_server_available);
    }

    #[test]
    fn reports_missing_cli_and_default_auth_path() {
        let status = probe_codex_dev_status(CodexDevProbe {
            codex_home: None,
            home_dir: None,
            user_profile: Some(PathBuf::from(r"C:\Users\dev")),
            auth_file_exists: false,
            cli_version_output: None,
            app_server_help_ok: false,
        });

        assert!(!status.cli_installed);
        assert_eq!(status.cli_version, None);
        assert!(!status.auth_file_found);
        assert_eq!(
            status.auth_file_path.as_deref(),
            Some(r"C:\Users\dev\.codex\auth.json")
        );
        assert!(!status.app_server_available);
    }
}
