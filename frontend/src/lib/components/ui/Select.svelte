<script lang="ts">
	import clsx from 'clsx';

	export let label: string | undefined = undefined;
	export let error: string | undefined = undefined;
	export let options: { value: string | number; label: string }[] = [];
	export let value: string | number = '';
	export let disabled = false;

	let selectedLabel = options.find((o) => String(o.value) === String(value))?.label ?? '';
</script>

<div class="flex flex-col gap-1">
	{#if label}
		<label for="select-{label?.replace(/\s+/g, '-').toLowerCase()}" class="text-sm font-medium text-text-primary">
			{label}
		</label>
	{/if}

	<div class="relative">
		<select
			id="select-{label?.replace(/\s+/g, '-').toLowerCase()}"
			bind:value
			{disabled}
			class={clsx(
				'input appearance-none pr-10 cursor-pointer',
				error && 'border-error focus:border-error focus:ring-error/30',
				disabled && 'opacity-50 cursor-not-allowed'
			)}
		>
			{#each options as option}
				<option value={option.value}>{option.label}</option>
			{/each}
		</select>

		<div
			class="absolute inset-y-0 right-0 flex items-center pr-3 pointer-events-none text-text-muted"
		>
			<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
			</svg>
		</div>
	</div>

	{#if error}
		<p class="text-xs text-error">{error}</p>
	{/if}
</div>
