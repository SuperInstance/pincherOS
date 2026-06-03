"""Intelligent model router for PincherOS — cost-aware request routing.

Classifies intent complexity using embedding similarity to known patterns,
then routes each request to the cheapest model that can handle it.  Enforces
per-session budgets and provides fallback chains when models fail.

Features:
    - Intent complexity classification (simple / medium / complex)
    - Cheapest-adequate-model routing
    - Cumulative cost tracking per session
    - Budget enforcement (stops routing when budget is exceeded)
    - Fallback chain: if primary model fails, try next tier
"""

from __future__ import annotations

import asyncio
import logging
import math
from enum import Enum
from typing import Any, Optional

from pydantic import BaseModel, Field

from .deepinfra_client import (
    ChatCompletionResponse,
    ChatMessage,
    CostTier,
    DeepInfraClient,
    ModelInfo,
)

logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Data models
# ---------------------------------------------------------------------------


class Complexity(str, Enum):
    """Estimated complexity of a user intent.

    Values:
        SIMPLE: Quick factual lookups, simple formatting, greetings.
        MEDIUM: Multi-step reasoning, summarisation, code generation.
        COMPLEX: Deep analysis, mathematical proofs, architecture design.
    """

    SIMPLE = "simple"
    MEDIUM = "medium"
    COMPLEX = "complex"


class ComplexityPattern(BaseModel):
    """A known pattern used to classify intent complexity.

    Attributes:
        text: Representative text for this pattern.
        complexity: The complexity level associated with this pattern.
    """

    text: str
    complexity: Complexity


# Default patterns for complexity classification
DEFAULT_PATTERNS: list[ComplexityPattern] = [
    # Simple — quick lookups, greetings, basic operations
    ComplexityPattern(text="What is the time?", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="Hello, how are you?", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="List the files in the directory", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="Show me the current status", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="What is the weather today?", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="Read the file contents", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="Tell me the date", complexity=Complexity.SIMPLE),
    ComplexityPattern(text="Run a simple command", complexity=Complexity.SIMPLE),
    # Medium — multi-step, summarisation, moderate reasoning
    ComplexityPattern(text="Summarize this document for me", complexity=Complexity.MEDIUM),
    ComplexityPattern(text="Write a Python function to sort a list", complexity=Complexity.MEDIUM),
    ComplexityPattern(text="Explain how this code works step by step", complexity=Complexity.MEDIUM),
    ComplexityPattern(text="Compare these two approaches", complexity=Complexity.MEDIUM),
    ComplexityPattern(text="Generate unit tests for this module", complexity=Complexity.MEDIUM),
    ComplexityPattern(text="Refactor this function to be more efficient", complexity=Complexity.MEDIUM),
    ComplexityPattern(text="Draft a response to this email", complexity=Complexity.MEDIUM),
    # Complex — deep analysis, architecture, proofs
    ComplexityPattern(text="Design a distributed system architecture for real-time data processing", complexity=Complexity.COMPLEX),
    ComplexityPattern(text="Prove that this algorithm has O(n log n) complexity", complexity=Complexity.COMPLEX),
    ComplexityPattern(text="Analyze the security vulnerabilities in this codebase and propose mitigations", complexity=Complexity.COMPLEX),
    ComplexityPattern(text="Create a comprehensive migration plan from monolith to microservices", complexity=Complexity.COMPLEX),
    ComplexityPattern(text="Design a novel consensus algorithm for edge computing", complexity=Complexity.COMPLEX),
    ComplexityPattern(text="Perform a detailed performance analysis and recommend optimizations", complexity=Complexity.COMPLEX),
]

# Complexity → cost tier mapping
COMPLEXITY_TO_TIER: dict[Complexity, CostTier] = {
    Complexity.SIMPLE: CostTier.CHEAP,
    Complexity.MEDIUM: CostTier.MEDIUM,
    Complexity.COMPLEX: CostTier.EXPENSIVE,
}

# Fallback chain: if the target tier fails, try these in order
FALLBACK_CHAIN: dict[CostTier, list[CostTier]] = {
    CostTier.CHEAP: [CostTier.MEDIUM, CostTier.EXPENSIVE],
    CostTier.MEDIUM: [CostTier.CHEAP, CostTier.EXPENSIVE],
    CostTier.EXPENSIVE: [CostTier.MEDIUM, CostTier.CHEAP],
}


