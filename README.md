# OpenClaw Coding Agent

Automated coding agent executor framework for running Claude Code and shell commands on remote hosts.

## Features

- **Multi-Executor Support**: SSH, local, Docker, and cloud executors
- **Detached Execution**: Fire-and-forget mode for long-running tasks
- **Heartbeat Monitoring**: Auto-detect stuck tasks after SSH disconnection
- **Dashboard Integration**: JSONL streaming for real-time status
- **Completion Webhooks**: Notify external services on task completion

## Installation

```bash
# Build from source
git clone https://github.com/jimmy-claw/openclaw-coding-agent
cd openclaw-coding-agent
cargo build --release

# Install
cp target/release/openclaw-agent ~/.local/bin/
chmod +x ~/.local/bin/openclaw-agent
```

## Usage

### Start a Task

```bash
# SSH executor (detached)
openclaw-agent start ssh "Fix bug in module X" --detach

# Shell command
openclaw-agent start ssh "cargo test" --type shell_command --detach

# Local executor
openclaw-agent start local "npm run build"
```

### Check Status

```bash
openclaw-agent status <task_id>

# Output:
# 🦞  Task:     abc-123
#    Type:     claude_code
#    Executor: crib (ssh)
#    Status:   running
#    ❤️  Heartbeat: 5s ago (interval: 30s)
#    PID:      12345
#    Started:  2026-02-28T21:00:00Z
#    Updated:  2026-02-28T21:05:05Z
```

### View Logs

```bash
openclaw-agent logs <task_id> --lines 100
```

### Kill a Task

```bash
openclaw-agent kill <task_id>
```

### Cleanup Stale Tasks

```bash
# Manual cleanup
openclaw-agent cleanup-stale

# Auto-cleanup (cron runs every 2 min by default)
# See: tasks/issue9-cron-setup.md
```

### Clean Up Task Artifacts

```bash
openclaw-agent cleanup <task_id>
```

## Configuration

### Config File

`~/.config/openclaw/coding-agent.yaml`:

```yaml
defaults:
  executor: ssh
  webhook_url: https://your-webhook-url.com/complete

executors:
  - name: crib
    type: ssh
    host: 192.168.0.152
    user: jimmy
    claude_binary: claude
    key_path: ~/.ssh/id_ed25519

  - name: local
    type: local
    claude_binary: claude
```

### Task Metadata

Each task creates:
- `~/.openclaw-agent/tasks/<task_id>/`: Remote artifacts
- `~/.openclaw-agent/tasks/<task_id>.meta.json`: Local metadata

Metadata fields:
```json
{
  "task_id": "abc-123",
  "executor_name": "crib",
  "executor_type": "ssh",
  "task_type": "claude_code",
  "pid": 12345,
  "status": "running",
  "prompt": "Fix bug in module X",
  "workspace": "~",
  "started_at": "2026-02-28T21:00:00Z",
  "updated_at": "2026-02-28T21:05:05Z",
  "finished_at": null,
  "exit_code": null,
  "error": null,
  "last_heartbeat": 1740896705,
  "heartbeat_interval": 30
}
```

## Heartbeat Monitoring

### How It Works

1. **Task Launch**: Starts heartbeat script + main process
2. **Heartbeat**: Writes `~/.openclaw-agent/tasks/<id>/heartbeat.json` every 30s
3. **Status Check**: Reads heartbeat, detects staleness (>5min = 10 intervals)
4. **Auto-Cleanup**: Cron job runs every 2min, fixes stuck tasks

### Thresholds

- **Heartbeat interval**: 30 seconds (default, configurable)
- **Stale threshold**: 300 seconds (5 minutes = 10 intervals)
- **Status when stale**: `heartbeat_timeout` (terminal state)

### Manual Testing

```bash
# Create artificial stale task
cat > ~/.openclaw-agent/tasks/test-stale.meta.json << 'EOF'
{"task_id":"test-123","executor_name":"crib","executor_type":"ssh","task_type":"claude_code","pid":9999,"status":"running","prompt":"test","workspace":"~","started_at":"2026-02-28T21:00:00Z","updated_at":"2026-02-28T21:00:00Z","finished_at":null,"exit_code":null,"error":null,"last_heartbeat":1709234567,"heartbeat_interval":30}
