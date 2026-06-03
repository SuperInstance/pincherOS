"""Client for DeepInfra API — multi-model pipeline with cost-tiered routing.

Provides an async HTTP client that communicates with the DeepInfra inference
platform, supporting chat completions and embeddings endpoints.  Requests are
automatically routed to models in the appropriate cost tier based on complexity
estimates supplied by the caller.

Features:
    - Async HTTP client using ``httpx``
    - Model registry with three cost tiers (cheap / medium / expensive)
    - Retry logic with exponential backoff
    - Token-bucket rate limiting
    - Graceful error handling — never crashes on API failures
"""

from __future__ import annotations

import asyncio
import logging
import time
from enum import Enum
from typing import Any, Optional

import httpx
from pydantic import BaseModel, Field

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Data models
# ---------------------------------------------------------------------------


class CostTier(str, Enum):
    """Cost tier for model selection."""

    CHEAP = "cheap"
    MEDIUM = "medium"
    EXPENSIVE = "expensive"


class ModelInfo(BaseModel):
    """Metadata about a single model available on DeepInfra.

    Attributes:
        model_id: The DeepInfra model identifier (e.g.
            ``"meta-llama/Meta-Llama-3-8B-Instruct"``).
        tier: Which cost tier this model belongs to.
        cost_per_1k_input: Approximate USD cost per 1 000 input tokens.
        cost_per_1k_output: Approximate USD cost per 1 000 output tokens.
        max_context: Maximum context window in tokens.
        supports_chat: Whether the model supports chat completions.
        supports_embeddings: Whether the model can produce embeddings.
    """

    model_id: str
    tier: CostTier
    cost_per_1k_input: float = 0.0
    cost_per_1k_output: float = 0.0
    max_context: int = 4096
    supports_chat: bool = True
    supports_embeddings: bool = False


class ChatMessage(BaseModel):
    """A single message in a chat conversation.

    Attributes:
        role: One of ``"system"``, ``"user"``, or ``"assistant"``.
        content: The text content of the message.
    """

    role: str
    content: str


class ChatCompletionRequest(BaseModel):
    """Request payload for a chat completion.

    Attributes:
        model: Model identifier to use.
        messages: Conversation history.
        max_tokens: Upper bound on generated tokens.
        temperature: Sampling temperature.
        top_p: Nucleus sampling parameter.
    """

    model: str
    messages: list[ChatMessage]
    max_tokens: int = 1024
    temperature: float = 0.7
    top_p: float = 0.9


class ChatCompletionResponse(BaseModel):
    """Response from a chat completion request.

    Attributes:
        id: Response identifier from the API.
        model: Model that produced the response.
        content: The generated text.
        usage: Token usage statistics.
        cost_usd: Estimated cost in USD for this request.
    """

    id: str = ""
    model: str = ""
    content: str = ""
    usage: dict[str, int] = Field(default_factory=dict)
    cost_usd: float = 0.0


class EmbeddingRequest(BaseModel):
    """Request payload for an embedding endpoint.

    Attributes:
        model: Model identifier to use.
        input: Text or list of texts to embed.
    """

    model: str
    input: str | list[str]


class EmbeddingResponse(BaseModel):
    """Response from an embedding request.

    Attributes:
        model: Model that produced the embeddings.
        embeddings: List of float vectors.
        usage: Token usage statistics.
        cost_usd: Estimated cost in USD for this request.
    """

    model: str = ""
    embeddings: list[list[float]] = Field(default_factory=list)
    usage: dict[str, int] = Field(default_factory=dict)
    cost_usd: float = 0.0


# ---------------------------------------------------------------------------
# Default model registry
# ---------------------------------------------------------------------------

