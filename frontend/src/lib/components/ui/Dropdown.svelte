<script lang="ts">
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import clsx from 'clsx';

	export let align: 'left' | 'right' = 'left';

	const dispatch = createEventDispatcher<{ select: void }>();

	let isOpen = false;
	let dropdownRef: HTMLDivElement;

	function toggle() {
		isOpen = !isOpen;
	}

	function close() {
		isOpen = false;
	}

	function handleClickOutside(e: MouseEvent) {
		if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
			close();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
		}
	}

	onMount(() => {
		document.addEventListener('click', handleClickOutside);
		document.addEventListener('keydown', handleKeydown);
	});

	onDestroy(() => {
		document.removeEventListener('click', handleClickOutside);
		document.removeEventListener('keydown', handleKeydown);
	});

	$: alignClasses = align === 'right' ? 'right-0' : 'left-0';
</script>

<div class="relative inline-block" bind:this={dropdownRef}>
	<!-- Trigger -->
	<div role="button" tabindex="0" on:click={toggle} on:keydown={(e) => e.key === 'Enter' && toggle()}>
		<slot name="trigger" {isOpen} />
	</div>

	<!-- Menu -->
	{#if isOpen}
		<div
			class={clsx(
				'absolute z-50 top-full mt-2 min-w-[10rem] rounded-lg glass py-1 shadow-xl',
				alignClasses
			)}
			role="menu"
		>
			<slot {close} />
		</div>
	{/if}
</div>
