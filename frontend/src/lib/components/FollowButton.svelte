<script lang="ts">
	import { api } from '$lib/api/client';
	import { toast } from '$stores/toast';

	export let username: string;
	export let initialIsFollowing: boolean = false;
	export let size: 'sm' | 'md' | 'lg' = 'md';

	type ButtonState = 'follow' | 'following' | 'unfollow' | 'loading';

	let isFollowing = initialIsFollowing;
	let state: ButtonState = initialIsFollowing ? 'following' : 'follow';
	let hovering = false;
	let loading = false;

	function getButtonLabel(): string {
		if (loading) return '';
		if (state === 'unfollow') return 'Unfollow';
		if (state === 'following' && hovering) return 'Unfollow';
		return 'Following';
	}

	async function handleClick() {
		if (loading) return;

		if (state === 'follow') {
			// Start following
			loading = true;
			state = 'loading';
			try {
				await api.post(`/api/v1/users/${username}/follow`);
				isFollowing = true;
				state = 'following';
			} catch (err: unknown) {
				const apiErr = err as { message?: string };
				toast.error(apiErr.message || 'Failed to follow user.');
				state = 'follow';
			} finally {
				loading = false;
			}
		} else if (state === 'following') {
			// Enter unfollow confirmation state
			state = 'unfollow';
		} else if (state === 'unfollow') {
			// Confirm unfollow
			loading = true;
			state = 'loading';
			try {
				await api.delete(`/api/v1/users/${username}/follow`);
				isFollowing = false;
				state = 'follow';
			} catch (err: unknown) {
				const apiErr = err as { message?: string };
				toast.error(apiErr.message || 'Failed to unfollow user.');
				state = 'following';
			} finally {
				loading = false;
			}
		}
	}

	const sizeClasses = {
		sm: 'px-3 py-1 text-xs',
		md: 'px-4 py-2 text-sm',
		lg: 'px-5 py-2.5 text-sm'
	};

	const iconSizes = {
		sm: 'w-3 h-3',
		md: 'w-4 h-4',
		lg: 'w-4 h-4'
	};

	$: label = getButtonLabel();
	$: isPrimary = state === 'follow';
</script>

<button
	type="button"
	class="
		inline-flex items-center justify-center gap-1.5 rounded-md font-medium transition-all
		{sizeClasses[size]}
		{isPrimary
			? 'bg-[var(--accent-gradient)] text-white shadow-md hover:shadow-lg hover:opacity-90 hover:-translate-y-px'
			: 'border font-medium hover:-translate-y-px'}
		{state === 'follow' || state === 'loading'
			? ''
			: state === 'unfollow'
				? 'border-[var(--error)]/50 text-[var(--error)] bg-[var(--error)]/10 hover:bg-[var(--error)]/20'
				: 'border-border text-text-primary bg-bg-card hover:border-border-active'}
		{loading ? 'opacity-70 cursor-wait' : 'hover:cursor-pointer'}
	"
	on:click={handleClick}
	on:mouseenter={() => (hovering = true)}
	on:mouseleave={() => (hovering = false)}
	disabled={loading}
>
	{#if loading}
		<svg
			class="animate-spin {iconSizes[size]}"
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
	{:else if state === 'follow'}
		<svg
			class="{iconSizes[size]} flex-shrink-0"
			xmlns="http://www.w3.org/2000/svg"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
			<circle cx="9" cy="7" r="4" />
			<line x1="19" y1="8" x2="19" y2="14" />
			<line x1="22" y1="11" x2="16" y2="11" />
		</svg>
		Follow
	{:else}
		{label}
	{/if}
</button>
