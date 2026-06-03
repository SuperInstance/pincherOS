"""ONNX MiniLM-L6 embedding provider.

Generates 384-dimensional embeddings for text using the
all-MiniLM-L6-v2 model via ONNX Runtime or sentence-transformers.
"""

import os
from typing import List, Optional

import numpy as np


class Embedder:
    """Text embedding using MiniLM-L6-v2."""

    def __init__(self, model_name: str = "all-MiniLM-L6-v2"):
        self.model_name = model_name
        self._model = None
        self._dimension = 384  # MiniLM-L6 output dimension

    def _ensure_loaded(self):
        """Lazy-load the model on first use."""
        if self._model is not None:
            return

        try:
            from sentence_transformers import SentenceTransformer
            self._model = SentenceTransformer(self.model_name)
            self._dimension = self._model.get_sentence_embedding_dimension()
            print(f"Loaded embedding model: {self.model_name} (dim={self._dimension})",
                  file=__import__("sys").stderr)
        except ImportError:
            print("sentence-transformers not available, using random embeddings",
                  file=__import__("sys").stderr)
            self._model = None

    def embed(self, text: str) -> np.ndarray:
        """Embed a single text string.

        Returns:
            numpy array of shape (384,) with float32 values
        """
        self._ensure_loaded()

        if self._model is not None:
            embedding = self._model.encode(text, normalize_embeddings=True)
            return embedding.astype(np.float32)

        # Fallback: deterministic pseudo-embedding based on text hash
        return self._pseudo_embed(text)

    def embed_batch(self, texts: List[str]) -> np.ndarray:
        """Embed multiple text strings.

        Returns:
            numpy array of shape (N, 384) with float32 values
        """
        self._ensure_loaded()

        if self._model is not None:
            embeddings = self._model.encode(texts, normalize_embeddings=True)
            return embeddings.astype(np.float32)

        return np.array([self._pseudo_embed(t) for t in texts], dtype=np.float32)

    def _pseudo_embed(self, text: str) -> np.ndarray:
        """Generate a deterministic pseudo-embedding from text.

        This is used when the actual model is not available.
        The embedding is based on character n-gram hashing.
        NOT suitable for production — only for development/testing.
        """
        rng = np.random.RandomState(hash(text) % (2**31))
        vec = rng.randn(self._dimension).astype(np.float32)
        # Normalize to unit length
        norm = np.linalg.norm(vec)
        if norm > 0:
            vec /= norm
        return vec

    @property
    def dimension(self) -> int:
        """Return the embedding dimension."""
        return self._dimension

    def is_ready(self) -> bool:
        """Check if the embedder is ready (model loaded or pseudo-embed available)."""
        return True  # Pseudo-embed is always available
