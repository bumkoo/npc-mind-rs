# NPC Mind Engine — Technical Debt Audit

**Date:** 2026-04-01
**Scope:** Full codebase (`src/`, `tests/`, `mcp/`, `docs/`, config files)
**Codebase size:** ~8,500 lines Rust (src) + 5,450 lines tests + 380 lines Python (MCP)

---

## Executive Summary

The npc-mind-rs codebase is **well-architected** with clean hexagonal design, rich domain models, and comprehensive test coverage. Technical debt is **low to moderate** — concentrated in infrastructure gaps (no CI/CD), code duplication in the presentation/adapter layers, and missing rustdoc on critical public APIs.

**Overall Health Score: 7.5 / 10**

| Category | Debt Level | Items Found |
|----------|-----------|-------------|
| Code | Low–Medium | 8 items |
| Architecture | Low | 4 items |
| Test | Low | 3 items |
| Dependency | Minimal | 1 item |
| Documentation | Medium | 5 items |
| Infrastructure | High | 3 items |

---

## Prioritized Debt Items

### Scoring Formula

> **Priority = (Impact + Risk) × (6 − Effort)**
>
> - Impact: How much does it slow development? (1–5)
> - Risk: What happens if we don't fix it? (1–5)
> - Effort: How hard is the fix? (1 = trivial, 5 = major rewrite)

---

### Tier 1 — Critical (Score ≥ 30)

#### INFRA-1: No CI/CD Pipeline
| Metric | Value |
|--------|-------|
| Impact | 4 |
| Risk | 5 |
| Effort | 2 |
| **Score** | **36** |

No `.github/workflows/` or any CI configuration exists. All testing is manual. A single `cargo test` + `cargo clippy` + `cargo fmt --check` pipeline would catch regressions and enforce style.

**Business justification:** As a solo developer relying on AI agents, automated gates prevent shipping broken code when iterating quickly.

**Fix:** Create `.github/workflows/ci.yml` with matrix build (default features + embed feature).

---

#### INFRA-2: No Linting Configuration (rustfmt / clippy)
| Metric | Value |
|--------|-------|
| Impact | 3 |
| Risk | 4 |
| Effort | 1 |
| **Score** | **35** |

No `rustfmt.toml` or `clippy.toml`. Code style is implicit. AI-generated code may diverge from existing conventions without enforcement.

**Fix:** Add `rustfmt.toml` (tab width, import grouping) and `clippy.toml` (cognitive complexity threshold). Pair with CI.

---

#### CODE-1: Locale Translation Method Duplication (10 identical methods)
| Metric | Value |
|--------|-------|
| Impact | 3 |
| Risk | 4 |
| Effort | 1 |
| **Score** | **35** |

`src/presentation/locale.rs` lines 275–344 contain 10 methods that repeat the exact same pattern: `self.map.get(key).map(|s| s.as_str()).unwrap_or(key)`. Adding new translatable enums requires copying the same method each time.

**Fix:** Extract a generic `fn lookup<T: HasVariantName>(&self, section: &str, obj: &T) -> &str` helper.

---

### Tier 2 — High (Score 20–29)

#### CODE-2: CRUD Handler Boilerplate in Mind Studio
| Metric | Value |
|--------|-------|
| Impact | 3 |
| Risk | 3 |
| Effort | 2 |
| **Score** | **24** |

`src/bin/mind-studio/handlers.rs` has three nearly identical CRUD handler sets (NPC, Relationship, Object) — each with `list_*`, `upsert_*`, `delete_*` methods following the same pattern.

**Fix:** Create a generic CRUD handler macro or trait-based factory.

---

#### CODE-3: JSON / TOML Anchor Source Duplication
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 3 |
| Effort | 2 |
| **Score** | **20** |

`src/adapter/json_anchor_source.rs` and `src/adapter/toml_anchor_source.rs` have ~95% identical deserialization structs (`AnchorJson`/`AnchorToml`, `AxisAnchorsJson`/`AxisAnchorsToml`, `AnchorMeta`).

**Fix:** Create shared intermediate struct and use format-specific parsing only at the boundary.

---

#### DOC-1: Missing Rustdoc on MindService Public Methods
| Metric | Value |
|--------|-------|
| Impact | 4 |
| Risk | 3 |
| Effort | 2 |
| **Score** | **28** |

`MindService` is the library's primary entry point (documented in CLAUDE.md), yet its public methods (`appraise()`, `apply_stimulus()`, `start_scene()`, `generate_guide()`, `after_dialogue()`, `after_beat()`) lack `///` rustdoc comments. This forces users to read integration-guide.md instead of using IDE tooltips.

**Fix:** Add `///` doc comments with examples to all 8 public methods on `MindService` and `FormattedMindService`.

---

#### DOC-2: DTOs Lack Field Documentation
| Metric | Value |
|--------|-------|
| Impact | 3 |
| Risk | 3 |
| Effort | 2 |
| **Score** | **24** |