DEFAULT_MODELS: list[ModelInfo] = [
    # Cheap tier — fast, low-cost models for routine tasks
    ModelInfo(
        model_id="meta-llama/Meta-Llama-3-8B-Instruct",
        tier=CostTier.CHEAP,
        cost_per_1k_input=0.00005,
        cost_per_1k_output=0.00005,
        max_context=8192,
        supports_chat=True,
    ),
    ModelInfo(
        model_id="google/gemma-2b-it",
        tier=CostTier.CHEAP,
        cost_per_1k_input=0.00003,
        cost_per_1k_output=0.00003,
        max_context=8192,
        supports_chat=True,
    ),
    # Medium tier — balanced models
    ModelInfo(
        model_id="meta-llama/Meta-Llama-3-70B-Instruct",
        tier=CostTier.MEDIUM,
        cost_per_1k_input=0.00060,
        cost_per_1k_output=0.00060,
        max_context=8192,
        supports_chat=True,
    ),
    ModelInfo(
        model_id="Qwen/Qwen2.5-72B-Instruct",
        tier=CostTier.MEDIUM,
        cost_per_1k_input=0.00055,
        cost_per_1k_output=0.00055,
        max_context=32768,
        supports_chat=True,
    ),
    # Expensive tier — heavy models for complex reasoning
    ModelInfo(
        model_id="deepseek-ai/DeepSeek-R1",
        tier=CostTier.EXPENSIVE,
        cost_per_1k_input=0.00150,
        cost_per_1k_output=0.00150,
        max_context=65536,
        supports_chat=True,
    ),
    ModelInfo(
        model_id="meta-llama/Meta-Llama-3.1-405B-Instruct",
        tier=CostTier.EXPENSIVE,
        cost_per_1k_input=0.00200,
        cost_per_1k_output=0.00200,
        max_context=131072,
        supports_chat=True,
    ),
    # Embedding models
    ModelInfo(
        model_id="BAAI/bge-small-en-v1.5",
        tier=CostTier.CHEAP,
        cost_per_1k_input=0.00001,
        cost_per_1k_output=0.0,
        max_context=512,
        supports_chat=False,
        supports_embeddings=True,
    ),
    ModelInfo(
        model_id="sentence-transformers/all-MiniLM-L6-v2",
        tier=CostTier.CHEAP,
        cost_per_1k_input=0.00001,
        cost_per_1k_output=0.0,
        max_context=512,
        supports_chat=False,
        supports_embeddings=True,
    ),
]


# ---------------------------------------------------------------------------
# Rate limiter — token bucket algorithm
# ---------------------------------------------------------------------------


class RateLimiter:
    """Token-bucket rate limiter for API calls.

    Args:
        max_tokens: Maximum number of tokens in the bucket.
        refill_rate: Number of tokens added per second.
    """

    def __init__(self, max_tokens: int = 60, refill_rate: float = 10.0) -> None:
        self._max_tokens = max_tokens
        self._refill_rate = refill_rate
        self._tokens: float = float(max_tokens)
        self._last_refill: float = time.monotonic()
        self._lock = asyncio.Lock()

    def _refill(self) -> None:
        """Refill tokens based on elapsed time."""
        now = time.monotonic()
        elapsed = now - self._last_refill
        self._tokens = min(
            self._max_tokens,
            self._tokens + elapsed * self._refill_rate,
        )
        self._last_refill = now

    async def acquire(self) -> None:
        """Wait until a token is available, then consume it."""
        while True:
            async with self._lock:
                self._refill()
                if self._tokens >= 1.0:
                    self._tokens -= 1.0
                    return
            # Not enough tokens — wait before retrying
            await asyncio.sleep(1.0 / self._refill_rate)


# ---------------------------------------------------------------------------
# DeepInfra client
# ---------------------------------------------------------------------------


