<script lang="ts">
</script>

<svelte:head>
	<title>Creative AI Studio</title>
	<meta name="description" content="AI-powered creative studio for generating and editing images and content" />
</svelte:head>

<main class="min-h-screen bg-bg-primary">
	<div class="flex flex-col items-center justify-center min-h-screen px-4">
		<div class="text-center">
			<h1 class="text-5xl font-bold bg-accent-gradient bg-clip-text text-transparent mb-4">
				Creative AI Studio
			</h1>
			<p class="text-text-secondary text-lg mb-8 max-w-md">
				AI-powered creative workspace for generating and editing images and content
			</p>
			<div class="flex gap-4 justify-center">
				<a href="/auth/login" class="btn-primary">
					Get Started
				</a>
				<a href="/auth/register" class="btn-secondary">
					Sign Up
				</a>
			</div>
		</div>
	</div>
</main>
