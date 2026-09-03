mod config;
mod detect;
mod engine;
mod filebrowser;
mod header;
mod port_cleanup;
mod ui;
mod wizard;

use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let force_reconfigure = env::args().any(|a| a == "--reconfigure");

    // Launch the interactive folder browser so the user can navigate to
    // (or create) a working directory before anything else runs.
    // Pressing Space confirms the chosen dir; Esc exits the program cleanly.
    let cwd = env::current_dir()?;
    let root = match filebrowser::browse_for_directory(&cwd) {
        Some(path) => path,
        None => {
            println!("No directory selected. Exiting.");
            return Ok(());
        }
    };

    let cfg = if force_reconfigure {
        wizard::run_setup_wizard(&root)?
    } else {
        match config::load_config(&root) {
            Some(c) => c,
            None => wizard::run_setup_wizard(&root)?,
        }
    };

    if cfg.processes.is_empty() {
        println!("No processes configured. Run again with --reconfigure to set some up.");
        return Ok(());
    }

    header::print_header(&cfg.title);

    let ports: Vec<Option<u16>> = cfg.processes.iter().map(|p| p.port).collect();
    port_cleanup::kill_ports(&ports);

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let handles: Vec<Arc<engine::ProcHandle>> =
        engine::run_engine(cfg.processes.clone(), root.clone(), tx);

    ui::run_ui(cfg.title.clone(), &cfg.processes, rx, handles).await?;

    Ok(())
}