class SessionBudget(BaseModel):
    """Budget tracker for a single session.

    Attributes:
        session_id: Unique identifier for the session.
        max_budget_usd: Maximum allowed spend in USD.
        spent_usd: Amount already spent.
        request_count: Number of requests made.
    """

    session_id: str
    max_budget_usd: float = 1.0
    spent_usd: float = 0.0
    request_count: int = 0

    @property
    def remaining_usd(self) -> float:
        """Remaining budget in USD."""
        return max(0.0, self.max_budget_usd - self.spent_usd)

    @property
    def is_exhausted(self) -> bool:
        """Whether the budget has been exceeded."""
        return self.spent_usd >= self.max_budget_usd

    def record_spend(self, amount_usd: float) -> bool:
        """Record a spend and return whether it fits within budget.

        Args:
            amount_usd: The cost to record.

        Returns:
            ``True`` if the spend was recorded (within budget),
            ``False`` if it would exceed the budget.
        """
        if self.spent_usd + amount_usd > self.max_budget_usd:
            logger.warning(
                "Session %s: budget exceeded ($%.4f + $%.4f > $%.4f)",
                self.session_id,
                self.spent_usd,
                amount_usd,
                self.max_budget_usd,
            )
            return False
        self.spent_usd += amount_usd
        self.request_count += 1
        return True


class RoutingDecision(BaseModel):
    """Result of a routing decision.

    Attributes:
        complexity: Estimated complexity of the intent.
        primary_tier: The cost tier initially selected.
        selected_model: The model that was actually selected.
        selected_tier: The tier of the selected model (may differ after fallback).
        budget_remaining: Remaining budget in USD after this request.
        fallback_used: Whether a fallback model was used.
        cost_usd: Cost of the completed request.
    """

    complexity: Complexity
    primary_tier: CostTier
    selected_model: str
    selected_tier: CostTier
    budget_remaining: float
    fallback_used: bool = False
    cost_usd: float = 0.0


# ---------------------------------------------------------------------------
# Model router
# ---------------------------------------------------------------------------


