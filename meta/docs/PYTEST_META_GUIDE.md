# Pytest meta guide

This template-only guide covers tests that validate the starter template. Use
[docs/PYTEST_STYLE.md](../../docs/PYTEST_STYLE.md) to decide whether a test belongs in the suite,
then use [docs/PYTEST_AUTHORING_GUIDE.md](../../docs/PYTEST_AUTHORING_GUIDE.md) to construct it.

Prefer fewer, stronger permanent meta tests that protect stable template behavior. Put one-time
template checks in the ignored `tests/_temp/` subtree, then promote or remove them before plan
completion. When in doubt, remove the test.

## Meta test checklist

- [ ] The behavior is intentionally stable and specific to starter-template maintenance.
- [ ] The test exercises behavior rather than duplicating a manifest or file inventory.
- [ ] The source belongs to `tests/meta/` rather than a consumer-shipped test location.
- [ ] The test provides lasting value beyond a temporary implementation check.
- [ ] A new blocking gate includes an actionable failure plan.

## Meta test coverage

`tests/meta/test_*.py` covers the starter template itself: propagation plans, reset behavior,
manifests, and shared test infrastructure. `tests/meta/e2e/` holds real template workflows for the
explicit E2E lane. The `tests/meta/` tree stays with the starter template.

## Related vendored tests

The root `tests/` tree supplies universal test files to consumer repositories. A test under
`templates/<type>/tests/` ships with that repository type. The propagation plan routes each source
by its folder.

`tests/conftest.py` preserves consumer-owned pytest configuration through the propagation merge.
Use its `REPO_HYGIENE_FILTERS` registry for consumer-specific hygiene patterns; keep template tests
focused on universal behavior.

## Vendored header

Start every shipped template pytest module with this exact comment:

```python
# This file is vendored. Local changes can and will be overwritten by propagation.
```

The header makes overwrite ownership visible before an editor reaches the implementation. Put it
after a required shebang in any shipped executable source. Template-meta, noexist, and
consumer-local tests retain their own ownership conventions.
