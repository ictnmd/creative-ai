# Calcium-D3-K2 THCS Morph Deck Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a 20-slide Vietnamese Morph-animated PPT for THCS students about Calcium, D3, and K2 with researched, safety-aware educational content and rich visuals.

**Architecture:** Use a content-first pipeline: (1) lock narrative and slide briefs, (2) produce visual assets, (3) generate the deck via OfficeCli + morph helpers, and (4) run strict structural validation plus manual polish pass. Keep scene actors stable across slides and ghost per-slide content to preserve clean Morph transitions.

**Tech Stack:** officecli (PPT generation), bash build script, morph-helpers.sh, image generation tool (if needed), local markdown specs/plans.

---

## File Structure (lock before implementation)

- Create: `brief.md`
  - Responsibility: single source of truth for story, slide objectives, key text, evidence notes.
- Create: `build-calcium-d3-k2-thcs.sh`
  - Responsibility: reproducible OfficeCli build pipeline for all 20 slides.
- Create: `assets/images/`
  - Responsibility: generated/curated visuals used by deck.
- Create: `assets/images/prompts.md`
  - Responsibility: prompt list + mapping image -> slide index.
- Create: `calcium-d3-k2-thcs-20slides.pptx`
  - Responsibility: final deck output.
- Create: `references/research-notes.md`
  - Responsibility: short evidence notes and links used for educational claims.
- Modify: `docs/superpowers/specs/2026-03-27-calcium-d3-k2-thcs-design.md` (only if safety wording gaps discovered during build)
  - Responsibility: alignment updates from implementation reality.

---

### Task 1: Prepare research-backed content baseline

**Files:**
- Create: `references/research-notes.md`
- Modify: `docs/superpowers/specs/2026-03-27-calcium-d3-k2-thcs-design.md` (if needed)
- Test: n/a (content verification via checklist)

- [ ] **Step 1: Draft evidence note skeleton**

```md
# Research Notes
## Role of nutrients
## Reference intakes (adolescents)
## Safety caveats
## Myth-vs-fact checks
## Source links
```

- [ ] **Step 2: Fill authoritative references for Ca/D3/K2 educational claims**

Run a focused research pass and record source title + URL + one-line relevance per source.
Expected: At least 6 reputable sources documented.

- [ ] **Step 3: Add age-appropriate safety wording snippets for slide reuse**

```md
- "Thông tin mang tính giáo dục, không thay thế tư vấn cá nhân hóa."
- "Không tự ý tăng liều, đặc biệt khi đang có bệnh nền hoặc dùng thuốc."
```

- [ ] **Step 4: Verify notes can support slides 5–16 without overclaims**

Checklist:
- No treatment claims
- No guaranteed height outcome claims
- No brand promotion language

- [ ] **Step 5: Commit**

```bash
git add references/research-notes.md docs/superpowers/specs/2026-03-27-calcium-d3-k2-thcs-design.md
git commit -m "docs: add research-backed safety notes for THCS nutrition deck"
```

---

### Task 2: Produce final `brief.md` from approved spec

**Files:**
- Create: `brief.md`
- Read: `docs/superpowers/specs/2026-03-27-calcium-d3-k2-thcs-design.md`
- Read: `references/research-notes.md`
- Test: n/a (outline completeness check)

- [ ] **Step 1: Write brief summary block**

```md
Topic: ...
Audience: THCS (11-14)
Purpose: Giáo dục sức khỏe học đường
Narrative: Hành trình 90 ngày
Style direction: truyền cảm hứng, học đường, trực quan cao
```

- [ ] **Step 2: Write 20-slide outline with page types**

Expected: S1..S20 all mapped with `hero/statement/pillars/evidence/transition/conclusion` where suitable.

- [ ] **Step 3: Write detailed page briefs for every slide**

Each slide must include: Objective, Core information, Evidence note, Page type, Hierarchy, Transition.

- [ ] **Step 4: Run completeness check on brief**

Checklist:
- 20/20 slides present
- Vietnamese text ready to paste into deck
- Safety notes present on relevant medical-content slides

- [ ] **Step 5: Commit**

```bash
git add brief.md
git commit -m "docs: finalize brief for 20-slide calcium-d3-k2 morph deck"
```

---

### Task 3: Generate and map image assets

**Files:**
- Create: `assets/images/prompts.md`
- Create: `assets/images/*` (PNG/JPG)
- Modify: `brief.md` (add image mapping if needed)
- Test: `officecli` picture path validation during build

- [ ] **Step 1: Write prompt manifest by slide**

```md
## S1
Prompt: "Vietnamese middle-school students running in school yard, positive sunlight..."
Output: assets/images/s01-cover.jpg
```

- [ ] **Step 2: Generate visual assets for key slides (at least 12/20)**

Expected: Clear educational style, no brand logo, no medical misinformation in text baked into images.

- [ ] **Step 3: Run asset quality gate**

Checklist:
- Resolution sufficient for full slide
- Consistent style palette
- No unreadable tiny text inside image

- [ ] **Step 4: Add fallback plan for missing images**

For each missing visual, define vector/shape-based backup in build script.

- [ ] **Step 5: Commit**

```bash
git add assets/images assets/images/prompts.md brief.md
git commit -m "assets: add generated visuals and slide mapping"
```