`src/application/dto.rs` (468 lines) defines all API request/response types used by external consumers but has no field-level documentation. Users must guess what `significance`, `power_gap`, or `prospect_status` mean.

**Fix:** Add `///` comments to all public DTO struct fields.

---

#### INFRA-3: Hardcoded Server Address
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 4 |
| Effort | 1 |
| **Score** | **30** |

`src/bin/mind-studio/main.rs` hardcodes `"127.0.0.1:3000"`. Cannot configure port via environment variable or CLI arg.

**Fix:** Read from `MIND_STUDIO_PORT` env var with fallback to 3000.

---

### Tier 3 — Medium (Score 12–19)

#### ARCH-1: MindService Approaching "Fat Service" (393 lines)
| Metric | Value |
|--------|-------|
| Impact | 3 |
| Risk | 2 |
| Effort | 3 |
| **Score** | **15** |

`MindService` handles appraisal orchestration, beat transitions, emotion merging, relationship updates, and scene management. At 393 lines it's manageable now, but adding more features (e.g., multi-NPC scenes, memory system) will push it past maintainability limits.

**Fix (when adding next major feature):** Extract `BeatTransitionService` and `RelationshipUpdater` as separate application services.

---

#### ARCH-2: DTO Conversion Reaches Into Repository
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 3 |
| Effort | 3 |
| **Score** | **15** |

`SituationInput::to_domain()` in `dto.rs` accepts a `MindRepository` reference to look up relationships. This creates a dependency from the DTO layer to the port layer.

**Fix:** Move conversion logic into `MindService` methods, keeping DTOs as pure data containers.

---

#### CODE-4: Unwrap in Production Paths
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 3 |
| Effort | 1 |
| **Score** | **15** |

8 instances of `.unwrap()` / `.expect()` in non-test code, including server startup (`main.rs:89-90`), mutex locks (`trace_collector.rs:27,73`), and TOML parsing (`korean.rs:19,21`).

**Fix:** Replace with proper error propagation (`?` operator) or `.expect()` with descriptive messages.

---

#### CODE-5: `build_situation_map()` Repetitive Inserts
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 2 |
| Effort | 1 |
| **Score** | **16** |

`handlers.rs` function `build_situation_map()` has 40+ lines of identical `sit.insert("key", value)` calls.

**Fix:** Use `serde_json::to_value()` on a struct instead of manual map building.

---

#### TEST-1: Mind Studio Handlers Lack Dedicated Tests
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 3 |
| Effort | 3 |
| **Score** | **15** |

The 659-line `handlers.rs` web layer has no dedicated HTTP-level tests. Integration is tested indirectly through `MindService` tests, but handler-specific logic (JSON parsing, error mapping, CORS) is untested.

**Fix:** Add axum test utilities (`axum::test::TestClient`) for key endpoints.

---

#### CODE-6: `#[allow(dead_code)]` on 11 Struct Fields
| Metric | Value |
|--------|-------|
| Impact | 1 |
| Risk | 2 |
| Effort | 1 |
| **Score** | **15** |

11 fields across adapter serde structs suppress dead_code warnings. All are legitimate (serde schema compatibility), but should be documented.

**Fix:** Add inline comments explaining why each suppression exists. Consider `#[serde(skip)]` where fields are truly unused.

---

### Tier 4 — Low (Score < 12)

#### ARCH-3: FormattedMindService Method Signature Duplication
| Metric | Value |
|--------|-------|
| Impact | 2 |
| Risk | 1 |
| Effort | 3 |
| **Score** | **9** |

`FormattedMindService` wraps `MindService` but duplicates method signatures with only formatting added.

**Fix (future):** Consider an extension trait pattern or decorator approach.

---

#### DOC-3: CLAUDE.md Minor Doc File Mismatch
| Metric | Value |
|--------|-------|
| Impact | 1 |
| Risk | 1 |
| Effort | 1 |
| **Score** | **10** |

CLAUDE.md references `pad-anchor-score-matrix.md` but actual file may differ in name.

**Fix:** Verify and update the reference.

---

#### TEST-2: Scene Focus Trigger Needs Parametrized Tests
| Metric | Value |
|--------|-------|
| Impact | 1 |
| Risk | 2 |
| Effort | 2 |
| **Score** | **12** |

`scene_test.rs` (98 lines) is the smallest test file. Focus trigger matching with complex OR/AND condition trees could benefit from more edge-case coverage.

**Fix:** Add parametrized tests for nested condition combinations.

---

#### TEST-3: No Benchmark Tests for Core Appraisal Performance
| Metric | Value |
|--------|-------|
| Impact | 1 |
| Risk | 2 |
| Effort | 2 |
| **Score** | **12** |

PAD benchmarks exist (embed feature), but core appraisal engine has no performance benchmarks.

**Fix:** Add `criterion` benchmarks for `appraise()` and `apply_stimulus()` hot paths.

