mod config;
mod detect;
mod engine;
mod filebrowser;
mod header;
mod home;
mod langscan;
mod port_cleanup;
mod project_creator;
mod ui;
mod wizard;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

fn downloads_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let home = env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let home = env::var_os("HOME");

    home.map(PathBuf::from)
        .map(|p| p.join("Downloads"))
        .filter(|p| p.is_dir())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let force_reconfigure = env::args().any(|a| a == "--reconfigure");

    loop {
        let choice = match home::show_home() {
            Some(c) => c,
            None => {
                println!("Goodbye!");
                return Ok(());
            }
        };

        match choice {
            home::HomeChoice::CreateProject => {
                let detected = langscan::scan_languages();
                match langscan::show_language_report(&detected) {
                    Some(true) => {}
                    _ => continue,
                }

                let framework_idx = match project_creator::select_framework() {
                    Some(idx) => idx,
                    None => continue,
                };

                let start_dir = downloads_dir().unwrap_or(env::current_dir()?);
                let project_path = match filebrowser::browse_for_directory(&start_dir) {
                    Some(path) => path,
                    None => continue,
                };

                project_creator::scaffold_project(framework_idx, &project_path)?;

                let cfg = match config::load_config(&project_path) {
                    Some(c) => c,
                    None => wizard::run_setup_wizard(&project_path)?,
                };

                if !cfg.processes.is_empty() {
                    header::print_header(&cfg.title);
                    let ports: Vec<Option<u16>> =
                        cfg.processes.iter().map(|p| p.port).collect();
                    port_cleanup::kill_ports(&ports);
                    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                    let handles: Vec<Arc<engine::ProcHandle>> =
                        engine::run_engine(cfg.processes.clone(), project_path.clone(), tx);
                    ui::run_ui(cfg.title.clone(), &cfg.processes, rx, handles).await?;
                }
            }

            home::HomeChoice::BrowsePC => {
                let cwd = env::current_dir()?;
                let root = match filebrowser::browse_for_directory(&cwd) {
                    Some(path) => path,
                    None => continue,
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
                    println!(
                        "No processes configured. Run again with --reconfigure to set some up."
                    );
                    continue;
                }

                header::print_header(&cfg.title);
                let ports: Vec<Option<u16>> =
                    cfg.processes.iter().map(|p| p.port).collect();
                port_cleanup::kill_ports(&ports);
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                let handles: Vec<Arc<engine::ProcHandle>> =
                    engine::run_engine(cfg.processes.clone(), root.clone(), tx);
                ui::run_ui(cfg.title.clone(), &cfg.processes, rx, handles).await?;
            }
        }
    }
}