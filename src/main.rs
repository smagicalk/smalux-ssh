#[cfg(feature = "desktop")]
fn main() -> anyhow::Result<()> {
    smagicalssh::app::run()
}

#[cfg(not(feature = "desktop"))]
fn main() -> anyhow::Result<()> {
    let core = smagicalssh::core::CoreState::try_default_runtime()?;
    println!(
        "headless core ready: hosts={}, groups={}, sessions={}, terminal_tabs={}, has_queued_commands={}",
        core.storage.host_count(),
        core.storage.group_count(),
        core.sessions.tab_count(),
        core.terminal.tab_count(),
        !core.backend_commands.is_empty()
    );
    Ok(())
}
