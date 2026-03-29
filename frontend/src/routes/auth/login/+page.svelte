<script lang="ts">
</script>

<svelte:head>
	<title>Sign In - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary flex items-center justify-center px-4">
	<div class="card w-full max-w-md">
		<h1 class="text-2xl font-bold text-text-primary mb-6 text-center">Sign In</h1>
		<form method="POST" class="space-y-4">
			<div>
				<label for="email" class="block text-sm text-text-secondary mb-1">Email</label>
				<input
					type="email"
					id="email"
					name="email"
					class="input"
					placeholder="you@example.com"
					required
				/>
			</div>
			<div>
				<label for="password" class="block text-sm text-text-secondary mb-1">Password</label>
				<input
					type="password"
					id="password"
					name="password"
					class="input"
					placeholder="Enter your password"
					required
				/>
			</div>
			<button type="submit" class="btn-primary w-full">Sign In</button>
		</form>
		<p class="text-center text-sm text-text-secondary mt-6">
			Don't have an account?
			<a href="/auth/register" class="text-accent hover:underline">Sign up</a>
		</p>
	</div>
</main>