---

#### DEP-1: Git Dependency for bge-m3-onnx-rust
| Metric | Value |
|--------|-------|
| Impact | 1 |
| Risk | 2 |
| Effort | 2 |
| **Score** | **12** |

`bge-m3-onnx-rust` is sourced from a personal GitHub repo. If the repo goes private or is deleted, builds break.

**Fix:** Pin to a specific commit hash (`rev = "abc123"`) or publish to crates.io when stable.

---

## Strengths (No Action Needed)

These are areas where the codebase excels:

- **Hexagonal architecture compliance** — Domain never imports from adapters. Ports are well-defined and segregated.
- **Rich domain models** — Not anemic. `Npc`, `EmotionState`, `Scene`, `Relationship` all encapsulate proper business logic.
- **Centralized tuning** — All 25+ magic numbers live in `tuning.rs` with clear documentation.
- **Zero `unsafe` code** — Pure safe Rust throughout.
- **Comprehensive test suite** — 5,450 lines across 14 test files, no ignored or flaky tests.
- **Clean feature flag design** — `embed` and `mind-studio` are properly optional.
- **Minimal dependencies** — Only 6 production dependencies, all well-chosen and current.
- **Locale consistency** — `ko.toml` and `en.toml` are perfectly aligned (22 emotions, 17 tones).
- **Good encapsulation** — Private fields with public getters throughout domain layer.
- **No TODO/FIXME/HACK markers** — Codebase is clean of deferred work.

---

## Phased Remediation Plan

### Phase 1 — Quick Wins (1–2 days, alongside feature work)

| Item | Effort | Action |
|------|--------|--------|
| INFRA-2 | 30 min | Create `rustfmt.toml` + `clippy.toml`, run `cargo fmt` |
| INFRA-3 | 30 min | Add env var for server port |
| CODE-1 | 1 hr | Extract generic locale lookup helper |
| CODE-4 | 1 hr | Replace 8 unwraps with proper error handling |
| CODE-6 | 30 min | Add comments to `#[allow(dead_code)]` fields |
| DOC-3 | 15 min | Fix CLAUDE.md doc reference |

### Phase 2 — Foundation (3–5 days, dedicated sprint)

| Item | Effort | Action |
|------|--------|--------|
| INFRA-1 | 2 hrs | Set up GitHub Actions CI (test + clippy + fmt) |
| DOC-1 | 3 hrs | Add rustdoc to all MindService/FormattedMindService public methods |
| DOC-2 | 2 hrs | Add field-level docs to all DTOs |
| CODE-5 | 1 hr | Replace `build_situation_map()` with serde struct |
| DEP-1 | 30 min | Pin git dependency to specific commit |

### Phase 3 — Structural Improvements (next feature addition)

| Item | Effort | Action |
|------|--------|--------|
| CODE-2 | 3 hrs | Create generic CRUD handler macro for Mind Studio |
| CODE-3 | 2 hrs | Unify JSON/TOML anchor source structs |
| ARCH-2 | 2 hrs | Move DTO→domain conversion into service layer |
| TEST-1 | 4 hrs | Add axum test client tests for handler layer |

### Phase 4 — Strategic Refactoring (when complexity demands)

| Item | Effort | Action |
|------|--------|--------|
| ARCH-1 | 1 day | Extract BeatTransitionService from MindService |
| ARCH-3 | 0.5 day | Refactor FormattedMindService to extension trait |
| TEST-2 | 2 hrs | Parametrized scene trigger tests |
| TEST-3 | 2 hrs | Add criterion benchmarks for core appraisal |

---

## Priority Ranking (All Items)

| Rank | ID | Category | Score | Phase |
|------|----|----------|-------|-------|
| 1 | INFRA-1 | Infrastructure | 36 | 2 |
| 2 | INFRA-2 | Infrastructure | 35 | 1 |
| 3 | CODE-1 | Code | 35 | 1 |
| 4 | INFRA-3 | Infrastructure | 30 | 1 |
| 5 | DOC-1 | Documentation | 28 | 2 |
| 6 | CODE-2 | Code | 24 | 3 |
| 7 | DOC-2 | Documentation | 24 | 2 |
| 8 | CODE-3 | Code | 20 | 3 |
| 9 | CODE-5 | Code | 16 | 2 |
| 10 | ARCH-1 | Architecture | 15 | 4 |
| 11 | ARCH-2 | Architecture | 15 | 3 |
| 12 | CODE-4 | Code | 15 | 1 |
| 13 | CODE-6 | Code | 15 | 1 |
| 14 | TEST-1 | Test | 15 | 3 |
| 15 | TEST-2 | Test | 12 | 4 |
| 16 | TEST-3 | Test | 12 | 4 |
| 17 | DEP-1 | Dependency | 12 | 2 |
| 18 | DOC-3 | Documentation | 10 | 1 |
| 19 | ARCH-3 | Architecture | 9 | 4 |
