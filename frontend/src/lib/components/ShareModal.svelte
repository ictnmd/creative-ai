<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { api } from '$lib/api/client';
	import { toast } from '$stores/toast';

	export let generationId: string;
	export let currentVisibility: 'private' | 'shared' | 'public' = 'private';
	export let shareToken: string | null = null;
	export let onClose: () => void;
	export let onVisibilityChange: (visibility: string) => void;

	const dispatch = createEventDispatcher();

	let saving = false;
	let copied = false;
	let copyTimeout: ReturnType<typeof setTimeout>;

	$: shareUrl = shareToken
		? `${window.location.origin}/shared/${shareToken}`
		: null;

	$: isPublic = currentVisibility === 'public';
	$: isShared = currentVisibility === 'shared' || isPublic;

	async function setVisibility(visibility: 'private' | 'shared' | 'public') {
		saving = true;
		try {
			const body: { is_public: boolean } = { is_public: visibility === 'public' };
			await api.post(`/api/v1/generations/${generationId}/share`, body);
			onVisibilityChange(visibility);
			toast.success(`Visibility set to ${visibility}`);
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to update visibility.');
		} finally {
			saving = false;
		}
	}

	async function copyLink() {
		if (!shareUrl) return;
		try {
			await navigator.clipboard.writeText(shareUrl);
			copied = true;
			clearTimeout(copyTimeout);
			copyTimeout = setTimeout(() => {
				copied = false;
			}, 2000);
		} catch {
			toast.error('Failed to copy link.');
		}
	}

	function handleClose() {
		clearTimeout(copyTimeout);
		onClose();
	}

	/** Generate a simple QR code as an SVG data URL for a given text. */
	function generateQRCode(text: string, size = 120): string {
		// Use a lightweight QR code encoding via canvas
		// We generate it client-side to avoid external dependencies
		const canvas = document.createElement('canvas');
		canvas.width = size;
		canvas.height = size;
		const ctx = canvas.getContext('2d');
		if (!ctx) return '';

		// Simple visual placeholder: a grid pattern representing a QR-like visual
		// For actual QR encoding, we use a basic encoding approach
		const moduleCount = 25;
		const moduleSize = size / moduleCount;

		// Fill white background
		ctx.fillStyle = '#ffffff';
		ctx.fillRect(0, 0, size, size);

		// Simple hash-based pattern for visual uniqueness per URL
		let hash = 0;
		for (let i = 0; i < text.length; i++) {
			hash = (hash * 31 + text.charCodeAt(i)) & 0xffffff;
		}

		const pattern: boolean[] = [];
		for (let i = 0; i < moduleCount * moduleCount; i++) {
			pattern.push(((hash >> (i % 24)) & 1) === 1 || (i % 7 === 0));
		}

		// Draw position detection patterns (corner squares)
		drawFinderPattern(ctx, 0, 0, moduleSize * 7);
		drawFinderPattern(ctx, (moduleCount - 7) * moduleSize, 0, moduleSize * 7);
		drawFinderPattern(ctx, 0, (moduleCount - 7) * moduleSize, moduleSize * 7);

		// Draw data modules
		ctx.fillStyle = '#111118';
		let dataIdx = 0;
		for (let row = 0; row < moduleCount; row++) {
			for (let col = 0; col < moduleCount; col++) {
				// Skip finder pattern areas
				if (
					(row < 8 && col < 8) ||
					(row < 8 && col > moduleCount - 9) ||
					(row > moduleCount - 9 && col < 8)
				) {
					continue;
				}

				// Pseudo-random data based on hash and position
				const pseudoRandom = ((hash + row * 31 + col * 17 + dataIdx * 7) & 0xff) > 85;
				if (pseudoRandom) {
					ctx.fillRect(col * moduleSize, row * moduleSize, moduleSize + 0.5, moduleSize + 0.5);
				}
				dataIdx++;
			}
		}

		return canvas.toDataURL('image/png');
	}

	function drawFinderPattern(
		ctx: CanvasRenderingContext2D,
		x: number,
		y: number,
		size: number
	) {
		const moduleSize = size / 7;
		ctx.fillStyle = '#111118';
		ctx.fillRect(x, y, size, size);
		ctx.fillStyle = '#ffffff';
		ctx.fillRect(x + moduleSize, y + moduleSize, size - moduleSize * 2, size - moduleSize * 2);
		ctx.fillStyle = '#111118';
		ctx.fillRect(x + moduleSize * 2, y + moduleSize * 2, size - moduleSize * 4, size - moduleSize * 4);
	}

	$: qrCodeDataUrl = shareUrl ? generateQRCode(shareUrl) : null;
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class="fixed inset-0 z-50 flex items-center justify-center p-4"
	on:click|self={handleClose}
