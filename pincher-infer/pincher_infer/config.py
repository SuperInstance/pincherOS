"""Configuration management for pincher-infer.

Reads from environment variables and ~/.pincher/infer.toml with sensible defaults.
Environment variables take precedence over the config file.
"""

from __future__ import annotations

import os
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ModuleNotFoundError:
        tomllib = None  # type: ignore[assignment]


DEFAULT_SOCKET_PATH = "/tmp/pincher.sock"
DEFAULT_MODEL = "gpt-4o-mini"
DEFAULT_MAX_TOKENS = 1024
DEFAULT_TEMPERATURE = 0.1
CONFIG_DIR = Path.home() / ".pincher"
CONFIG_FILE = CONFIG_DIR / "infer.toml"


@dataclass(frozen=True)
class InferConfig:
    """Immutable configuration for the inference sidecar.

    Attributes:
        socket_path: Path to the pincher-core Unix domain socket.
        model_name: OpenAI model identifier (or local model name).
        api_key: OpenAI API key. ``None`` triggers local-only fallback.
        max_tokens: Maximum tokens for LLM completions.
        temperature: Sampling temperature — low for deterministic output.
    """

    socket_path: str = DEFAULT_SOCKET_PATH
    model_name: str = DEFAULT_MODEL
    api_key: Optional[str] = None
    max_tokens: int = DEFAULT_MAX_TOKENS
    temperature: float = DEFAULT_TEMPERATURE


def _read_toml(path: Path) -> dict:
    """Read a TOML file, returning an empty dict on failure."""
    if tomllib is None:
        return {}
    if not path.is_file():
        return {}
    try:
        with open(path, "rb") as fh:
            return tomllib.load(fh)  # type: ignore[union-attr]
    except Exception:
        return {}


def load_config() -> InferConfig:
    """Load configuration from env vars and ``~/.pincher/infer.toml``.

    Priority (highest → lowest):
      1. Environment variables
      2. ``~/.pincher/infer.toml``
      3. Built-in defaults
    """
    file_cfg: dict = _read_toml(CONFIG_FILE)

    # Environment variables always win.
    socket_path = (
        os.environ.get("PINCHER_SOCKET")
        or file_cfg.get("socket_path")
        or DEFAULT_SOCKET_PATH
    )
    model_name = (
        os.environ.get("PINCHER_MODEL")
        or file_cfg.get("model_name")
        or DEFAULT_MODEL
    )
    api_key = (
        os.environ.get("OPENAI_API_KEY")
        or file_cfg.get("api_key")
        or None
    )
    max_tokens_str = os.environ.get("PINCHER_MAX_TOKENS")
    max_tokens = (
        int(max_tokens_str)
        if max_tokens_str is not None
        else int(file_cfg.get("max_tokens", DEFAULT_MAX_TOKENS))
    )
    temperature_str = os.environ.get("PINCHER_TEMPERATURE")
    temperature = (
        float(temperature_str)
        if temperature_str is not None
        else float(file_cfg.get("temperature", DEFAULT_TEMPERATURE))
    )

    return InferConfig(
        socket_path=socket_path,
        model_name=model_name,
        api_key=api_key,
        max_tokens=max_tokens,
        temperature=temperature,
    )
