<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import { auth } from '$stores/auth';
	import { toast } from '$stores/toast';
	import { generations, type Generation, type GenerationStatus } from '$lib/stores/generations';
	import { wsClient } from '$lib/api/ws-client';
	import Spinner from '$lib/components/ui/Spinner.svelte';

	// ---------------------------------------------------------------------------
	// Types
	// ---------------------------------------------------------------------------

	type SortOption = 'newest' | 'oldest' | 'az';
	type FilterStatus = 'all' | GenerationStatus;

	interface StatusOption {
		value: FilterStatus;
		label: string;
	}

	// ---------------------------------------------------------------------------
	// Constants
	// ---------------------------------------------------------------------------

	const PAGE_SIZE = 12;

	const STATUS_OPTIONS: StatusOption[] = [
		{ value: 'all', label: 'All' },
		{ value: 'pending', label: 'Pending' },
		{ value: 'processing', label: 'Processing' },
		{ value: 'completed', label: 'Completed' },
		{ value: 'failed', label: 'Failed' }
	];

	const STATUS_BADGE_CLASS: Record<GenerationStatus, string> = {
		pending: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
		processing: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
		completed: 'bg-green-500/20 text-green-400 border-green-500/30',
		failed: 'bg-red-500/20 text-red-400 border-red-500/30'
	};

	// ---------------------------------------------------------------------------
	// UI State
	// ---------------------------------------------------------------------------

	let searchQuery = '';
	let filterStatus: FilterStatus = 'all';
	let sortBy: SortOption = 'newest';
	let visibleCount = PAGE_SIZE;
	let loading = true;

	// Detail modal
	let selectedGeneration: Generation | null = null;
	let showDeleteConfirm = false;
	let deletingId: string | null = null;
	let deleting = false;

	// ---------------------------------------------------------------------------
	// Computed / Derived
	// ---------------------------------------------------------------------------

	$: filteredItems = (() => {
		let items = $generations.items;

		// Filter by status
		if (filterStatus !== 'all') {
			items = items.filter((g) => g.status === filterStatus);
		}

		// Filter by search query
		if (searchQuery.trim()) {
			const q = searchQuery.toLowerCase();
			items = items.filter(
				(g) =>
					g.prompt.toLowerCase().includes(q) ||
					g.model.toLowerCase().includes(q) ||
					(g.negative_prompt && g.negative_prompt.toLowerCase().includes(q))
			);
		}

		// Sort
		items = [...items].sort((a, b) => {
			switch (sortBy) {
				case 'newest':
					return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
				case 'oldest':
					return new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
				case 'az':
					return a.prompt.localeCompare(b.prompt);
				default:
					return 0;
			}
		});

		return items;
	})();

	$: visibleItems = filteredItems.slice(0, visibleCount);
	$: hasMore = visibleCount < filteredItems.length;

	// ---------------------------------------------------------------------------
	// Lifecycle
	// ---------------------------------------------------------------------------

	let wsUnsub: (() => void) | null = null;

	onMount(() => {
		// Connect WebSocket if authenticated
		const currentAuth = $auth;
		if (currentAuth.user) {
			wsClient.connect(currentAuth.user.id);
		}

		// Kick off async data fetch
		(async () => {
			await generations.fetchAll();
			loading = false;
		})();

		// Subscribe to real-time updates (sync, returned as cleanup)
		wsUnsub = wsClient.onUpdate((update) => {
			const exists = $generations.items.some((g) => g.id === update.generation_id);

			if (exists) {
				generations.updateGeneration({
					id: update.generation_id,
					status: update.status,
					image_url: update.image_url,
					error_message: update.status === 'failed' ? update.message : undefined
				});

				if (update.status === 'completed') toast.success('Image generation completed!');
				else if (update.status === 'failed')
					toast.error(`Generation failed: ${update.message}`);
			} else {
				// New generation from another source — add it
				const newGen: Generation = {
					id: update.generation_id,
					prompt: update.message || '',
					model: '',
					width: 0,
					height: 0,
					status: update.status,
					image_url: update.image_url,
					created_at: update.timestamp
				};
				generations.addGeneration(newGen);
			}
		});

		return () => {
			if (wsUnsub) wsUnsub();
		};
	});

	// ---------------------------------------------------------------------------
	// Handlers
	// ---------------------------------------------------------------------------

	function loadMore() {
		visibleCount += PAGE_SIZE;
	}

	function openDetail(gen: Generation) {
		selectedGeneration = gen;
		showDeleteConfirm = false;
	}

	function closeDetail() {
		selectedGeneration = null;
		showDeleteConfirm = false;
	}

	async function handleDelete() {
		if (!selectedGeneration) return;
		deleting = true;
		try {
			await generations.deleteGeneration(selectedGeneration.id);
			closeDetail();
		} catch {
			// Error handled in store
		} finally {
			deleting = false;
		}
	}

	async function handleDeleteQuick(id: string) {
		if (!confirm('Delete this generation?')) return;
		try {
			await generations.deleteGeneration(id);
		} catch {
			// Error handled in store
		}
	}

	function handleDownload(gen: Generation) {
		if (!gen.image_url) return;
		const a = document.createElement('a');
		a.href = gen.image_url;
		a.download = `generation-${gen.id}.png`;
		a.target = '_blank';
		a.rel = 'noopener noreferrer';
		a.click();
	}

	function handleShare(gen: Generation) {
		const url = `${window.location.origin}/gallery/${gen.id}`;
		if (navigator.clipboard) {
			navigator.clipboard.writeText(url).then(() => {
				toast.success('Link copied to clipboard!');
			});
		} else {
			prompt('Copy this link:', url);
		}
	}

	function handleCreateSimilar(gen: Generation) {
		// Navigate to creator with the prompt pre-filled via query param
		goto(`/app/creator?prompt=${encodeURIComponent(gen.prompt)}`);
	}

	// ---------------------------------------------------------------------------
	// Helpers
	// ---------------------------------------------------------------------------

	function formatDate(dateStr: string): string {
		if (!dateStr) return '—';
		try {
			return new Date(dateStr).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return dateStr;
		}
	}

	function formatDuration(seconds?: number): string {
		if (!seconds) return '—';
		if (seconds < 60) return `${seconds.toFixed(1)}s`;
		return `${Math.floor(seconds / 60)}m ${(seconds % 60).toFixed(0)}s`;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && selectedGeneration) {
			closeDetail();
		}
	}
