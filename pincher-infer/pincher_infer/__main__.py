"""Entry point for pincher-infer.

Usage:
    python -m pincher_infer --socket /tmp/pincher.sock
    python -m pincher_infer  # defaults to /tmp/pincher-infer.sock
"""

from __future__ import annotations

import argparse
import logging
import sys

from .config import InferConfig
from .embedder import Embedder
from .llm import create_llm
from .distiller import ReflexDistiller
from .server import JsonRpcServer


def _setup_logging(verbose: bool = False) -> None:
    """Configure logging for the server process."""
    level = logging.DEBUG if verbose else logging.INFO
    logging.basicConfig(
        level=level,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    # Quiet down noisy libraries
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)


def _parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        prog="pincher-infer",
        description="PincherOS AI inference bridge — JSON-RPC 2.0 over UDS",
    )
    parser.add_argument(
        "--socket",
        default=None,
        help="Unix Domain Socket path (default: /tmp/pincher-infer.sock)",
    )
    parser.add_argument(
        "--models-dir",
        default=None,
        help="Directory for model files (default: ~/.pincher/models)",
    )
    parser.add_argument(
        "--llm",
        choices=["ollama", "llama.cpp", "stub", "none"],
        default="none",
        help="LLM backend to use (default: none)",
    )
    parser.add_argument(
        "--embedding-model",
        default="all-MiniLM-L6-v2",
        help="Embedding model name (default: all-MiniLM-L6-v2)",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable debug logging",
    )
    return parser.parse_args()


def main() -> None:
    """Main entry point."""
    args = _parse_args()
    _setup_logging(verbose=args.verbose)

    logger = logging.getLogger("pincher_infer")

    # Build config from env, then override with CLI args
    config = InferConfig.from_env()
    if args.socket:
        config.socket_path = args.socket
    if args.models_dir:
        config.models_dir = args.models_dir

    # Initialize components
    embedder = Embedder(model_name=args.embedding_model)

    llm = create_llm(backend=args.llm)

    distiller = ReflexDistiller(llm=llm, embedder=embedder) if llm else None

    logger.info(
        "Starting pincher-infer v0.1.0 — socket=%s, llm=%s",
        config.resolved_socket_path,
        args.llm,
    )

    # Create and run the server
    server = JsonRpcServer(
        uds_path=config.resolved_socket_path,
        embedder=embedder,
        llm=llm,
        distiller=distiller,
    )

    try:
        server.start()
    except KeyboardInterrupt:
        logger.info("Interrupted, shutting down")
        server.stop()
    except Exception as e:
        logger.critical("Fatal error: %s", e, exc_info=True)
        sys.exit(1)


if __name__ == "__main__":
    main()
