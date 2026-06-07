# 🦀 pincher — Engineer Onboarding

**From 0 to reflex master in 5 days.**

---

## 🎒 Day 1: Hello, Reflexes

### Morning (30 min)
- [ ] Read [README.md](../README.md) — understand the hermit crab philosophy
- [ ] Read the [TUTORIALS.md](../TUTORIALS.md) — run Tutorial 1
- [ ] Install pincher: `cargo build --release -p pincher-cli`

### Afternoon (2 hours)
- [ ] Run `pincher status` — verify it's alive
- [ ] Teach 5 reflexes of your own:
  ```bash
  pincher teach "list files" "ls -la"
  pincher teach "check date" "date"
  pincher teach "disk space" "df -h"
  pincher teach "whoami" "whoami"
  pincher teach "cpu info" "cat /proc/cpuinfo"
  ```
- [ ] Fire each one with variations: `pincher do "what's the date"`, `pincher do "show me files"`, etc.

### Evening (optional)
- [ ] Explore `pincher reflexes` — see the confidence scores
- [ ] Read `API_REFERENCE.md`

**🎯 Checkpoint:** You have 5+ reflexes, you've fired them from natural language, and you've seen the matching in action.

---

## ⚡ Day 2: CLI Power User

### Morning (1 hour)
- [ ] Run Tutorial 2 — chain reflexes
- [ ] Run Tutorial 4 — real-time monitoring
- [ ] Master all CLI flags:
  ```bash
  pincher do --help
  pincher teach --help
  pincher pack --help
  ```

### Afternoon (2 hours)
- [ ] Run Tutorial 7 — benchmark your reflex database
- [ ] Try fuzzy matching with similar-but-different intents:
  ```bash
  pincher do "what files exist"
  pincher do "show directory contents"
  pincher do "list everything"
  ```
- [ ] Watch confidence grow with repetition

### Evening (optional)
- [ ] Explore the embedding fallback: `pincher doctor`
- [ ] Set up ONNX if not already: `pincher doctor --fix`

**🎯 Checkpoint:** You can teach, match, chain, and monitor reflexes like a CLI ninja.

---

## 🔐 Day 3: Security + Sandbox

### Morning (1.5 hours)
- [ ] Run Tutorial 5 — capability tokens
- [ ] Create a "read-only" profile
- [ ] Create a "sysadmin" profile
- [ ] Test both with blocked and allowed commands

### Afternoon (2 hours)
- [ ] Set up bubblewrap sandbox:
  ```bash
  sudo apt-get install bubblewrap
  pincher doctor  # Should show sandbox: active
  ```
- [ ] Test a dangerous reflex:
  ```bash
  pincher teach "delete everything" "rm -rf /"
  pincher do "delete everything"
  # Should block with VETO: "rm -rf /"
  ```

### Evening (optional)
- [ ] Read `pincher-core/src/security/veto.rs`
- [ ] Read `pincher-core/src/security/saep.rs`

**🎯 Checkpoint:** Your pincher is sandboxed, veto'd, and capability-restricted.

---

## 📦 Day 4: Agent Portability

### Morning (1 hour)
- [ ] Run Tutorial 3 — pack and unpack
- [ ] Create a portable "developer agent" `.nail`:
  ```bash
  pincher pack --output dev-agent.nail
  ```

### Afternoon (2 hours)
- [ ] Set up the RPC server:
  ```bash
  pincher daemon --port 8847
  ```
- [ ] Run Tutorial 6 — Python integration
- [ ] Write a small Python script that uses pincher for automation

### Evening (optional)
- [ ] Read about migration in `pincher-core/src/migration/`
- [ ] Explore `.nail` internals: `pincher pack --inspect`

**🎯 Checkpoint:** You can create, pack, ship, and programmatically control agents.

---

## 🚢 Day 5: Build Something Real

### All Day
- [ ] Design and implement your own pincher agent:
  - **Code review agent**: reflexes for code quality checks
  - **Deployment agent**: reflexes for Docker/Kubernetes operations
  - **Monitoring agent**: reflexes for system health checks
  - **Personal assistant**: your own custom reflex library
- [ ] Package it as a `.nail` bundle
- [ ] Write reflexes for a specific domain (10+ reflexes minimum)

### Stretch Goals
- [ ] Contribute a custom reflex set back to the repo
- [ ] Create a new security profile template
- [ ] Integrate with `agent-sync` for timing-aware reflexes
- [ ] Write integration tests for your agent

**🎯 Checkpoint:** You have a production-ready pincher agent with a well-trained reflex database.

---

## 📚 Quick Reference

| Resource | What It Is | Read When |
|----------|-----------|-----------|
| `README.md` | Hook, architecture, philosophy | Day 1 |
| `TUTORIALS.md` | 7 hands-on tutorials | Day 1–2 |
| `API_REFERENCE.md` | Full API documentation | Day 1 evening |
| `ARCHITECTURE.md` | System design deep dive | Day 2 evening |
| `GETTING_STARTED.md` | First-time setup guide | Day 1 |
| `LOW_LEVEL.md` | Embeddings, vector math | Day 3 |
| `PLUG_AND_PLAY.md` | Integration patterns | Day 4 |
| `CONTRIBUTING.md` | How to contribute | Day 5 |
| `examples/code-review/` | Full code review agent example | Day 5 |
| `examples/deploy-agent/` | Deployment agent example | Day 5 |
| `examples/smart-home/` | Smart home agent example | Day 5 |

---

## ❓ Help

- **Bug report?** GitHub issues
- **Fleet coordination?** `construct-coordination` repo
- **Architecture questions?** `ARCHITECTURE.md`

---

*The cortex teaches the spinal cord. The spinal cord gets faster.*
