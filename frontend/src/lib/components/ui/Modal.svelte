<script lang="ts">
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import clsx from 'clsx';

	export let open = false;
	export let title: string | undefined = undefined;
	export let size: 'sm' | 'md' | 'lg' | 'xl' | 'full' = 'md';

	const dispatch = createEventDispatcher<{ close: void }>();

	function close() {
		open = false;
		dispatch('close');
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && open) {
			close();
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}

	const sizeClasses = {
		sm: 'max-w-sm',
		md: 'max-w-lg',
		lg: 'max-w-2xl',
		xl: 'max-w-4xl',
		full: 'max-w-[95vw] max-h-[95vh]'
	};

	onMount(() => {
		document.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		document.removeEventListener('keydown', handleKeydown);
	});
</script>

{#if open}
	<!-- svelte-ignore a11y-click-events-have-key-events -->
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center p-4"
		on:click={handleBackdropClick}
	>
		<!-- Backdrop -->
		<div class="absolute inset-0 bg-black/60 backdrop-blur-sm" aria-hidden="true" />

		<!-- Modal Panel -->
		<div
			class={clsx(
				'relative w-full rounded-xl glass',
				sizeClasses[size],
				size !== 'full' && 'max-h-[90vh] overflow-y-auto'
			)}
			role="dialog"
			aria-modal="true"
			aria-labelledby={title ? 'modal-title' : undefined}
		>
			<!-- Header -->
			{#if title || $$slots.header}
				<div class="flex items-center justify-between p-4 border-b border-border">
					{#if $$slots.header}
						<slot name="header" />
					{:else}
						<h2 id="modal-title" class="text-lg font-semibold text-text-primary">{title}</h2>
					{/if}
					<button
						type="button"
						class="p-1 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors"
						on:click={close}
						aria-label="Close modal"
					>
						<svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</button>
				</div>
			{/if}

			<!-- Body -->
			<div class="p-4">
				<slot />
			</div>

			<!-- Footer -->
			{#if $$slots.footer}
				<div class="p-4 border-t border-border">
					<slot name="footer" />
				</div>
			{/if}
		</div>
	</div>
{/if}
