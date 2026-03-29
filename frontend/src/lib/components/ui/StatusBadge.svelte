<script lang="ts">
	import clsx from 'clsx';
	import type { GenerationStatus } from '$lib/types';

	export let status: GenerationStatus = 'pending';

	const config: Record<GenerationStatus, { label: string; class: string }> = {
		pending: {
			label: 'Pending',
			class: 'bg-info/20 text-info border-info/30'
		},
		queued: {
			label: 'Queued',
			class: 'bg-accent/20 text-accent border-accent/30'
		},
		processing: {
			label: 'Processing',
			class: 'bg-warning/20 text-warning border-warning/30'
		},
		completed: {
			label: 'Completed',
			class: 'bg-success/20 text-success border-success/30'
		},
		failed: {
			label: 'Failed',
			class: 'bg-error/20 text-error border-error/30'
		}
	};

	$: current = config[status];
</script>

<span
	class={clsx(
		'inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium border',
		current.class
	)}
>
	{#if status === 'processing'}
		<svg class="w-2.5 h-2.5 animate-spin" fill="none" viewBox="0 0 24 24">
			<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
			<path
				class="opacity-75"
				fill="currentColor"
				d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
			/>
		</svg>
	{/if}
	{current.label}
</span>
