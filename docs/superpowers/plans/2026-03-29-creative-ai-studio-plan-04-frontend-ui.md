# Plan 04: Frontend UI

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task.

> **IMPORTANT:** Before implementing components, use the `ui-ux-pro-max` skill to refine the color system and component designs from the spec. Specify the exact output: updated tailwind.config.js CSS variables and a component design guide.

> **⚠️ CRITICAL:** Every `// ...` stub comment must be fully implemented. Stubs are NOT final code.

> **Known issues from review:** (1) Create `(app)/+layout.svelte` parent layout with auth guard + sidebar. (2) All component stubs must be fully implemented. (3) Add chart.js to frontend package.json. (4) Coordinate `SubscriptionCard` and `CreditPurchaseModal` with Plan 05. (5) Public profile page must include FollowerList.

**Goal:** Build the complete frontend UI for Phase 1 — the creator workspace, gallery, profile, credits, and admin pages with all specified components.

**Prerequisites:** Plan 01 (scaffold) and Plan 02 (auth) should be completed first.

**Tech Stack:** SvelteKit, Tailwind CSS, CSS custom properties for design tokens.

---

## SECTION 1: Refine Design System with ui-ux-pro-max

### Task 1: Apply ui-ux-pro-max Design System

> **Sub-skill:** Use `ui-ux-pro-max` skill to finalize color tokens, typography, and motion system from the spec's initial palette.

**Files:**
- Modify: `frontend/src/app.css`
- Create: `frontend/tailwind.config.js` (finalized)

- [ ] **Step 1: Invoke ui-ux-pro-max skill**

```bash
# Use the skill to generate refined design tokens based on:
# - Dark minimal aesthetic
# - Accent gradient: #6366f1 -> #8b5cf6 -> #a855f7
# - All components from the spec's component inventory
```

- [ ] **Step 2: Update tailwind.config.js with refined design tokens**

- [ ] **Step 3: Commit**

```bash
git add frontend/src/app.css frontend/tailwind.config.js
git commit -m "feat(frontend): refine design system with ui-ux-pro-max"
```

---

## SECTION 2: Shared UI Components

### Task 2: Core Components

**Files:**
- Create: `frontend/src/lib/components/Sidebar.svelte`
- Create: `frontend/src/lib/components/Toast.svelte`
- Create: `frontend/src/lib/components/LoadingSkeleton.svelte`
- Create: `frontend/src/lib/components/EmptyState.svelte`
- Create: `frontend/src/lib/components/UserAvatar.svelte`
- Create: `frontend/src/lib/components/Button.svelte`
- Create: `frontend/src/lib/components/Badge.svelte`
- Create: `frontend/src/lib/components/Modal.svelte`
- Create: `frontend/src/lib/components/Input.svelte`
- Create: `frontend/src/lib/components/Dropdown.svelte`
- Create: `frontend/src/lib/components/TabGroup.svelte`
- Create: `frontend/src/lib/components/CreditBadge.svelte`
- Create: `frontend/src/lib/components/QuotaIndicator.svelte`

- [ ] **Step 1: Create base components**

Each component follows the design tokens from Section 1. Example: Button variants (`primary`, `secondary`, `ghost`, `danger`) with proper hover, focus, and disabled states.

- [ ] **Step 2: Create Toast system (already partially done in Plan 02)**

```svelte
<!-- Toast.svelte -->
<!-- Reads from toasts store, renders stacked toasts in bottom-right -->
<!-- Animations: slide-in from right, fade-out on dismiss -->
<!-- Types: success (green), error (red), warning (amber), info (blue), loading (purple) -->
```

- [ ] **Step 3: Create Sidebar navigation**

```svelte
<!-- Sidebar.svelte -->
<!-- Collapsible sidebar with nav links: Create, Gallery, Profile, Credits -->
<!-- Shows CreditBadge and UserAvatar at bottom -->
<!-- Hover states, active route highlighting -->
<!-- Collapse button to icon-only mode -->
```

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/components/
git commit -m "feat(frontend): add shared UI components library"
```

---

## SECTION 3: Creator Workspace (/app/create)

### Task 3: Main Creator Page

**Files:**
- Create: `frontend/src/routes/(app)/create/+page.svelte`
- Create: `frontend/src/lib/components/PromptInput.svelte`
- Create: `frontend/src/lib/components/EnhanceButton.svelte`
- Create: `frontend/src/lib/components/StylePresetGallery.svelte`
- Create: `frontend/src/lib/components/StylePresetCard.svelte`
- Create: `frontend/src/lib/components/ModelSelect.svelte`
- Create: `frontend/src/lib/components/AspectRatioSelect.svelte`
- Create: `frontend/src/lib/components/NumImagesSlider.svelte`
- Create: `frontend/src/lib/components/ReferencePanel.svelte`
- Create: `frontend/src/lib/components/SketchCanvas.svelte`
- Create: `frontend/src/lib/components/GenerateButton.svelte`
- Create: `frontend/src/lib/components/GenerationResults.svelte`

- [ ] **Step 1: Create PromptInput.svelte**

```svelte
<script lang="ts">
  export let value = '';
  export let placeholder = 'Type a prompt...';
  export let loading = false;

  let textarea: HTMLTextAreaElement;

  function autoResize() {
    if (textarea) {
      textarea.style.height = 'auto';
      textarea.style.height = Math.min(textarea.scrollHeight, 300) + 'px';
    }
  }
