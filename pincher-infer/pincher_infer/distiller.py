"""Reflex distiller — compiles interactions into reflexes using LLM-as-compiler.

The distiller takes a (user_input, action, result) triple and produces
a Reflex definition that can be stored and matched in the future.
This is the core learning mechanism: after the third time you ask the
agent to do something, it compiles a reflex and doesn't need the LLM anymore.
"""

import json
import re
from typing import Any, Dict, Optional

from .embedder import Embedder
from .llm import LLMInterface, StubLLM

DISTILL_PROMPT = """You are a reflex compiler for PincherOS. Given a user interaction, produce a reflex definition.

A reflex is a Hoare triple: {{guard}} action {{postcondition}}
- Sigma (trigger): What natural language pattern should trigger this reflex?
- Gamma (guard): What safety conditions must be true? (Tenuo DSL)
- Delta (action): What shell command template to execute? Use {{slot}} for variables.
- Lambda (postcondition): What should be true after execution? (Tenuo DSL)

User input: {user_input}
Action taken: {action}
Result: {result}

Respond with JSON only:
{{
    "trigger_pattern": "...",
    "action_template": "...",
    "guard_expr": "...",
    "post_condition": "...",
    "composable": false,
    "capability_hints": ["FsRead", "FsWrite", "NetHttp", "Execute"]
}}
"""


class ReflexDistiller:
    """Compiles user interactions into reflex definitions."""

    def __init__(self, llm: Optional[LLMInterface] = None, embedder: Optional[Embedder] = None):
        self.llm = llm or StubLLM()
        self.embedder = embedder or Embedder()

    def distill(self, user_input: str, action: str, result: str) -> Dict[str, Any]:
        """Distill a reflex from a user interaction.

        Args:
            user_input: What the user said/asked
            action: What action was taken
            result: What the result was

        Returns:
            A reflex definition dict
        """
        # Try LLM-based distillation
        prompt = DISTILL_PROMPT.format(
            user_input=user_input,
            action=action,
            result=result,
        )

        response = self.llm.infer(prompt)

        # Parse the response
        reflex = self._parse_response(response, user_input, action)

        # Add embedding
        if self.embedder:
            embedding = self.embedder.embed(reflex["trigger_pattern"])
            reflex["trigger_embedding"] = embedding.tolist()

        return reflex

    def _parse_response(self, response: str, user_input: str, action: str) -> Dict[str, Any]:
        """Parse the LLM response into a reflex definition.

        Falls back to heuristic distillation if parsing fails.
        """
        # Try to extract JSON from the response
        try:
            # Find JSON in the response (may be wrapped in markdown code blocks)
            json_match = re.search(r'\{[^{}]*\}', response, re.DOTALL)
            if json_match:
                parsed = json.loads(json_match.group())
                return {
                    "trigger_pattern": parsed.get("trigger_pattern", user_input),
                    "action_template": parsed.get("action_template", action),
                    "guard_expr": parsed.get("guard_expr"),
                    "post_condition": parsed.get("post_condition"),
                    "composable": parsed.get("composable", False),
                    "capability_hints": parsed.get("capability_hints", []),
                }
        except json.JSONDecodeError:
            pass

        # Heuristic fallback
        return self._heuristic_distill(user_input, action)

    def _heuristic_distill(self, user_input: str, action: str) -> Dict[str, Any]:
        """Heuristic reflex distillation when LLM is unavailable.

        This creates a simple reflex that matches similar user inputs
        and replays the same action.
        """
        # Extract key words from user input
        words = re.findall(r'\w+', user_input.lower())
        trigger = " ".join(words[:5])  # First 5 words as trigger

        return {
            "trigger_pattern": trigger,
            "action_template": action,
            "guard_expr": None,
            "post_condition": None,
            "composable": False,
            "capability_hints": self._infer_capabilities(action),
        }

    def _infer_capabilities(self, action: str) -> list:
        """Infer required capabilities from an action template."""
        capabilities = []

        if any(cmd in action for cmd in ["cat ", "ls ", "head ", "tail ", "grep ", "find "]):
            capabilities.append("FsRead")

        if any(cmd in action for cmd in ["mkdir ", "touch ", "rm ", "cp ", "mv ", "write "]):
            capabilities.append("FsWrite")

        if any(cmd in action for cmd in ["curl ", "wget ", "http ", "fetch "]):
            capabilities.append("NetHttp")

        if any(cmd in action for cmd in ["python", "bash", "sh ", "./", "node "]):
            capabilities.append("Execute")

        return capabilities if capabilities else ["FsRead"]
