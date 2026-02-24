# OpenClaw Coding Agent — Executor Framework

A pluggable executor framework for AI coding agents. Dispatch Claude Code tasks to remote SSH hosts, Docker containers, or local execution — then monitor, fetch logs, and manage them from one CLI.

## Installation

```bash
cargo build --release
# Binary: target/release/openclaw-agent
cp target/release/openclaw-agent ~/.local/bin/
```

## Configuration

Create `~/.config/openclaw/coding-agent.yaml`:

```yaml
executors:
  - name: crib
    type: ssh
    host: 192.168.0.152
    port: 22
    user: jimmy
    claude_path: /home/jimmy/.npm-global/bin/claude
    labels:
      - rust
      - heavy-compute

  - name: builder
    type: container
    image: claude-code:latest
    runtime: docker   # or podman
    volumes:
      - /home/jimmy/repos:/work
    labels:
      - isolated
      - reproducible

  - name: local
    type: local
    labels:
      - quick-tasks
      - lightweight

defaults:
  max_turns: 100
  claude_path: claude
```

Or generate a sample config:

```bash
openclaw-agent config --init
```

## Usage

### Start a task

```bash
# Run on SSH host
openclaw-agent start --executor crib --prompt "Fix the Nix build so nix build .#app works" --workspace ~/lez-hello-world

# Run in Docker container
openclaw-agent start --executor builder --prompt "Add error handling to the API" --workspace /work/myproject

# Run locally
openclaw-agent start --executor local --prompt "Write tests for src/lib.rs" --workspace ~/myproject

# With options
openclaw-agent start --executor crib \
  --prompt "Refactor the auth module" \
  --workspace ~/myapp \
  --max-turns 150
```

### Monitor a task

```bash
# Check status
openclaw-agent status --task-id <task-id>

# JSON output (for scripting/dashboard)
openclaw-agent status --task-id <task-id> --json
```

### Fetch logs

```bash
# Last 50 lines (default)
openclaw-agent logs --task-id <task-id>

# Last 100 lines
openclaw-agent logs --task-id <task-id> --lines 100

# Follow (poll every 5 seconds)
openclaw-agent logs --task-id <task-id> --follow 5
```

### List tasks

```bash
# All tasks
openclaw-agent list

# Filter by status
openclaw-agent list --status running
openclaw-agent list --status completed

# Filter by executor
openclaw-agent list --executor crib

# JSON/JSONL output
openclaw-agent list --json
openclaw-agent list --jsonl
```

### Kill a task

```bash
openclaw-agent kill --task-id <task-id>
```

### Cleanup

```bash
openclaw-agent cleanup --task-id <task-id>
```

### Dashboard

```bash
# Snapshot of all tasks as JSONL
openclaw-agent dashboard --stream

# Watch mode (refresh every 10 seconds)
openclaw-agent dashboard --watch 10
```

### List configured executors

```bash
openclaw-agent executors
openclaw-agent executors --json
```

## Architecture

```
┌─────────────────────────────────────────┐
│         OpenClaw Agent (Brain)          │
│  - Decides which executor to use        │
│  - Monitors task progress               │
│  - Fetches logs & activity              │
└──────────────┬──────────────────────────┘
               │
    ┌──────────┼──────────┐
    ▼          ▼          ▼
┌───────┐ ┌─────────┐ ┌────────┐
│  SSH  │ │Container│ │ Local  │
│Executor│ │Executor │ │Executor│
└───────┘ └─────────┘ └────────┘
    │          │          │
    ▼          ▼          ▼
┌───────┐ ┌─────────┐ ┌────────┐
│ Crib  │ │ Docker  │ │ Pi5    │
│(remote│ │(isolated│ │(local  │
│ host) │ │ builds) │ │ tasks) │
└───────┘ └─────────┘ └────────┘
```

### Crates

| Crate | Purpose |
|---|---|
| `executor-core` | Shared traits, types, config, metadata |
| `executor-ssh` | SSH executor (ssh2 crate, nohup + PID tracking) |
| `executor-container` | Docker/Podman executor |
| `executor-local` | Local process executor |
| `executor-cli` | Clap-based CLI binary |

### Task Metadata

Each task writes a `.meta.json` file tracking:
- Task ID (UUID)
- Executor name + type
- PID
- Status (pending / running / completed / failed / killed)
- Start/end timestamps
- Workspace path
- Prompt

SSH executor stores metadata at `/tmp/openclaw-tasks/<task-id>/` on the remote host, and mirrors it locally at `~/.local/share/openclaw/tasks/`.

## How SSH Execution Works

1. Connect to remote host via SSH (key or agent auth)
2. Create task directory at `/tmp/openclaw-tasks/<task-id>/`
3. Launch: `nohup claude --dangerously-skip-permissions --max-turns N -p "..." > task.log 2>&1 &`
4. Write PID to `task.pid`, metadata to `.meta.json`
5. Log fetching reads `~/.claude/projects/` JSONL on the remote host

## References

- [jimmy-tools](https://github.com/jimmy-claw/jimmy-tools) — Original shell script pattern this is based on
- [logos-lez-multisig-module](https://github.com/jimmy-claw/logos-lez-multisig-module) — Production Nix example

## License

MIT
