"""LLM-as-Compiler distillation module.

The Distiller transforms free-form natural-language intents into structured,
parameterised action templates that the PincherOS core can store, execute,
and evolve.  Think of it as a compiler front-end where the source language
is English and the target language is executable shell / Python snippets.

Design principles
-----------------
* **Deterministic over creative** — temperature defaults to 0.1; prompts
  demand concise, machine-readable output, not prose.
* **Stateless** — all persistent state lives in the Rust core.  The
  distiller only transforms data in-flight.
* **Fallback-friendly** — if no OpenAI key is present the distiller
  degrades to pattern-matched templates rather than crashing.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from typing import Any, Optional

from openai import OpenAI

from .config import InferConfig

# ---------------------------------------------------------------------------
# Data structures
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class DistillationResult:
    """Structured output of a distillation pass.

    Attributes:
        action_template: A parameterised shell command or Python snippet.
            Parameters are ``{{param_name}}`` placeholders.
        parameters: Mapping of placeholder names → extracted values.
        confidence_hint: Suggested initial confidence (0.0–1.0).
        tags: Free-form categorisation labels.
    """

    action_template: str
    parameters: dict[str, str] = field(default_factory=dict)
    confidence_hint: float = 0.5
    tags: list[str] = field(default_factory=list)


# ---------------------------------------------------------------------------
# System prompts
# ---------------------------------------------------------------------------

DISTILL_SYSTEM_PROMPT = """\
You are the distillation engine of PincherOS, a "post-model operating system".
Your job is to compile a natural-language INTENT into a structured ACTION that
a shell can execute.  You must respond with **only** valid JSON — no prose, no
markdown fences.

JSON schema:
{
  "action_template": "shell command or python snippet with {{param}} placeholders",
  "parameters": {"param": "value extracted from the intent"},
  "confidence_hint": 0.0-1.0 float,
  "tags": ["category", "tags"]
}

Rules:
- action_template MUST be self-contained and directly executable after
  placeholder substitution.
- Use {{param}} syntax for any variable parts.
- confidence_hint: 0.9+ for trivial/well-known ops, 0.5 for ambiguous, <0.3 for risky.
- tags: include the domain (e.g. "git", "docker", "fs", "process", "network").
- Keep templates SHORT.  One-liners preferred.
- If the intent is ambiguous, pick the most common interpretation and lower
  confidence_hint accordingly.
"""

REFINE_SYSTEM_PROMPT = """\
You are the refinement engine of PincherOS.  You receive an existing ACTION
(shell command / Python snippet) and user FEEDBACK describing what went wrong
or what to change.  Produce a **revised** action that incorporates the feedback.

