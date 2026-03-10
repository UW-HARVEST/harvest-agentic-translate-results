# harvest-agentic-translate-results

Results from agentic C-to-Rust translations for the [DARPA TRACTOR](https://www.darpa.mil/program/translating-all-c-to-rust) program.

## Approach

Uses [kiro-cli](https://github.com/aws/kiro-cli) as an agentic translator instead of single-shot LLM calls. The agent can read, reason, build, and iteratively fix Rust translations of C code.

The translation harness lives in [UW-HARVEST/harvest](https://github.com/UW-HARVEST/harvest) (`scripts/kiro-translate.sh`).

## Structure

```
<battery>_<model>/
├── results.csv          # build/test outcomes per test case
├── <test_case>/
│   └── test_case/       # translated Rust project (Cargo.toml + src/)
└── ...
```

## Validating results

```bash
cd ~/Git/Test-Corpus
PYTHONPATH=deployment/scripts/github-actions:$PYTHONPATH \
  python3 -m runtests.rust \
  --root <results_dir> \
  --subset <results_dir> \
  --keep-going
```
