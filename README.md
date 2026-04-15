# harvest-agentic-translate-results

Translation results for [harvest-agentic](https://github.com/UW-HARVEST/harvest-agentic). Each directory contains the output Rust translation, input C source, logs, and test results for a given agent and test case.

## Directory Layout

```
results/
├── Test-Corpus/            # MIT TRACTOR Test-Corpus results
│   ├── <agent>/
│   │   ├── <battery>/
│   │   │   ├── summary.json            # Battery-level aggregate (CI-validated)
│   │   │   └── <case>/                 # Per-case results
│   │   │       ├── result.json          # Test outcome + metrics
│   │   │       ├── translation.json     # Translation metadata
│   │   │       ├── translated_rust/     # Final Rust output (post-verify)
│   │   │       ├── translated_rust_original/  # Pre-verify snapshot
│   │   │       └── logs/               # Agent logs
│   │   └── ...
│   └── ...
├── CRUST/                  # CRUST-bench results (cargo test)
│   └── <agent>/
│       └── <project>/
│           ├── result.json
│           ├── src/                     # Translated Rust source
│           ├── c_src/                   # Original C source (copied in)
│           └── logs/
└── CRUST-blind/            # CRUST-bench blind mode (no ground-truth tests)
    └── <agent>/
        └── <project>/
            ├── translate/               # Agent's translation
            ├── verify/                  # Agent-generated tests
            └── result.json
```

## Agents

| Agent | Directory name | Type |
|-------|---------------|------|
| kiro | `kiro/` | Agentic (kiro-cli, multi-turn) |
| kiro-translate | `kiro-translate/` | Agentic (translate-only, no verify loop) |
| claude | `claude/` | Agentic (Claude Code) |
| c2rust | `c2rust/` | Mechanical transpiler |
| laertes | `laertes/` | c2rust + Laertes rule-based refactoring |
| Kimi K2.5 | `kimi/` | One-shot LLM (Bedrock) |
| GPT-5.4 | `gpt-5.4/` | One-shot LLM (OpenRouter) |
| Gemini 3.1 Pro | `gemini-3.1-pro-preview/` | One-shot LLM (OpenRouter) |

## Where to Find C Source

The input C source is in different locations depending on the dataset:

**Test-Corpus (B01, B02, P00, P01):**
- Original C: `test-corpus/Public-Tests/<battery>/<case>/project/` (in the [test-corpus submodule](https://github.com/UW-HARVEST/test-corpus))
- Copied into results: `translated_rust/c_src/src/` (`.c` files) and `translated_rust/c_src/include/` (`.h` files)

**CRUST-bench:**
- Original C: `crust-bench/datasets/CBench/<project>/` or `crust-bench/datasets/RBench/<project>/`
- Copied into results: `<project>/c_src/`

## Per-Case Files

### result.json

Test outcome and code metrics for each case:

```json
{
  "battery": "B01_organic",
  "case": "bin2hex_lib",
  "passed": true,
  "vectors_failed": 0,
  "loc": { "code": 38 },
  "unsafe": { "blocks": 4, "fns": 1, "impls": 0, "lines": 40 },
  "translate": { "credits": 1.41, "wall_secs": 59 },
  "verify": { "credits": 3.59, "wall_secs": 132 }
}
```

| Field | Description |
|-------|-------------|
| `passed` | All test vectors pass |
| `vectors_failed` | Number of failing test vectors |
| `loc.code` | Lines of Rust code (excluding blanks/comments) |
| `unsafe.lines` | Lines inside `unsafe` blocks |
| `unsafe.blocks` | Number of `unsafe {}` blocks |
| `unsafe.fns` | Number of `unsafe fn` declarations |
| `unsafe.impls` | Number of `unsafe impl` blocks |
| `translate.credits` | kiro-cli credits consumed for translation |
| `translate.wall_secs` | Wall-clock translation time |
| `verify.credits` | kiro-cli credits consumed for C-as-oracle verification |
| `verify.wall_secs` | Wall-clock verification time |

For one-shot agents (kimi, gpt-5.4, gemini), `translate`/`verify` are absent. Token usage is in `logs/usage.json`.

### translation.json

```json
{
  "agent": "kiro",
  "duration_secs": 62,
  "success": true,
  "timestamp": "2026-04-06T21:52:22Z"
}
```

### summary.json (per-battery)

Aggregate results validated by CI:

```json
{
  "cases_passed": 38,
  "cases_tested": 38,
  "vectors_passed": 775,
  "vectors_failed": 0,
  "failed_cases": []
}
```

### logs/

| File | Present for | Description |
|------|-------------|-------------|
| `translation.log` | All agents | Human-readable translation summary |
| `verify.log` | kiro | C-as-oracle verification output |
| `translation.request.json` | One-shot LLMs | Full API request body |
| `translation.response.json` | One-shot LLMs | Full API response (content + tokens) |
| `usage.json` | One-shot LLMs | `{ model, input_tokens, output_tokens }` |

### translated_rust/ vs translated_rust_original/

- `translated_rust_original/` — the agent's initial translation output
- `translated_rust/` — final version after the verify/repair loop (kiro only; identical to original for other agents)

Both contain:
- `src/lib.rs` or `src/main.rs` — the Rust translation
- `c_src/` — copy of the input C source
- `Cargo.toml` — build configuration
