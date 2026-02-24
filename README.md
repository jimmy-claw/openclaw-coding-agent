# OpenClaw Coding Agent — Executor Framework

A pluggable executor framework for AI coding agents. Supports SSH hosts, Docker containers, local execution, and cloud VMs.

## Overview

This framework allows OpenClaw agents to delegate coding tasks to various execution environments:

- **SSH** — Remote machines (e.g., build servers with Rust/Cargo)
- **Container** — Docker containers (fresh or existing)
- **Local** — Run directly on the OpenClaw host
- **Cloud** — Spin up VMs on-demand (Hetzner, AWS, etc.)

## Architecture

```
┌─────────────────────────────────────────┐
│         OpenClaw Agent (Brain)         │
│  - Decides which executor to use       │
│  - Monitors task progress              │
│  - Fetches logs & activity            │
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
│(192.168│ │(isolated│ │(local  │
│.0.152) │ │ builds) │ │ tasks) │
└───────┘ └─────────┘ └────────┘
```

## Configuration

```yaml
# ~/.config/openclaw/coding-agent.yaml
executors:
  - name: crib
    type: ssh
    host: 192.168.0.152
    user: jimmy
    claude_path: ~/.npm-global/bin/claude
    labels:
      - rust
      - heavy-compute
      
  - name: builder
    type: container
    image: claude-code:latest
    runtime: docker  # or podman
    volumes:
      - /home/jimmy/repos:/work
    labels:
      - isolated
      - reproducible
      
  - name: local
    type: local
    claude_path: /usr/local/bin/claude
    labels:
      - quick-tasks
      - lightweight
```

## Task Dispatch

```python
# OpenClaw agent dispatches a task
from coding_agent import dispatch

task = dispatch(
    prompt="Fix the build...",
    executor="crib",  # or "builder", "local"
    max_turns=100,
    labels={"rust", "heavy"}
)

# Monitor
while task.running:
    status = task.status()
    print(f"CPU: {status.cpu}%, MEM: {status.mem}MB")
    print(task.recent_activity())  # last 10 tool calls
    time.sleep(30)
```

## Executor Interface

Each executor implements:

- `start(task)` — Begin execution, return task ID
- `status(task_id)` — Get process info (PID, CPU, MEM, uptime)
- `fetch_logs(task_id, n=10)` — Get recent JSONL activity
- `kill(task_id)` — Terminate the task
- `cleanup(task_id)` — Remove containers, temp files, etc.

## Roadmap

- [ ] SSH executor (basic)
- [ ] Container executor (Docker)
- [ ] Local executor
- [ ] Task metadata system (.meta.json)
- [ ] Dashboard integration
- [ ] Cloud executor (Hetzner)
- [ ] GPU executor support
- [ ] Multi-executor task queue

## Design Considerations

### Log Fetching

Each executor type has different log access:
- **SSH**: SSH to host, read ~/.claude/debug/ or ~/.claude/projects/
- **Container**: docker exec cat /root/.claude/debug/
- **Local**: Direct filesystem read
- **Cloud**: SSH or cloud API logs

### Security

- SSH keys must be configured
- Containers should be unprivileged
- Cloud VMs should have IAM roles
- Secrets (tokens) never in task prompts

### Resource Management

- Per-executor concurrency limits
- Auto-kill tasks exceeding max_turns
- Resource quotas (CPU, MEM, disk)

## References

- [jimmy-tools](https://github.com/jimmy-claw/jimmy-tools) — Current implementation
- [lez-multisig-module](https://github.com/jimmy-claw/logos-lez-multisig-module) — Production Nix example

## License

MIT