</script>

<div class="relative">
  <textarea
    bind:this={textarea}
    bind:value
    on:input={autoResize}
    {placeholder}
    class="w-full min-h-[120px] max-h-[300px] p-4 rounded-[var(--radius-lg)] bg-[var(--bg-card)] border border-[var(--border)] text-[var(--text-primary)] resize-none outline-none transition-all focus:border-[var(--border-active)] focus:shadow-[0_0_0_3px_var(--accent-glow)] font-[JetBrains_Mono] text-sm"
    disabled={loading}
  ></textarea>
  <div class="absolute bottom-3 right-3 text-xs text-[var(--text-muted)]">
    {value.length} chars
  </div>
</div>
```

- [ ] **Step 2: Create StylePresetGallery.svelte**

```svelte
<!-- Horizontal scrollable grid of StylePresetCard -->
<!-- + button to create custom preset (CustomPresetModal) -->
<!-- Selected preset highlighted with border -->
<!-- Shows usage count on hover -->
```

- [ ] **Step 3: Create SketchCanvas.svelte**

```svelte
<!-- HTML5 Canvas drawing tool -->
<!-- Black canvas on dark background -->
<!-- Drawing with mouse/touch -->
<!-- Brush size control -->
<!-- Clear button -->
<!-- Export as base64 data URL -->
<!-- Eraser mode -->
```

- [ ] **Step 4: Create EnhanceButton.svelte**

```svelte
<!-- Button with sparkle icon -->
<!-- States: idle (click to enhance), loading (pulse animation), success (checkmark) -->
<!-- Calls /api/v1/enhance endpoint -->
<!-- On success: replaces/updates prompt with enhanced version -->
```

- [ ] **Step 5: Create GenerateButton.svelte**

```svelte
<!-- Large gradient button -->
<!-- States: idle (enabled), disabled (quota/credits exhausted), loading (gradient animation + spinner) -->
<!-- On click: POST /api/v1/generations -->
<!-- Shows real-time status as generation progresses -->
```

- [ ] **Step 6: Create GenerationResults.svelte**

```svelte
<!-- Grid of generated images -->
<!-- Shows loading skeleton cards while pending/processing -->
<!-- On completion: display image grid -->
<!-- Click image to open in lightbox or detail panel -->
<!-- Download button per image -->
<!-- Remix button (re-generate with same prompt) -->
```

- [ ] **Step 7: Create main create/+page.svelte**

```svelte
<!-- Two-column layout -->
<!-- Left: TabGroup (Basic | Advanced | Models) -->
<!--   - Basic: Style presets, reference panel -->
<!--   - Advanced: Aspect ratio, quality, steps -->
<!--   - Models: Provider/model selection -->
<!-- Right: PromptInput + action bar + results -->
<!-- WebSocket connection for real-time generation status -->
```

- [ ] **Step 8: Commit**

```bash
git add frontend/src/routes/\(app\)/create/
git commit -m "feat(frontend): build creator workspace page with all generation components"
```

---

## SECTION 4: Gallery Page

### Task 4: Gallery with Detail Panel

**Files:**
- Create: `frontend/src/routes/(app)/gallery/+page.svelte`
- Create: `frontend/src/lib/components/FilterBar.svelte`
- Create: `frontend/src/lib/components/GenerationCard.svelte`
- Create: `frontend/src/lib/components/DetailPanel.svelte`
- Create: `frontend/src/lib/components/Pagination.svelte`
- Create: `frontend/src/lib/components/ShareModal.svelte`
- Create: `frontend/src/lib/components/DeleteConfirmationModal.svelte`

- [ ] **Step 1: Create GenerationCard.svelte**

```svelte
<!-- Thumbnail card for gallery grid -->
<!-- Shows image preview -->
<!-- Hover: shows quick actions overlay (download, remix, share) -->
<!-- Selected state: highlighted border -->
<!-- Status indicator for pending/processing (pulsing overlay) -->
```

- [ ] **Step 2: Create FilterBar.svelte**

```svelte
<!-- Filter dropdowns: Provider, Status -->
<!-- Date range picker -->
<!-- Sort options: Recent, Oldest, Most viewed -->
<!-- Clear filters button -->
```

- [ ] **Step 3: Create DetailPanel.svelte**

```svelte
<!-- Slide-in panel from right (400px wide) -->
<!-- Large image preview -->
<!-- Metadata: prompt, model, style, provider, timestamp -->
<!-- Actions: Download, Remix, Share, Delete -->
<!-- Visibility toggle: Private / Shared / Public -->
```

- [ ] **Step 4: Create main gallery/+page.svelte**

```svelte
<!-- Full-width grid layout -->
<!-- FilterBar at top -->
<!-- Grid of GenerationCard -->
<!-- Pagination at bottom -->
<!-- Click card: open DetailPanel -->
```

- [ ] **Step 5: Commit**

```bash
git add frontend/src/routes/\(app\)/gallery/
git commit -m "feat(frontend): build gallery page with filters, grid, and detail panel"
```

---

## SECTION 5: Credits & Billing Pages

### Task 5: Credits and Subscription Pages

**Files:**
- Create: `frontend/src/routes/(app)/credits/+page.svelte`
- Create: `frontend/src/lib/components/CreditPurchaseModal.svelte`
- Create: `frontend/src/lib/components/SubscriptionCard.svelte`
- Create: `frontend/src/lib/components/TransactionTable.svelte`
- Create: `frontend/src/lib/components/QuotaIndicator.svelte`
- Create: `frontend/src/lib/components/SubscriptionManagementModal.svelte`

- [ ] **Step 1: Create credits page**

```svelte
<!-- Current credit balance display (large number) -->
<!-- Quota indicator (progress bar for monthly usage) -->
<!-- Credit packs grid (choose pack, proceed to Stripe checkout) -->
<!-- Transaction history table -->
<!-- Auto-top-up toggle -->
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/routes/\(app\)/credits/
git commit -m "feat(frontend): build credits page with purchase flow and history"
```

---

## SECTION 6: Admin Dashboard

### Task 6: Admin Pages

**Files:**
- Create: `frontend/src/routes/(app)/admin/+page.svelte`
- Create: `frontend/src/routes/(app)/admin/users/+page.svelte`
- Create: `frontend/src/routes/(app)/admin/api-keys/+page.svelte`
- Create: `frontend/src/routes/(app)/admin/moderation/+page.svelte`
- Create: `frontend/src/routes/(app)/admin/analytics/+page.svelte`
- Create: `frontend/src/lib/components/AdminUserTable.svelte`
- Create: `frontend/src/lib/components/AdminAPIKeyTable.svelte`
- Create: `frontend/src/lib/components/AdminContentModerationTable.svelte`
- Create: `frontend/src/lib/components/AnalyticsDashboard.svelte`

- [ ] **Step 1: Create admin layout**

```svelte
<!-- Protected route (admin role only) -->
<!-- Sidebar navigation for admin sections -->
<!-- Role check with redirect if not admin -->
```

- [ ] **Step 2: Create AdminUserTable.svelte**

```svelte
<!-- Paginated table: username, email, tier, join date, status -->
<!-- Search by username/email -->
<!-- Actions: Edit quota, Suspend, Delete -->
```

- [ ] **Step 3: Create AnalyticsDashboard.svelte**

```svelte
<!-- Charts using a lightweight chart library (Chart.js or similar) -->
<!-- Stats: Total users, total generations, provider usage breakdown -->
<!-- Line chart: generations over time -->
<!-- Bar chart: usage by provider -->
```

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/\(app\)/admin/
git commit -m "feat(frontend): build admin dashboard with user management and analytics"
```

---

## SECTION 7: Public Pages

### Task 7: Public Profile & Shared Generation

**Files:**
- Create: `frontend/src/routes/public/[username]/+page.svelte`
- Create: `frontend/src/routes/shared/[token]/+page.svelte`
- Create: `frontend/src/routes/+layout.svelte` (landing page)

- [ ] **Step 1: Create landing page layout**

```svelte
<!-- Landing page for unauthenticated users -->
<!-- Hero section with product name + tagline -->
<!-- Feature highlights -->
<!-- Call to action: Sign up -->
<!-- Footer with links -->
```

- [ ] **Step 2: Create public profile page**

```svelte
<!-- Public profile: avatar, name, bio -->
<!-- Stats: total generations, followers -->
<!-- Follow button (if logged in) -->
<!-- Showcase featured images (grid) -->
<!-- Full gallery grid -->
```

- [ ] **Step 3: Create shared generation page**

```svelte
<!-- View shared generation without login -->
<!-- Large image display -->
<!-- Prompt text -->
<!-- Creator attribution -->
<!-- Link to sign up -->
```

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/public/ frontend/src/routes/shared/ frontend/src/routes/+layout.svelte
git commit -m "feat(frontend): build public profile, shared view, and landing page"
```
