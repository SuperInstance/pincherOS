"""Unit tests for pincher_infer.distill.

Uses mocking for LLM calls so tests run without an OpenAI API key.
"""

from __future__ import annotations

import json
import unittest
from unittest.mock import MagicMock, patch

from pincher_infer.config import InferConfig
from pincher_infer.distill import DistillationResult, Distiller, _match_template


# ---------------------------------------------------------------------------
# Helper: build a mock OpenAI response
# ---------------------------------------------------------------------------

def _mock_openai_response(content: str) -> MagicMock:
    """Create a mock that mimics ``openai.ChatCompletion.create()`` return."""
    mock_message = MagicMock()
    mock_message.content = content
    mock_choice = MagicMock()
    mock_choice.message = mock_message
    mock_response = MagicMock()
    mock_response.choices = [mock_choice]
    return mock_response


# ---------------------------------------------------------------------------
# Template matching tests (no LLM needed)
# ---------------------------------------------------------------------------

class TestTemplateMatching(unittest.TestCase):
    """Test built-in template pattern matching."""

    def test_list_files(self) -> None:
        result = _match_template("list files in /tmp")
        self.assertIsNotNone(result)
        assert result is not None  # for type checker
        self.assertEqual(result.action_template, "ls -la {{dir}}")
        self.assertEqual(result.parameters, {"dir": "/tmp"})
        self.assertIn("fs", result.tags)
        self.assertGreater(result.confidence_hint, 0.8)

    def test_remove_file(self) -> None:
        result = _match_template("remove file /var/log/app.log")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "rm {{path}}")
        self.assertEqual(result.parameters, {"path": "/var/log/app.log"})
        self.assertIn("delete", result.tags)

    def test_copy_file(self) -> None:
        result = _match_template("copy /etc/hosts to /tmp/hosts")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "cp {{src}} {{dst}}")
        self.assertEqual(result.parameters, {"src": "/etc/hosts", "dst": "/tmp/hosts"})

    def test_move_file(self) -> None:
        result = _match_template("move /tmp/old to /tmp/new")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "mv {{src}} {{dst}}")

    def test_create_directory(self) -> None:
        result = _match_template("create directory /opt/myapp")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "mkdir -p {{path}}")
        self.assertEqual(result.parameters, {"path": "/opt/myapp"})

    def test_show_contents(self) -> None:
        result = _match_template("show contents of /etc/hostname")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "cat {{path}}")
        self.assertEqual(result.parameters, {"path": "/etc/hostname"})

    def test_kill_process(self) -> None:
        result = _match_template("kill process 12345")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "kill {{pid}}")
        self.assertEqual(result.parameters, {"pid": "12345"})

    def test_git_status(self) -> None:
        result = _match_template("git status")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertIn("git", result.tags)

    def test_docker_ps(self) -> None:
        result = _match_template("docker ps")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertIn("docker", result.tags)

    def test_ping(self) -> None:
        result = _match_template("ping google.com")
        self.assertIsNotNone(result)
        assert result is not None
        self.assertEqual(result.action_template, "ping -c 4 {{host}}")
        self.assertEqual(result.parameters, {"host": "google.com"})

    def test_no_match_returns_none(self) -> None:
        result = _match_template("do something completely unexpected and weird")
        self.assertIsNone(result)


# ---------------------------------------------------------------------------
# Distiller — template fallback path
# ---------------------------------------------------------------------------

class TestDistillerTemplateFallback(unittest.TestCase):
    """Distiller without an API key should use template matching."""

    def setUp(self) -> None:
        self.config = InferConfig(api_key=None)
        self.distiller = Distiller(self.config)

    def test_compile_known_intent(self) -> None:
        result = self.distiller.compile_intent("list files in /home")
        self.assertEqual(result.action_template, "ls -la {{dir}}")
        self.assertEqual(result.parameters, {"dir": "/home"})

    def test_compile_unknown_intent_falls_back(self) -> None:
        result = self.distiller.compile_intent("deploy to production")
        # Should get the generic fallback template.
        self.assertIn("{{intent}}", result.action_template)
        self.assertEqual(result.confidence_hint, 0.1)
        self.assertIn("unknown", result.tags)

    def test_refine_without_api_key_returns_unchanged(self) -> None:
        original = "ls -la {{dir}}"
        result = self.distiller.refine_action(original, "show hidden files too")
        # Without API key, the action should be returned unchanged.
        self.assertEqual(result, original)

    def test_explain_without_api_key_returns_stub(self) -> None:
        result = self.distiller.explain("ls -la {{dir}}")
        self.assertIn("ls -la", result)


