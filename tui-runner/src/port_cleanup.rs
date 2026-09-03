use std::process::Command;

fn kill_port_unix(port: u16) {
    let output = match Command::new("lsof").arg(format!("-ti:{}", port)).output() {
        Ok(o) => o,
        Err(_) => return,
    };
    let text = String::from_utf8_lossy(&output.stdout);
    for pid in text.lines().filter(|l| !l.trim().is_empty()) {
        if Command::new("kill").arg("-9").arg(pid.trim()).status().is_ok() {
            println!("Freed port {} (killed stale PID {})", port, pid.trim());
        }
    }
}

fn kill_port_windows(port: u16) {
    let output = match Command::new("cmd").args(["/C", &format!("netstat -ano | findstr :{}", port)]).output() {
        Ok(o) => o,
        Err(_) => return,
    };
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(pid) = line.split_whitespace().last() {
            if line.contains("LISTENING") {
                if Command::new("taskkill").args(["/PID", pid, "/F"]).status().is_ok() {
                    println!("Freed port {} (killed stale PID {})", port, pid);
                }
            }
        }
    }
}

pub fn kill_port(port: u16) {
    if cfg!(windows) {
        kill_port_windows(port);
    } else {
        kill_port_unix(port);
    }
}

pub fn kill_ports(ports: &[Option<u16>]) {
    for port in ports.iter().flatten() {
        kill_port(*port);
    }
}
