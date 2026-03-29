<script lang="ts">
	import { goto } from '$app/navigation';
	import { toast } from '$stores/toast';
	import { api } from '$lib/api/client';

	let username = '';
	let email = '';
	let password = '';
	let confirmPassword = '';
	let loading = false;
	let error = '';

	const PASSWORD_MIN_LENGTH = 8;

	async function handleSubmit() {
		error = '';

		if (password !== confirmPassword) {
			error = 'Passwords do not match.';
			return;
		}

		if (password.length < PASSWORD_MIN_LENGTH) {
			error = `Password must be at least ${PASSWORD_MIN_LENGTH} characters.`;
			return;
		}

		loading = true;

		try {
			await api.post<{ user: { id: string; email: string; username: string; role: string } }>(
				'/api/v1/auth/register',
				{ username, email, password }
			);
			toast.success('Account created! Please sign in.');
			await goto('/auth/login');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			error = apiErr.message || 'Registration failed. Please try again.';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Sign Up - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary flex items-center justify-center px-4 py-8">
	<div class="w-full max-w-md">
		<div class="card">
			<!-- Header -->
			<div class="text-center mb-8">
				<h1 class="text-2xl font-bold text-text-primary">Create Account</h1>
				<p class="text-text-secondary text-sm mt-2">Join Creative AI Studio today</p>
			</div>

			<!-- Registration form -->
			<form on:submit|preventDefault={handleSubmit} class="space-y-4">
				{#if error}
					<div class="p-3 rounded-md bg-[var(--error)]/10 border border-[var(--error)]/30 text-sm text-[var(--error)]">
						{error}
					</div>
				{/if}

				<div>
					<label for="username" class="block text-sm text-text-secondary mb-1">Username</label>
					<input
						type="text"
						id="username"
						bind:value={username}
						class="input"
						placeholder="your_username"
						minlength="3"
						maxlength="30"
						pattern="[a-zA-Z0-9_-]+"
						title="Letters, numbers, underscores, and hyphens only"
						required
						disabled={loading}
					/>
					<p class="text-xs text-text-muted mt-1">Letters, numbers, underscores, hyphens. 3-30 characters.</p>
				</div>

				<div>
					<label for="email" class="block text-sm text-text-secondary mb-1">Email</label>
					<input
						type="email"
						id="email"
						bind:value={email}
						class="input"
						placeholder="you@example.com"
						required
						disabled={loading}
					/>
				</div>

				<div>
					<label for="password" class="block text-sm text-text-secondary mb-1">Password</label>
					<input
						type="password"
						id="password"
						bind:value={password}
						class="input"
						placeholder="Create a strong password"
						minlength={PASSWORD_MIN_LENGTH}
						required
						disabled={loading}
					/>
					<p class="text-xs text-text-muted mt-1">
						Minimum {PASSWORD_MIN_LENGTH} characters. Include letters, numbers, and symbols.
					</p>
				</div>

				<div>
					<label for="confirmPassword" class="block text-sm text-text-secondary mb-1">Confirm Password</label>
					<input
						type="password"
						id="confirmPassword"
						bind:value={confirmPassword}
						class="input"
						placeholder="Re-enter your password"
						required
						disabled={loading}
					/>
				</div>

				<div class="flex items-start gap-2 text-xs text-text-muted">
					<input type="checkbox" required class="accent-[var(--accent)] mt-0.5" />
					<span>
						I agree to the
						<a href="/terms" class="text-accent hover:underline">Terms of Service</a>
						and
						<a href="/privacy" class="text-accent hover:underline">Privacy Policy</a>
					</span>
				</div>

				<button
					type="submit"
					class="btn-primary w-full"
					disabled={loading}
				>
					{#if loading}
						<span class="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
					{/if}
					Create Account
				</button>
			</form>

			<p class="text-center text-sm text-text-secondary mt-6">
				Already have an account?
				<a href="/auth/login" class="text-accent hover:underline">Sign in</a>
			</p>
		</div>
	</div>
</main>
