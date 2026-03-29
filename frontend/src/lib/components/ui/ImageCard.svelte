<script lang="ts">
	import clsx from 'clsx';
	import StatusBadge from './StatusBadge.svelte';

	export let src: string;
	export let alt: string;
	export let prompt: string | undefined = undefined;
	export let status: 'pending' | 'processing' | 'completed' | 'failed' = 'completed';
	export let createdAt: string | undefined = undefined;
	export let selected = false;

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
</script>

<div
	class={clsx(
		'relative rounded-xl overflow-hidden card p-0 cursor-pointer group',
		selected && 'ring-2 ring-accent border-border-active'
	)}
>
	<!-- Image -->
	<div class="relative aspect-square bg-bg-secondary">
		<img {src} {alt} class="w-full h-full object-cover" loading="lazy" />

		<!-- Status Badge -->
		<div class="absolute top-2 left-2">
			<StatusBadge {status} />
		</div>

		<!-- Hover Overlay -->
		{#if prompt}
			<div
				class="absolute inset-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent opacity-0 group-hover:opacity-100 transition-opacity flex flex-col justify-end p-3"
			>
				<p class="text-xs text-white line-clamp-3">{prompt}</p>
				{#if createdAt}
					<p class="text-[10px] text-white/60 mt-1">{formatDate(createdAt)}</p>
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
