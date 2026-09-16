# TUI Runner

A terminal UI toolkit for scaffolding new projects and running multi-process dev environments, written in Rust. Pick a framework, scaffold it into any folder, then run and monitor all of a project's dev processes (backend, frontend, mobile emulator, etc.) from one live dashboard — with dependency ordering, readiness detection, and automatic cleanup of stale ports.

## Install

**Windows** (PowerShell):
```powershell
irm https://raw.githubusercontent.com/Krishmal2004/MTR/main/tui-runner/install.ps1 | iex
```

**macOS / Linux**:
```bash
curl -fsSL https://raw.githubusercontent.com/Krishmal2004/MTR/main/tui-runner/install.sh | bash
```

Each installer downloads the latest prebuilt `tui-runner` binary from [GitHub Releases](https://github.com/Krishmal2004/MTR/releases) and puts it on your `PATH`:

- Windows: installed to `%LOCALAPPDATA%\Programs\tui-runner` and added to your user `PATH`.
- macOS / Linux: installed to `~/.local/bin` (universal binary on macOS, covering both Intel and Apple Silicon).

Once installed, run it from anywhere:
```bash
tui-runner
```

### Build from source

```bash
git clone https://github.com/Krishmal2004/MTR.git
cd MTR/tui-runner
cargo build --release
./target/release/tui-runner   # tui-runner.exe on Windows
```

## Features

- **Project scaffolder** — pick a framework from a TUI menu, choose a target folder in the built-in file browser, and it runs that framework's own CLI to scaffold the project (installing dependencies automatically where applicable).
- **Multi-process dev runner** — define a project's dev processes once in a config file and start them all together, each with its own color-coded output pane.
- **Dependency ordering & readiness detection** — a process can wait for another to be "ready" first, matched either by a regex on its output or by polling a command (e.g. waiting for a Flutter emulator to finish booting before launching the app onto it).
- **Stale port cleanup** — before starting, it detects and kills leftover processes still holding onto a port a dev server needs (via `netstat`/`taskkill` on Windows, `lsof`/`kill` on Unix).
- **Language/toolchain detector** — scans your system for installed language runtimes and CLIs before offering scaffolding options, so you're only shown frameworks you can actually build.
- **File browser** — navigate the filesystem from within the TUI to pick where a project should be created or which existing project folder to run.

## Supported project scaffolds

| Framework | Language | Command used |
|---|---|---|
| React (Vite) | JavaScript / TypeScript | `npm create vite@latest` |
| Next.js | JavaScript / TypeScript | `npx create-next-app@latest` |
| Vue 3 (Vite) | JavaScript / TypeScript | `npm create vite@latest` |
| Angular | TypeScript | `npx @angular/cli ng new` |
| SvelteKit | JavaScript / TypeScript | `npm create svelte@latest` |
| Deno | TypeScript / JavaScript | `deno init` |
| Bun | TypeScript / JavaScript | `bun init` |
| Node.js (Express) | JavaScript | `npx express-generator` |
| Flutter | Dart | `flutter create` |
| Rust (Cargo) | Rust | `cargo init` |
| .NET Web API | C# | `dotnet new webapi` |
| Go (module) | Go | `go mod init` |
| Ballerina | Ballerina | `bal new` |
| Elixir (Mix) | Elixir | `mix new` |
| Swift Package | Swift | `swift package init` |
| Java (Maven) | Java | `mvn archetype:generate` |
| Python (venv) | Python | `python -m venv` |
| Python (Poetry) | Python | `poetry init` |
| PHP (Laravel) | PHP | `composer create-project laravel/laravel` |

Each scaffold only appears in the menu if its underlying CLI (`npm`, `flutter`, `cargo`, etc.) is found on your system `PATH`.

## Usage

Run `tui-runner` and use the arrow keys + Enter to navigate, `q` to quit:

- **Create New Project** — scan installed toolchains, pick a framework, pick a destination folder, and scaffold it.
- **Browse PC / File Browser** — navigate to an existing project folder and run it using its `tui.config.json` (or launch a setup wizard to create one if it doesn't exist yet).

Pass `--reconfigure` to force the setup wizard to run again for a project you've already configured:
```bash
tui-runner --reconfigure
```

## Configuration (`tui.config.json`)

Each project run with TUI Runner has a `tui.config.json` describing its dev processes. See [`tui-runner/tui.config.example.json`](tui-runner/tui.config.example.json) for a full example. Key fields:

- `title` — display name shown in the TUI header.
- `processes[]` — the list of processes to run together, each with:
  - `name`, `cmd`, `args`, `cwd`, `color` — identity, command, and output styling.
  - `port` (optional) — a port this process listens on; TUI Runner frees it from stale processes before starting.
  - `dependsOn` (optional) — names of other processes that must be "ready" first.
  - `readyWhen` (optional) — how to detect readiness: a `regex` matched against the process's stdout, or a `poll` that repeatedly runs a command until a field in its output matches an expected value (used, for example, to wait for an emulator to finish booting).
  - `type: "multistep"` — instead of a single command, run an ordered list of `steps`, each with its own `readyWhen`, passing captured values (like a booted emulator's device ID) between steps via `vars`.

## License

No license is currently specified. If you plan to open-source this project, add a `LICENSE` file (for example MIT, Apache-2.0, or GPL-3.0) and update this section.