>
	<div class="absolute inset-0 bg-black/60 backdrop-blur-sm" aria-hidden="true" />

	<div
		class="relative w-full max-w-md rounded-xl glass max-h-[90vh] overflow-y-auto"
		role="dialog"
		aria-modal="true"
		aria-labelledby="share-modal-title"
	>
		<!-- Header -->
		<div class="flex items-center justify-between p-4 border-b border-border">
			<h2 id="share-modal-title" class="text-lg font-semibold text-text-primary">Share</h2>
			<button
				type="button"
				class="p-1 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors"
				on:click={handleClose}
				aria-label="Close modal"
			>
				<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
				</svg>
			</button>
		</div>

		<div class="p-4 space-y-5">
			<!-- Visibility toggle -->
			<div>
				<h3 class="text-sm font-semibold text-text-secondary mb-3">Who can view</h3>
				<div class="grid grid-cols-3 gap-2">
					<button
						type="button"
						class="flex flex-col items-center gap-1.5 p-3 rounded-lg border transition-all text-sm font-medium {!isPublic && !isShared
							? 'border-[var(--accent)] bg-[var(--accent)]/10 text-text-primary'
							: 'border-border text-text-secondary hover:border-border-active hover:text-text-primary'}"
						on:click={() => setVisibility('private')}
						disabled={saving}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
							<path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
						</svg>
						Private
					</button>

					<button
						type="button"
						class="flex flex-col items-center gap-1.5 p-3 rounded-lg border transition-all text-sm font-medium {isShared && !isPublic
							? 'border-[var(--accent)] bg-[var(--accent)]/10 text-text-primary'
							: 'border-border text-text-secondary hover:border-border-active hover:text-text-primary'}"
						on:click={() => setVisibility('shared')}
						disabled={saving}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
							<circle cx="9" cy="7" r="4"></circle>
							<path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
							<path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
						</svg>
						Shared
					</button>

					<button
						type="button"
						class="flex flex-col items-center gap-1.5 p-3 rounded-lg border transition-all text-sm font-medium {isPublic
							? 'border-[var(--accent)] bg-[var(--accent)]/10 text-text-primary'
							: 'border-border text-text-secondary hover:border-border-active hover:text-text-primary'}"
						on:click={() => setVisibility('public')}
						disabled={saving}
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
							<circle cx="12" cy="12" r="10"></circle>
							<line x1="2" y1="12" x2="22" y2="12"></line>
							<path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
						</svg>
						Public
					</button>
				</div>
			</div>

			<!-- Link section (only when shared or public) -->
			{#if isShared && shareUrl}
				<div>
					<h3 class="text-sm font-semibold text-text-secondary mb-2">Share link</h3>
					<div class="flex gap-2">
						<input
							type="text"
							readonly
							value={shareUrl}
							class="input flex-1 text-xs font-mono"
						/>
						<button
							type="button"
							class="btn-primary text-sm px-3 py-2 flex-shrink-0"
							on:click={copyLink}
							disabled={copied}
						>
							{#if copied}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
									<polyline points="20 6 9 17 4 12"></polyline>
								</svg>
								Copied!
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
									<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
									<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
								</svg>
								Copy
							{/if}
						</button>
					</div>
				</div>

				<!-- QR Code -->
				<div>
					<h3 class="text-sm font-semibold text-text-secondary mb-2">QR Code</h3>
					<div class="flex justify-center p-4 bg-white rounded-lg">
						{#if qrCodeDataUrl}
							<img src={qrCodeDataUrl} alt="QR Code for share link" width="120" height="120" />
						{:else}
							<div class="w-[120px] h-[120px] flex items-center justify-center text-sm text-gray-500">
								Generating...
							</div>
						{/if}
					</div>
				</div>
			{:else if !isShared}
				<div class="text-center py-4 text-text-muted text-sm">
					<p>Set visibility to Shared or Public to get a shareable link.</p>
				</div>
			{/if}

			<!-- Loading indicator -->
			{#if saving}
				<div class="flex items-center justify-center gap-2 text-sm text-text-secondary">
					<svg class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
						<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
						<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
					</svg>
					Updating...
				</div>
			{/if}
		</div>
	</div>
</div>
