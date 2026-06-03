"""JSON-RPC server for the PincherOS inference sidecar.

Connects to pincher-core over a Unix domain socket and exposes three RPC
methods that the core can call:

* ``distill(intent)``      — compile intent → action template
* ``refine(reflex_id, feedback)`` — refine an existing reflex
* ``explain(reflex_id)``   — human-readable explanation

The server is **stateless** — all persistent data lives in the Rust core.
This process is a pure function: intent in, result out.

Usage::

    pincher-infer --socket /tmp/pincher.sock --model gpt-4o-mini
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import signal
import socket
import sys
from typing import Any, Optional

from jsonrpclib.SimpleJSONRPCServer import SimpleJSONRPCServer  # type: ignore[import-untyped]

from .config import InferConfig, load_config
from .distill import Distiller
from .embed import EmbedService

logger = logging.getLogger("pincher-infer")

# ---------------------------------------------------------------------------
# Core-side RPC helper
# ---------------------------------------------------------------------------

class CoreBridge:
    """Thin wrapper around a Unix-domain-socket connection to pincher-core.

    Used to fetch reflex data (action text) when the core calls
    ``refine`` or ``explain`` — the sidecar needs the current action
    to send to the LLM.
    """

    def __init__(self, socket_path: str) -> None:
        self._socket_path = socket_path

    def call(self, method: str, params: Optional[list[Any]] = None) -> Any:
        """Send a JSON-RPC request to pincher-core and return the result.

        Uses a fresh socket connection for each call (simple, no connection
        management needed for the low request volume expected).
        """
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or [],
            "id": 1,
        }
        data = json.dumps(payload).encode("utf-8")

        try:
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
                sock.settimeout(5.0)
                sock.connect(self._socket_path)
                sock.sendall(data + b"\n")
                # Read response — simple newline-delimited JSON.
                buf = b""
                while True:
                    chunk = sock.recv(4096)
                    if not chunk:
                        break
                    buf += chunk
                    if b"\n" in buf:
                        break
            resp = json.loads(buf.strip())
            if "error" in resp:
                raise RuntimeError(f"Core RPC error: {resp['error']}")
            return resp.get("result")
        except FileNotFoundError:
            logger.warning(
                "Core socket not found at %s — core may not be running",
                self._socket_path,
            )
            return None
        except Exception as exc:
            logger.error("Core RPC call '%s' failed: %s", method, exc)
            return None


# ---------------------------------------------------------------------------
# Server class
# ---------------------------------------------------------------------------

class PincherInferServer:
    """Stateless JSON-RPC inference server.

    Parameters:
        config: Inference configuration.
    """

    def __init__(self, config: Optional[InferConfig] = None) -> None:
        self._config = config or load_config()
        self._distiller = Distiller(self._config)
        self._embed = EmbedService()
        self._core: Optional[CoreBridge] = None
        self._rpc_server: Optional[SimpleJSONRPCServer] = None

    # -- JSON-RPC methods (exposed to pincher-core) -------------------------

    def distill(self, intent: str) -> dict[str, Any]:
        """Compile a natural-language *intent* into a structured action.

        Returns a dict matching :class:`DistillationResult` fields.
        """
        logger.info("distill(%s)", intent[:80])
        result = self._distiller.compile_intent(intent)
        return {
            "action_template": result.action_template,
            "parameters": result.parameters,
            "confidence_hint": result.confidence_hint,
            "tags": result.tags,
        }

    def refine(self, reflex_id: str, feedback: str) -> str:
        """Refine an existing reflex's action using LLM feedback.

        Fetches the current action from pincher-core, asks the LLM to
        refine it, and returns the revised action string.
        """
        logger.info("refine(%s, %s)", reflex_id, feedback[:60])
        core = self._ensure_core()
        if core is None:
            return ""

        # Fetch current reflex data from core.
        reflex_data = core.call("get_reflex", [reflex_id])
        if not reflex_data or not isinstance(reflex_data, dict):
            logger.error("Could not fetch reflex %s from core", reflex_id)
            return ""

        current_action = reflex_data.get("action_template", "")
        if not current_action:
            logger.error("Reflex %s has no action_template", reflex_id)
            return ""

        return self._distiller.refine_action(current_action, feedback)

    def explain(self, reflex_id: str) -> str:
        """Generate a human-readable explanation for a reflex."""
        logger.info("explain(%s)", reflex_id)
        core = self._ensure_core()
        if core is None:
            return ""

        reflex_data = core.call("get_reflex", [reflex_id])
        if not reflex_data or not isinstance(reflex_data, dict):
            logger.error("Could not fetch reflex %s from core", reflex_id)
            return ""

        action = reflex_data.get("action_template", "")
        if not action:
            return "No action associated with this reflex."

        return self._distiller.explain(action)

    def embed(self, text: str) -> list[float]:
        """Compute an embedding vector for *text*."""
        logger.debug("embed(%s)", text[:60])
        return self._embed.embed(text)

    def batch_embed(self, texts: list[str]) -> list[list[float]]:
        """Compute embedding vectors for a batch of texts."""
        logger.debug("batch_embed(%d texts)", len(texts))
        return self._embed.batch_embed(texts)

    def cosine_similarity(self, a: list[float], b: list[float]) -> float:
        """Compute cosine similarity between two vectors."""
        return EmbedService.cosine_similarity(a, b)

    # -- Server lifecycle ---------------------------------------------------

    def start(self, host: str = "localhost", port: int = 0) -> None:
        """Start the JSON-RPC server.

        When *port* is 0 the server listens on the Unix domain socket
        derived from the config's ``socket_path`` (appending ``-infer``).
        """
        self._core = CoreBridge(self._config.socket_path)

        # We listen on a separate UDS so pincher-core can connect to us.
        infer_socket = self._config.socket_path + "-infer"

        # Remove stale socket file.
        if os.path.exists(infer_socket):
            os.unlink(infer_socket)

        self._rpc_server = SimpleJSONRPCServer(infer_socket)
        self._rpc_server.register_function(self.distill, "distill")
        self._rpc_server.register_function(self.refine, "refine")
        self._rpc_server.register_function(self.explain, "explain")
        self._rpc_server.register_function(self.embed, "embed")
        self._rpc_server.register_function(self.batch_embed, "batch_embed")
        self._rpc_server.register_function(self.cosine_similarity, "cosine_similarity")

        logger.info(
            "PincherInferServer listening on %s (model=%s)",
            infer_socket,
            self._config.model_name,
        )
        self._rpc_server.serve_forever()

    def shutdown(self) -> None:
        """Gracefully shut down the RPC server."""
        if self._rpc_server is not None:
            logger.info("Shutting down PincherInferServer")
            self._rpc_server.shutdown()

    # -- internals ----------------------------------------------------------

    def _ensure_core(self) -> Optional[CoreBridge]:
        """Lazily initialise the core bridge."""
        if self._core is None:
            self._core = CoreBridge(self._config.socket_path)
        return self._core


# ---------------------------------------------------------------------------
# CLI entry-point
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="pincher-infer",
        description="PincherOS inference sidecar — stateless LLM bridge",
    )
    parser.add_argument(
        "--socket",
        default=None,
        help="Path to pincher-core Unix domain socket "
             "(default: /tmp/pincher.sock; env: PINCHER_SOCKET)",
    )
    parser.add_argument(
        "--model",
        default=None,
        help="OpenAI model name (default: gpt-4o-mini; env: PINCHER_MODEL)",
    )
    parser.add_argument(
        "--api-key",
        default=None,
        help="OpenAI API key (env: OPENAI_API_KEY takes precedence)",
    )
    parser.add_argument(
        "--max-tokens",
        type=int,
        default=None,
        help="Max tokens for LLM completions (default: 1024)",
    )
    parser.add_argument(
        "--temperature",
        type=float,
        default=None,
        help="Sampling temperature (default: 0.1)",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable debug logging",
    )
    return parser


def main() -> None:
    """CLI entry-point for ``pincher-infer``."""
    parser = _build_parser()
    args = parser.parse_args()

    # Logging setup.
    level = logging.DEBUG if args.verbose else logging.INFO
    logging.basicConfig(
        level=level,
        format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
        datefmt="%Y-%m-%dT%H:%M:%S",
    )

    # Build config: file/env defaults first, then CLI overrides.
    config = load_config()

    overrides: dict[str, Any] = {}
    if args.socket is not None:
        overrides["socket_path"] = args.socket
    if args.model is not None:
        overrides["model_name"] = args.model
    if args.api_key is not None:
        overrides["api_key"] = args.api_key
    if args.max_tokens is not None:
        overrides["max_tokens"] = args.max_tokens
    if args.temperature is not None:
        overrides["temperature"] = args.temperature

    if overrides:
        # Reconstruct the frozen dataclass with overrides.
        from dataclasses import replace
        config = replace(config, **overrides)  # type: ignore[arg-type]

    server = PincherInferServer(config)

    # Graceful shutdown on signals.
    def _signal_handler(signum: int, _frame: Any) -> None:
        logger.info("Received signal %d, shutting down…", signum)
        server.shutdown()
        sys.exit(0)

    signal.signal(signal.SIGTERM, _signal_handler)
    signal.signal(signal.SIGINT, _signal_handler)

    logger.info(
        "Starting pincher-infer  socket=%s  model=%s  has_api_key=%s",
        config.socket_path,
        config.model_name,
        bool(config.api_key),
    )
    server.start()


if __name__ == "__main__":
    main()
