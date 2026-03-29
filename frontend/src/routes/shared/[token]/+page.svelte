<script lang="ts">
	import type { PageData } from './$types';
	import { isAuthenticated } from '$stores/auth';
	import FollowButton from '$lib/components/FollowButton.svelte';
	import type { SharedGeneration } from './+page';

	export let data: PageData;

	$: gen = data.generation as SharedGeneration;
	$: mainImage = gen.output_urls[0] ?? null;

	function formatDate(dateStr: string): string {
		try {
			return new Date(dateStr).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'long',
				day: 'numeric'
			});
		} catch {
			return dateStr;
		}
	}

	function downloadImage(url: string, index: number) {
		const a = document.createElement('a');
		a.href = url;
		a.download = `generation-${gen.generation_id}-${index + 1}.png`;
		a.target = '_blank';
		a.rel = 'noopener noreferrer';
		a.click();
	}

	function copyPrompt(text: string) {
		if (!text) return;
		navigator.clipboard.writeText(text).catch(() => {
			// Fallback for non-secure contexts
			const ta = document.createElement('textarea');
			ta.value = text;
			document.body.appendChild(ta);
			ta.select();
			document.execCommand('copy');
			document.body.removeChild(ta);
		});
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}
</script>

<svelte:head>
	<title>Shared Generation - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary">
	<div class="max-w-6xl mx-auto px-4 py-8">
		<!-- Back link -->
		<a
			href="/"
			class="inline-flex items-center gap-1.5 text-sm text-text-secondary hover:text-text-primary transition-colors mb-6"
		>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<line x1="19" y1="12" x2="5" y2="12"></line>
				<polyline points="12 19 5 12 12 5"></polyline>
			</svg>
			Back to home
		</a>

		<div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
			<!-- Left: Image(s) -->
			<div class="lg:col-span-2 space-y-4">
				{#if mainImage}
					<div class="relative rounded-xl overflow-hidden bg-bg-card border border-border">
						<div class="relative aspect-square lg:aspect-auto lg:max-h-[600px] flex items-center justify-center bg-black/30">
							<img
								src={mainImage}
								alt="Generated image"
								class="max-w-full max-h-[600px] object-contain"
							/>
						</div>

						<!-- Download button -->
						<button
							type="button"
							class="absolute bottom-4 right-4 btn-primary text-sm flex items-center gap-2"
							on:click={() => mainImage && downloadImage(mainImage, 0)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
								<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
								<polyline points="7 10 12 15 17 10"></polyline>
								<line x1="12" y1="15" x2="12" y2="3"></line>
							</svg>
							Download
						</button>
					</div>

					<!-- Additional images -->
					{#if gen.output_urls.length > 1}
						<div class="grid grid-cols-3 gap-2">
							{#each gen.output_urls.slice(1) as url, i}
								<button
									type="button"
									class="relative rounded-lg overflow-hidden bg-bg-card border border-border hover:border-border-active transition-colors aspect-square"
									on:click={() => downloadImage(url, i + 1)}
								>
									<img src={url} alt="Generated image {i + 2}" class="w-full h-full object-cover" loading="lazy" />
									<div class="absolute inset-0 bg-black/60 opacity-0 hover:opacity-100 transition-opacity flex items-center justify-center">
										<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-6 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
											<polyline points="7 10 12 15 17 10"></polyline>
											<line x1="12" y1="15" x2="12" y2="3"></line>
										</svg>
									</div>
								</button>
							{/each}
						</div>
					{/if}
				{:else}
					<div class="flex items-center justify-center aspect-square rounded-xl bg-bg-card border border-border">
						<p class="text-text-muted">No image available</p>
					</div>
				{/if}

				<!-- Metadata -->
				<div class="card space-y-4">
					<h2 class="text-sm font-semibold text-text-secondary uppercase tracking-wider">Generation Details</h2>

					<!-- Prompts -->
					<div class="space-y-3">
						<div>
							<div class="flex items-center justify-between mb-1">
								<h3 class="text-xs font-semibold text-text-secondary uppercase tracking-wider">Prompt</h3>
								<button
									type="button"
									class="text-xs text-text-muted hover:text-text-primary transition-colors flex items-center gap-1"
									on:click={() => copyPrompt(gen.prompt)}
									title="Copy prompt"
								>
									<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
										<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
										<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
									</svg>
									Copy
								</button>
							</div>
							<p class="text-sm text-text-primary leading-relaxed">{gen.prompt}</p>
						</div>

						{#if gen.enhanced_prompt}
							<div>
								<div class="flex items-center justify-between mb-1">
									<h3 class="text-xs font-semibold text-text-secondary uppercase tracking-wider">Enhanced Prompt</h3>
									<button
										type="button"
										class="text-xs text-text-muted hover:text-text-primary transition-colors flex items-center gap-1"
										on:click={() => gen.enhanced_prompt && copyPrompt(gen.enhanced_prompt)}
										title="Copy enhanced prompt"
									>
										<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
											<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
										</svg>
										Copy
									</button>
								</div>
								<p class="text-sm text-text-primary leading-relaxed opacity-80">{gen.enhanced_prompt}</p>
							</div>
						{/if}
					</div>

					<!-- Model info -->
					<div class="grid grid-cols-2 gap-3 text-sm">
						<div>
							<p class="text-text-muted text-xs">Provider</p>
							<p class="text-text-primary font-medium">{gen.provider || '—'}</p>
						</div>
						<div>
							<p class="text-text-muted text-xs">Model</p>
							<p class="text-text-primary font-medium truncate">{gen.model || '—'}</p>
						</div>
						{#if gen.style_preset_name}
							<div>
								<p class="text-text-muted text-xs">Style Preset</p>
								<p class="text-text-primary font-medium">{gen.style_preset_name}</p>
							</div>
						{/if}
						<div>
							<p class="text-text-muted text-xs">Views</p>
							<p class="text-text-primary font-medium flex items-center gap-1">
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
									<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
									<circle cx="12" cy="12" r="3"></circle>
								</svg>
								{gen.view_count}
							</p>
						</div>
						<div>
							<p class="text-text-muted text-xs">Created</p>
							<p class="text-text-primary font-medium">{formatDate(gen.created_at)}</p>
						</div>
					</div>
				</div>
			</div>

			<!-- Right: Creator sidebar -->
			<div class="space-y-4">
				<div class="card">
					<h2 class="text-sm font-semibold text-text-secondary uppercase tracking-wider mb-4">Created by</h2>

					<div class="flex items-center gap-3 mb-4">
						<a href="/public/{gen.creator_username}" class="flex-shrink-0">
							{#if gen.creator_avatar_url}
								<img
									src={gen.creator_avatar_url}
									alt={gen.creator_username}
									class="w-14 h-14 rounded-full object-cover border border-border"
								/>
							{:else}
								<div
									class="w-14 h-14 rounded-full flex items-center justify-center text-sm font-bold text-white border border-border"
									style="background: var(--accent-gradient);"
								>
									{getInitials(gen.creator_username)}
								</div>
							{/if}
						</a>

						<div class="flex-1 min-w-0">
							<a href="/public/{gen.creator_username}" class="hover:underline">
								<p class="text-sm font-semibold text-text-primary truncate">{gen.creator_username}</p>
							</a>
						</div>
					</div>

					{#if $isAuthenticated}
						<FollowButton username={gen.creator_username} initialIsFollowing={false} size="md" />
					{:else}
						<a href="/auth/login" class="btn-primary text-sm w-full text-center block">
							Sign in to follow
						</a>
					{/if}
				</div>

				<!-- Action: view all generations -->
				<a href="/public/{gen.creator_username}" class="card hover:border-border-active transition-colors flex items-center gap-3">
					<div class="w-10 h-10 rounded-full bg-bg-secondary flex items-center justify-center">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
							<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
							<circle cx="8.5" cy="8.5" r="1.5"></circle>
							<polyline points="21 15 16 10 5 21"></polyline>
						</svg>
					</div>
					<div>
						<p class="text-sm font-medium text-text-primary">View all generations</p>
						<p class="text-xs text-text-muted">by {gen.creator_username}</p>
					</div>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-text-muted ml-auto" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="9 18 15 12 9 6"></polyline>
					</svg>
				</a>
			</div>
		</div>
	</div>
</main>
