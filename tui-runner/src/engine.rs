use crate::config::{FallbackMatch, ProcessConfig, ReadyWhen};
use regex::RegexBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, watch, Mutex};
use tokio::time::{sleep, Duration, Instant};

#[derive(Debug, Clone)]
pub enum LogKind {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone)]
pub struct LogEvent {
    pub pane: usize,
    pub kind: LogKind,
    pub line: String,
}

pub struct ProcHandle {
    pub kill_tx: watch::Sender<bool>,
    pub pid: Arc<Mutex<Option<u32>>>,
    pub child: Arc<Mutex<Option<Child>>>,
}

impl ProcHandle {
    pub async fn kill(&self) {
        let _ = self.kill_tx.send(true);
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            let _ = child.start_kill();
        }
    }
}

fn subst(input: &str, vars: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut chars = input.char_indices().peekable();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = input[i..].find('}') {
                let key = &input[i + 1..i + end];
                if let Some(val) = vars.get(key) {
                    out.push_str(val);
                    i += end + 1;
                    continue;
                }
            }
        }
        let ch = input[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    let _ = chars.peek();
    out
}

fn subst_args(args: &[String], vars: &HashMap<String, String>) -> Vec<String> {
    args.iter().map(|a| subst(a, vars)).collect()
}

fn resolve_cwd(root: &Path, cwd: &Option<String>) -> PathBuf {
    match cwd {
        Some(c) => root.join(c),
        None => root.to_path_buf(),
    }
}

fn spawn_cmd(cmd: &str, args: &[String], cwd: &Path) -> anyhow::Result<Child> {
    let child = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    Ok(child)
}

fn pipe_output(child: &mut Child, pane: usize, tx: mpsc::UnboundedSender<LogEvent>) {
    if let Some(stdout) = child.stdout.take() {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogEvent { pane, kind: LogKind::Stdout, line });
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogEvent { pane, kind: LogKind::Stderr, line });
            }
        });
    }
}

async fn wait_for_ready(
    ready_when: &Option<ReadyWhen>,
    vars: &HashMap<String, String>,
    cwd: &Path,
    pane: usize,
    tx: &mpsc::UnboundedSender<LogEvent>,
    mut line_rx: Option<mpsc::UnboundedReceiver<String>>,
) -> HashMap<String, String> {
    let mut captured = HashMap::new();
    let Some(rw) = ready_when else { return captured };

    match rw {
        ReadyWhen::Delay { ms } => {
            sleep(Duration::from_millis(*ms)).await;
        }
        ReadyWhen::Regex { pattern, flags } => {
            let case_insensitive = flags.as_deref().unwrap_or("").contains('i');
            let re = match RegexBuilder::new(pattern).case_insensitive(case_insensitive).build() {
                Ok(r) => r,
                Err(_) => return captured,
            };
            if let Some(rx) = line_rx.as_mut() {
                while let Some(line) = rx.recv().await {
                    if re.is_match(&line) {
                        break;
                    }
                }
            }
        }
        ReadyWhen::Http { url, timeout_ms, interval_ms } => {
            let timeout = Duration::from_millis(timeout_ms.unwrap_or(60000));
            let interval = Duration::from_millis(interval_ms.unwrap_or(1000));
            let deadline = Instant::now() + timeout;
            let target = subst(url, vars);
            let addr = target
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .split('/')
                .next()
                .unwrap_or("")
                .to_string();
            let addr = if addr.contains(':') { addr } else { format!("{}:80", addr) };
            loop {
                if tokio::net::TcpStream::connect(&addr).await.is_ok() {
                    break;
                }
                if Instant::now() >= deadline {
                    let _ = tx.send(LogEvent {
                        pane,
                        kind: LogKind::System,
                        line: format!("Timed out waiting on {}", target),
                    });
                    break;
                }
                sleep(interval).await;
            }
        }
        ReadyWhen::Poll {
            command,
            args,
            cwd: poll_cwd,
            timeout_ms,
            interval_ms,
            match_field,
            match_value,
            fallback_match,
            capture_field,
            capture_as,
        } => {
            let timeout = Duration::from_millis(timeout_ms.unwrap_or(90000));
            let interval = Duration::from_millis(interval_ms.unwrap_or(2000));
            let deadline = Instant::now() + timeout;
            let resolved_cwd = match poll_cwd {
                Some(c) => cwd.join(c),
                None => cwd.to_path_buf(),
            };
            let resolved_command = subst(command, vars);
            let resolved_args = subst_args(args, vars);
            let resolved_match_value = match_value.as_ref().map(|v| subst(v, vars));

            loop {
                let output = tokio::process::Command::new(&resolved_command)
                    .args(&resolved_args)
                    .current_dir(&resolved_cwd)
                    .output()
                    .await;

                if let Ok(out) = output {
                    if let Ok(items) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout) {
                        let found = match_field.as_ref().and_then(|field| {
                            resolved_match_value.as_ref().and_then(|want| {
                                items.iter().find(|it| {
                                    it.get(field).map(|v| value_to_string(v)) == Some(want.clone())
                                })
                            })
                        });
                        let found = found.or_else(|| {
                            fallback_match.as_ref().and_then(|fb: &FallbackMatch| {
                                items.iter().find(|it| it.get(&fb.field) == Some(&fb.value))
                            })
                        });
                        if let Some(item) = found {
                            if let (Some(cf), Some(ca)) = (capture_field, capture_as) {
                                if let Some(v) = item.get(cf) {
                                    captured.insert(ca.clone(), value_to_string(v));
                                }
                            }
                            break;
                        }
                    }
                }

                if Instant::now() >= deadline {
                    let _ = tx.send(LogEvent {
                        pane,
                        kind: LogKind::System,
                        line: format!("Timed out waiting for {:?}", resolved_match_value),
                    });
                    break;
                }
                let _ = tx.send(LogEvent {
                    pane,
                    kind: LogKind::System,
                    line: format!("Still waiting for \"{}\"...", resolved_match_value.clone().unwrap_or_default()),
                });
                sleep(interval).await;
            }
        }
    }
    captured
}

fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

async fn run_simple(
    proc: ProcessConfig,
    pane: usize,
    root: PathBuf,
    tx: mpsc::UnboundedSender<LogEvent>,
    handle: Arc<ProcHandle>,
    ready_tx: watch::Sender<bool>,
) {
    let cwd = resolve_cwd(&root, &proc.cwd);
    let cmd = subst(proc.cmd.as_deref().unwrap_or(""), &proc.vars);
    let args = subst_args(&proc.args, &proc.vars);
    let _ = tx.send(LogEvent {
        pane,
        kind: LogKind::System,
        line: format!("$ {} {}  (cwd: {})", cmd, args.join(" "), cwd.display()),
    });

    let mut child = match spawn_cmd(&cmd, &args, &cwd) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(LogEvent { pane, kind: LogKind::System, line: format!("Failed to start: {}", e) });
            return;
        }
    };

    *handle.pid.lock().await = child.id();

    let regex_line_rx = if matches!(proc.ready_when, Some(ReadyWhen::Regex { .. })) {
        let (ltx, lrx) = mpsc::unbounded_channel();
        tap_output_for_regex(&mut child, pane, tx.clone(), ltx);
        Some(lrx)
    } else {
        pipe_output(&mut child, pane, tx.clone());
        None
    };

    *handle.child.lock().await = Some(child);

    let ready_captured = wait_for_ready(&proc.ready_when, &proc.vars, &cwd, pane, &tx, regex_line_rx).await;
    let _ = ready_captured;
    let _ = ready_tx.send(true);

    let mut kill_rx = handle.kill_tx.subscribe();
    let mut guard = handle.child.lock().await;
    if let Some(child) = guard.as_mut() {
        tokio::select! {
            status = child.wait() => {
                if let Ok(status) = status {
                    let _ = tx.send(LogEvent { pane, kind: LogKind::System, line: format!("--- {} exited (code {:?}) ---", proc.name, status.code()) });
                }
            }
            _ = kill_rx.changed() => {}
        }
    }
}

