# Code Review Assistant — Teach PincherOS Your Team's Review Patterns

Code review has repetitive patterns. "Check for SQL injection." "Find TODO comments." "Verify error handling." Every team has its checklist. PincherOS turns that checklist into reflexes — each one executes in ~50ms, costs nothing, and gets stronger over time.

---

## The Concept

A typical code review involves running the same checks on every PR:

1. **Security**: SQL injection, hardcoded secrets, unsafe deserialization
2. **Quality**: TODO/FIXME comments, unwrap() calls, missing error handling
3. **Conventions**: naming patterns, file structure, import organization
4. **Testing**: coverage thresholds, missing test files

Each check is a pattern you run repeatedly. That's exactly what reflexes are for.

Instead of writing GitHub Actions workflows with 15 steps, or maintaining a giant linter config, you teach PincherOS each check as a reflex. The reflex engine handles matching ("check for sql injection" → run `rg -i "SELECT.*FROM.*WHERE.*\+" src/`) and execution. You get the speed of a linter with the flexibility of natural language.

---

## Teaching Review Reflexes

### Security Checks

**SQL Injection Detection**

```bash
pincher teach \
  --intent "check for sql injection" \
  --action "rg -i 'SELECT.*FROM.*WHERE.*\\+' src/"
```

This reflex searches for string-concatenated SQL queries — a classic injection vector. The embedding means that "look for SQL injection vulnerabilities" and "find sql injection" will both match.

**Hardcoded Secrets**

```bash
pincher teach \
  --intent "check for hardcoded secrets" \
  --action "rg -i 'password\\s*=\\s*[\"\\x27][^\"\\x27]+[\"\\x27]|api_key\\s*=\\s*[\"\\x27][^\"\\x27]+[\"\\x27]' src/"
```

**Unsafe Deserialization**

```bash
pincher teach \
  --intent "check for unsafe deserialization" \
  --action "rg -i 'pickle\\.load|yaml\\.load\\(.*unsafe|deserialize' src/"
```

### Quality Checks

**TODO / FIXME / HACK Comments**

```bash
pincher teach \
  --intent "find todo comments" \
  --action "rg -i 'TODO|FIXME|HACK' src/"
```

This is one of the most commonly run checks. After a few executions, the confidence will be high enough that "any todos?" or "show me hack comments" will match instantly.

**Error Handling Gaps**

```bash
pincher teach \
  --intent "check error handling" \
  --action "rg -i 'unwrap\\(\\)|expect\\(' src/"
```

For Rust codebases, `unwrap()` and `expect()` in production code are red flags. This reflex catches them.

**Missing Documentation**

```bash
pincher teach \
  --intent "check for missing docs" \
  --action "rg -i '^pub fn|^pub struct|^pub enum' src/ | rg -v '///'"
```

### Naming Conventions

**Enforce Naming Conventions**

For custom checks that can't be expressed as a single `rg` command, write a script:

```bash
# Save as check-naming.sh
#!/usr/bin/env bash
# Check that public functions use snake_case
rg -i 'pub fn [A-Z]' src/ && echo "❌ Found PascalCase public functions" && exit 1
echo "✅ All public functions follow snake_case"
```

Then teach it:

```bash
pincher teach \
  --intent "enforce naming conventions" \
  --action "./check-naming.sh"
```

### Testing Checks

**Test Coverage**

```bash
pincher teach \
  --intent "check test coverage" \
  --action "cargo tarpaulin --out Stdout 2>&1 | tail -5"
```

**Missing Test Files**

```bash
pincher teach \
  --intent "check for missing tests" \
  --action "for f in src/**/*.rs; do testf=\"tests/$(basename \$f)\"; [ ! -f \"\$testf\" ] && echo \"Missing test: \$testf\"; done"
```

### Verify All Reflexes

```bash
pincher reflexes
```

---

## Using with CI: GitHub Actions

The real power comes from integrating PincherOS into your CI pipeline. Instead of writing separate workflow steps for each check, use `pincher do`:

```yaml
# .github/workflows/code-review.yml
name: PincherOS Code Review

on: [pull_request]

jobs:
  reflex-review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install PincherOS
        run: |
          curl -sSL https://github.com/SuperInstance/pincherOS/releases/latest/download/pincher-linux-amd64 -o /usr/local/bin/pincher
          chmod +x /usr/local/bin/pincher

      - name: Unpack review agent
        run: pincher unpack review-agent.nail

      - name: Check for SQL injection
        run: pincher do "check for sql injection"

      - name: Find TODO comments
        run: pincher do "find todo comments"

      - name: Check error handling
        run: pincher do "check error handling"

      - name: Check test coverage
        run: pincher do "check test coverage"
```

