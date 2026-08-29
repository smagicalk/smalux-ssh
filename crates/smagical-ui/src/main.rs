fn main() -> anyhow::Result<()> {
    let _tracing_guard = smagical_debug::init_tracing("smalux", None)?;
    smagical_ui::run()
}
