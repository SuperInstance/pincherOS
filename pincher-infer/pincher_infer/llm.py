"""LLM interface — Ollama and llama.cpp backends.

Provides a unified interface for LLM inference, supporting:
- Ollama (local or remote)
- llama.cpp (local)
- Stub (no LLM available)
"""

import json
import subprocess
from typing import Optional


class LLMInterface:
    """Base class for LLM interfaces."""

    def infer(self, prompt: str, context: str = "") -> str:
        """Run inference on the LLM.

        Args:
            prompt: The user's prompt
            context: Additional context (previous turns, etc.)

        Returns:
            The LLM's response text
        """
        raise NotImplementedError


class OllamaInterface(LLMInterface):
    """Ollama-based LLM inference."""

    def __init__(self, model: str = "llama3.2", host: str = "http://localhost:11434"):
        self.model = model
        self.host = host

    def infer(self, prompt: str, context: str = "") -> str:
        """Run inference via Ollama API."""
        try:
            import ollama
            client = ollama.Client(host=self.host)
            full_prompt = f"{context}\n\n{prompt}" if context else prompt
            response = client.generate(model=self.model, prompt=full_prompt)
            return response.get("response", "")
        except ImportError:
            # Fallback to CLI
            return self._infer_cli(prompt, context)

    def _infer_cli(self, prompt: str, context: str = "") -> str:
        """Run inference via ollama CLI."""
        full_prompt = f"{context}\n\n{prompt}" if context else prompt
        try:
            result = subprocess.run(
                ["ollama", "run", self.model, full_prompt],
                capture_output=True,
                text=True,
                timeout=120,
            )
            return result.stdout.strip()
        except (subprocess.TimeoutExpired, FileNotFoundError) as e:
            return f"[LLM error: {e}]"


class LlamaCppInterface(LLMInterface):
    """llama.cpp-based LLM inference."""

    def __init__(self, model_path: Optional[str] = None, n_ctx: int = 2048):
        self.model_path = model_path
        self.n_ctx = n_ctx
        self._llm = None

    def _ensure_loaded(self):
        """Lazy-load the model."""
        if self._llm is not None:
            return

        try:
            from llama_cpp import Llama
            self._llm = Llama(
                model_path=self.model_path,
                n_ctx=self.n_ctx,
                verbose=False,
            )
        except ImportError:
            raise RuntimeError("llama-cpp-python not installed")

    def infer(self, prompt: str, context: str = "") -> str:
        """Run inference via llama.cpp."""
        self._ensure_loaded()

        full_prompt = f"{context}\n\n{prompt}" if context else prompt
        response = self._llm(
            full_prompt,
            max_tokens=512,
            temperature=0.7,
        )
        return response["choices"][0]["text"].strip()


class StubLLM(LLMInterface):
    """Stub LLM for testing — returns canned responses."""

    def infer(self, prompt: str, context: str = "") -> str:
        return f"[stub] Received prompt: {prompt[:100]}..."


def create_llm(backend: str = "ollama", **kwargs) -> Optional[LLMInterface]:
    """Factory function to create an LLM interface.

    Args:
        backend: "ollama", "llama.cpp", "stub", or "none"
        **kwargs: Additional arguments for the LLM backend

    Returns:
        An LLMInterface instance, or None if no LLM is available
    """
    if backend == "ollama":
        return OllamaInterface(**kwargs)
    elif backend == "llama.cpp":
        return LlamaCppInterface(**kwargs)
    elif backend == "stub":
        return StubLLM()
    else:
        return None