class ModelRouter:
    """Intelligent model router with cost-aware routing and budget enforcement.

    Classifies intent complexity, routes to the cheapest adequate model,
    tracks cumulative costs per session, and falls back through tiers on
    failure.

    Args:
        client: A :class:`DeepInfraClient` for making API calls.
        patterns: Custom complexity patterns.  Defaults to :data:`DEFAULT_PATTERNS`.
        default_budget_usd: Default per-session budget in USD.
        similarity_threshold: Minimum cosine similarity to consider a pattern
            match.  Below this threshold, complexity defaults to MEDIUM.
    """

    def __init__(
        self,
        client: DeepInfraClient,
        patterns: Optional[list[ComplexityPattern]] = None,
        default_budget_usd: float = 1.0,
        similarity_threshold: float = 0.3,
    ) -> None:
        self._client = client
        self._patterns = patterns or DEFAULT_PATTERNS
        self._default_budget = default_budget_usd
        self._similarity_threshold = similarity_threshold
        self._sessions: dict[str, SessionBudget] = {}
        # Cache for pattern embeddings (populated lazily)
        self._pattern_embeddings: Optional[list[tuple[list[float], Complexity]]] = None
        self._embed_lock = asyncio.Lock()

    # -- session management --------------------------------------------------

    def create_session(
        self,
        session_id: str,
        budget_usd: Optional[float] = None,
    ) -> SessionBudget:
        """Create a new session with a budget.

        Args:
            session_id: Unique identifier for the session.
            budget_usd: Budget in USD.  Uses the default if not specified.

        Returns:
            The created :class:`SessionBudget`.
        """
        budget = SessionBudget(
            session_id=session_id,
            max_budget_usd=budget_usd or self._default_budget,
        )
        self._sessions[session_id] = budget
        logger.info(
            "Session %s created with budget $%.4f",
            session_id,
            budget.max_budget_usd,
        )
        return budget

    def get_session(self, session_id: str) -> Optional[SessionBudget]:
        """Retrieve a session by ID.

        Args:
            session_id: The session identifier.

        Returns:
            The :class:`SessionBudget` if it exists, else ``None``.
        """
        return self._sessions.get(session_id)

    def get_or_create_session(
        self,
        session_id: str,
        budget_usd: Optional[float] = None,
    ) -> SessionBudget:
        """Get an existing session or create a new one.

        Args:
            session_id: The session identifier.
            budget_usd: Budget for a new session (ignored if session exists).

        Returns:
            The :class:`SessionBudget`.
        """
        if session_id not in self._sessions:
            return self.create_session(session_id, budget_usd)
        return self._sessions[session_id]

    # -- complexity classification -------------------------------------------

    async def _ensure_pattern_embeddings(self) -> list[tuple[list[float], Complexity]]:
        """Lazily compute and cache embeddings for all complexity patterns.

        Returns:
            List of (embedding, complexity) tuples.
        """
        if self._pattern_embeddings is not None:
            return self._pattern_embeddings

        async with self._embed_lock:
            # Double-check after acquiring lock
            if self._pattern_embeddings is not None:
                return self._pattern_embeddings

            texts = [p.text for p in self._patterns]
            response = await self._client.create_embedding(texts)

            if not response.embeddings:
                logger.warning(
                    "No embeddings returned — using heuristic classification"
                )
                return []

            embeddings_with_complexity: list[tuple[list[float], Complexity]] = []
            for i, emb in enumerate(response.embeddings):
                if i < len(self._patterns):
                    embeddings_with_complexity.append(
                        (emb, self._patterns[i].complexity)
                    )

            self._pattern_embeddings = embeddings_with_complexity
            logger.info(
                "Cached %d pattern embeddings for complexity classification",
                len(embeddings_with_complexity),
            )
            return embeddings_with_complexity

    @staticmethod
    def _cosine_similarity(a: list[float], b: list[float]) -> float:
        """Compute cosine similarity between two vectors.

        Args:
            a: First vector.
            b: Second vector.

        Returns:
            Cosine similarity in ``[-1.0, 1.0]``.
        """
        if not a or not b or len(a) != len(b):
            return 0.0
        dot = sum(x * y for x, y in zip(a, b))
        norm_a = math.sqrt(sum(x * x for x in a))
        norm_b = math.sqrt(sum(x * x for x in b))
        if norm_a == 0.0 or norm_b == 0.0:
            return 0.0
        return dot / (norm_a * norm_b)

    async def classify_complexity(self, intent: str) -> Complexity:
        """Classify the complexity of a user intent.

        Uses embedding similarity to known patterns.  If the best match
        is below the similarity threshold, defaults to MEDIUM.

        Args:
            intent: The user's intent text.

        Returns:
            The estimated :class:`Complexity`.
        """
        pattern_embeddings = await self._ensure_pattern_embeddings()

        if not pattern_embeddings:
            # Fallback to heuristic classification
            return self._heuristic_classify(intent)

        # Get embedding for the intent
        intent_response = await self._client.create_embedding(intent)
        if not intent_response.embeddings:
            logger.warning("Failed to embed intent — defaulting to MEDIUM")
            return Complexity.MEDIUM

        intent_embedding = intent_response.embeddings[0]

        # Find the most similar pattern
        best_similarity = -1.0
        best_complexity = Complexity.MEDIUM

        for pattern_emb, complexity in pattern_embeddings:
            sim = self._cosine_similarity(intent_embedding, pattern_emb)
            if sim > best_similarity:
                best_similarity = sim
                best_complexity = complexity

        if best_similarity < self._similarity_threshold:
            logger.info(
                "Low similarity (%.3f) — defaulting to MEDIUM for: %s",
                best_similarity,
                intent[:80],
            )
            return Complexity.MEDIUM

        logger.info(
            "Classified as %s (similarity=%.3f): %s",
            best_complexity.value,
            best_similarity,
            intent[:80],
        )
        return best_complexity

    @staticmethod
    def _heuristic_classify(intent: str) -> Complexity:
        """Classify complexity using simple heuristic rules.

        Used as a fallback when embeddings are unavailable.

        Args:
            intent: The user's intent text.

        Returns:
            The estimated :class:`Complexity`.
        """
        text = intent.lower()
        complex_keywords = [
            "design", "architect", "prove", "analyze", "comprehensive",
            "migration plan", "consensus", "novel", "research",
        ]
        medium_keywords = [
            "summarize", "summarise", "write", "explain", "compare",
            "generate", "refactor", "draft", "implement", "debug",
        ]
        simple_keywords = [
            "list", "show", "what is", "hello", "time", "date",
            "status", "read", "run", "tell me",
        ]

        for kw in complex_keywords:
            if kw in text:
                return Complexity.COMPLEX
        for kw in medium_keywords:
            if kw in text:
                return Complexity.MEDIUM
        for kw in simple_keywords:
            if kw in text:
                return Complexity.SIMPLE

        # Default based on length — longer intents tend to be more complex
        if len(text) > 200:
            return Complexity.COMPLEX
        if len(text) > 50:
            return Complexity.MEDIUM
        return Complexity.SIMPLE

    # -- routing -------------------------------------------------------------

    async def route(
        self,
        messages: list[ChatMessage],
        session_id: str,
        complexity: Optional[Complexity] = None,
        max_tokens: int = 1024,
        temperature: float = 0.7,
    ) -> tuple[ChatCompletionResponse, RoutingDecision]:
        """Route a chat request to the cheapest adequate model.

        Classifies intent complexity (if not provided), checks budget,
        selects a model, and falls back through tiers on failure.

        Args:
            messages: Conversation history.
            session_id: Session identifier for budget tracking.
            complexity: Pre-determined complexity.  Auto-classified if ``None``.
            max_tokens: Maximum tokens to generate.
            temperature: Sampling temperature.

        Returns:
            A tuple of (completion response, routing decision).

        Raises:
            RuntimeError: If the session budget is exhausted.
        """
        # Get or create session
        session = self.get_or_create_session(session_id)

        # Check budget
        if session.is_exhausted:
            logger.warning("Session %s: budget exhausted ($%.4f spent)", session_id, session.spent_usd)
            raise RuntimeError(
                f"Session {session_id} budget exhausted "
                f"(${session.spent_usd:.4f} / ${session.max_budget_usd:.4f})"
            )

        # Classify complexity
        if complexity is None:
            # Use the last user message for classification
            user_messages = [m for m in messages if m.role == "user"]
            intent = user_messages[-1].content if user_messages else ""
            complexity = await self.classify_complexity(intent)

        # Determine target tier
        primary_tier = COMPLEXITY_TO_TIER[complexity]
        selected_model_info: Optional[ModelInfo] = None
        selected_tier = primary_tier
        fallback_used = False

        # Try primary tier, then fallback chain
        tiers_to_try = [primary_tier] + FALLBACK_CHAIN.get(primary_tier, [])
        response: Optional[ChatCompletionResponse] = None

        for tier in tiers_to_try:
            model_info = self._client.get_cheapest_chat_model(tier)
            if model_info is None:
                logger.warning("No chat model in tier %s — trying next", tier.value)
                continue

            logger.info(
                "Trying model %s (tier=%s) for complexity=%s",
                model_info.model_id,
                tier.value,
                complexity.value,
            )

            result = await self._client.chat_completion(
                messages=messages,
                model_id=model_info.model_id,
                tier=tier,
                max_tokens=max_tokens,
                temperature=temperature,
            )

            # Check if the response indicates an error
            if result.content.startswith("[error]"):
                logger.warning(
                    "Model %s failed: %s",
                    model_info.model_id,
                    result.content[:100],
                )
                fallback_used = True
                continue

            response = result
            selected_model_info = model_info
            selected_tier = tier
            break

        if response is None:
            # All models failed
            error_msg = f"All models failed for complexity={complexity.value}"
            logger.error(error_msg)
            response = ChatCompletionResponse(content=f"[error] {error_msg}")
            selected_model_info = self._client.get_cheapest_chat_model(primary_tier)
            if selected_model_info is None:
                selected_model_info = ModelInfo(model_id="none", tier=primary_tier)

        # Record spend
        if not response.content.startswith("[error]"):
            session.record_spend(response.cost_usd)

        decision = RoutingDecision(
            complexity=complexity,
            primary_tier=primary_tier,
            selected_model=selected_model_info.model_id,
            selected_tier=selected_tier,
            budget_remaining=session.remaining_usd,
            fallback_used=fallback_used,
            cost_usd=response.cost_usd,
        )

        logger.info(
            "Routed to %s (tier=%s, cost=$%.6f, remaining=$%.4f, fallback=%s)",
            selected_model_info.model_id,
            selected_tier.value,
            response.cost_usd,
            session.remaining_usd,
            fallback_used,
        )

        return response, decision

    async def route_simple(
        self,
        prompt: str,
        session_id: str,
        system: str = "",
        complexity: Optional[Complexity] = None,
        max_tokens: int = 1024,
        temperature: float = 0.7,
    ) -> tuple[str, RoutingDecision]:
        """Convenience method for single-turn routing.

        Args:
            prompt: The user's prompt text.
            session_id: Session identifier for budget tracking.
            system: Optional system prompt.
            complexity: Pre-determined complexity.  Auto-classified if ``None``.
            max_tokens: Maximum tokens to generate.
            temperature: Sampling temperature.

        Returns:
            A tuple of (response text, routing decision).
        """
        messages: list[ChatMessage] = []
        if system:
            messages.append(ChatMessage(role="system", content=system))
        messages.append(ChatMessage(role="user", content=prompt))

        response, decision = await self.route(
            messages=messages,
            session_id=session_id,
            complexity=complexity,
            max_tokens=max_tokens,
            temperature=temperature,
        )
        return response.content, decision

    # -- reporting -----------------------------------------------------------

    def session_summary(self, session_id: str) -> Optional[dict[str, Any]]:
        """Generate a summary of a session's routing activity.

        Args:
            session_id: The session to summarize.

        Returns:
            A dict with session stats, or ``None`` if the session doesn't exist.
        """
        session = self._sessions.get(session_id)
        if session is None:
            return None
        return {
            "session_id": session.session_id,
            "max_budget_usd": session.max_budget_usd,
            "spent_usd": round(session.spent_usd, 6),
            "remaining_usd": round(session.remaining_usd, 6),
            "request_count": session.request_count,
            "is_exhausted": session.is_exhausted,
        }

    def all_sessions_summary(self) -> list[dict[str, Any]]:
        """Generate summaries for all sessions.

        Returns:
            A list of session summary dicts.
        """
        return [
            self.session_summary(sid)
            for sid in self._sessions
            if self.session_summary(sid) is not None
        ]
