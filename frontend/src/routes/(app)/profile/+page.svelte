<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$stores/auth';
	import { toast } from '$stores/toast';
	import { api } from '$lib/api/client';

	// Form fields
	let name = '';
	let bio = '';
	let email = '';
	let role = '';
	let isVerified = false;
	let isActive = false;
	let avatarUrl = '';
	let createdAt = '';

	// UI state
	let saving = false;
	let loading = true;

	onMount(async () => {
		// Populate form from auth store if available
		if ($auth.user) {
			name = $auth.user.username || '';
			bio = $auth.user.bio || '';
			email = $auth.user.email || '';
			role = $auth.user.role || '';
			isVerified = $auth.user.is_verified || false;
			isActive = $auth.user.is_active || false;
			avatarUrl = $auth.user.avatar_url || '';
			createdAt = $auth.user.created_at || '';
		}

		// Fetch latest profile from API
		try {
			const response = await api.get<{
				id: string;
				username: string;
				email: string;
				role: string;
				is_verified: boolean;
				is_active: boolean;
				avatar_url?: string;
				bio?: string;
				created_at?: string;
			}>('/api/v1/user/profile');

			const profile = response.data;
			name = profile.username || name;
			bio = profile.bio || '';
			email = profile.email || email;
			role = profile.role || role;
			isVerified = profile.is_verified ?? isVerified;
			isActive = profile.is_active ?? isActive;
			avatarUrl = profile.avatar_url || '';
			createdAt = profile.created_at || createdAt;

			// Sync back to auth store
			auth.setUser({ ...$auth.user!, ...profile });
		} catch {
			toast.error('Failed to load profile.');
		} finally {
			loading = false;
		}
	});

	async function handleSave() {
		saving = true;

		try {
			const response = await api.put<{ user: { id: string; username: string; bio?: string } }>(
				'/api/v1/user/profile',
				{ username: name, bio }
			);

			// Update auth store with saved data
			if (response.data.user) {
				auth.setUser({ ...$auth.user!, ...response.data.user });
			}

			toast.success('Profile saved successfully.');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to save profile.');
		} finally {
			saving = false;
		}
	}

	/** Get initials from username for avatar fallback. */
	function getInitials(username: string): string {
		return username
			.slice(0, 2)
			.toUpperCase();
	}

	/** Format a date string for display. */
	function formatDate(dateStr: string): string {
		if (!dateStr) return '—';
		try {
			return new Date(dateStr).toLocaleDateString('en-US', {
				year: 'numeric',
				month: 'long',
				day: 'numeric'
			});
		} catch {
			return dateStr;
		}
	}

	/** Role badge color mapping. */
	function roleBadgeClass(r: string): string {
		switch (r) {
			case 'admin': return 'bg-red-500/20 text-red-400 border-red-500/30';
			case 'pro': return 'bg-purple-500/20 text-purple-400 border-purple-500/30';
			case 'team': return 'bg-blue-500/20 text-blue-400 border-blue-500/30';
			default: return 'bg-gray-500/20 text-gray-400 border-gray-500/30';
		}
	}
</script>

