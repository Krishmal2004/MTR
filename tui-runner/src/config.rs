use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub title: String,
    pub processes: Vec<ProcessConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub name: String,
    #[serde(rename = "type", default = "default_type")]
    pub kind: String, // "simple" | "multistep"
    pub cmd: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default, rename = "readyWhen")]
    pub ready_when: Option<ReadyWhen>,
    #[serde(default, rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub vars: HashMap<String, String>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

fn default_type() -> String {
    "simple".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default, rename = "readyWhen")]
    pub ready_when: Option<ReadyWhen>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ReadyWhen {
    Delay {
        ms: u64,
    },
    Regex {
        pattern: String,
        #[serde(default)]
        flags: Option<String>,
    },
    Http {
        url: String,
        #[serde(default, rename = "timeoutMs")]
        timeout_ms: Option<u64>,
        #[serde(default, rename = "intervalMs")]
        interval_ms: Option<u64>,
    },
    Poll {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default, rename = "timeoutMs")]
        timeout_ms: Option<u64>,
        #[serde(default, rename = "intervalMs")]
        interval_ms: Option<u64>,
        #[serde(default, rename = "matchField")]
        match_field: Option<String>,
        #[serde(default, rename = "matchValue")]
        match_value: Option<String>,
        #[serde(default, rename = "fallbackMatch")]
        fallback_match: Option<FallbackMatch>,
        #[serde(default, rename = "captureField")]
        capture_field: Option<String>,
        #[serde(default, rename = "captureAs")]
        capture_as: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackMatch {
    pub field: String,
    pub value: serde_json::Value,
}

pub fn config_path(root: &Path) -> PathBuf {
    root.join("tui.config.json")
}

pub fn load_config(root: &Path) -> Option<Config> {
    let path = config_path(root);
    if !path.exists() {
        return None;
    }
    let text = fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<Config>(&text) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("Could not parse {}: {}", path.display(), e);
            None
        }
    }
}

pub fn save_config(root: &Path, config: &Config) -> anyhow::Result<PathBuf> {
    let path = config_path(root);
    let text = serde_json::to_string_pretty(config)?;
    fs::write(&path, text + "\n")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multistep_flutter_style_config() {
        let json = r#"{
          "title": "T",
          "processes": [
            {"name":"mobile","type":"multistep","cwd":"mobile","vars":{"emulatorId":"x"},
             "steps":[
               {"cmd":"flutter","args":["emulators","--launch","{emulatorId}"],
                "readyWhen":{"type":"poll","command":"flutter","args":["devices","--machine"],
                  "matchField":"emulatorId","matchValue":"{emulatorId}",
                  "fallbackMatch":{"field":"emulator","value":true},
                  "captureField":"id","captureAs":"deviceId"}},
               {"cmd":"flutter","args":["run","-d","{deviceId}"]}
             ]}
          ]}"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.processes[0].kind, "multistep");
        assert_eq!(cfg.processes[0].steps.len(), 2);
    }

    #[test]
    fn parses_the_bundled_example_config() {
        let text = include_str!("../tui.config.example.json");
        let cfg: Config = serde_json::from_str(text).unwrap();
        assert_eq!(cfg.processes.len(), 3);
        let mobile = cfg.processes.iter().find(|p| p.name == "mobile").unwrap();
        assert_eq!(mobile.kind, "multistep");
        assert_eq!(mobile.steps.len(), 2);
        let web = cfg.processes.iter().find(|p| p.name == "web").unwrap();
        assert_eq!(web.depends_on, vec!["backend".to_string()]);
        
    }
}
