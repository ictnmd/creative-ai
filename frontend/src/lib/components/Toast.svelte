<script lang="ts">
	import { toast, toasts } from '$stores/toast';
	import type { Toast } from '$stores/toast';
	import { fly } from 'svelte/transition';
	import { flip } from 'svelte/animate';

	// ---------------------------------------------------------------------------
	// Helpers
	// ---------------------------------------------------------------------------

	/** SVG icons for each toast type. */
	const icons: Record<Toast['type'], string> = {
		success: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="18" height="18"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.857-9.809a.75.75 0 00-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 10-1.06 1.061l2.5 2.5a.75.75 0 001.137-.089l4-5.5z" clip-rule="evenodd" /></svg>`,
		error: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="18" height="18"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z" clip-rule="evenodd" /></svg>`,
		info: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="18" height="18"><path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a.75.75 0 000 1.5h.253a.25.25 0 01.244.304l-.459 2.066A1.75 1.75 0 0010.747 15H11a.75.75 0 000-1.5h-.253a.25.25 0 01-.244-.304l.459-2.066A1.75 1.75 0 009.253 9H9z" clip-rule="evenodd" /></svg>`,
		warning: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="18" height="18"><path fill-rule="evenodd" d="M8.485 2.495c.673-1.167 2.357-1.167 3.03 0l6.28 10.875c.673 1.167-.17 2.625-1.516 2.625H3.72c-1.347 0-2.189-1.458-1.515-2.625L8.485 2.495zM10 5a.75.75 0 01.75.75v3.5a.75.75 0 01-1.5 0v-3.5A.75.75 0 0110 5zm0 9a1 1 0 100-2 1 1 0 000 2z" clip-rule="evenodd" /></svg>`
	};

	/** CSS class per toast type (color + border). */
	const typeClasses: Record<Toast['type'], { border: string; icon: string; text: string; bg: string }> = {
		success: {
			border: 'border-[var(--success)]',
			icon: 'text-[var(--success)]',
			text: 'text-[var(--success)]',
			bg: 'bg-[var(--success)]/10'
		},
		error: {
			border: 'border-[var(--error)]',
			icon: 'text-[var(--error)]',
			text: 'text-[var(--error)]',
			bg: 'bg-[var(--error)]/10'
		},
		info: {
			border: 'border-[var(--info)]',
			icon: 'text-[var(--info)]',
			text: 'text-[var(--info)]',
			bg: 'bg-[var(--info)]/10'
		},
		warning: {
			border: 'border-[var(--warning)]',
			icon: 'text-[var(--warning)]',
			text: 'text-[var(--warning)]',
			bg: 'bg-[var(--warning)]/10'
		}
	};
</script>

<div
	class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none"
	aria-live="polite"
	aria-label="Notifications"
>
	{#each $toasts as t (t.id)}
		<div
			animate:flip={{ duration: 200 }}
			in:fly={{ x: 80, duration: 250 }}
			out:fly={{ x: 80, duration: 200, opacity: 0 }}
			class="pointer-events-auto flex items-start gap-3 px-4 py-3 rounded-lg border backdrop-blur-sm
				shadow-lg min-w-[280px] max-w-sm
				{typeClasses[t.type].bg}
				{typeClasses[t.type].border}"
			style="background-color: var(--bg-card);"
			role="alert"
		>
			<!-- Icon -->
			<span class={typeClasses[t.type].icon + ' flex-shrink-0 mt-0.5'}>
				{@html icons[t.type]}
			</span>

			<!-- Message -->
			<p class="flex-1 text-sm text-text-primary leading-snug">{t.message}</p>

			<!-- Dismiss button -->
			<button
				type="button"
				on:click={() => toast.remove(t.id)}
				class="flex-shrink-0 text-text-muted hover:text-text-primary transition-colors"
				aria-label="Dismiss notification"
			>
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" width="14" height="14">
					<path d="M3.72 3.72a.75.75 0 011.06 0L8 6.94l3.22-3.22a.75.75 0 111.06 1.06L9.06 8l3.22 3.22a.75.75 0 11-1.06 1.06L8 9.06l-3.22 3.22a.75.75 0 01-1.06-1.06L6.94 8 3.72 4.78a.75.75 0 010-1.06z" />
				</svg>
			</button>
		</div>
	{/each}
</div>
