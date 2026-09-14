## Description
<!-- Provide a brief description of the changes made in this PR -->

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Code hygiene / refactoring

## Target Branch
- [ ] Target is `dev` (Active development branch)
- [ ] Target is `main` (Only for official release PRs)

## Quality Checklist
- [ ] Code passes `cargo fmt --all -- --check`
- [ ] Code passes `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] Tests pass via `cargo test --workspace`
- [ ] `cargo deny check` passes with no new license or vulnerability violations
- [ ] Hardware safety invariants preserved (no speculative writes to flash, safe pacing maintained)