async fn run_multistep(
    proc: ProcessConfig,
    pane: usize,
    root: PathBuf,
    tx: mpsc::UnboundedSender<LogEvent>,
    handle: Arc<ProcHandle>,
    ready_tx: watch::Sender<bool>,
) {
    let cwd = resolve_cwd(&root, &proc.cwd);
    let mut vars = proc.vars.clone();
    let step_count = proc.steps.len();

    for (i, step) in proc.steps.into_iter().enumerate() {
        if *handle.kill_tx.subscribe().borrow() {
            break;
        }
        let is_last = i == step_count - 1;
        let cmd = subst(&step.cmd, &vars);
        let args = subst_args(&step.args, &vars);
        let _ = tx.send(LogEvent {
            pane,
            kind: LogKind::System,
            line: format!("$ {} {}  (cwd: {})", cmd, args.join(" "), cwd.display()),
        });

        let mut child = match spawn_cmd(&cmd, &args, &cwd) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(LogEvent { pane, kind: LogKind::System, line: format!("Failed to start: {}", e) });
                return;
            }
        };
        *handle.pid.lock().await = child.id();

        let regex_line_rx = if matches!(step.ready_when, Some(ReadyWhen::Regex { .. })) {
            let (ltx, lrx) = mpsc::unbounded_channel();
            tap_output_for_regex(&mut child, pane, tx.clone(), ltx);
            Some(lrx)
        } else {
            pipe_output(&mut child, pane, tx.clone());
            None
        };

        if is_last {
            *handle.child.lock().await = Some(child);
            let captured = wait_for_ready(&step.ready_when, &vars, &cwd, pane, &tx, regex_line_rx).await;
            vars.extend(captured);
            let _ = ready_tx.send(true);

            let mut kill_rx = handle.kill_tx.subscribe();
            let mut guard = handle.child.lock().await;
            if let Some(child) = guard.as_mut() {
                tokio::select! {
                    status = child.wait() => {
                        if let Ok(status) = status {
                            let _ = tx.send(LogEvent { pane, kind: LogKind::System, line: format!("--- {} exited (code {:?}) ---", proc.name, status.code()) });
                        }
                    }
                    _ = kill_rx.changed() => {}
                }
            }
        } else {
            let captured = wait_for_ready(&step.ready_when, &vars, &cwd, pane, &tx, regex_line_rx).await;
            vars.extend(captured);
        }
    }
    if step_count == 0 {
        let _ = ready_tx.send(true);
    }
}

fn tap_output_for_regex(
    child: &mut Child,
    pane: usize,
    tx: mpsc::UnboundedSender<LogEvent>,
    line_tx: mpsc::UnboundedSender<String>,
) {
    if let Some(stdout) = child.stdout.take() {
        let tx = tx.clone();
        let line_tx = line_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogEvent { pane, kind: LogKind::Stdout, line: line.clone() });
                let _ = line_tx.send(line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let tx = tx.clone();
        let line_tx = line_tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogEvent { pane, kind: LogKind::Stderr, line: line.clone() });
                let _ = line_tx.send(line);
            }
        });
    }
}

pub fn run_engine(
    processes: Vec<ProcessConfig>,
    root: PathBuf,
    tx: mpsc::UnboundedSender<LogEvent>,
) -> Vec<Arc<ProcHandle>> {
    let mut ready_txs: HashMap<String, watch::Sender<bool>> = HashMap::new();
    let mut ready_rxs: HashMap<String, watch::Receiver<bool>> = HashMap::new();
    for proc in &processes {
        let (rtx, rrx) = watch::channel(false);
        ready_txs.insert(proc.name.clone(), rtx);
        ready_rxs.insert(proc.name.clone(), rrx);
    }

    let mut handles = Vec::with_capacity(processes.len());

    for (pane, proc) in processes.into_iter().enumerate() {
        let (kill_tx, _) = watch::channel(false);
        let handle = Arc::new(ProcHandle {
            kill_tx,
            pid: Arc::new(Mutex::new(None)),
            child: Arc::new(Mutex::new(None)),
        });
        handles.push(handle.clone());

        let depends_on = proc.depends_on.clone();
        let name = proc.name.clone();
        let ready_tx = ready_txs.get(&name).unwrap().clone();
        let dep_rxs: Vec<watch::Receiver<bool>> =
            depends_on.iter().filter_map(|d| ready_rxs.get(d).cloned()).collect();
        let root = root.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            for mut rx in dep_rxs {
                let _ = tx.send(LogEvent { pane, kind: LogKind::System, line: format!("Waiting on: {}...", depends_on.join(", ")) });
                let _ = rx.wait_for(|ready| *ready).await;
            }
            if *handle.kill_tx.subscribe().borrow() {
                return;
            }
            if proc.kind == "multistep" {
                run_multistep(proc, pane, root, tx.clone(), handle.clone(), ready_tx).await;
            } else {
                run_simple(proc, pane, root, tx.clone(), handle.clone(), ready_tx).await;
            }
        });
    }

    handles
}
