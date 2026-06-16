use smagical_core::CoreState;

use crate::projection::sync_home;
use crate::state::DesktopAppState;
use crate::view_model::HomeViewModel;

slint::include_modules!();

pub fn bootstrap_app() -> anyhow::Result<()> {
    let mut core = CoreState::new();
    core.seed_example_host();
    let state = DesktopAppState::new(core);
    let home = HomeViewModel::from_core(&state.core);

    let window = AppWindow::new()?;
    sync_home(&window, &home);
    window.run()?;
    Ok(())
}
