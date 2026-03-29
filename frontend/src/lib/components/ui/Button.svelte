<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import clsx from 'clsx';

	export let variant: 'primary' | 'secondary' | 'ghost' | 'danger' = 'primary';
	export let size: 'sm' | 'md' | 'lg' = 'md';
	export let loading = false;
	export let disabled = false;
	export let type: 'button' | 'submit' | 'reset' = 'button';

	const dispatch = createEventDispatcher<{ click: MouseEvent }>();

	function handleClick(e: MouseEvent) {
		if (!disabled && !loading) {
			dispatch('click', e);
		}
	}

	const sizeClasses = {
		sm: 'px-3 py-1.5 text-xs',
		md: 'px-4 py-2 text-sm',
		lg: 'px-6 py-3 text-base'
	};

	const variantClasses = {
		primary: 'btn-primary',
		secondary: 'btn-secondary',
		ghost: 'inline-flex items-center justify-center gap-2 px-4 py-2 rounded-md font-medium text-text-primary hover:bg-bg-hover transition-all',
		danger: 'inline-flex items-center justify-center gap-2 px-4 py-2 rounded-md font-medium bg-error text-white hover:bg-red-600 transition-all'
	};
</script>

<button
	{type}
	disabled={disabled || loading}
	class={clsx(
		'inline-flex items-center justify-center gap-2 rounded-md font-medium transition-all',
		variantClasses[variant],
		sizeClasses[size],
		(disabled || loading) && 'opacity-50 cursor-not-allowed pointer-events-none'
	)}
	on:click={handleClick}
	{...$$restProps}
>
	{#if loading}
		<svg
			class="animate-spin {size === 'sm' ? 'w-3 h-3' : size === 'lg' ? 'w-5 h-5' : 'w-4 h-4'}"
			xmlns="http://www.w3.org/2000/svg"
			fill="none"
			viewBox="0 0 24 24"
		>
			<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
			<path
				class="opacity-75"
				fill="currentColor"
				d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
			/>
		</svg>
	{/if}
	<slot />
</button>