Respond with **only** the revised action string — no explanation, no markdown.
Preserve {{param}} placeholders unless the feedback specifically changes them.
"""

EXPLAIN_SYSTEM_PROMPT = """\
You are the explanation engine of PincherOS.  You receive an ACTION (a shell
command or Python snippet, possibly with {{param}} placeholders).  Produce a
clear, concise, human-readable explanation of what this action does — as if
explaining to a junior developer.  Keep it under 3 sentences.
"""

# ---------------------------------------------------------------------------
# Built-in templates for common intents (used when no LLM key is available)
# ---------------------------------------------------------------------------

_TEMPLATE_PATTERNS: list[tuple[re.Pattern[str], dict[str, Any]]] = [
    # File operations
    (
        re.compile(r"list files in (?P<dir>\S+)", re.IGNORECASE),
        {
            "action_template": "ls -la {{dir}}",
            "tags": ["fs", "list"],
            "confidence_hint": 0.95,
        },
    ),
    (
        re.compile(r"remove file (?P<path>\S+)", re.IGNORECASE),
        {
            "action_template": "rm {{path}}",
            "tags": ["fs", "delete"],
            "confidence_hint": 0.7,
        },
    ),
    (
        re.compile(r"copy (?P<src>\S+) to (?P<dst>\S+)", re.IGNORECASE),
        {
            "action_template": "cp {{src}} {{dst}}",
            "tags": ["fs", "copy"],
            "confidence_hint": 0.9,
        },
    ),
    (
        re.compile(r"move (?P<src>\S+) to (?P<dst>\S+)", re.IGNORECASE),
        {
            "action_template": "mv {{src}} {{dst}}",
            "tags": ["fs", "move"],
            "confidence_hint": 0.9,
        },
    ),
    (
        re.compile(r"create directory (?P<path>\S+)", re.IGNORECASE),
        {
            "action_template": "mkdir -p {{path}}",
            "tags": ["fs", "mkdir"],
            "confidence_hint": 0.95,
        },
    ),
    (
        re.compile(r"show contents of (?P<path>\S+)", re.IGNORECASE),
        {
            "action_template": "cat {{path}}",
            "tags": ["fs", "read"],
            "confidence_hint": 0.9,
        },
    ),
    # Process management
    (
        re.compile(r"kill process (?P<pid>\d+)", re.IGNORECASE),
        {
            "action_template": "kill {{pid}}",
            "tags": ["process", "kill"],
            "confidence_hint": 0.7,
        },
    ),
    (
        re.compile(r"show running processes", re.IGNORECASE),
        {
            "action_template": "ps aux",
            "tags": ["process", "list"],
            "confidence_hint": 0.95,
        },
    ),
    # Git operations
    (
        re.compile(r"git status", re.IGNORECASE),
        {
            "action_template": "git status",
            "tags": ["git", "status"],
            "confidence_hint": 0.95,
        },
    ),
    (
        re.compile(r"git commit (?P<msg>.+)", re.IGNORECASE),
        {
            "action_template": 'git commit -m "{{msg}}"',
            "tags": ["git", "commit"],
            "confidence_hint": 0.85,
        },
    ),
    (
        re.compile(r"git push", re.IGNORECASE),
        {
            "action_template": "git push",
            "tags": ["git", "push"],
            "confidence_hint": 0.85,
        },
    ),
    (
        re.compile(r"git pull", re.IGNORECASE),
        {
            "action_template": "git pull",
            "tags": ["git", "pull"],
            "confidence_hint": 0.85,
        },
    ),
    # Docker
    (
        re.compile(r"docker ps", re.IGNORECASE),
        {
            "action_template": "docker ps",
            "tags": ["docker", "list"],
            "confidence_hint": 0.95,
        },
    ),
    (
        re.compile(
            r"docker run (?P<image>\S+)(?:\s+(?P<args>.*))?", re.IGNORECASE
        ),
        {
            "action_template": "docker run {{image}} {{args}}",
            "tags": ["docker", "run"],
            "confidence_hint": 0.7,
        },
    ),
    (
        re.compile(r"docker stop (?P<container>\S+)", re.IGNORECASE),
        {
            "action_template": "docker stop {{container}}",
            "tags": ["docker", "stop"],
            "confidence_hint": 0.8,
        },
    ),
    # Network
    (
        re.compile(r"check port (?P<port>\d+)", re.IGNORECASE),
        {
            "action_template": "ss -tlnp | grep {{port}}",
            "tags": ["network", "port"],
            "confidence_hint": 0.8,
        },
    ),
    (
        re.compile(r"ping (?P<host>\S+)", re.IGNORECASE),
        {
            "action_template": "ping -c 4 {{host}}",
            "tags": ["network", "ping"],
            "confidence_hint": 0.9,
        },
    ),
]


def _match_template(intent: str) -> Optional[DistillationResult]:
    """Try to match *intent* against the built-in template patterns.

    Returns a :class:`DistillationResult` on success, or ``None`` if no
    pattern matches.
    """
    for pattern, template in _TEMPLATE_PATTERNS:
        m = pattern.match(intent.strip())
        if m:
            params: dict[str, str] = {
                k: v for k, v in m.groupdict().items() if v is not None
            }
            return DistillationResult(
                action_template=template["action_template"],
                parameters=params,
                confidence_hint=float(template.get("confidence_hint", 0.5)),
                tags=list(template.get("tags", [])),
            )
    return None


# ---------------------------------------------------------------------------
# Distiller
# ---------------------------------------------------------------------------

class Distiller:
    """Compile natural-language intents into executable action templates.

    Parameters:
        config: Inference configuration (model, API key, etc.).
    """

    def __init__(self, config: Optional[InferConfig] = None) -> None:
        self._config = config or InferConfig()
        self._client: Optional[OpenAI] = None
        if self._config.api_key:
            self._client = OpenAI(api_key=self._config.api_key)

    # -- public API ---------------------------------------------------------

    def compile_intent(
        self,
        intent: str,
        context: Optional[dict[str, Any]] = None,
    ) -> DistillationResult:
        """Compile a natural-language *intent* into a :class:`DistillationResult`.

        Strategy:
          1. If an OpenAI key is configured, ask the LLM.
          2. Otherwise, fall back to built-in template matching.
          3. If neither produces a result, return a generic placeholder.
        """
        if self._client is not None:
            return self._compile_via_llm(intent, context or {})
        return self._compile_via_template(intent)

    def refine_action(self, action: str, feedback: str) -> str:
        """Use the LLM to refine an existing *action* given *feedback*.

        If no LLM key is available the original *action* is returned unchanged.
        """
        if self._client is None:
            return action
        response = self._client.chat.completions.create(
            model=self._config.model_name,
            messages=[
                {"role": "system", "content": REFINE_SYSTEM_PROMPT},
                {"role": "user", "content": f"Action: {action}\nFeedback: {feedback}"},
            ],
            max_tokens=self._config.max_tokens,
            temperature=self._config.temperature,
        )
        content = response.choices[0].message.content
        return content.strip() if content else action

    def explain(self, action: str) -> str:
        """Generate a human-readable explanation of *action*.

        Falls back to a generic stub if no LLM key is configured.
        """
        if self._client is None:
            return f"Executes: {action}"
        response = self._client.chat.completions.create(
            model=self._config.model_name,
            messages=[
                {"role": "system", "content": EXPLAIN_SYSTEM_PROMPT},
                {"role": "user", "content": action},
            ],
            max_tokens=self._config.max_tokens,
            temperature=self._config.temperature,
        )
        content = response.choices[0].message.content
        return content.strip() if content else f"Executes: {action}"

    # -- private helpers ----------------------------------------------------

    def _compile_via_llm(
        self,
        intent: str,
        context: dict[str, Any],
    ) -> DistillationResult:
        """Send *intent* to the LLM and parse the structured JSON response."""
        user_parts = [f"Intent: {intent}"]
        if context:
            user_parts.append(f"Context: {json.dumps(context)}")

        response = self._client.chat.completions.create(  # type: ignore[union-attr]
            model=self._config.model_name,
            messages=[
                {"role": "system", "content": DISTILL_SYSTEM_PROMPT},
                {"role": "user", "content": "\n".join(user_parts)},
            ],
            max_tokens=self._config.max_tokens,
            temperature=self._config.temperature,
        )
        content = response.choices[0].message.content
        if not content:
            return self._compile_via_template(intent)

        try:
            parsed = json.loads(content)
        except json.JSONDecodeError:
            # LLM sometimes wraps JSON in markdown fences — strip them.
            stripped = re.sub(r"^```(?:json)?\s*|\s*```$", "", content, flags=re.MULTILINE)
            try:
                parsed = json.loads(stripped)
            except json.JSONDecodeError:
                return self._compile_via_template(intent)

        return DistillationResult(
            action_template=parsed.get("action_template", ""),
            parameters=parsed.get("parameters", {}),
            confidence_hint=float(parsed.get("confidence_hint", 0.5)),
            tags=list(parsed.get("tags", [])),
        )

    @staticmethod
    def _compile_via_template(intent: str) -> DistillationResult:
        """Match *intent* against built-in templates as a fallback."""
        result = _match_template(intent)
        if result is not None:
            return result
        # Ultimate fallback — wrap the intent as an echo so something runs.
        return DistillationResult(
            action_template='echo "Unrecognised intent: {{intent}}"',
            parameters={"intent": intent},
            confidence_hint=0.1,
            tags=["unknown"],
        )
