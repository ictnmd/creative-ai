<script lang="ts">
	import clsx from 'clsx';

	export let label: string | undefined = undefined;
	export let error: string | undefined = undefined;
	export let hint: string | undefined = undefined;
	export let type: string = 'text';
	export let placeholder = '';
	export let value: string | number = '';
	export let disabled = false;
</script>

<div class="flex flex-col gap-1">
	{#if label}
		<label class="text-sm font-medium text-text-primary">
			{label}
		</label>
	{/if}

	{#if type === 'textarea'}
		<textarea
			{placeholder}
			{disabled}
			bind:value
			class={clsx(
				'input resize-none',
				error && 'border-error focus:border-error focus:ring-error/30'
			)}
			{...$$restProps}
		/>
	{:else if type === 'password'}
		<input
			type="password"
			{placeholder}
			{disabled}
			bind:value
			class={clsx(
				'input',
				error && 'border-error focus:border-error focus:ring-error/30'
			)}
			{...$$restProps}
		/>
	{:else if type === 'email'}
		<input
			type="email"
			{placeholder}
			{disabled}
			bind:value
			class={clsx(
				'input',
				error && 'border-error focus:border-error focus:ring-error/30'
			)}
			{...$$restProps}
		/>
	{:else if type === 'number'}
		<input
			type="number"
			{placeholder}
			{disabled}
			bind:value
			class={clsx(
				'input',
				error && 'border-error focus:border-error focus:ring-error/30'
			)}
			{...$$restProps}
		/>
	{:else}
		<input
			type="text"
			{placeholder}
			{disabled}
			bind:value
			class={clsx(
				'input',
				error && 'border-error focus:border-error focus:ring-error/30'
			)}
			{...$$restProps}
		/>
	{/if}

	{#if error}
		<p class="text-xs text-error">{error}</p>
	{:else if hint}
		<p class="text-xs text-text-muted">{hint}</p>
	{/if}
</div>