The `.nail` file approach means your review agent is portable — the same reflexes run on CI, on your laptop, and on your teammate's machine.

---

## Building a Review Pipeline: Chain Multiple Reflexes

The `review-pipeline.sh` script in this directory runs a full review cycle — all security, quality, and testing checks in sequence:

```bash
chmod +x review-pipeline.sh
./review-pipeline.sh /path/to/project
```

The pipeline:

1. Changes into the target project directory
2. Runs each reflex in sequence using `pincher do`
3. Collects results and prints a summary
4. Exits with non-zero status if any critical check fails

This is useful for:
- **Pre-commit hooks**: Run the pipeline before every commit
- **CI integration**: Use as a single step that runs all checks
- **Manual reviews**: Run on a colleague's branch before approving

The pipeline script also demonstrates how to parse `pincher do` output programmatically — check exit codes and capture stdout for reporting.

---

## The Confidence Feedback Loop

Here's the beautiful part: as your review reflexes succeed, they get stronger. Let's trace the lifecycle:

**Day 1**: You teach "check for sql injection" (confidence: 0.50)

**Day 5**: You've run it 20 times on real PRs. Each successful run increases confidence by +0.05. Confidence is now ~1.0 (capped). The reflex always matches directly — no LLM, no hesitation.

**Day 10**: A developer writes a SQL query using a format string instead of string concatenation: `format!("SELECT * FROM users WHERE id = {}", id)`. The reflex runs `rg -i "SELECT.*FROM.*WHERE.*\+" src/` and finds... nothing. The check "succeeded" (exit code 0) but didn't catch the real issue. Confidence goes up, but the reflex is incomplete.

**Day 11**: A senior reviewer catches the format string injection. They teach a new reflex:

```bash
pincher teach \
  --intent "check for format string sql injection" \
  --action "rg -i 'format!.*SELECT|format!.*INSERT|format!.*UPDATE' src/"
```

Now there are two SQL injection reflexes. Both match the general "check for sql injection" intent. The system runs both and combines the results.

Over time, your review agent accumulates team-specific knowledge. Patterns that caused bugs get their own reflexes. The confidence scores reflect real-world accuracy, not theoretical completeness. The agent literally learns from your team's mistakes.

---

## Security: File-Read Capability Only

Code review reflexes should **never** modify code. The capability manifest enforces this:

```toml
[capabilities.filesystem]
read_paths = ["src/**", "tests/**", "Cargo.toml", "Cargo.lock"]
write_paths = []   # ← Nothing. Zero write access.

[capabilities.network]
allowed = false    # ← No network access. No data exfiltration.

[capabilities.subprocess]
allowed = true     # ← Need rg, cargo, etc.
allowed_commands = ["rg", "cargo", "grep", "wc", "tail", "head"]
```

The veto engine will block any reflex that tries to:
- Write to any file (even `/tmp`)
- Make a network request
- Run a command not in the allowlist

If a reflex somehow bypasses the veto engine, the sandbox (bwrap + landlock) enforces the same restrictions at the OS level. Defense in depth.

See `code-review-capabilities.toml` in this directory for the full manifest.

---

## Quick Reference

| Command | What It Does |
|---|---|
| `pincher teach -i "check for sql injection" -a "rg ..."` | Teach a security check reflex |
| `pincher do "check for sql injection"` | Run the check (or match via reflex) |
| `pincher do "find todo comments"` | Find TODO/FIXME/HACK markers |
| `pincher match "any security issues?"` | Preview what would match |
| `pincher reflexes` | List all review reflexes |
| `pincher pack review-agent.nail` | Pack for CI deployment |
| `./review-pipeline.sh /path/to/project` | Run the full review pipeline |

---

## Next Steps

- **[Hello Reflex](../hello-reflex/)** — The 5-minute basics tutorial
- **[Smart Home Controller](../smart-home/)** — Reflexes for home automation
- **[Deploy Agent](../deploy-agent/)** — Train then deploy to CI
- **[Migration Demo](../migration-demo/)** — Move your review agent between machines
