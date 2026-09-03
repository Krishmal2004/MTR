use crate::config::{self, Config, FallbackMatch, ProcessConfig, ReadyWhen};
use crate::detect::{detect_frameworks, Candidate};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

const PALETTE: &[&str] = &["green", "cyan", "magenta", "yellow", "blue", "red", "white"];

fn ask(question: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{} ", question);
    } else {
        print!("{} [{}] ", question, default);
    }
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn ask_yes_no(question: &str, default: bool) -> bool {
    let hint = if default { "Y/n" } else { "y/N" };
    let answer = ask(&format!("{} ({})", question, hint), "").to_lowercase();
    if answer.is_empty() {
        return default;
    }
    answer == "y" || answer == "yes"
}

fn ask_choice(question: &str, items: &[String]) -> Option<usize> {
    if items.is_empty() {
        return None;
    }
    println!("{}", question);
    for (i, item) in items.iter().enumerate() {
        println!("  {}) {}", i + 1, item);
    }
    let answer = ask(&format!("Select [1-{}]:", items.len()), "1");
    match answer.parse::<usize>() {
        Ok(n) if n >= 1 && n <= items.len() => Some(n - 1),
        _ => {
            println!("Invalid selection, defaulting to \"{}\".", items[0]);
            Some(0)
        }
    }
}

struct FlutterEmulator {
    id: String,
    name: String,
}

fn list_flutter_emulators() -> Vec<FlutterEmulator> {
    let output = match Command::new("flutter").arg("emulators").output() {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter(|l| !l.to_lowercase().contains("available emulator"))
        .filter(|l| !(l.to_lowercase().starts_with("id") && l.to_lowercase().contains("name")))
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\u{2022}').map(|p| p.trim()).collect();
            let id = parts.first()?.to_string();
            if id.is_empty() {
                return None;
            }
            let name = parts.get(1).unwrap_or(&id.as_str()).to_string();
            Some(FlutterEmulator { id, name })
        })
        .collect()
}

fn build_flutter_process(name: String, cwd: String) -> ProcessConfig {
    let emulators = list_flutter_emulators();
    let emulator_id = if !emulators.is_empty() {
        let labels: Vec<String> = emulators.iter().map(|e| format!("{} ({})", e.name, e.id)).collect();
        let idx = ask_choice("Available Flutter emulators:", &labels).unwrap_or(0);
        emulators[idx].id.clone()
    } else {
        println!("  No emulators found via `flutter emulators` - you can edit tui.config.json later.");
        ask("  Emulator id to launch:", "")
    };

    let mut vars = HashMap::new();
    vars.insert("emulatorId".to_string(), emulator_id);

    let launch_step = config::Step {
        cmd: "flutter".into(),
        args: vec!["emulators".into(), "--launch".into(), "{emulatorId}".into()],
        ready_when: Some(ReadyWhen::Poll {
            command: "flutter".into(),
            args: vec!["devices".into(), "--machine".into()],
            cwd: None,
            timeout_ms: Some(90000),
            interval_ms: Some(2000),
            match_field: Some("emulatorId".into()),
            match_value: Some("{emulatorId}".into()),
            fallback_match: Some(FallbackMatch {
                field: "emulator".into(),
                value: serde_json::Value::Bool(true),
            }),
            capture_field: Some("id".into()),
            capture_as: Some("deviceId".into()),
        }),
    };
    let run_step = config::Step {
        cmd: "flutter".into(),
        args: vec!["run".into(), "-d".into(), "{deviceId}".into()],
        ready_when: None,
    };

    ProcessConfig {
        name,
        kind: "multistep".into(),
        cmd: None,
        args: vec![],
        cwd: Some(cwd),
        color: None,
        port: None,
        ready_when: None,
        depends_on: vec![],
        vars,
        steps: vec![launch_step, run_step],
    }
}

