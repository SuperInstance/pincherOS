"""Fallback embedding service for PincherOS.

Provides vector embeddings for semantic matching of intents to reflexes.
Uses *sentence-transformers* when available; otherwise falls back to asking
pincher-core over RPC (which may have its own embedding backend).

All heavy model state is lazily initialised on first use so that importing
the module is cheap.
"""

from __future__ import annotations

import logging
from typing import Optional

import numpy as np
from numpy.typing import NDArray

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Lazy-loaded sentence-transformers model
# ---------------------------------------------------------------------------

_model: Optional[object] = None
_model_name: str = "all-MiniLM-L6-v2"


def _get_model() -> object:
    """Lazily load and cache the sentence-transformers model."""
    global _model  # noqa: PLW0603
    if _model is not None:
        return _model
    try:
        from sentence_transformers import SentenceTransformer  # type: ignore[import-untyped]

        _model = SentenceTransformer(_model_name)
        logger.info("Loaded sentence-transformers model: %s", _model_name)
        return _model
    except ImportError:
        logger.warning(
            "sentence-transformers not available — embeddings will be zero-vectors. "
            "Install with: pip install sentence-transformers"
        )
        return None
    except Exception as exc:
        logger.error("Failed to load sentence-transformers model: %s", exc)
        return None


# ---------------------------------------------------------------------------
# EmbedService
# ---------------------------------------------------------------------------

class EmbedService:
    """Stateless embedding service.

    Parameters:
        rpc_call: Optional callable that forwards an RPC request to
            pincher-core.  Signature: ``rpc_call(method: str, params: list) -> Any``.
            Used as a fallback when sentence-transformers is unavailable.
        dimension: Embedding dimension.  Defaults to 384 (all-MiniLM-L6-v2).
    """

    def __init__(
        self,
        rpc_call: Optional[object] = None,
        dimension: int = 384,
    ) -> None:
        self._rpc_call = rpc_call
        self._dimension = dimension

    # -- public API ---------------------------------------------------------

    def embed(self, text: str) -> list[float]:
        """Return a float-vector embedding for *text*.

        Strategy:
          1. Try sentence-transformers (local, fast).
          2. Fall back to pincher-core RPC.
          3. Ultimate fallback: zero-vector (better than crashing).
        """
        model = _get_model()
        if model is not None:
            vec: NDArray[np.float32] = model.encode(text, convert_to_numpy=True)  # type: ignore[union-attr]
            return vec.tolist()

        if self._rpc_call is not None:
            try:
                result = self._rpc_call("embed", [text])  # type: ignore[operator]
                if isinstance(result, list):
                    return result
            except Exception as exc:
                logger.warning("RPC embed fallback failed: %s", exc)

        logger.warning("Returning zero-vector embedding — no backend available")
        return [0.0] * self._dimension

    def batch_embed(self, texts: list[str]) -> list[list[float]]:
        """Embed a batch of texts.

        Uses the model's native batch encoding when available for better
        throughput; otherwise falls back to sequential :meth:`embed` calls.
        """
        model = _get_model()
        if model is not None:
            vecs: NDArray[np.float32] = model.encode(texts, convert_to_numpy=True)  # type: ignore[union-attr]
            return vecs.tolist()

        # No model — try RPC batch or sequential fallback.
        if self._rpc_call is not None:
            try:
                result = self._rpc_call("batch_embed", [texts])  # type: ignore[operator]
                if isinstance(result, list):
                    return result
            except Exception as exc:
                logger.warning("RPC batch_embed fallback failed: %s", exc)

        return [self.embed(t) for t in texts]

    @staticmethod
    def cosine_similarity(a: list[float], b: list[float]) -> float:
        """Compute cosine similarity between two float vectors.

        Returns a value in ``[-1.0, 1.0]``.  Returns ``0.0`` if either
        vector has zero magnitude.
        """
        va = np.asarray(a, dtype=np.float64)
        vb = np.asarray(b, dtype=np.float64)
        dot = float(np.dot(va, vb))
        norm_a = float(np.linalg.norm(va))
        norm_b = float(np.linalg.norm(vb))
        if norm_a == 0.0 or norm_b == 0.0:
            return 0.0
        return dot / (norm_a * norm_b)