---

### Task 4: Build reproducible Morph script

**Files:**
- Create: `build-calcium-d3-k2-thcs.sh`
- Read: `.claude/skills/morph-ppt/reference/morph-helpers.sh`
- Output: `calcium-d3-k2-thcs-20slides.pptx`
- Test: script execution + helper verification

- [ ] **Step 1: Initialize script with OfficeCli version check and helper import**

```bash
#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/.claude/skills/morph-ppt/reference/morph-helpers.sh"
OUTPUT="calcium-d3-k2-thcs-20slides.pptx"
```

- [ ] **Step 2: Implement slide 1 baseline with scene actors**

Use naming:
- Scene actors: `'!!calcium'`, `'!!d3'`, `'!!k2'`, `'!!timeline'`, `'!!progress'`
- Content: `'#s1-*'`

- [ ] **Step 3: Implement slides 2–20 with clone/ghost/add pattern**

Pattern for each slide:
```bash
morph_clone_slide "$OUTPUT" <n-1> <n>
morph_ghost_content "$OUTPUT" <n> <prev_content_indices...>
# add #sN-* content
# move !! actors for motion
morph_verify_slide "$OUTPUT" <n>
```

- [ ] **Step 4: Add images and text blocks per `brief.md`**

Expected: all 20 slides contain final Vietnamese copy and mapped visuals.

- [ ] **Step 5: Add final structural checks in script**

```bash
officecli validate "$OUTPUT"
officecli view "$OUTPUT" outline
morph_final_check "$OUTPUT"
```

- [ ] **Step 6: Run script and confirm output file exists**

Run:
```bash
bash build-calcium-d3-k2-thcs.sh
```
Expected: build exits 0 and outputs `calcium-d3-k2-thcs-20slides.pptx`.

- [ ] **Step 7: Commit**

```bash
git add build-calcium-d3-k2-thcs.sh calcium-d3-k2-thcs-20slides.pptx
git commit -m "feat: generate 20-slide morph deck for calcium d3 k2 thcs"
```

---

### Task 5: Verify educational correctness and deck quality

**Files:**
- Modify: `build-calcium-d3-k2-thcs.sh` (if fixes needed)
- Modify: `brief.md` (if content simplification needed)
- Modify: `calcium-d3-k2-thcs-20slides.pptx` (rebuilt)
- Test: validation + manual review checklist

- [ ] **Step 1: Run full validation suite**

Run:
```bash
officecli validate calcium-d3-k2-thcs-20slides.pptx
officecli view calcium-d3-k2-thcs-20slides.pptx issues
```
Expected: no blocking structural issues.

- [ ] **Step 2: Run readability sweep (THCS level)**

Checklist:
- Average bullet length short
- No dense medical jargon
- One key message per slide

- [ ] **Step 3: Run safety language sweep**

Checklist:
- Slide with intake references includes caution text
- No “guaranteed height increase” statements
- Supplement content framed as optional + guided

- [ ] **Step 4: Run Morph continuity sweep**

Checklist:
- Slide 2..20 all morph-enabled
- No accidental unghosted `#sN-` content from prior slides
- Scene actor movement is smooth and purposeful

- [ ] **Step 5: Rebuild if any check fails**

Run:
```bash
bash build-calcium-d3-k2-thcs.sh
```
Expected: all checks pass after fixes.

- [ ] **Step 6: Commit**

```bash
git add build-calcium-d3-k2-thcs.sh brief.md calcium-d3-k2-thcs-20slides.pptx
git commit -m "fix: polish safety wording and morph flow for final deck"
```

---

### Task 6: Final delivery packaging

**Files:**
- Ensure exists: `calcium-d3-k2-thcs-20slides.pptx`
- Ensure exists: `build-calcium-d3-k2-thcs.sh`
- Ensure exists: `brief.md`
- Ensure exists: `references/research-notes.md`

- [ ] **Step 1: Confirm required output set**

Expected files:
- `calcium-d3-k2-thcs-20slides.pptx`
- `build-calcium-d3-k2-thcs.sh`
- `brief.md`
- `references/research-notes.md`

- [ ] **Step 2: Provide user preview instructions**

Required reminder before/while generation:
- PPT may be rewritten multiple times
- Preview directly in AionUi workspace
- Do not click “Open with system app” during generation

Required completion message:
- Deck is ready
- Open PPT now to preview motion effects

- [ ] **Step 3: Commit (if policy allows in execution session)**

```bash
git add calcium-d3-k2-thcs-20slides.pptx build-calcium-d3-k2-thcs.sh brief.md references/research-notes.md
git commit -m "chore: package final thcs calcium d3 k2 morph presentation"
```

---

## Global Verification Commands

Run after Task 6:

```bash
officecli validate calcium-d3-k2-thcs-20slides.pptx
officecli view calcium-d3-k2-thcs-20slides.pptx outline
```

Expected:
- 20 slides present
- Morph transition correctly set on slide 2..20
- No blocking validation errors

---

## Skills to invoke during execution

- `@superpowers:subagent-driven-development` (recommended orchestrator)
- `@superpowers:verification-before-completion` (before claiming done)
- `@superpowers:requesting-code-review` (optional final quality gate)
