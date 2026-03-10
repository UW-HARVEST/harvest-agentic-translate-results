# harvest-agentic-translate-results

Results from agentic C-to-Rust translations for the [DARPA TRACTOR](https://www.darpa.mil/program/translating-all-c-to-rust) program.

## Approach

Uses [kiro-cli](https://github.com/aws/kiro-cli) as an agentic translator instead of single-shot LLM calls. The agent can read, reason, build, and iteratively fix Rust translations of C code.

The translation harness lives in [UW-HARVEST/harvest](https://github.com/UW-HARVEST/harvest) (`scripts/kiro-translate.sh`).

## Structure

```
<battery>_<model>/
├── progress.csv                 # translation progress tracker
├── <test_case>/
│   ├── translated_rust/         # translated Rust project (Cargo.toml + src/)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── c_src/               # original C source (for reference)
│   └── test_vectors/            # copied from Test-Corpus
└── logs_<timestamp>/
    └── <test_case>.log          # full kiro-cli session log
```

Each run stores both the translated code and the complete kiro-cli logs, so we can analyze the agent's reasoning and debug failures.

## Validating results

```bash
cd ~/Git/Test-Corpus
PYTHONPATH=deployment/scripts/github-actions:$PYTHONPATH \
  python3 -m runtests.rust \
  --root <results_dir>/<battery> \
  --subset <results_dir>/<battery> \
  --keep-going
```