</script>

<svelte:head>
	<title>Gallery - Creative AI Studio</title>
</svelte:head>

<svelte:window on:keydown={handleKeydown} />

<!-- Detail Modal -->
{#if selectedGeneration}
	<!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
		on:click|self={closeDetail}
	>
		<div
			class="relative w-full max-w-5xl max-h-[90vh] bg-bg-card rounded-xl border border-border overflow-hidden flex flex-col md:flex-row"
		>
			<!-- Close button -->
			<button
				type="button"
				class="absolute top-4 right-4 z-10 w-8 h-8 rounded-full bg-black/50 text-white flex items-center justify-center hover:bg-black/70 transition-colors"
				on:click={closeDetail}
				aria-label="Close"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
			</button>

			<!-- Image panel -->
			<div class="flex-shrink-0 flex items-center justify-center bg-black/30 w-full md:w-2/3 max-h-[90vh]">
				{#if selectedGeneration.image_url}
					<img
						src={selectedGeneration.image_url}
						alt="Generated image"
						class="max-w-full max-h-[60vh] md:max-h-[90vh] object-contain"
					/>
				{:else}
					<div class="flex flex-col items-center justify-center py-24 text-text-muted">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-16 h-16 mb-3 opacity-40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline></svg>
						<p class="text-sm">No image available</p>
					</div>
				{/if}
			</div>

			<!-- Details sidebar -->
			<div class="flex-1 overflow-y-auto p-6 space-y-5 w-full md:w-1/3 border-t md:border-t-0 md:border-l border-border">
				<!-- Status badge -->
				<div>
					<span class="px-2 py-0.5 rounded text-xs border font-medium {STATUS_BADGE_CLASS[selectedGeneration.status]}">
						{selectedGeneration.status.charAt(0).toUpperCase() + selectedGeneration.status.slice(1)}
					</span>
				</div>

				<!-- Prompt -->
				<div>
					<h3 class="text-xs font-semibold text-text-secondary uppercase tracking-wider mb-1">Prompt</h3>
					<p class="text-sm text-text-primary leading-relaxed">{selectedGeneration.prompt || '—'}</p>
				</div>

				<!-- Negative prompt -->
				{#if selectedGeneration.negative_prompt}
					<div>
						<h3 class="text-xs font-semibold text-text-secondary uppercase tracking-wider mb-1">Negative Prompt</h3>
						<p class="text-sm text-text-primary leading-relaxed opacity-80">{selectedGeneration.negative_prompt}</p>
					</div>
				{/if}

				<!-- Metadata grid -->
				<div class="grid grid-cols-2 gap-3 text-sm">
					<div>
						<p class="text-text-muted text-xs">Model</p>
						<p class="text-text-primary font-medium truncate">{selectedGeneration.model || '—'}</p>
					</div>
					<div>
						<p class="text-text-muted text-xs">Dimensions</p>
						<p class="text-text-primary font-medium">{selectedGeneration.width && selectedGeneration.height ? `${selectedGeneration.width} x ${selectedGeneration.height}` : '—'}</p>
					</div>
					<div>
						<p class="text-text-muted text-xs">Gen Time</p>
						<p class="text-text-primary font-medium">{formatDuration(selectedGeneration.generation_time)}</p>
					</div>
					<div>
						<p class="text-text-muted text-xs">Created</p>
						<p class="text-text-primary font-medium">{formatDate(selectedGeneration.created_at)}</p>
					</div>
				</div>

				<!-- Error message -->
				{#if selectedGeneration.error_message}
					<div class="p-3 rounded-md bg-red-500/10 border border-red-500/20">
						<p class="text-xs font-semibold text-red-400 mb-1">Error</p>
						<p class="text-sm text-red-300">{selectedGeneration.error_message}</p>
					</div>
				{/if}

				<!-- Actions -->
				<div class="space-y-2 pt-2">
					{#if selectedGeneration.image_url}
						<button
							type="button"
							class="w-full btn-primary text-sm"
							on:click={() => selectedGeneration && handleDownload(selectedGeneration)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
							Download
						</button>

						<button
							type="button"
							class="w-full btn-secondary text-sm"
							on:click={() => selectedGeneration && handleShare(selectedGeneration)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"></circle><circle cx="6" cy="12" r="3"></circle><circle cx="18" cy="19" r="3"></circle><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line></svg>
							Share
						</button>
					{/if}

					<button
						type="button"
						class="w-full btn-secondary text-sm"
						on:click={() => selectedGeneration && handleCreateSimilar(selectedGeneration)}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>
						Create Similar
					</button>

					<!-- Delete section -->
					{#if showDeleteConfirm}
						<div class="p-3 rounded-md border border-red-500/30 bg-red-500/5 space-y-2">
							<p class="text-sm text-red-400 text-center">Are you sure?</p>
							<div class="flex gap-2">
								<button
									type="button"
									class="flex-1 px-3 py-1.5 rounded text-xs font-medium border border-red-500/30 text-red-400 hover:bg-red-500/10 transition-colors"
									on:click={() => (showDeleteConfirm = false)}
									disabled={deleting}
								>
									Cancel
								</button>
								<button
									type="button"
									class="flex-1 px-3 py-1.5 rounded text-xs font-medium bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors"
									on:click={handleDelete}
									disabled={deleting}
								>
									{deleting ? 'Deleting...' : 'Delete'}
								</button>
							</div>
						</div>
					{:else}
						<button
							type="button"
							class="w-full px-4 py-2 rounded-md text-sm font-medium text-red-400 border border-red-500/20 hover:bg-red-500/10 transition-colors flex items-center justify-center gap-2"
							on:click={() => (showDeleteConfirm = true)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
							Delete
						</button>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Main page -->
<main class="min-h-screen bg-bg-primary">
	<div class="max-w-7xl mx-auto px-4 py-8 space-y-6">

		<!-- Header row -->
		<div class="flex flex-col sm:flex-row items-start sm:items-center gap-4 justify-between">
			<div>
				<h1 class="text-2xl font-bold text-text-primary">Gallery</h1>
				<p class="text-text-secondary text-sm mt-0.5">
					{$generations.items.length} generation{$generations.items.length !== 1 ? 's' : ''}
					{#if filterStatus !== 'all' || searchQuery}
						<span class="text-text-muted"> (filtered)</span>
					{/if}
				</p>
			</div>

			<div class="flex flex-wrap items-center gap-3">
				<!-- Search -->
				<div class="relative">
					<svg xmlns="http://www.w3.org/2000/svg" class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
					<input
						type="text"
						bind:value={searchQuery}
						placeholder="Search prompts..."
						class="input pl-9 w-52"
					/>
				</div>

				<!-- Sort dropdown -->
				<select
					bind:value={sortBy}
					class="input w-auto"
				>
					<option value="newest">Newest</option>
					<option value="oldest">Oldest</option>
					<option value="az">A-Z</option>
				</select>
			</div>
		</div>

		<!-- Filter chips -->
		<div class="flex flex-wrap gap-2">
			{#each STATUS_OPTIONS as opt}
				<button
					type="button"
					class="px-3 py-1.5 rounded-full text-xs font-medium border transition-all {filterStatus === opt.value
						? 'border-accent text-white'
						: 'border-border text-text-secondary hover:border-border-active hover:text-text-primary'}"
					style={filterStatus === opt.value ? 'background: var(--accent-gradient);' : ''}
					on:click={() => {
						filterStatus = opt.value;
						visibleCount = PAGE_SIZE;
					}}
				>
					{opt.label}
				</button>
			{/each}
		</div>

		<!-- Results -->
		{#if loading}
			<div class="flex items-center justify-center py-24">
				<Spinner />
			</div>
		{:else if $generations.items.length === 0}
			<!-- Empty state — no generations at all -->
			<div class="flex flex-col items-center justify-center py-24 space-y-4">
				<div class="w-20 h-20 rounded-full bg-bg-card border border-border flex items-center justify-center">
					<svg xmlns="http://www.w3.org/2000/svg" class="w-10 h-10 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline></svg>
				</div>
				<div class="text-center">
					<h2 class="text-lg font-semibold text-text-primary">No generations yet</h2>
					<p class="text-text-secondary text-sm mt-1">Create your first AI image to see it here.</p>
				</div>
				<button
					type="button"
					class="btn-primary mt-2"
					on:click={() => goto('/app/creator')}
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon></svg>
					Go to Creator
				</button>
			</div>
		{:else if filteredItems.length === 0}
			<!-- Empty filtered state -->
			<div class="flex flex-col items-center justify-center py-24 space-y-4">
				<div class="w-16 h-16 rounded-full bg-bg-card border border-border flex items-center justify-center">
					<svg xmlns="http://www.w3.org/2000/svg" class="w-8 h-8 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
				</div>
				<div class="text-center">
					<h2 class="text-base font-semibold text-text-primary">No matching results</h2>
					<p class="text-text-secondary text-sm mt-1">Try a different search or filter.</p>
				</div>
				<button
					type="button"
					class="btn-secondary text-sm"
					on:click={() => {
						searchQuery = '';
						filterStatus = 'all';
						visibleCount = PAGE_SIZE;
					}}
				>
					Clear filters
				</button>
			</div>
		{:else}
			<!-- Masonry-style grid -->
			<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
				{#each visibleItems as gen (gen.id)}
					<div
						class="group relative bg-bg-card rounded-lg border border-border overflow-hidden hover:border-border-active transition-all duration-micro"
					>
						<!-- Image -->
						<div class="relative aspect-video bg-bg-secondary overflow-hidden">
							{#if gen.image_url}
								<img
									src={gen.image_url}
									alt={gen.prompt || 'Generated image'}
									class="w-full h-full object-cover"
									loading="lazy"
								/>
							{:else}
								<div class="w-full h-full flex items-center justify-center">
									<Spinner size="sm" />
								</div>
							{/if}

							<!-- Hover overlay with full prompt -->
							<!-- svelte-ignore a11y-no-static-element-interactions -->
							<div
								class="absolute inset-0 bg-black/80 opacity-0 group-hover:opacity-100 transition-opacity duration-200 flex flex-col items-center justify-center p-4"
								on:click={() => openDetail(gen)}
							>
								<p class="text-xs text-white/90 text-center line-clamp-5 leading-relaxed">
									{gen.prompt || 'No prompt'}
								</p>
							</div>

							<!-- Status badge -->
							<div class="absolute top-2 left-2">
								<span class="px-1.5 py-0.5 rounded text-[10px] border font-medium {STATUS_BADGE_CLASS[gen.status]}">
									{gen.status}
								</span>
							</div>
						</div>

						<!-- Quick action buttons on hover -->
						<div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1">
							{#if gen.image_url}
								<button
									type="button"
									class="w-7 h-7 rounded-full bg-black/60 text-white flex items-center justify-center hover:bg-black/80 transition-colors"
									on:click|stopPropagation={() => handleDownload(gen)}
									title="Download"
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
								</button>
								<button
									type="button"
									class="w-7 h-7 rounded-full bg-black/60 text-white flex items-center justify-center hover:bg-black/80 transition-colors"
									on:click|stopPropagation={() => handleShare(gen)}
									title="Share"
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"></circle><circle cx="6" cy="12" r="3"></circle><circle cx="18" cy="19" r="3"></circle><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line></svg>
								</button>
							{/if}
							<button
								type="button"
								class="w-7 h-7 rounded-full bg-black/60 text-white flex items-center justify-center hover:bg-black/80 transition-colors"
								on:click|stopPropagation={() => openDetail(gen)}
								title="View details"
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle></svg>
							</button>
							<button
								type="button"
								class="w-7 h-7 rounded-full bg-black/60 text-red-400 flex items-center justify-center hover:bg-black/80 transition-colors"
								on:click|stopPropagation={() => handleDeleteQuick(gen.id)}
								title="Delete"
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path></svg>
							</button>
						</div>

						<!-- Card footer -->
						<div class="p-3">
							<p class="text-xs text-text-secondary truncate">{gen.prompt || 'No prompt'}</p>
							<p class="text-[10px] text-text-muted mt-1">{formatDate(gen.created_at)}</p>
						</div>
					</div>
				{/each}
			</div>

			<!-- Load more -->
			{#if hasMore}
				<div class="flex justify-center pt-4">
					<button
						type="button"
						class="btn-secondary"
						on:click={loadMore}
					>
						Load more
						<span class="text-text-muted text-xs">
							({filteredItems.length - visibleCount} remaining)
						</span>
					</button>
				</div>
			{/if}
		{/if}
	</div>
</main>
