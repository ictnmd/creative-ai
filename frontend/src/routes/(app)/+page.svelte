<script lang="ts">
	import { ImageCard, Modal, Button } from '$lib/components/ui';
	import type { GenerationStatus } from '$lib/types';

	// ---------------------------------------------------------------------------
	// Types
	// ---------------------------------------------------------------------------

	interface Generation {
		id: string;
		imageUrl?: string;
		prompt: string;
		negativePrompt?: string;
		status: GenerationStatus;
		timestamp: Date;
		model: string;
		style: string;
		aspectRatio: string;
	}

	interface GenerationForm {
		prompt: string;
		negativePrompt: string;
		stylePreset: string;
		aspectRatio: string;
		model: string;
		numImages: number;
	}

	// ---------------------------------------------------------------------------
	// State
	// ---------------------------------------------------------------------------

	let form: GenerationForm = {
		prompt: '',
		negativePrompt: '',
		stylePreset: 'none',
		aspectRatio: '1:1',
		model: 'dalle3',
		numImages: 1
	};

	let showNegativePrompt = false;
	let generating = false;
	let generations: Generation[] = [];
	let selectedGeneration: Generation | null = null;
	let detailModalOpen = false;
	let genCount = 0;

	// Mock user stats
	const userCredits = 47;
	const monthlyQuota = 100;
	const monthlyUsed = 23;

	// Style presets
	const stylePresets = [
		{ value: 'none', label: 'None' },
		{ value: 'photorealistic', label: 'Photorealistic' },
		{ value: 'anime', label: 'Anime' },
		{ value: 'cinematic', label: 'Cinematic' },
		{ value: 'digital-art', label: 'Digital Art' },
		{ value: 'concept-art', label: 'Concept Art' },
		{ value: 'illustration', label: 'Illustration' },
		{ value: '3d-render', label: '3D Render' },
		{ value: 'pixel-art', label: 'Pixel Art' }
	];

	// Models with cost
	const models = [
		{ value: 'dalle3', label: 'DALL-E 3', cost: 4 },
		{ value: 'gemini-imagen3', label: 'Gemini Imagen 3', cost: 2 },
		{ value: 'claude-image', label: 'Claude Image', cost: 3 }
	];

	// Aspect ratios
	const aspectRatios = [
		{ value: '1:1', label: '1:1', icon: '□' },
		{ value: '16:9', label: '16:9', icon: '▬' },
		{ value: '9:16', label: '9:16', icon: '▌' },
		{ value: '4:3', label: '4:3', icon: '▭' },
		{ value: '3:4', label: '3:4', icon: '▮' }
	];

	// ---------------------------------------------------------------------------
	// Computed
	// ---------------------------------------------------------------------------

	$: selectedModel = models.find((m) => m.value === form.model);
	$: estimatedCost = (selectedModel?.cost ?? 0) * form.numImages;

	// ---------------------------------------------------------------------------
	// Methods
	// ---------------------------------------------------------------------------

	function toggleNegativePrompt() {
		showNegativePrompt = !showNegativePrompt;
	}

	function setAspectRatio(ratio: string) {
		form.aspectRatio = ratio;
	}

	function adjustNumImages(delta: number) {
		form.numImages = Math.max(1, Math.min(4, form.numImages + delta));
	}

	function openDetail(gen: Generation) {
		selectedGeneration = gen;
		detailModalOpen = true;
	}

	async function handleGenerate() {
		if (!form.prompt.trim() || generating) return;

		generating = true;
		genCount++;

		// Create pending generation entries
		const newGens: Generation[] = Array.from({ length: form.numImages }, (_, i) => ({
			id: `gen-${Date.now()}-${i}`,
			prompt: form.prompt,
			negativePrompt: form.negativePrompt,
			status: 'queued' as GenerationStatus,
			timestamp: new Date(),
			model: selectedModel?.label ?? form.model,
			style: form.stylePreset,
			aspectRatio: form.aspectRatio
		}));

		generations = [...newGens, ...generations];

		// Simulate generation with staggered status updates
		for (const gen of newGens) {
			// Delay between each
			await new Promise((r) => setTimeout(r, 800));

			// Update to processing
			generations = generations.map((g) =>
				g.id === gen.id ? { ...g, status: 'processing' as GenerationStatus } : g
			);

			// Processing duration
			await new Promise((r) => setTimeout(r, 1500 + Math.random() * 2000));

			// Simulate success (90%) or failure (10%)
			const success = Math.random() > 0.1;
			generations = generations.map((g) =>
				g.id === gen.id
					? {
							...g,
							status: (success ? 'completed' : 'failed') as GenerationStatus,
							imageUrl: success
								? `https://picsum.photos/seed/${gen.id}/512/512`
								: undefined
						}
					: g
			);
		}

		generating = false;
	}

	function formatDate(date: Date): string {
		return date.toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head>
	<title>Create - Creative AI Studio</title>
</svelte:head>

<div class="flex flex-col h-full">
	<!-- Top bar -->
	<div
		class="flex items-center justify-between px-6 py-4 border-b border-[var(--border)]"
		style="background-color: var(--bg-secondary);"
	>
		<div>
			<h1 class="text-xl font-bold text-text-primary">New Generation</h1>
			<p class="text-xs text-text-secondary mt-0.5">
				{monthlyUsed} of {monthlyQuota} generations used this month
			</p>
		</div>
		<div class="flex items-center gap-4">
			<a href="/app/gallery" class="text-sm text-accent hover:underline">My Generations</a>
			<div class="flex items-center gap-2 text-xs text-text-secondary">
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" width="14" height="14" class="text-accent">
					<path d="M8 1.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13zM0 8a8 8 0 1116 0A8 8 0 010 8z"/>
					<path d="M8 4a.75.75 0 01.75.75v2.5h2.5a.75.75 0 010 1.5h-2.5v2.5a.75.75 0 01-1.5 0v-2.5h-2.5a.75.75 0 010-1.5h2.5v-2.5A.75.75 0 018 4z"/>
				</svg>
				<span>{userCredits} credits</span>
			</div>
		</div>
	</div>

	<!-- Two-panel layout -->
	<div class="flex flex-1 min-h-0">
		<!-- Left panel: Prompt input -->
		<aside
			class="w-full md:w-[40%] lg:w-[38%] border-b md:border-b-0 border-r border-[var(--border)] overflow-y-auto"
			style="background-color: var(--bg-secondary);"
		>
			<div class="p-6 space-y-6">
				<!-- Prompt textarea -->
				<div>
					<label for="prompt" class="block text-sm font-medium text-text-primary mb-2">
						Prompt
					</label>
					<textarea
						id="prompt"
						bind:value={form.prompt}
						placeholder="Describe the image you want to create..."
						disabled={generating}
						rows="5"
						class="w-full px-3 py-2.5 rounded-lg text-sm resize-none"
						style="
							background-color: var(--bg-primary);
							color: var(--text-primary);
							border: 1px solid var(--border);
							outline: none;
							min-height: 120px;
						"
						on:focus={(e) => {
							e.currentTarget.style.borderColor = 'var(--border-active)';
							e.currentTarget.style.boxShadow = '0 0 0 3px var(--accent-glow)';
						}}
						on:blur={(e) => {
							e.currentTarget.style.borderColor = 'var(--border)';
							e.currentTarget.style.boxShadow = 'none';
						}}
					></textarea>
				</div>

				<!-- Negative prompt (collapsible) -->
				<div>
					<button
						type="button"
						class="flex items-center justify-between w-full text-sm font-medium text-text-secondary hover:text-text-primary transition-colors"
						on:click={toggleNegativePrompt}
					>
						<span>Negative prompt</span>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 20 20"
							fill="currentColor"
							width="16"
							height="16"
							class="transition-transform duration-200"
							class:rotate-180={showNegativePrompt}
						>
							<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
						</svg>
					</button>

					{#if showNegativePrompt}
						<div class="mt-2">
							<textarea
								id="neg-prompt"
								bind:value={form.negativePrompt}
								placeholder="What you don't want in the image..."
								disabled={generating}
								rows="3"
								class="w-full px-3 py-2 rounded-lg text-sm resize-none"
								style="
									background-color: var(--bg-primary);
									color: var(--text-primary);
									border: 1px solid var(--border);
									outline: none;
								"
								on:focus={(e) => {
									e.currentTarget.style.borderColor = 'var(--border-active)';
									e.currentTarget.style.boxShadow = '0 0 0 3px var(--accent-glow)';
								}}
								on:blur={(e) => {
									e.currentTarget.style.borderColor = 'var(--border)';
									e.currentTarget.style.boxShadow = 'none';
								}}
							></textarea>
						</div>
					{/if}
				</div>

				<!-- Style preset -->
				<div>
					<label for="style-preset" class="block text-sm font-medium text-text-primary mb-2">
						Style Preset
					</label>
					<div class="relative">
						<select
							id="style-preset"
							bind:value={form.stylePreset}
							disabled={generating}
							class="w-full px-3 py-2 rounded-lg text-sm appearance-none cursor-pointer"
							style="
								background-color: var(--bg-primary);
								color: var(--text-primary);
								border: 1px solid var(--border);
								outline: none;
							"
						>
							{#each stylePresets as preset}
								<option value={preset.value}>{preset.label}</option>
							{/each}
						</select>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 20 20"
							fill="currentColor"
							width="16"
							height="16"
							class="absolute right-3 top-1/2 -translate-y-1/2 text-text-muted pointer-events-none"
						>
							<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
						</svg>
					</div>
				</div>

				<!-- Aspect ratio -->
				<div>
					<label class="block text-sm font-medium text-text-primary mb-2">Aspect Ratio</label>
					<div class="flex gap-2">
						{#each aspectRatios as ar}
							<button
								type="button"
								class="flex-1 py-2 px-3 rounded-lg text-sm font-medium transition-all border"
								disabled={generating}
								style="
									background-color: {form.aspectRatio === ar.value
									? 'var(--accent)'
									: 'var(--bg-primary)'};
									color: {form.aspectRatio === ar.value
									? 'white'
									: 'var(--text-secondary)'};
									border-color: {form.aspectRatio === ar.value
									? 'var(--accent)'
									: 'var(--border)'};
									outline: none;
								"
								on:click={() => setAspectRatio(ar.value)}
							>
								{ar.label}
							</button>
						{/each}
					</div>
				</div>

				<!-- Model selector -->
				<div>
					<label for="model" class="block text-sm font-medium text-text-primary mb-2">
						Model
					</label>
					<div class="relative">
						<select
							id="model"
							bind:value={form.model}
							disabled={generating}
							class="w-full px-3 py-2 rounded-lg text-sm appearance-none cursor-pointer"
							style="
								background-color: var(--bg-primary);
								color: var(--text-primary);
								border: 1px solid var(--border);
								outline: none;
							"
						>
							{#each models as model}
								<option value={model.value}>
									{model.label} ({model.cost} credit{model.cost > 1 ? 's' : ''}/img)
								</option>
							{/each}
						</select>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 20 20"
							fill="currentColor"
							width="16"
							height="16"
							class="absolute right-3 top-1/2 -translate-y-1/2 text-text-muted pointer-events-none"
						>
							<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
						</svg>
					</div>
				</div>

				<!-- Number of images -->
				<div>
					<label class="block text-sm font-medium text-text-primary mb-2">
						Number of Images
					</label>
					<div class="flex items-center gap-3">
						<button
							type="button"
							class="w-9 h-9 rounded-lg flex items-center justify-center border text-text-primary transition-all"
							disabled={form.numImages <= 1 || generating}
							style="
								background-color: var(--bg-primary);
								border-color: var(--border);
								opacity: {form.numImages <= 1 ? 0.4 : 1};
							"
							on:click={() => adjustNumImages(-1)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
								<path d="M4 10a.75.75 0 01.75-.75h10.5a.75.75 0 010 1.5H4.75A.75.75 0 014 10z" />
							</svg>
						</button>
						<div
							class="w-12 h-9 rounded-lg flex items-center justify-center text-sm font-medium border"
							style="
								background-color: var(--bg-primary);
								color: var(--text-primary);
								border-color: var(--border);
							"
						>
							{form.numImages}
						</div>
						<button
							type="button"
							class="w-9 h-9 rounded-lg flex items-center justify-center border text-text-primary transition-all"
							disabled={form.numImages >= 4 || generating}
							style="
								background-color: var(--bg-primary);
								border-color: var(--border);
								opacity: {form.numImages >= 4 ? 0.4 : 1};
							"
							on:click={() => adjustNumImages(1)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="16" height="16">
								<path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" />
							</svg>
						</button>
						{#if estimatedCost > 0}
							<span class="text-xs text-text-muted ml-1">= {estimatedCost} credit{estimatedCost > 1 ? 's' : ''}</span>
						{/if}
					</div>
				</div>

				<!-- Generate button -->
				<div class="pt-2">
					<button
						type="button"
						class="w-full py-3 rounded-lg font-semibold text-white flex items-center justify-center gap-2 transition-all"
						disabled={!form.prompt.trim() || generating}
						style="
							background: {form.prompt.trim() && !generating
								? 'var(--accent-gradient)'
								: 'var(--bg-primary)'};
							opacity: {form.prompt.trim() && !generating ? 1 : 0.5};
							cursor: {form.prompt.trim() && !generating ? 'pointer' : 'not-allowed'};
							box-shadow: {form.prompt.trim() && !generating
								? '0 0 20px var(--accent-glow)'
								: 'none'};
						"
						on:click={handleGenerate}
					>
						{#if generating}
							<svg class="w-5 h-5 animate-spin" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
								<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
								<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
							</svg>
							<span>Generating...</span>
						{:else}
							<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="18" height="18">
								<path fill-rule="evenodd" d="M8 2a.75.75 0 01.75.75v4.5h4.5a.75.75 0 010 1.5h-4.5v4.5a.75.75 0 01-1.5 0v-4.5h-4.5a.75.75 0 010-1.5h4.5v-4.5A.75.75 0 018 2z" clip-rule="evenodd" />
							</svg>
							<span>Generate</span>
						{/if}
					</button>
				</div>
			</div>
		</aside>

		<!-- Right panel: Results grid -->
		<main
			class="hidden md:flex flex-1 flex-col min-h-0 overflow-hidden"
			style="background-color: var(--bg-primary);"
		>
			<div class="flex-1 overflow-y-auto p-6">
				{#if generations.length === 0}
					<!-- Empty state -->
					<div class="flex flex-col items-center justify-center h-full text-center py-16">
						<div
							class="w-24 h-24 rounded-2xl flex items-center justify-center mb-6"
							style="background: var(--accent-gradient); opacity: 0.15;"
						>
							<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="40" height="40" class="text-accent">
								<path stroke-linecap="round" stroke-linejoin="round" d="M6.827 6.175A2.31 2.31 0 015.186 7.23c-.38.054-.757.112-1.134.175C2.999 7.58 2.25 8.507 2.25 9.574V18a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9.574c0-1.067-.75-1.994-1.802-2.169a47.865 47.865 0 00-1.134-.175 2.31 2.31 0 01-1.64-1.055l-.822-1.316a2.192 2.192 0 00-1.736-1.039 48.774 48.774 0 00-5.232 0 2.192 2.192 0 00-1.736 1.039l-.821 1.316z" />
								<path stroke-linecap="round" stroke-linejoin="round" d="M16.5 12.75a4.5 4.5 0 11-9 0 4.5 4.5 0 019 0zM18.75 10.5h.008v.008h-.008V10.5z" />
							</svg>
						</div>
						<h3 class="text-lg font-semibold text-text-primary mb-2">No generations yet</h3>
						<p class="text-sm text-text-secondary max-w-xs">
							Enter a prompt on the left and click Generate to create your first AI-powered image.
						</p>
					</div>
				{:else}
					<!-- Results grid -->
					<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
						{#each generations as gen (gen.id)}
							<ImageCard
								imageUrl={gen.imageUrl}
								prompt={gen.prompt}
								status={gen.status}
								timestamp={gen.timestamp}
								model={gen.model}
								on:click={() => openDetail(gen)}
							/>
						{/each}

						<!-- Skeleton cards while generating -->
						{#if generating}
							{#each Array(Math.max(0, 4 - generations.filter((g) => g.status === 'completed' || g.status === 'failed').length)) as _}
								<div class="card p-0 overflow-hidden animate-pulse">
									<div class="aspect-square bg-bg-secondary relative">
										<div class="absolute inset-0 flex items-center justify-center">
											<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="32" height="32" class="text-text-muted animate-pulse">
												<path fill-rule="evenodd" d="M4.224 2.927a.75.75 0 010 1.046l-1.67 1.696a.75.75 0 01-1.06 0L.75 3.958A.75.75 0 011.81 2.927l1.67 1.696L6.96 2.09a.75.75 0 011.06 0l3.48 3.528a.75.75 0 010 1.046l-3.48 3.528a.75.75 0 01-1.06 0L4.224 7.61a.75.75 0 010-1.046l1.67-1.696a.75.75 0 011.06 0l3.48 3.528a.75.75 0 010 1.046L7.47 12.46a.75.75 0 01-1.06 0L3.936 10.8a.75.75 0 010-1.046l1.67-1.696a.75.75 0 011.06 0l1.974 2a.75.75 0 01-1.06 1.046L5.616 12.17a.75.75 0 00-1.06 1.046L8.04 16.7a.75.75 0 001.06 0l3.484-3.528a.75.75 0 000-1.046L10.52 9.154a.75.75 0 011.06 0l1.974 2a.75.75 0 01-1.06 1.046L10.52 10.8a.75.75 0 00-1.06 0L6.976 13.272a.75.75 0 000 1.046l1.974 2a.75.75 0 001.06-1.046l-1.67-1.696a.75.75 0 010-1.046L10.52 11.54a.75.75 0 011.06 0l1.974 2a.75.75 0 01-1.06 1.046L10.52 13.172a.75.75 0 00-1.06 0L6.976 15.644a.75.75 0 000 1.046l1.974 2a.75.75 0 001.06-1.046l-1.67-1.696a.75.75 0 010-1.046L10.52 14.012a.75.75 0 011.06 0l1.974 2a.75.75 0 01-1.06 1.046l-1.974-2z" clip-rule="evenodd" />
											</svg>
										</div>
									</div>
									<div class="p-3">
										<div class="h-3 bg-bg-secondary rounded w-3/4 mb-2"></div>
										<div class="h-2 bg-bg-secondary rounded w-1/2"></div>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
		</main>
	</div>
</div>

<!-- Detail Modal -->
<Modal bind:open={detailModalOpen} title="Generation Details" size="lg">
	{#if selectedGeneration}
		<div class="space-y-4">
			<!-- Image -->
			{#if selectedGeneration.imageUrl}
				<div class="rounded-lg overflow-hidden bg-bg-secondary">
					<img
						src={selectedGeneration.imageUrl}
						alt={selectedGeneration.prompt}
						class="w-full max-h-96 object-contain"
					/>
				</div>
			{:else}
				<div class="rounded-lg aspect-square bg-bg-secondary flex items-center justify-center">
					<span class="text-text-muted">No image available</span>
				</div>
			{/if}

			<!-- Details -->
			<div class="space-y-3">
				<div>
					<p class="text-xs text-text-muted uppercase tracking-wide mb-1">Prompt</p>
					<p class="text-sm text-text-primary">{selectedGeneration.prompt}</p>
				</div>
				{#if selectedGeneration.negativePrompt}
					<div>
						<p class="text-xs text-text-muted uppercase tracking-wide mb-1">Negative Prompt</p>
						<p class="text-sm text-text-secondary">{selectedGeneration.negativePrompt}</p>
					</div>
				{/if}
				<div class="grid grid-cols-2 gap-4 text-sm">
					<div>
						<p class="text-xs text-text-muted mb-1">Model</p>
						<p class="text-text-primary">{selectedGeneration.model}</p>
					</div>
					<div>
						<p class="text-xs text-text-muted mb-1">Aspect Ratio</p>
						<p class="text-text-primary">{selectedGeneration.aspectRatio}</p>
					</div>
					<div>
						<p class="text-xs text-text-muted mb-1">Style</p>
						<p class="text-text-primary capitalize">{selectedGeneration.style.replace('-', ' ')}</p>
					</div>
					<div>
						<p class="text-xs text-text-muted mb-1">Created</p>
						<p class="text-text-primary">{formatDate(selectedGeneration.timestamp)}</p>
					</div>
				</div>
			</div>
		</div>
	{/if}

	<svelte:fragment slot="footer">
		{#if selectedGeneration?.imageUrl}
			<a
				href={selectedGeneration.imageUrl}
				download
				class="btn-secondary px-4 py-2 rounded-md text-sm"
			>
				Download
			</a>
		{/if}
		<Button variant="secondary" on:click={() => (detailModalOpen = false)}>Close</Button>
	</svelte:fragment>
</Modal>
