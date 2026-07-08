# Command reference

All commands run through the guard OS. Run `bebop help` for the live list.

| Command | What it does |
| --- | --- |
| `bebop boot` | Self-test the guard OS (red-line + scope + certify). The entry point. |
| `bebop` | Run the interactive agent loop (uses your configured backend). |
| `bebop init [--preset bebop\|--json {...}]` | Write a profile (origin, class, narration, patrons, looks, backend rotation). |
| `bebop status` | Show guard-OS status, granted scope, backend rotation, availability. |
| `bebop run [doer\|reason\|redline]` | Run the agentic loop at a task class. |
| `bebop dispatch "<task>"` | Run a task through the copilot (doer + distinct checker) and the telemetry governor. |
| `bebop route <class>` | Classify a task and show the routing decision (cheapest adequate backend). |
| `bebop recall <query>` | Associative recall from living memory. |
| `bebop remember <concept> :: <payload>` | Write a concept into living memory. |
| `bebop memory [tick\|layers]` | Inspect / advance the living-memory forgetting clock. |
| `bebop store <dir> [append\|put\|verify]` | Exercise the content-addressed, hash-chained store. |
| `bebop node [--path P --pass P]` | Show this node's post-quantum self-certifying identity. |
| `bebop govern "<samples>"` | Run the telemetry governor on a quality stream (0..1). |
| `bebop self [maintain\|evolve\|session\|loop]` | Bebop soul: self-maintenance / evolution / session-as-node. |
| `bebop sync [--port N]` | Start the optional self-hosted Better Auth sync node. |
| `bebop mcp` | Start the MCP stdio server (see [integrations/mcp](./integrations/mcp.md)). |

## Examples

```bash
# Certify the guard OS before trusting autonomy
bebop boot

# Watch the governor refuse an under-damped loop
bebop govern "0.9,0.95,0.9,0.2,0.1,0.95,0.9"

# Record this session as a living-memory node (freestyle bebop soul)
bebop self session hermes-now "active hermes session node"

# Plug Bebop into an MCP client
bebop mcp
```