class DeepInfraClient:
    """Async HTTP client for the DeepInfra API.

    Manages model selection, rate limiting, retries with exponential backoff,
    and cost tracking.

    Args:
        api_key: DeepInfra API key.  If ``None``, reads from the
            ``DEEPINFRA_API_KEY`` environment variable.
        base_url: Base URL for the DeepInfra API.
        models: Custom model registry.  Defaults to :data:`DEFAULT_MODELS`.
        max_retries: Maximum number of retry attempts per request.
        retry_base_delay: Base delay in seconds for exponential backoff.
        rate_limit_max: Maximum burst size for the rate limiter.
        rate_limit_refill: Token refill rate per second for the rate limiter.
    """

    def __init__(
        self,
        api_key: Optional[str] = None,
        base_url: str = "https://api.deepinfra.com/v1/openai",
        models: Optional[list[ModelInfo]] = None,
        max_retries: int = 3,
        retry_base_delay: float = 1.0,
        rate_limit_max: int = 60,
        rate_limit_refill: float = 10.0,
    ) -> None:
        import os

        self._api_key = api_key or os.environ.get("DEEPINFRA_API_KEY", "")
        self._base_url = base_url.rstrip("/")
        self._models: dict[str, ModelInfo] = {
            m.model_id: m for m in (models or DEFAULT_MODELS)
        }
        self._max_retries = max_retries
        self._retry_base_delay = retry_base_delay
        self._rate_limiter = RateLimiter(
            max_tokens=rate_limit_max,
            refill_rate=rate_limit_refill,
        )
        self._client: Optional[httpx.AsyncClient] = None
        self._total_cost_usd: float = 0.0

    # -- lifecycle -----------------------------------------------------------

    async def _ensure_client(self) -> httpx.AsyncClient:
        """Lazily create the HTTP client."""
        if self._client is None or self._client.is_closed:
            self._client = httpx.AsyncClient(
                base_url=self._base_url,
                headers={
                    "Authorization": f"Bearer {self._api_key}",
                    "Content-Type": "application/json",
                },
                timeout=httpx.Timeout(connect=10.0, read=120.0, write=10.0, pool=10.0),
            )
        return self._client

    async def close(self) -> None:
        """Close the underlying HTTP client."""
        if self._client is not None and not self._client.is_closed:
            await self._client.aclose()
            self._client = None

    async def __aenter__(self) -> "DeepInfraClient":
        """Enter async context manager."""
        return self

    async def __aexit__(self, *args: Any) -> None:
        """Exit async context manager."""
        await self.close()

    # -- model registry ------------------------------------------------------

    def get_model(self, model_id: str) -> Optional[ModelInfo]:
        """Look up a model by its identifier.

        Args:
            model_id: The DeepInfra model identifier.

        Returns:
            The :class:`ModelInfo` if found, otherwise ``None``.
        """
        return self._models.get(model_id)

    def get_models_by_tier(self, tier: CostTier) -> list[ModelInfo]:
        """Return all models in a given cost tier.

        Args:
            tier: The cost tier to filter by.

        Returns:
            List of :class:`ModelInfo` objects in the requested tier.
        """
        return [m for m in self._models.values() if m.tier == tier]

    def get_cheapest_chat_model(self, tier: CostTier = CostTier.CHEAP) -> Optional[ModelInfo]:
        """Find the cheapest chat model in the given tier.

        Args:
            tier: Cost tier to search within.

        Returns:
            The cheapest :class:`ModelInfo` for chat, or ``None``.
        """
        candidates = [
            m for m in self._models.values()
            if m.tier == tier and m.supports_chat
        ]
        if not candidates:
            return None
        return min(candidates, key=lambda m: m.cost_per_1k_input + m.cost_per_1k_output)

    def get_embedding_model(self) -> Optional[ModelInfo]:
        """Find the cheapest embedding model.

        Returns:
            The cheapest embedding :class:`ModelInfo`, or ``None``.
        """
        candidates = [
            m for m in self._models.values()
            if m.supports_embeddings
        ]
        if not candidates:
            return None
        return min(candidates, key=lambda m: m.cost_per_1k_input)

    def register_model(self, model: ModelInfo) -> None:
        """Add or update a model in the registry.

        Args:
            model: The :class:`ModelInfo` to register.
        """
        self._models[model.model_id] = model
        logger.info("Registered model: %s (tier=%s)", model.model_id, model.tier.value)

    # -- cost tracking -------------------------------------------------------

    @property
    def total_cost_usd(self) -> float:
        """Cumulative cost in USD across all requests."""
        return self._total_cost_usd

    def _estimate_cost(
        self,
        model_info: ModelInfo,
        input_tokens: int,
        output_tokens: int,
    ) -> float:
        """Estimate the cost of a request in USD.

        Args:
            model_info: The model used.
            input_tokens: Number of input tokens consumed.
            output_tokens: Number of output tokens produced.

        Returns:
            Estimated cost in USD.
        """
        cost = (
            (input_tokens / 1000.0) * model_info.cost_per_1k_input
            + (output_tokens / 1000.0) * model_info.cost_per_1k_output
        )
        self._total_cost_usd += cost
        return cost

    # -- core request with retry and rate limiting ---------------------------

    async def _request_with_retry(
        self,
        method: str,
        path: str,
        json_body: Optional[dict[str, Any]] = None,
    ) -> httpx.Response:
        """Execute an HTTP request with exponential-backoff retry and rate limiting.

        Args:
            method: HTTP method (``"POST"``, ``"GET"``, etc.).
            path: URL path relative to the base URL.
            json_body: Optional JSON body for the request.

        Returns:
            The successful :class:`httpx.Response`.

        Raises:
            httpx.HTTPStatusError: If all retries are exhausted with
                non-retryable status codes.
            httpx.RequestError: If all retries fail due to network errors.
        """
        client = await self._ensure_client()
        last_exception: Optional[Exception] = None

        for attempt in range(self._max_retries + 1):
            await self._rate_limiter.acquire()
            try:
                response = await client.request(
                    method,
                    path,
                    json=json_body,
                )
                # Retry on 429 (rate-limited) and 5xx (server errors)
                if response.status_code == 429:
                    retry_after = float(response.headers.get("Retry-After", "2"))
                    logger.warning(
                        "Rate limited by API (429), retrying after %.1fs (attempt %d/%d)",
                        retry_after,
                        attempt + 1,
                        self._max_retries + 1,
                    )
                    await asyncio.sleep(retry_after)
                    continue

                if response.status_code >= 500:
                    logger.warning(
                        "Server error %d, will retry (attempt %d/%d)",
                        response.status_code,
                        attempt + 1,
                        self._max_retries + 1,
                    )
                    response.raise_for_status()

                # Success or non-retryable client error
                response.raise_for_status()
                return response

            except httpx.HTTPStatusError as exc:
                last_exception = exc
                status = exc.response.status_code
                # Don't retry 4xx errors (except 429 handled above)
                if 400 <= status < 500:
                    logger.error(
                        "Non-retryable API error %d: %s",
                        status,
                        exc.response.text[:200],
                    )
                    raise

            except (httpx.RequestError, httpx.TimeoutException) as exc:
                last_exception = exc
                logger.warning(
                    "Request error (attempt %d/%d): %s",
                    attempt + 1,
                    self._max_retries + 1,
                    exc,
                )

            # Exponential backoff before next retry
            if attempt < self._max_retries:
                delay = self._retry_base_delay * (2 ** attempt)
                jitter = delay * 0.1  # small jitter to avoid thundering herd
                import random
                actual_delay = delay + random.uniform(0, jitter)
                logger.info("Retrying in %.2fs …", actual_delay)
                await asyncio.sleep(actual_delay)

        # All retries exhausted
        if last_exception is not None:
            raise last_exception
        raise httpx.RequestError("All retries exhausted without a specific error")

    # -- public API: chat completions ----------------------------------------

    async def chat_completion(
        self,
        messages: list[ChatMessage],
        model_id: Optional[str] = None,
        tier: CostTier = CostTier.CHEAP,
        max_tokens: int = 1024,
        temperature: float = 0.7,
        top_p: float = 0.9,
    ) -> ChatCompletionResponse:
        """Generate a chat completion using DeepInfra.

        If *model_id* is not provided, the cheapest model in the requested
        *tier* is selected automatically.

        Args:
            messages: Conversation history as a list of :class:`ChatMessage`.
            model_id: Specific model to use.  Overrides *tier* if provided.
            tier: Cost tier for automatic model selection.
            max_tokens: Maximum tokens to generate.
            temperature: Sampling temperature.
            top_p: Nucleus sampling parameter.

        Returns:
            A :class:`ChatCompletionResponse` with the generated content.
        """
        # Resolve model
        if model_id is not None:
            model_info = self._models.get(model_id)
            if model_info is None:
                logger.error("Unknown model: %s", model_id)
                return ChatCompletionResponse(
                    model=model_id,
                    content=f"[error] Unknown model: {model_id}",
                )
        else:
            model_info = self.get_cheapest_chat_model(tier)
            if model_info is None:
                logger.error("No chat model available in tier: %s", tier.value)
                return ChatCompletionResponse(
                    content=f"[error] No chat model in tier {tier.value}",
                )
            model_id = model_info.model_id

        request = ChatCompletionRequest(
            model=model_id,
            messages=messages,
            max_tokens=max_tokens,
            temperature=temperature,
            top_p=top_p,
        )

        try:
            response = await self._request_with_retry(
                "POST",
                "/chat/completions",
                json_body=request.model_dump(),
            )
            data = response.json()

            # Parse response
            content = ""
            usage: dict[str, int] = {}
            resp_id = ""
            resp_model = model_id

            if isinstance(data, dict):
                resp_id = data.get("id", "")
                resp_model = data.get("model", model_id)
                usage = data.get("usage", {})
                choices = data.get("choices", [])
                if choices and isinstance(choices, list):
                    message = choices[0].get("message", {})
                    content = message.get("content", "")

            # Estimate cost
            input_tokens = usage.get("prompt_tokens", 0)
            output_tokens = usage.get("completion_tokens", 0)
            cost = self._estimate_cost(model_info, input_tokens, output_tokens)

            return ChatCompletionResponse(
                id=resp_id,
                model=resp_model,
                content=content,
                usage=usage,
                cost_usd=cost,
            )

        except Exception as exc:
            logger.error("Chat completion failed: %s", exc)
            return ChatCompletionResponse(
                model=model_id,
                content=f"[error] Chat completion failed: {exc}",
            )

    # -- public API: embeddings ----------------------------------------------

    async def create_embedding(
        self,
        text: str | list[str],
        model_id: Optional[str] = None,
    ) -> EmbeddingResponse:
        """Generate embeddings using DeepInfra.

        If *model_id* is not provided, the cheapest embedding model is used.

        Args:
            text: A single string or list of strings to embed.
            model_id: Specific model to use.

        Returns:
            An :class:`EmbeddingResponse` with the embedding vectors.
        """
        if model_id is not None:
            model_info = self._models.get(model_id)
            if model_info is None:
                logger.error("Unknown embedding model: %s", model_id)
                return EmbeddingResponse(
                    model=model_id or "",
                    embeddings=[],
                )
        else:
            model_info = self.get_embedding_model()
            if model_info is None:
                logger.error("No embedding model available")
                return EmbeddingResponse(embeddings=[])
            model_id = model_info.model_id

        request = EmbeddingRequest(
            model=model_id,
            input=text,
        )

        try:
            response = await self._request_with_retry(
                "POST",
                "/embeddings",
                json_body=request.model_dump(),
            )
            data = response.json()

            embeddings: list[list[float]] = []
            usage: dict[str, int] = {}
            resp_model = model_id

            if isinstance(data, dict):
                resp_model = data.get("model", model_id)
                usage = data.get("usage", {})
                raw_data = data.get("data", [])
                for item in raw_data:
                    emb = item.get("embedding", [])
                    if isinstance(emb, list):
                        embeddings.append(emb)

            # Estimate cost
            input_tokens = usage.get("prompt_tokens", 0)
            cost = self._estimate_cost(model_info, input_tokens, 0)

            return EmbeddingResponse(
                model=resp_model,
                embeddings=embeddings,
                usage=usage,
                cost_usd=cost,
            )

        except Exception as exc:
            logger.error("Embedding request failed: %s", exc)
            return EmbeddingResponse(
                model=model_id or "",
                embeddings=[],
            )

    # -- convenience helpers -------------------------------------------------

    async def simple_chat(
        self,
        prompt: str,
        system: str = "",
        tier: CostTier = CostTier.CHEAP,
        model_id: Optional[str] = None,
        max_tokens: int = 1024,
        temperature: float = 0.7,
    ) -> str:
        """Send a single-turn chat prompt and return the response text.

        This is a convenience wrapper around :meth:`chat_completion`.

        Args:
            prompt: The user's prompt text.
            system: Optional system prompt.
            tier: Cost tier for model selection.
            model_id: Specific model to use (overrides *tier*).
            max_tokens: Maximum tokens to generate.
            temperature: Sampling temperature.

        Returns:
            The generated text, or an error message prefixed with ``[error]``.
        """
        messages: list[ChatMessage] = []
        if system:
            messages.append(ChatMessage(role="system", content=system))
        messages.append(ChatMessage(role="user", content=prompt))

        result = await self.chat_completion(
            messages=messages,
            model_id=model_id,
            tier=tier,
            max_tokens=max_tokens,
            temperature=temperature,
        )
        return result.content
