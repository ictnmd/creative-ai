<script lang="ts">
	import type { PageData } from './$types';
	import { isAuthenticated, currentUser } from '$stores/auth';
	import FollowButton from '$lib/components/FollowButton.svelte';
	import FollowerList from '$lib/components/FollowerList.svelte';
	import type { PublicProfile } from './+page';

	export let data: PageData;

	$: profile = data.profile as PublicProfile;

	let activeTab: 'generations' | 'followers' | 'following' = 'generations';

	function formatDate(dateStr: string): string {
		try {
			return new Date(dateStr).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'short',
				day: 'numeric'
			});
		} catch {
			return dateStr;
		}
	}

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}
</script>

<svelte:head>
	<title>{profile.name || profile.username} - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary">
	<!-- Profile header -->
	<div class="border-b border-border">
		<div class="max-w-5xl mx-auto px-4 py-8">
			<div class="flex items-start gap-6">
				<!-- Avatar -->
				{#if profile.avatar_url}
					<img
						src={profile.avatar_url}
						alt={profile.name}
						class="w-24 h-24 rounded-full object-cover border-2 border-border"
					/>
				{:else}
					<div
						class="w-24 h-24 rounded-full flex items-center justify-center text-2xl font-bold text-white border-2 border-border"
						style="background: var(--accent-gradient);"
					>
						{getInitials(profile.name || profile.username)}
					</div>
				{/if}

				<!-- Info -->
				<div class="flex-1 min-w-0">
					<div class="flex items-start justify-between gap-4">
						<div>
							<h1 class="text-xl font-bold text-text-primary">
								{profile.name || profile.username}
							</h1>
							<p class="text-sm text-text-secondary mt-0.5">@{profile.username}</p>
						</div>

						{#if $isAuthenticated}
							<FollowButton
								username={profile.username}
								initialIsFollowing={false}
								size="md"
							/>
						{/if}
					</div>

					{#if profile.bio}
						<p class="text-sm text-text-secondary mt-3 leading-relaxed max-w-xl">
							{profile.bio}
						</p>
					{/if}

					<!-- Stats -->
					<div class="flex gap-6 mt-4">
						<button
							type="button"
							class="group"
							on:click={() => activeTab = 'generations'}
						>
							<span class="text-base font-bold text-text-primary group-hover:text-white transition-colors">
								{profile.generation_count}
							</span>
							<span class="text-sm text-text-muted ml-1.5">generations</span>
						</button>

						<button
							type="button"
							class="group"
							on:click={() => activeTab = 'followers'}
						>
							<span class="text-base font-bold text-text-primary group-hover:text-white transition-colors">
								{profile.follower_count}
							</span>
							<span class="text-sm text-text-muted ml-1.5">followers</span>
						</button>

						<button
							type="button"
							class="group"
							on:click={() => activeTab = 'following'}
						>
							<span class="text-base font-bold text-text-primary group-hover:text-white transition-colors">
								{profile.following_count}
							</span>
							<span class="text-sm text-text-muted ml-1.5">following</span>
						</button>
					</div>
				</div>
			</div>
		</div>
	</div>

	<!-- Content -->
	<div class="max-w-5xl mx-auto px-4 py-6">
		{#if activeTab === 'generations'}
			{#if profile.public_generations.length === 0}
				<div class="flex flex-col items-center justify-center py-24 space-y-3">
					<div class="w-16 h-16 rounded-full bg-bg-card border border-border flex items-center justify-center">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-8 h-8 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
							<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
							<circle cx="8.5" cy="8.5" r="1.5"></circle>
							<polyline points="21 15 16 10 5 21"></polyline>
						</svg>
					</div>
					<div class="text-center">
						<h2 class="text-base font-semibold text-text-primary">No public generations</h2>
						<p class="text-text-secondary text-sm mt-1">
							{profile.username} hasn't shared any generations publicly yet.
						</p>
					</div>
				</div>
			{:else}
				<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
					{#each profile.public_generations as gen (gen.id)}
						<a
							href="/shared/{gen.share_token}"
							class="group block bg-bg-card rounded-lg border border-border overflow-hidden hover:border-border-active transition-all duration-micro"
						>
							<!-- Thumbnail -->
							<div class="relative aspect-video bg-bg-secondary overflow-hidden">
								{#if gen.thumbnail_url}
									<img
										src={gen.thumbnail_url}
										alt={gen.prompt || 'Generated image'}
										class="w-full h-full object-cover"
										loading="lazy"
									/>
								{:else}
									<div class="w-full h-full flex items-center justify-center">
										<svg xmlns="http://www.w3.org/2000/svg" class="w-10 h-10 text-text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
											<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
											<circle cx="8.5" cy="8.5" r="1.5"></circle>
											<polyline points="21 15 16 10 5 21"></polyline>
										</svg>
									</div>
								{/if}

								<!-- Hover overlay -->
								<div class="absolute inset-0 bg-black/80 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col items-center justify-center p-4">
									<p class="text-xs text-white/90 text-center line-clamp-4 leading-relaxed">
										{gen.prompt || 'No prompt'}
									</p>
								</div>
							</div>

							<!-- Footer -->
							<div class="p-3 space-y-1">
								<p class="text-xs text-text-secondary truncate">{gen.prompt || 'No prompt'}</p>
								<div class="flex items-center justify-between text-[10px] text-text-muted">
									<span>{formatDate(gen.created_at)}</span>
									<span class="flex items-center gap-1">
										<svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
											<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
											<circle cx="12" cy="12" r="3"></circle>
										</svg>
										{gen.view_count}
									</span>
								</div>
							</div>
						</a>
					{/each}
				</div>
			{/if}
		{:else if activeTab === 'followers'}
			<FollowerList
				username={profile.username}
				initialMode="followers"
				initialFollowersCount={profile.follower_count}
				initialFollowingCount={profile.following_count}
				currentUserUsername={$currentUser?.username ?? null}
			/>
		{:else if activeTab === 'following'}
			<FollowerList
				username={profile.username}
				initialMode="following"
				initialFollowersCount={profile.follower_count}
				initialFollowingCount={profile.following_count}
				currentUserUsername={$currentUser?.username ?? null}
			/>
		{/if}
	</div>
</main>
