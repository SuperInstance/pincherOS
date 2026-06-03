"""Data models for pincher-infer.

Defines the structured types used throughout the inference bridge:
- ReflexCandidate: output of the distiller
- CapabilityManifest: security/capability description
- EmbedResult: embedding output
- InferResult: LLM inference output
- HealthResult: health check output
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


# ---------------------------------------------------------------------------
# Capability Manifest
# ---------------------------------------------------------------------------


@dataclass
class Capability:
    """A single capability requirement for a reflex."""

    kind: str  # e.g. "FsWrite", "FsRead", "NetConnect"
    target: str  # e.g. "parent({name})", "{url}"

    def to_dict(self) -> dict[str, str]:
        return {"kind": self.kind, "target": self.target}

    @classmethod
    def from_dict(cls, d: dict[str, str]) -> Capability:
        return cls(kind=d["kind"], target=d["target"])


@dataclass
class CapabilityManifest:
    """Describes the capabilities a reflex requires to execute."""

    capabilities: list[Capability] = field(default_factory=list)
    network_scope: str = "none"  # none, outbound, full

    def to_dict(self) -> dict[str, Any]:
        return {
            "capabilities": [c.to_dict() for c in self.capabilities],
            "network_scope": self.network_scope,
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> CapabilityManifest:
        caps = [Capability.from_dict(c) for c in d.get("capabilities", [])]
        return cls(capabilities=caps, network_scope=d.get("network_scope", "none"))


# ---------------------------------------------------------------------------
# Reflex Candidate (distiller output)
# ---------------------------------------------------------------------------


@dataclass
class ReflexCandidate:
    """A distilled reflex produced by the LLM distiller."""

    trigger_pattern: str
    action_template: str
    guard_expr: str
    capability_manifest: CapabilityManifest
    confidence: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "trigger_pattern": self.trigger_pattern,
            "action_template": self.action_template,
            "guard_expr": self.guard_expr,
            "capability_manifest": self.capability_manifest.to_dict(),
            "confidence": round(self.confidence, 2),
        }

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> ReflexCandidate:
        manifest = CapabilityManifest.from_dict(
            d.get("capability_manifest", {})
        )
        return cls(
            trigger_pattern=d["trigger_pattern"],
            action_template=d["action_template"],
            guard_expr=d.get("guard_expr", ""),
            capability_manifest=manifest,
            confidence=float(d.get("confidence", 0.0)),
        )


# ---------------------------------------------------------------------------
# RPC Result Types
# ---------------------------------------------------------------------------


@dataclass
class EmbedResult:
    """Result of an embedding call."""

    embedding: list[float]
    dim: int

    def to_dict(self) -> dict[str, Any]:
        return {"embedding": self.embedding, "dim": self.dim}


@dataclass
class InferResult:
    """Result of an LLM inference call."""

    text: str
    tokens_used: int
    duration_ms: int

    def to_dict(self) -> dict[str, Any]:
        return {
            "text": self.text,
            "tokens_used": self.tokens_used,
            "duration_ms": self.duration_ms,
        }


@dataclass
class HealthResult:
    """Result of a health check."""

    status: str
    embedding_model: str
    llm_model: str
    llm_available: bool
    uptime_secs: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "status": self.status,
            "embedding_model": self.embedding_model,
            "llm_model": self.llm_model,
            "llm_available": self.llm_available,
            "uptime_secs": round(self.uptime_secs, 1),
        }


# ---------------------------------------------------------------------------
# JSON-RPC 2.0 types
# ---------------------------------------------------------------------------


@dataclass
class JsonRpcRequest:
    """A JSON-RPC 2.0 request."""

    id: int | str | None
    method: str
    params: dict[str, Any] = field(default_factory=dict)
    jsonrpc: str = "2.0"


@dataclass
class JsonRpcError:
    """A JSON-RPC 2.0 error object."""

    code: int
    message: str
    data: Any | None = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"code": self.code, "message": self.message}
        if self.data is not None:
            d["data"] = self.data
        return d


# Standard JSON-RPC error codes
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
INTERNAL_ERROR = -32603
PARSE_ERROR = -32700
INVALID_REQUEST = -32600
