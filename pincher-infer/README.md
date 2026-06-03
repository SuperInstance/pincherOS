# pincher-infer

LLM sidecar for **PincherOS** — a stateless inference bridge that compiles natural-language intents into executable action templates.

## Architecture

```
┌──────────────┐    JSON-RPC     ┌────────────────┐    HTTP     ┌───────────┐
│  pincher-core │ ◄─────────────► │ pincher-infer  │ ──────────► │  OpenAI   │
│   (Rust)      │    over UDS     │  (Python)      │             │  API      │
└──────────────┘                  └────────────────┘             └───────────┘
                                         │
                                         ▼
                                  sentence-transformers
                                    (local fallback)
```

The sidecar is **stateless** — all persistent state lives in the Rust core. This process is a pure function: intent in, result out.

## Installation

```bash
pip install -e .
```

### Dependencies

- Python ≥ 3.10
- [openai](https://pypi.org/project/openai/) — for LLM completions
- [sentence-transformers](https://pypi.org/project/sentence-transformers/) — for local embeddings (optional)
- [jsonrpclib-pelix](https://pypi.org/project/jsonrpclib-pelix/) — JSON-RPC server
- [numpy](https://pypi.org/project/numpy/) — vector math

## Usage

### Start the sidecar

```bash
pincher-infer --socket /tmp/pincher.sock
```

### CLI options

| Flag            | Default            | Env variable        | Description                         |
| --------------- | ------------------ | ------------------- | ----------------------------------- |
| `--socket`      | `/tmp/pincher.sock`| `PINCHER_SOCKET`    | Path to pincher-core UDS            |
| `--model`       | `gpt-4o-mini`      | `PINCHER_MODEL`     | OpenAI model name                   |
| `--api-key`     | —                  | `OPENAI_API_KEY`    | OpenAI API key                      |
| `--max-tokens`  | `1024`             | `PINCHER_MAX_TOKENS`| Max tokens for LLM completions      |
| `--temperature` | `0.1`              | `PINCHER_TEMPERATURE`| Sampling temperature               |
| `-v` / `--verbose` | off             | —                   | Enable debug logging                |

### RPC Methods

The sidecar listens on `<socket>-infer` and exposes:

| Method                   | Description                                          |
| ------------------------ | ---------------------------------------------------- |
| `distill(intent)`        | Compile intent → action template (the "teach" flow)  |
| `refine(reflex_id, feedback)` | Refine an existing reflex using LLM feedback    |
| `explain(reflex_id)`     | Generate human-readable explanation of a reflex      |
| `embed(text)`            | Compute embedding vector for text                    |
| `batch_embed(texts)`     | Compute embeddings for a batch of texts              |
| `cosine_similarity(a,b)` | Cosine similarity between two vectors               |

### Fallback Mode

If no `OPENAI_API_KEY` is set, the sidecar degrades gracefully:

- **Distillation** — uses built-in regex templates for common intents (file ops, git, docker, process management, networking)
- **Embedding** — uses local `sentence-transformers`; if unavailable, returns zero-vectors
- **Refine/Explain** — returns stubs (no LLM to call)

## Configuration File

Optional: `~/.pincher/infer.toml`

```toml
socket_path = "/tmp/pincher.sock"
model_name = "gpt-4o-mini"
api_key = "sk-..."
max_tokens = 1024
temperature = 0.1
```

Environment variables take precedence over the config file.

## Running Tests

```bash
python -m pytest tests/
```

## License

Part of PincherOS.
