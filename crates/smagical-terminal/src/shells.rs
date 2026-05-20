//! 本地 shell 入口配置。

use super::{LocalShellProfile, TerminalManager};

impl TerminalManager {
    /// 保存或更新一个本地 shell 配置。
    pub fn upsert_local_shell(&mut self, profile: LocalShellProfile) {
        if let Some(existing) = self
            .local_shells
            .iter_mut()
            .find(|existing| existing.name == profile.name)
        {
            *existing = profile;
        } else {
            self.local_shells.push(profile);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_shell_profiles_can_be_upserted() {
        let mut terminal = TerminalManager::default();

        terminal.upsert_local_shell(LocalShellProfile {
            name: "PowerShell".to_owned(),
            program: "powershell.exe".to_owned(),
            args: vec!["-NoLogo".to_owned()],
            working_directory: None,
        });
        terminal.upsert_local_shell(LocalShellProfile {
            name: "PowerShell".to_owned(),
            program: "pwsh.exe".to_owned(),
            args: vec!["-NoLogo".to_owned()],
            working_directory: Some("C:/Users".to_owned()),
        });

        assert_eq!(terminal.local_shell_count(), 1);
        assert_eq!(terminal.local_shells[0].program, "pwsh.exe");
        assert_eq!(
            terminal.local_shells[0].working_directory.as_deref(),
            Some("C:/Users")
        );
    }
}
