# Hello Reflex — 5-Minute Tutorial

The simplest possible PincherOS example. You'll teach one reflex, execute it, and watch the confidence climb. By the end, you'll understand the core loop that makes PincherOS tick.

---

## Prerequisites

You need a built copy of PincherOS. If you haven't built it yet:

```bash
git clone https://github.com/SuperInstance/pincherOS.git
cd pincherOS
cargo build --release
```

The `pincher` binary will be at `./target/release/pincher`. For brevity, the commands below assume you've either added it to your `$PATH` or are running from the project root with the full path.

---

## Step 1: Check Your Shell

Every PincherOS agent lives on a **shell** — the hardware it runs on. Let's see what shell you're on:

```bash
pincher status
```

You'll see something like:

```
   🦀 PincherOS v0.1.0
  ╱╱╱╱╱╱╱╱╱╱╱╱╱
 ╱  Shell: my-laptop    ╲
╱   Reflexes: 0          ╲
╲   State: Normal        ╱
 ╲  RAM: 34.2%         ╱
  ╰────────────────────╯
```

Zero reflexes — your crab is a blank slate. The "Shell" name is your hostname, and the resource state is determined by the PID controller. Right now it says **Normal**, which means full LLM access is available if needed.

---

## Step 2: Teach a Reflex

Teaching a reflex is how you give your agent "muscle memory." You provide an **intent** (what the user might say) and an **action** (what to actually run):

```bash
pincher teach --intent "list docker containers" --action "docker ps"
```

Output:

```
✓ Reflex stored! (2.1ms)
  Intent:  "list docker containers"
  Action:  docker ps
  Confidence: 0.50 (initial)
```

The confidence starts at **0.50** — a neutral baseline. Every reflex begins here. It goes up with successful executions and down with failures.

What just happened internally? PincherOS:

1. Embedded your intent into a **384-dimensional vector** using the embedder (SHA-256 trigram hash → ONNX MiniLM)
2. Stored the vector, the original intent text, the action template, and the confidence score in **SQLite**
3. The reflex is now part of your agent's "rigging" — the portable state that moves with the crab

---

## Step 3: Execute It

Now let's use the reflex. The key insight: you don't have to say the intent exactly — just something similar:

```bash
pincher do "show me my containers"
```

Output:

```
✓ Matched reflex: "list docker containers" (confidence 0.92, 48ms)

CONTAINER ID   IMAGE         STATUS
abc123def456   nginx:latest  Up 2 hours

✓ Confidence updated: 0.50 → 0.55
```

Two important things happened:

1. **The match was instant.** "Show me my containers" was embedded, compared against known reflex vectors, and matched to "list docker containers" with a similarity of 0.92. Since 0.92 > 0.90, this is an **EXACT MATCH** — the LLM was never called. Zero API cost. ~48ms total.

2. **Confidence went up.** The execution succeeded (exit code 0), so confidence increased by +0.05. The more this reflex succeeds, the stronger it gets.

Try a different phrasing:

```bash
pincher do "what's running in docker"
```

```
✓ Matched reflex: "list docker containers" (confidence 0.88, 51ms)

CONTAINER ID   IMAGE         STATUS
abc123def456   nginx:latest  Up 2 hours

✓ Confidence updated: 0.55 → 0.60
```

Same reflex, different words. The embedding captures semantic similarity, not string matching.

---

## Step 4: Match Without Executing

Sometimes you want to see what would match without actually running anything. That's what `pincher match` is for:

```bash
pincher match "are there any dockers running"
```

Output:

```
Best match: "list docker containers"
  Similarity:  0.89
  Confidence:  0.60
  Action:      docker ps
  Would execute: YES (similarity > 0.70)
```

This is useful for debugging and for CI pipelines where you want to verify that an intent maps to the right reflex before you trust it in production.

---

## Step 5: Watch Your Reflex List

See everything your agent knows:

```bash
pincher reflexes
```

Output:

```
Reflexes (1):
  1. "list docker containers"
     Action:      docker ps
     Confidence:  0.60
     Executions:  2
     Last used:   2025-01-15 14:32:01
```

For more detail:

```bash
pincher reflexes --verbose
```

This shows embedding dimensions, creation timestamps, and the full execution history.

---

## Step 6: Run Benchmarks

The benchmark command measures how fast your reflex engine is on this hardware:

```bash
pincher bench
```

Output:

```
PincherOS Benchmark Results
━━━━━━━━━━━━━━━━━━━━━━━━━━━
Embed (trigram):     0.3ms
Embed (MiniLM):      8.2ms
Match (1 reflex):    0.1ms
Match (1000 reflexes): 2.4ms
Execute (docker ps):  47ms
Total end-to-end:     ~55ms

System state: Normal
LLM available: yes (sidecar idle)
```

These numbers matter because they tell you how your agent will perform as it learns more reflexes. Even at 1000 reflexes, matching takes under 3ms. The bottleneck is the action itself (e.g., `docker ps` takes ~47ms), not the reflex engine.

---

## What Happened: The Embedding → Match → Short-Circuit Path

Let's trace exactly what happens when you run `pincher do "show me my containers"`:

1. **Embed**: Your intent string is converted to a 384-dim vector. First, a fast SHA-256 trigram hash produces a 256-dim vector (~0.3ms). Then, if the ONNX MiniLM model is loaded, it produces a 384-dim vector (~8ms). The two are concatenated into a composite embedding.

2. **Match**: The composite embedding is compared against all stored reflex embeddings using **cosine similarity** in SQLite-vec. With one reflex, this takes ~0.1ms. With 1000, it takes ~2.4ms. The match threshold is:
   - **> 0.90**: EXACT MATCH → execute directly, no confirmation needed
   - **0.70–0.90**: PROBABLE MATCH → confirm with user, then execute
   - **< 0.70**: NOVEL INTENT → route to LLM for compilation into a new reflex

3. **Short-circuit**: Since the similarity was 0.92 (EXACT MATCH), the LLM was never invoked. The action `docker ps` was executed directly in the sandbox, the result was returned, and confidence was updated. Total time: ~50ms. Total API cost: $0.00.

This is the core loop: **embed → match → execute → update confidence**. Every reflex follows this path. The more reflexes your agent accumulates, and the higher their confidence, the more often the LLM is short-circuited. Your agent literally gets faster and cheaper the more you use it.

---

## Next Steps

Now that you understand the basic loop, try these examples:

- **[Smart Home Controller](../smart-home/)** — Teach reflexes for lights, sensors, and thermostat on a Raspberry Pi
- **[Code Review Assistant](../code-review/)** — Automate repetitive review patterns with reflexes
- **[Deploy Agent](../deploy-agent/)** — Train on your workstation, deploy to the cloud
- **[Migration Demo](../migration-demo/)** — Watch an agent move between shells
