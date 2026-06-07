# 🦀 pincher — TEMPLATES

Boilerplate for common pincher agent configurations.

| Template | Description | Use Case |
|----------|-------------|----------|
| `agent-bundle/` | Full agent identity + reflex pack | Portable agent setup |
| `custom-reflex/` | Pre-defined reflex set for a domain | Domain-specific agents |
| `security-profile/` | Capability token + sandbox config | Restricted agents |

### Using a Template

```bash
cp -r TEMPLATES/agent-bundle/ my-agent
cd my-agent
pincher unpack agent.nail
pincher do "hello"
```