fn configure_candidate(candidate: &Candidate, index: usize, existing_names: &[String]) -> Option<ProcessConfig> {
    println!("\nDetected: {} project in \"{}\"", candidate.language, candidate.cwd);
    if !candidate.notes.is_empty() {
        println!("  {}", candidate.notes);
    }

    if !ask_yes_no(&format!("Include \"{}\"?", candidate.suggested_name), true) {
        return None;
    }

    let mut name = ask("  Name:", &candidate.suggested_name);
    while existing_names.contains(&name) {
        name = ask(&format!("  \"{}\" is already used. Pick another name:", name), &format!("{}-{}", name, index));
    }

    let cwd = ask("  Working directory (relative to project root):", &candidate.cwd);

    let mut proc = if candidate.kind == "multistep-mobile" {
        build_flutter_process(name.clone(), cwd.clone())
    } else {
        let mut cmd = candidate.cmd.clone().unwrap_or_default();
        let mut args = candidate.args.clone();

        if candidate.available_scripts.len() > 1 {
            let idx = ask_choice("  Which npm script should this run?", &candidate.available_scripts);
            if let Some(idx) = idx {
                args = vec!["run".into(), candidate.available_scripts[idx].clone()];
            }
        }

        let default_line = format!("{} {}", cmd, args.join(" ")).trim().to_string();
        let cmd_line = ask("  Command to run:", &default_line);
        let parts: Vec<String> = cmd_line.split_whitespace().map(|s| s.to_string()).collect();
        cmd = parts.first().cloned().unwrap_or_default();
        args = parts[1.min(parts.len())..].to_vec();

        ProcessConfig {
            name: name.clone(),
            kind: "simple".into(),
            cmd: Some(cmd),
            args,
            cwd: Some(cwd.clone()),
            color: None,
            port: None,
            ready_when: None,
            depends_on: vec![],
            vars: HashMap::new(),
            steps: vec![],
        }
    };

    let port_answer = ask("  Port to free before starting (blank = none):", "");
    if !port_answer.is_empty() {
        proc.port = port_answer.parse().ok();
    }

    let color = ask("  Pane color (blank = auto):", "");
    if !color.is_empty() {
        proc.color = Some(color);
    }

    let depends_line = ask(
        &format!(
            "  Depends on (comma-separated names, currently: {}; blank = none):",
            if existing_names.is_empty() { "none".to_string() } else { existing_names.join(", ") }
        ),
        "",
    );
    if !depends_line.is_empty() {
        proc.depends_on = depends_line.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    }

    if ask_yes_no("  Should other processes be able to wait for this one to be ready?", false) {
        let options = vec![
            "A line of output matches a pattern".to_string(),
            "An HTTP endpoint responds OK".to_string(),
            "Just wait a fixed number of seconds".to_string(),
        ];
        match ask_choice("  How should readiness be detected?", &options) {
            Some(0) => {
                let pattern = ask("  Regex pattern to watch for in the output:", "ready|listening");
                proc.ready_when = Some(ReadyWhen::Regex { pattern, flags: Some("i".into()) });
            }
            Some(1) => {
                let url = ask("  URL to poll:", "http://localhost:3000");
                proc.ready_when = Some(ReadyWhen::Http { url, timeout_ms: Some(60000), interval_ms: Some(1000) });
            }
            Some(2) => {
                let seconds = ask("  Seconds to wait:", "5");
                let ms = seconds.parse::<u64>().unwrap_or(5) * 1000;
                proc.ready_when = Some(ReadyWhen::Delay { ms });
            }
            _ => {}
        }
    }

    Some(proc)
}

fn add_custom_process(existing_names: &[String], index: usize) -> ProcessConfig {
    let mut name = ask("  Name:", &format!("custom-{}", index));
    while existing_names.contains(&name) {
        name = ask(&format!("  \"{}\" is already used. Pick another name:", name), &format!("{}-{}", name, index));
    }
    let cwd = ask("  Working directory (relative to project root):", ".");
    let cmd_line = ask("  Command to run:", "");
    let parts: Vec<String> = cmd_line.split_whitespace().map(|s| s.to_string()).collect();
    let cmd = parts.first().cloned().unwrap_or_else(|| "echo".to_string());
    let args = if parts.len() > 1 { parts[1..].to_vec() } else { vec![] };
    let mut proc = ProcessConfig {
        name,
        kind: "simple".into(),
        cmd: Some(cmd),
        args,
        cwd: Some(cwd),
        color: None,
        port: None,
        ready_when: None,
        depends_on: vec![],
        vars: HashMap::new(),
        steps: vec![],
    };
    let port_answer = ask("  Port to free before starting (blank = none):", "");
    if !port_answer.is_empty() {
        proc.port = port_answer.parse().ok();
    }
    proc
}

pub fn run_setup_wizard(root: &Path) -> anyhow::Result<Config> {
    println!("\n=== TUI Runner setup ===");
    println!("Scanning {} for known project frameworks...\n", root.display());

    let candidates = detect_frameworks(root);
    let mut processes: Vec<ProcessConfig> = Vec::new();

    if candidates.is_empty() {
        println!("No known frameworks were auto-detected in this directory or its immediate subfolders.");
    }

    for (i, candidate) in candidates.iter().enumerate() {
        let existing: Vec<String> = processes.iter().map(|p| p.name.clone()).collect();
        if let Some(mut proc) = configure_candidate(candidate, i, &existing) {
            if proc.color.is_none() {
                proc.color = Some(PALETTE[processes.len() % PALETTE.len()].to_string());
            }
            processes.push(proc);
        }
    }

    let mut custom_index = 1;
    while ask_yes_no("\nAdd a custom process not auto-detected?", false) {
        let existing: Vec<String> = processes.iter().map(|p| p.name.clone()).collect();
        let mut proc = add_custom_process(&existing, custom_index);
        custom_index += 1;
        proc.color = Some(PALETTE[processes.len() % PALETTE.len()].to_string());
        processes.push(proc);
    }

    let default_title = root
        .file_name()
        .map(|s| s.to_string_lossy().to_uppercase())
        .unwrap_or_else(|| "TUI RUNNER".to_string());
    let title = ask("\nProject title (shown as ASCII art header):", &default_title);

    let cfg = Config { title, processes };
    let path = config::save_config(root, &cfg)?;
    println!("\nSaved config to {}", path.display());
    println!("Run again with --reconfigure to redo this wizard, or edit tui.config.json by hand.\n");

    Ok(cfg)
}