<svelte:head>
	<title>Profile - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary">
	<div class="max-w-2xl mx-auto px-4 py-8">

		<!-- Page header -->
		<div class="mb-8">
			<h1 class="text-2xl font-bold text-text-primary">Profile Settings</h1>
			<p class="text-text-secondary text-sm mt-1">Manage your account information</p>
		</div>

		{#if loading}
			<!-- Loading state -->
			<div class="card flex items-center justify-center py-16">
				<div class="w-8 h-8 border-2 border-[var(--border)] border-t-[var(--accent)] rounded-full animate-spin"></div>
			</div>
		{:else}
			<div class="space-y-6">

				<!-- Avatar section -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Avatar</h2>
					<div class="flex items-center gap-4">
						{#if avatarUrl}
							<img
								src={avatarUrl}
								alt="Avatar"
								class="w-20 h-20 rounded-full object-cover border-2 border-[var(--border)]"
							/>
						{:else}
							<div
								class="w-20 h-20 rounded-full flex items-center justify-center text-lg font-bold text-white border-2 border-[var(--border)]"
								style="background: var(--accent-gradient);"
							>
								{getInitials(name || 'U')}
							</div>
						{/if}
						<div>
							<p class="text-sm text-text-secondary">Profile photo</p>
							<p class="text-xs text-text-muted mt-1">Avatar upload coming soon</p>
						</div>
					</div>
				</div>

				<!-- Account info (read-only) -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Account Information</h2>
					<dl class="space-y-3 text-sm">
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Email</dt>
							<dd class="text-text-primary font-mono text-xs">{email}</dd>
						</div>
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Role</dt>
							<dd>
								<span class="px-2 py-0.5 rounded text-xs border {roleBadgeClass(role)}">
									{role}
								</span>
							</dd>
						</div>
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Verified</dt>
							<dd class="text-text-primary">
								{#if isVerified}
									<span class="text-[var(--success)]">Yes</span>
								{:else}
									<span class="text-text-muted">Not verified</span>
								{/if}
							</dd>
						</div>
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Status</dt>
							<dd>
								{#if isActive}
									<span class="text-[var(--success)]">Active</span>
								{:else}
									<span class="text-[var(--error)]">Inactive</span>
								{/if}
							</dd>
						</div>
						{#if createdAt}
							<div class="flex justify-between items-center">
								<dt class="text-text-secondary">Member since</dt>
								<dd class="text-text-primary text-xs">{formatDate(createdAt)}</dd>
							</div>
						{/if}
					</dl>
				</div>

				<!-- Editable profile form -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Public Profile</h2>
					<form on:submit|preventDefault={handleSave} class="space-y-4">
						<div>
							<label for="name" class="block text-sm text-text-secondary mb-1">Username</label>
							<input
								type="text"
								id="name"
								bind:value={name}
								class="input"
								placeholder="Your username"
								minlength="3"
								maxlength="30"
								required
								disabled={saving}
							/>
						</div>

						<div>
							<label for="bio" class="block text-sm text-text-secondary mb-1">
								Bio
								<span class="text-text-muted font-normal">(optional)</span>
							</label>
							<textarea
								id="bio"
								bind:value={bio}
								class="input resize-y min-h-[80px]"
								placeholder="Tell us a little about yourself..."
								maxlength="500"
								rows="3"
								disabled={saving}
							></textarea>
							<p class="text-xs text-text-muted mt-1 text-right">{bio.length}/500</p>
						</div>

						<div class="flex justify-end pt-2">
							<button
								type="submit"
								class="btn-primary"
								disabled={saving}
							>
								{#if saving}
									<span class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
								{/if}
								Save Changes
							</button>
						</div>
					</form>
				</div>

				<!-- Danger zone -->
				<div class="card border-[var(--error)]/30">
					<h2 class="text-sm font-semibold text-[var(--error)] mb-2">Danger Zone</h2>
					<p class="text-sm text-text-secondary mb-4">
						Permanently delete your account and all associated data.
						This action cannot be undone.
					</p>
					<button
						type="button"
						class="px-4 py-2 rounded-md text-sm font-medium border border-[var(--error)]/50 text-[var(--error)] hover:bg-[var(--error)]/10 transition-colors"
						on:click={async () => {
							if (!confirm('Are you sure you want to delete your account? This cannot be undone.')) return;
							try {
								await api.post('/api/v1/user/delete');
								auth.clearUser();
								toast.info('Account deletion scheduled. You have 30 days to cancel.');
								await goto('/');
							} catch (err: unknown) {
								const apiErr = err as { message?: string };
								toast.error(apiErr.message || 'Failed to delete account.');
							}
						}}
					>
						Delete Account
					</button>
				</div>

			</div>
		{/if}
	</div>
</main>