# ---------------------------------------------------------------------------
# Distiller — LLM path (mocked)
# ---------------------------------------------------------------------------

class TestDistillerLLMPath(unittest.TestCase):
    """Distiller with a mocked OpenAI client."""

    def setUp(self) -> None:
        self.config = InferConfig(api_key="test-key", model_name="gpt-4o-mini")
        self.distiller = Distiller(self.config)

    def test_compile_intent_via_llm(self) -> None:
        llm_response = json.dumps({
            "action_template": "git log --oneline -n {{count}}",
            "parameters": {"count": "10"},
            "confidence_hint": 0.85,
            "tags": ["git", "log"],
        })
        mock_resp = _mock_openai_response(llm_response)

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.compile_intent("show recent git commits")

        self.assertEqual(result.action_template, "git log --oneline -n {{count}}")
        self.assertEqual(result.parameters, {"count": "10"})
        self.assertAlmostEqual(result.confidence_hint, 0.85)
        self.assertIn("git", result.tags)

    def test_compile_intent_llm_returns_markdown_wrapped_json(self) -> None:
        """LLM sometimes wraps JSON in ```json ... ``` fences."""
        raw_json = json.dumps({
            "action_template": "docker build -t {{tag}} .",
            "parameters": {"tag": "myapp"},
            "confidence_hint": 0.75,
            "tags": ["docker", "build"],
        })
        llm_response = f"```json\n{raw_json}\n```"
        mock_resp = _mock_openai_response(llm_response)

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.compile_intent("build a docker image")

        self.assertEqual(result.action_template, "docker build -t {{tag}} .")

    def test_compile_intent_llm_returns_invalid_json_falls_back(self) -> None:
        """Invalid LLM output should fall back to template matching."""
        mock_resp = _mock_openai_response("This is not JSON at all!")

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            # Use an intent that matches a built-in template.
            result = self.distiller.compile_intent("git status")

        self.assertEqual(result.action_template, "git status")
        self.assertIn("git", result.tags)

    def test_compile_intent_llm_empty_content(self) -> None:
        """Empty LLM response should fall back to template matching."""
        mock_resp = _mock_openai_response(None)

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.compile_intent("git status")

        self.assertEqual(result.action_template, "git status")

    def test_refine_action(self) -> None:
        mock_resp = _mock_openai_response("ls -la {{dir}} --color=auto")

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.refine_action(
                "ls -la {{dir}}", "add color output"
            )

        self.assertEqual(result, "ls -la {{dir}} --color=auto")

    def test_explain(self) -> None:
        mock_resp = _mock_openai_response(
            "Lists all files in the specified directory with details."
        )

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.explain("ls -la {{dir}}")

        self.assertEqual(result, "Lists all files in the specified directory with details.")

    def test_refine_empty_llm_response_returns_original(self) -> None:
        mock_resp = _mock_openai_response(None)

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.refine_action("ls {{dir}}", "fix it")

        self.assertEqual(result, "ls {{dir}}")

    def test_explain_empty_llm_response_returns_stub(self) -> None:
        mock_resp = _mock_openai_response(None)

        with patch.object(self.distiller, "_client") as mock_client:
            mock_client.chat.completions.create.return_value = mock_resp
            result = self.distiller.explain("ls {{dir}}")

        self.assertIn("ls", result)


# ---------------------------------------------------------------------------
# DistillationResult data structure tests
# ---------------------------------------------------------------------------

class TestDistillationResult(unittest.TestCase):
    """Test the DistillationResult dataclass."""

    def test_default_values(self) -> None:
        result = DistillationResult(action_template="echo hello")
        self.assertEqual(result.action_template, "echo hello")
        self.assertEqual(result.parameters, {})
        self.assertEqual(result.confidence_hint, 0.5)
        self.assertEqual(result.tags, [])

    def test_frozen(self) -> None:
        result = DistillationResult(action_template="echo hello")
        with self.assertRaises(AttributeError):
            result.action_template = "echo world"  # type: ignore[misc]

    def test_full_construction(self) -> None:
        result = DistillationResult(
            action_template="rm {{path}}",
            parameters={"path": "/tmp/test"},
            confidence_hint=0.7,
            tags=["fs", "delete"],
        )
        self.assertEqual(result.parameters["path"], "/tmp/test")
        self.assertEqual(len(result.tags), 2)


if __name__ == "__main__":
    unittest.main()
