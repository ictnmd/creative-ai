<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import FollowButton from './FollowButton.svelte';

	export let username: string;
	export let initialMode: 'followers' | 'following' = 'followers';
	export let initialFollowersCount: number = 0;
	export let initialFollowingCount: number = 0;
	export let currentUserUsername: string | null = null;

	type Mode = 'followers' | 'following';

	interface FollowUser {
		username: string;
		name: string;
		avatar_url: string | null;
		is_following?: boolean;
	}

	interface PaginatedResponse {
		items: FollowUser[];
		total: number;
		page: number;
		per_page: number;
		has_more: boolean;
	}

	const PAGE_SIZE = 20;

	let mode: Mode = initialMode;
	let page = 1;
	let users: FollowUser[] = [];
	let loading = false;
	let hasMore = false;
	let total = initialMode === 'followers' ? initialFollowersCount : initialFollowingCount;

	async function loadUsers() {
		loading = true;
		try {
			const endpoint =
				mode === 'followers'
					? `/api/v1/users/${username}/followers`
					: `/api/v1/users/${username}/following`;
			const response = await api.get<PaginatedResponse>(`${endpoint}?page=${page}&per_page=${PAGE_SIZE}`);
			users = response.data.items;
			hasMore = response.data.has_more;
			total = response.data.total;
		} catch {
			// Silently fail — the parent component manages error display
		} finally {
			loading = false;
		}
	}

	function switchMode(newMode: Mode) {
		if (newMode === mode) return;
		mode = newMode;
		page = 1;
		users = [];
		total = newMode === 'followers' ? initialFollowersCount : initialFollowingCount;
		loadUsers();
	}

	function prevPage() {
		if (page <= 1) return;
		page--;
		loadUsers();
	}

	function nextPage() {
		if (!hasMore) return;
		page++;
		loadUsers();
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	onMount(() => {
		loadUsers();
	});
</script>

<div class="space-y-4">
	<!-- Tabs -->
	<div class="flex gap-1 p-1 rounded-lg bg-bg-secondary border border-border">
		<button
			type="button"
			class="flex-1 px-4 py-2 rounded-md text-sm font-medium transition-all {mode === 'followers'
				? 'bg-bg-card text-text-primary shadow-sm'
				: 'text-text-secondary hover:text-text-primary'}"
			on:click={() => switchMode('followers')}
		>
			Followers
			<span class="ml-1 text-text-muted text-xs">{initialFollowersCount}</span>
		</button>
		<button
			type="button"
			class="flex-1 px-4 py-2 rounded-md text-sm font-medium transition-all {mode === 'following'
				? 'bg-bg-card text-text-primary shadow-sm'
				: 'text-text-secondary hover:text-text-primary'}"
			on:click={() => switchMode('following')}
		>
			Following
			<span class="ml-1 text-text-muted text-xs">{initialFollowingCount}</span>
		</button>
	</div>

	<!-- Loading skeleton -->
	{#if loading}
		<div class="space-y-3">
			{#each Array(5) as _}
				<div class="flex items-center gap-3 p-3 rounded-lg bg-bg-card border border-border animate-pulse">
					<div class="w-10 h-10 rounded-full bg-bg-secondary flex-shrink-0" />
					<div class="flex-1 space-y-1.5">
						<div class="h-3 bg-bg-secondary rounded w-24" />
						<div class="h-2.5 bg-bg-secondary rounded w-16" />
					</div>
					<div class="w-16 h-6 bg-bg-secondary rounded" />
				</div>
			{/each}
		</div>
	{:else if users.length === 0}
		<!-- Empty state -->
		<div class="flex flex-col items-center justify-center py-12 space-y-2 text-center">
			<div class="w-14 h-14 rounded-full bg-bg-card border border-border flex items-center justify-center">
				<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-6 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
					<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
					<circle cx="9" cy="7" r="4" />
					<path d="M23 21v-2a4 4 0 0 0-3-3.87" />
					<path d="M16 3.13a4 4 0 0 1 0 7.75" />
				</svg>
			</div>
			<p class="text-sm font-medium text-text-secondary">
				{mode === 'followers' ? 'No followers yet' : 'Not following anyone yet'}
			</p>
		</div>
	{:else}
		<!-- User list -->
		<div class="space-y-2">
			{#each users as user (user.username)}
				<div class="flex items-center gap-3 p-3 rounded-lg bg-bg-card border border-border hover:border-border-active transition-colors">
					<!-- Avatar -->
					<a href="/public/{user.username}" class="flex-shrink-0">
						{#if user.avatar_url}
							<img
								src={user.avatar_url}
								alt={user.name}
								class="w-10 h-10 rounded-full object-cover border border-border"
							/>
						{:else}
							<div
								class="w-10 h-10 rounded-full flex items-center justify-center text-xs font-bold text-white border border-border"
								style="background: var(--accent-gradient);"
							>
								{getInitials(user.name || user.username)}
							</div>
						{/if}
					</a>

					<!-- Name & username -->
					<a href="/public/{user.username}" class="flex-1 min-w-0">
						<p class="text-sm font-medium text-text-primary truncate">{user.name || user.username}</p>
						<p class="text-xs text-text-muted truncate">@{user.username}</p>
					</a>

					<!-- Follow button (hide if viewing own list) -->
					{#if currentUserUsername && currentUserUsername !== user.username}
						<FollowButton username={user.username} initialIsFollowing={user.is_following ?? false} size="sm" />
					{/if}
				</div>
			{/each}
		</div>

		<!-- Pagination -->
		{#if total > PAGE_SIZE || page > 1}
			<div class="flex items-center justify-between pt-2">
				<p class="text-xs text-text-muted">
					Page {page}
					{#if total > 0}
						<span class="mx-1">-</span>
						{total} total
					{/if}
				</p>
				<div class="flex gap-2">
					<button
						type="button"
						class="px-3 py-1.5 rounded-md text-xs font-medium border border-border text-text-secondary hover:border-border-active hover:text-text-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
						on:click={prevPage}
						disabled={page <= 1 || loading}
					>
						Previous
					</button>
					<button
						type="button"
						class="px-3 py-1.5 rounded-md text-xs font-medium border border-border text-text-secondary hover:border-border-active hover:text-text-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
						on:click={nextPage}
						disabled={!hasMore || loading}
					>
						Next
					</button>
				</div>
			</div>
		{/if}
	{/if}
</div>
