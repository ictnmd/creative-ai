<script lang="ts">
	import clsx from 'clsx';
	import StatusBadge from './StatusBadge.svelte';
	import type { GenerationStatus as Status } from '$lib/types';
	// Flexible props - use whichever are needed
	export let src: string | undefined = undefined;
	export let alt: string = '';
	export let imageUrl: string | undefined = undefined;
	export let prompt: string | undefined = undefined;
	export let status: Status = 'completed';
	export let createdAt: string | undefined = undefined;
	export let timestamp: Date | string | undefined = undefined;
	export let model: string | undefined = undefined;
	export let selected = false;

	// Use imageUrl if provided, fall back to src
	$: displaySrc = imageUrl ?? src;
	$: displayAlt = alt || prompt || 'Generated image';

	function formatTime(date: Date | string | undefined): string {
		if (!date) return '';
		try {
			const d = typeof date === 'string' ? new Date(date) : date;
			const now = new Date();
			const diff = now.getTime() - d.getTime();
			const minutes = Math.floor(diff / 60000);
			const hours = Math.floor(diff / 3600000);
			const days = Math.floor(diff / 86400000);

			if (minutes < 1) return 'Just now';
			if (minutes < 60) return `${minutes}m ago`;
			if (hours < 24) return `${hours}h ago`;
			if (days < 7) return `${days}d ago`;
			return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
		} catch {
			return '';
		}
	}

	function formatDate(dateStr: string): string {
		try {
			const date = new Date(dateStr);
			return date.toLocaleDateString('en-US', {
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return dateStr;
		}
	}

	$: timeLabel = formatTime(timestamp ?? createdAt);
</script>

<div
	class={clsx(
		'relative rounded-xl overflow-hidden card p-0 cursor-pointer group',
		selected && 'ring-2 ring-accent border-border-active'
	)}
	on:click
	on:keydown
	role="button"
	tabindex="0"
>
	<!-- Image -->
	<div class="relative aspect-square bg-bg-secondary">
		{#if displaySrc}
			<img src={displaySrc} alt={displayAlt} class="w-full h-full object-cover" loading="lazy" />
		{:else}
			<div class="absolute inset-0 flex items-center justify-center">
				{#if status === 'processing' || status === 'queued'}
					<div class="absolute inset-0 bg-gradient-to-br from-[var(--accent-secondary)]/20 via-[var(--accent)]/20 to-[var(--accent-glow)]/20 animate-pulse"></div>
					<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="32" height="32" class="text-text-muted z-10">
						<path stroke-linecap="round" stroke-linejoin="round" d="M9.813 15.904L9 18.75l-.813-2.846a4.5 4.5 0 00-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 003.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 003.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 00-3.09 3.09zM18.259 8.715L18 9.75l-.259-1.035a3.375 3.375 0 00-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 002.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 002.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 00-2.456 2.456z" />
					</svg>
				{:else if status === 'failed'}
					<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="32" height="32" class="text-[var(--error)]/50">
						<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
					</svg>
				{:else}
					<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="32" height="32" class="text-text-muted">
						<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" />
					</svg>
				{/if}
			</div>
		{/if}

		<!-- Status Badge -->
		<div class="absolute top-2 right-2">
			<StatusBadge {status} />
		</div>

		<!-- Hover Overlay -->
		{#if prompt}
			<div
				class="absolute inset-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-3"
			>
				<p class="text-xs text-white line-clamp-3">{prompt}</p>
				{#if timeLabel}
					<p class="text-[10px] text-white/60 mt-1">{timeLabel}</p>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.line-clamp-3 {
		display: -webkit-box;
		-webkit-line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
		line-clamp: 3;
	}
</style>
