<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { auth } from '$stores/auth';
	import Toast from '$lib/components/Toast.svelte';
	import { api } from '$lib/api/client';

	// Auth guard: redirect paths that do NOT require auth
	const PUBLIC_PATHS = ['/auth/login', '/auth/register', '/auth/forgot-password'];

	onMount(async () => {
		// Skip if already on a public path
		if (PUBLIC_PATHS.some((p) => $page.url.pathname.startsWith(p))) {
			auth.setInitialized(true);
			return;
		}

		// Skip if already authenticated
		if ($auth.initialized && $auth.user) {
			return;
		}

		auth.setLoading(true);

		try {
			const response = await api.get<{
				id: string;
				email: string;
				username: string;
				role: string;
				is_verified: boolean;
				is_active: boolean;
				avatar_url?: string;
				bio?: string;
				created_at?: string;
			}>('/api/v1/user/profile');

			auth.setUser(response.data);
		} catch {
			// Not authenticated - redirect to login
			auth.clearUser();
			await goto('/auth/login');
		} finally {
			auth.setLoading(false);
		}
	});
</script>

{#if $auth.loading && !$auth.initialized}
	<!-- Loading spinner during auth check -->
	<div class="min-h-screen bg-bg-primary flex items-center justify-center">
		<div class="flex flex-col items-center gap-4">
			<!-- Spinner -->
			<div class="w-10 h-10 rounded-full border-2 border-[var(--border)] border-t-[var(--accent)] animate-spin"></div>
			<p class="text-text-secondary text-sm">Loading...</p>
		</div>
	</div>
{:else}
	<slot />
	<Toast />
{/if}
