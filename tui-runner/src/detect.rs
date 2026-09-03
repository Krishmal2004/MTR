use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const IGNORE_DIRS: &[&str] = &[
    "node_modules", ".git", "bin", "obj", "dist", "build", ".idea", ".vscode", "target",
    "__pycache__", ".venv", "venv",
];

#[derive(Debug, Clone)]
pub struct Candidate {
    pub suggested_name: String,
    /// "simple" | "multistep-mobile"
    pub kind: String,
    pub cmd: Option<String>,
    pub args: Vec<String>,
    /// relative to the scan root
    pub cwd: String,
    pub language: String,
    pub notes: String,
    pub available_scripts: Vec<String>,
}

fn entries(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn has(list: &[String], name: &str) -> bool {
    list.iter().any(|e| e == name)
}

fn detect_in_dir(abs_dir: &Path, rel_dir: &str) -> Option<Candidate> {
    let list = entries(abs_dir);
    let dir_label = if rel_dir == "." {
        abs_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string())
    } else {
        Path::new(rel_dir)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| rel_dir.to_string())
    };

    // Node / JS / TS
    if has(&list, "package.json") {
        let pkg_path = abs_dir.join("package.json");
        let pkg: Value = fs::read_to_string(&pkg_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        let name = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| dir_label.clone());
        let scripts: Vec<String> = pkg
            .get("scripts")
            .and_then(|v| v.as_object())
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let preferred = ["dev", "start", "serve"]
            .iter()
            .find(|s| scripts.contains(&s.to_string()))
            .map(|s| s.to_string());
        let args = match &preferred {
            Some(s) => vec!["run".to_string(), s.clone()],
            None => {
                if let Some(first) = scripts.first() {
                    vec!["run".to_string(), first.clone()]
                } else {
                    vec!["start".to_string()]
                }
            }
        };
        let notes = if scripts.is_empty() {
            "package.json has no scripts defined".to_string()
        } else {
            format!("package.json scripts: {}", scripts.join(", "))
        };
        return Some(Candidate {
            suggested_name: name,
            kind: "simple".into(),
            cmd: Some("npm".into()),
            args,
            cwd: rel_dir.to_string(),
            language: "node".into(),
            notes,
            available_scripts: scripts,
        });
    }

    // .NET
    if let Some(dotnet_file) = list.iter().find(|f| f.ends_with(".csproj") || f.ends_with(".sln")) {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("dotnet".into()),
            args: vec!["run".into()],
            cwd: rel_dir.to_string(),
            language: "dotnet".into(),
            notes: format!("found {}", dotnet_file),
            available_scripts: vec![],
        });
    }

    // Flutter / Dart
    if has(&list, "pubspec.yaml") {
        let content = fs::read_to_string(abs_dir.join("pubspec.yaml")).unwrap_or_default();
        let is_flutter = content.contains("sdk: flutter") || content.lines().any(|l| l.trim() == "flutter:");
        return Some(Candidate {
            suggested_name: dir_label,
            kind: if is_flutter { "multistep-mobile".into() } else { "simple".into() },
            cmd: if is_flutter { None } else { Some("dart".into()) },
            args: if is_flutter { vec![] } else { vec!["run".into()] },
            cwd: rel_dir.to_string(),
            language: if is_flutter { "flutter".into() } else { "dart".into() },
            notes: if is_flutter {
                "Flutter project - will prompt for an emulator at setup time".into()
            } else {
                "Dart project".into()
            },
            available_scripts: vec![],
        });
    }

    // Rust
    if has(&list, "Cargo.toml") {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("cargo".into()),
            args: vec!["run".into()],
            cwd: rel_dir.to_string(),
            language: "rust".into(),
            notes: "found Cargo.toml".into(),
            available_scripts: vec![],
        });
    }

    // Go
    if has(&list, "go.mod") {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("go".into()),
            args: vec!["run".into(), ".".into()],
            cwd: rel_dir.to_string(),
            language: "go".into(),
            notes: "found go.mod".into(),
            available_scripts: vec![],
        });
    }

    // Python
    if has(&list, "manage.py") {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("python".into()),
            args: vec!["manage.py".into(), "runserver".into()],
            cwd: rel_dir.to_string(),
            language: "python-django".into(),
            notes: "found manage.py (Django guess - please confirm the command)".into(),
            available_scripts: vec![],
        });
    }
    if has(&list, "pyproject.toml") {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("poetry".into()),
            args: vec!["run".into(), "start".into()],
            cwd: rel_dir.to_string(),
            language: "python".into(),
            notes: "found pyproject.toml (guessed poetry command - please confirm)".into(),
            available_scripts: vec![],
        });
    }
    if has(&list, "requirements.txt") {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("python".into()),
            args: vec!["main.py".into()],
            cwd: rel_dir.to_string(),
            language: "python".into(),
            notes: "found requirements.txt (best-guess entry point - please confirm)".into(),
            available_scripts: vec![],
        });
    }

    // Ruby
    if has(&list, "Gemfile") {
        let is_rails = abs_dir.join("config").join("environment.rb").exists();
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("bundle".into()),
            args: if is_rails {
                vec!["exec".into(), "rails".into(), "server".into()]
            } else {
                vec!["exec".into(), "ruby".into(), "app.rb".into()]
            },
            cwd: rel_dir.to_string(),
            language: "ruby".into(),
            notes: "found Gemfile (please confirm the command)".into(),
            available_scripts: vec![],
        });
    }

    // Java (Gradle / Maven)
    if has(&list, "gradlew") || has(&list, "build.gradle") || has(&list, "build.gradle.kts") {
        let cmd = if cfg!(windows) { "gradlew.bat" } else { "./gradlew" };
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some(cmd.into()),
            args: vec!["bootRun".into()],
            cwd: rel_dir.to_string(),
            language: "java-gradle".into(),
            notes: "found Gradle project (please confirm the task, e.g. bootRun)".into(),
            available_scripts: vec![],
        });
    }
    if has(&list, "pom.xml") {
        return Some(Candidate {
            suggested_name: dir_label,
            kind: "simple".into(),
            cmd: Some("mvn".into()),
            args: vec!["spring-boot:run".into()],
            cwd: rel_dir.to_string(),
            language: "java-maven".into(),
            notes: "found pom.xml (please confirm the goal)".into(),
            available_scripts: vec![],
        });
    }

    None
}

pub fn detect_frameworks(root: &Path) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();

    let mut try_dir = |abs_dir: PathBuf, rel_dir: String| {
        if seen.contains(&abs_dir) {
            return;
        }
        seen.push(abs_dir.clone());
        if let Some(c) = detect_in_dir(&abs_dir, &rel_dir) {
            candidates.push(c);
        }
    };

    try_dir(root.to_path_buf(), ".".to_string());

    if let Ok(rd) = fs::read_dir(root) {
        for entry in rd.filter_map(|e| e.ok()) {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if IGNORE_DIRS.contains(&name.as_str()) || name.starts_with('.') {
                continue;
            }
            try_dir(root.join(&name), name);
        }
    }

    candidates
}
