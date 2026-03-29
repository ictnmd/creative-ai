<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$stores/auth';
	import { toast } from '$stores/toast';
	import { billing, quotaPercentage, quotaStatus } from '$stores/billing';
	import { api } from '$lib/api/client';
	import { Button, Input, Modal, Spinner } from '$lib/components/ui';

	// ---------------------------------------------------------------------------
	// Local state
	// ---------------------------------------------------------------------------

	interface RecentGeneration {
		id: string;
		prompt: string;
		image_url?: string;
		status: 'pending' | 'processing' | 'completed' | 'failed';
		created_at: string;
	}

	// Mock / real data
	let username = $auth.user?.username || 'User';
	let avatarUrl = $auth.user?.avatar_url || '';

	// Billing data
	let creditBalance = 0;
	let generationsUsed = 0;
	let quotaLimit = 100;
	let subscriptionTier = 'Free';
	let subscriptionPrice = 0;
	let billingCycle = 'monthly';
	let recentActivity: RecentGeneration[] = [];

	// UI state
	let loading = true;
	let billingModalOpen = false;
	let billingTab: 'subscribe' | 'credits' = 'subscribe';

	// Subscribe tab
	let selectedPlan = '';
	const plans = [
		{
			id: 'starter',
			name: 'Starter',
			price: 9.99,
			generation_limit: 100,
			features: ['100 generations/month', 'Basic models', 'Standard resolution', 'Email support'],
			is_popular: false
		},
		{
			id: 'pro',
			name: 'Pro',
			price: 29.99,
			generation_limit: 500,
			features: [
				'500 generations/month',
				'All models',
				'High resolution',
				'Priority support',
				'Custom presets'
			],
			is_popular: true
		},
		{
			id: 'team',
			name: 'Team',
			price: 99.99,
			generation_limit: 2000,
			features: [
				'2000 generations/month',
				'All models',
				'Ultra resolution',
				'Dedicated support',
				'Team sharing',
				'API access'
			],
			is_popular: false
		}
	];

	// Credit packs
	const creditPacks = [
		{ id: 'pack-100', credits: 100, price: 4.99, label: '100 Credits' },
		{ id: 'pack-500', credits: 500, price: 19.99, label: '500 Credits' },
		{ id: 'pack-1000', credits: 1000, price: 34.99, label: '1000 Credits' }
	];
	let customCredits = 0;
	let purchasing = false;

	// ---------------------------------------------------------------------------
	// Helpers
	// ---------------------------------------------------------------------------

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

	function formatTimeAgo(dateStr: string): string {
		try {
			const date = new Date(dateStr);
			const now = new Date();
			const diff = Math.floor((now.getTime() - date.getTime()) / 1000);
			if (diff < 60) return `${diff}s ago`;
			if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
			if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
			return `${Math.floor(diff / 86400)}d ago`;
		} catch {
			return dateStr;
		}
	}

	function getQuotaColor(status: string): string {
		switch (status) {
			case 'critical':
				return 'bg-[var(--error)]';
			case 'warning':
				return 'bg-[var(--warning)]';
			default:
				return 'bg-accent-gradient';
		}
	}

	// ---------------------------------------------------------------------------
	// Data loading
	// ---------------------------------------------------------------------------

	onMount(async () => {
		if ($auth.user) {
			username = $auth.user.username;
			avatarUrl = $auth.user.avatar_url || '';
		}

		// Fetch balance
		await billing.fetchBalance();

		// Also try to load recent generations
		try {
			const response = await api.get<{ generations: RecentGeneration[] }>(
				'/api/v1/generations?limit=5'
			);
			recentActivity = response.data.generations || [];
		} catch {
			// Use mock data if backend not ready
			recentActivity = [
				{
					id: '1',
					prompt: 'A serene mountain landscape at golden hour',
					status: 'completed',
					created_at: new Date(Date.now() - 1000 * 60 * 15).toISOString()
				},
				{
					id: '2',
					prompt: 'Abstract geometric art with purple tones',
					status: 'completed',
					created_at: new Date(Date.now() - 1000 * 60 * 45).toISOString()
				},
				{
					id: '3',
					prompt: 'Futuristic city skyline cyberpunk',
					status: 'completed',
					created_at: new Date(Date.now() - 1000 * 60 * 120).toISOString()
				},
				{
					id: '4',
					prompt: 'Portrait in oil painting style',
					status: 'completed',
					created_at: new Date(Date.now() - 1000 * 60 * 240).toISOString()
				},
				{
					id: '5',
					prompt: 'Space nebula with colorful stars',
					status: 'completed',
					created_at: new Date(Date.now() - 1000 * 60 * 360).toISOString()
				}
			];
		}

		// Apply billing store data if available
		if ($billing.balance) {
			creditBalance = $billing.balance.credit_balance;
			generationsUsed = $billing.balance.generations_used;
			quotaLimit = $billing.balance.quota_limit;
			subscriptionTier = $billing.balance.subscription_tier;
		}

		loading = false;
	});

	// ---------------------------------------------------------------------------
	// Actions
	// ---------------------------------------------------------------------------

	async function handleSubscribe() {
		if (!selectedPlan) {
			toast.error('Please select a plan first.');
			return;
		}
		purchasing = true;
		try {
			const checkoutUrl = await billing.subscribeToPlan(selectedPlan);
			if (checkoutUrl) {
				window.location.href = checkoutUrl;
			} else {
				toast.error('Failed to initiate subscription. Please try again.');
			}
		} catch {
			toast.error('Failed to subscribe. Please try again.');
		} finally {
			purchasing = false;
		}
	}

	async function handlePurchasePack(packId: string) {
		purchasing = true;
		try {
			const checkoutUrl = await billing.purchaseCredits(packId);
			if (checkoutUrl) {
				window.location.href = checkoutUrl;
			} else {
				// Simulate purchase for demo
				toast.success('Credits purchased successfully! (Demo mode)');
				billingModalOpen = false;
				await billing.fetchBalance();
				creditBalance += 100;
			}
		} catch {
			toast.error('Failed to purchase credits. Please try again.');
		} finally {
			purchasing = false;
		}
	}

	async function handleCustomPurchase() {
		if (customCredits <= 0) {
			toast.error('Please enter a valid amount of credits.');
			return;
		}
		purchasing = true;
		try {
			const checkoutUrl = await billing.purchaseCustomCredits(customCredits);
			if (checkoutUrl) {
				window.location.href = checkoutUrl;
			} else {
				toast.success(`Purchased ${customCredits} credits! (Demo mode)`);
				billingModalOpen = false;
				await billing.fetchBalance();
				creditBalance += customCredits;
			}
		} catch {
			toast.error('Failed to purchase credits. Please try again.');
		} finally {
			purchasing = false;
		}
	}
</script>

<svelte:head>
	<title>Dashboard - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary">
	<div class="max-w-[800px] mx-auto px-4 py-8">

		<!-- Welcome header -->
		<div class="flex items-center gap-4 mb-8">
			{#if avatarUrl}
				<img
					src={avatarUrl}
					alt={username}
					class="w-14 h-14 rounded-full object-cover border-2 border-border"
				/>
			{:else}
				<div
					class="w-14 h-14 rounded-full flex items-center justify-center text-lg font-bold text-white border-2 border-border"
					style="background: var(--accent-gradient);"
				>
					{getInitials(username)}
				</div>
			{/if}
			<div>
				<h1 class="text-2xl font-bold text-text-primary">Welcome back, {username}!</h1>
				<p class="text-sm text-text-secondary mt-0.5">Here's an overview of your account</p>
			</div>
		</div>

		{#if loading}
			<div class="flex items-center justify-center py-20 gap-3">
				<Spinner size="lg" />
				<span class="text-sm text-text-secondary">Loading dashboard...</span>
			</div>
		{:else}
			<div class="space-y-8">

				<!-- Usage Stats row (3 cards) -->
				<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
					<!-- Generations This Month -->
					<div class="card">
						<div class="flex items-center gap-2 mb-2">
							<svg class="w-4 h-4 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
							</svg>
							<span class="text-xs text-text-secondary uppercase tracking-wide font-medium">This Month</span>
						</div>
						<p class="text-3xl font-bold text-text-primary">{generationsUsed}</p>
						<p class="text-xs text-text-muted mt-1">generations used</p>
						<div class="mt-3 h-1.5 rounded-full bg-bg-secondary overflow-hidden">
							<div
								class="h-full rounded-full transition-all duration-500 {getQuotaColor($quotaStatus)}"
								style="width: {Math.min($quotaPercentage, 100)}%"
							></div>
						</div>
					</div>

					<!-- Credit Balance -->
					<div class="card">
						<div class="flex items-center gap-2 mb-2">
							<svg class="w-4 h-4 text-[var(--success)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
							</svg>
							<span class="text-xs text-text-secondary uppercase tracking-wide font-medium">Balance</span>
						</div>
						<p class="text-3xl font-bold text-text-primary">{creditBalance}</p>
						<p class="text-xs text-text-muted mt-1">credits remaining</p>
					</div>

					<!-- Subscription -->
					<div class="card">
						<div class="flex items-center gap-2 mb-2">
							<svg class="w-4 h-4 text-[var(--info)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
							</svg>
							<span class="text-xs text-text-secondary uppercase tracking-wide font-medium">Plan</span>
						</div>
						<p class="text-2xl font-bold text-text-primary capitalize">{subscriptionTier}</p>
						<span
							class="inline-block mt-1 px-2 py-0.5 rounded text-xs font-medium border bg-[var(--accent)]/15 text-[var(--accent)] border-[var(--accent)]/25"
						>
							{subscriptionTier === 'free' ? 'Free Tier' : subscriptionTier}
						</span>
					</div>
				</div>

				<!-- Quota Usage section -->
				<div class="card">
					<div class="flex items-center justify-between mb-3">
						<h2 class="text-sm font-semibold text-text-primary">Quota Usage</h2>
						<span class="text-xs text-text-secondary">
							{generationsUsed} of {quotaLimit} generations used ({$quotaPercentage}%)
						</span>
					</div>
					<div class="h-3 rounded-full bg-bg-secondary overflow-hidden">
						<div
							class="h-full rounded-full transition-all duration-700 {getQuotaColor($quotaStatus)}"
							style="width: {Math.min($quotaPercentage, 100)}%"
						></div>
					</div>
					{#if $quotaStatus === 'warning'}
						<p class="text-xs text-[var(--warning)] mt-2">
							<svg class="w-3 h-3 inline mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
							</svg>
							You're approaching your monthly limit. Consider upgrading or buying credits.
						</p>
					{:else if $quotaStatus === 'critical'}
						<p class="text-xs text-[var(--error)] mt-2">
							<svg class="w-3 h-3 inline mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
							</svg>
							You've almost reached your limit. Upgrade now to continue generating.
						</p>
					{/if}
				</div>

				<!-- Subscription section -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Current Subscription</h2>
					<div class="flex items-center justify-between">
						<div>
							<div class="flex items-center gap-2">
								<p class="text-lg font-bold text-text-primary capitalize">{subscriptionTier}</p>
								<span
									class="px-2 py-0.5 rounded text-xs font-medium border bg-[var(--accent)]/15 text-[var(--accent)] border-[var(--accent)]/25"
								>
									{subscriptionTier === 'free' ? 'Free' : subscriptionTier}
								</span>
							</div>
							<p class="text-sm text-text-secondary mt-0.5">
								{#if subscriptionTier === 'free'}
									Free plan with limited generations
								{:else}
									${subscriptionPrice.toFixed(2)} / {billingCycle}
								{/if}
							</p>
						</div>
						<div class="flex items-center gap-2">
							<Button variant="secondary" size="sm" on:click={() => goto('/settings')}>
								Manage Billing
							</Button>
							<Button size="sm" on:click={() => { billingTab = 'subscribe'; billingModalOpen = true; }}>
								Upgrade Plan
							</Button>
						</div>
					</div>
				</div>

				<!-- Quick Actions row -->
				<div class="flex flex-wrap items-center gap-3">
					<Button
						size="sm"
						on:click={() => { billingTab = 'credits'; billingModalOpen = true; }}
					>
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
						</svg>
						Buy Credits
					</Button>
					<Button variant="secondary" size="sm" on:click={() => goto('/gallery')}>
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
						</svg>
						View Gallery
					</Button>
					<Button variant="ghost" size="sm" on:click={() => goto('/settings')}>
						<svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
						</svg>
						API Keys
					</Button>
				</div>

				<!-- Recent Activity -->
				<div class="card">
					<div class="flex items-center justify-between mb-4">
						<h2 class="text-sm font-semibold text-text-primary">Recent Activity</h2>
						<Button variant="ghost" size="sm" on:click={() => goto('/gallery')}>
							View All
						</Button>
					</div>

					{#if recentActivity.length === 0}
						<div class="text-center py-8">
							<svg class="w-10 h-10 mx-auto text-text-muted mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
							</svg>
							<p class="text-sm text-text-muted">No generations yet. Start creating!</p>
							<Button size="sm" class="mt-3" on:click={() => goto('/')}>
								Generate Image
							</Button>
						</div>
					{:else}
						<div class="space-y-3">
							{#each recentActivity as item (item.id)}
								<div class="flex items-center gap-3 p-2 rounded-lg hover:bg-bg-secondary transition-colors">
									<!-- Thumbnail -->
									<div class="w-12 h-12 rounded-lg overflow-hidden bg-bg-secondary flex-shrink-0 flex items-center justify-center">
										{#if item.image_url}
											<img src={item.image_url} alt="" class="w-full h-full object-cover" />
										{:else}
											<svg class="w-5 h-5 text-text-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
												<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
											</svg>
										{/if}
									</div>

									<!-- Prompt preview -->
									<div class="flex-1 min-w-0">
										<p class="text-sm text-text-primary truncate">{item.prompt}</p>
										<p class="text-xs text-text-muted mt-0.5">{formatTimeAgo(item.created_at)}</p>
									</div>

									<!-- Status -->
									<span
										class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium border flex-shrink-0 {item.status === 'completed' ? 'bg-[var(--success)]/15 text-[var(--success)] border-[var(--success)]/25' : item.status === 'failed' ? 'bg-[var(--error)]/15 text-[var(--error)] border-[var(--error)]/25' : item.status === 'processing' ? 'bg-[var(--info)]/15 text-[var(--info)] border-[var(--info)]/25' : 'bg-[var(--warning)]/15 text-[var(--warning)] border-[var(--warning)]/25'}"
									>
										{item.status}
									</span>
								</div>
							{/each}
						</div>
					{/if}
				</div>

			</div>
		{/if}
	</div>
</main>

<!-- Billing Modal -->
<Modal
	bind:open={billingModalOpen}
	title={billingTab === 'subscribe' ? 'Choose a Plan' : 'Buy Credits'}
	size="lg"
	on:close={() => (billingModalOpen = false)}
>
	<!-- Tab switcher -->
	<div class="flex gap-1 p-1 mb-6 rounded-lg bg-bg-secondary w-fit">
		<button
			class="px-4 py-1.5 rounded-md text-sm font-medium transition-all {billingTab === 'subscribe' ? 'bg-bg-card text-text-primary shadow-sm' : 'text-text-secondary hover:text-text-primary'}"
			on:click={() => (billingTab = 'subscribe')}
		>
			Subscribe
		</button>
		<button
			class="px-4 py-1.5 rounded-md text-sm font-medium transition-all {billingTab === 'credits' ? 'bg-bg-card text-text-primary shadow-sm' : 'text-text-secondary hover:text-text-primary'}"
			on:click={() => (billingTab = 'credits')}
		>
			Buy Credits
		</button>
	</div>

	{#if billingTab === 'subscribe'}
		<!-- Plan cards -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-6">
			{#each plans as plan}
				<div
					class="relative rounded-xl border-2 p-4 cursor-pointer transition-all {selectedPlan === plan.id ? 'border-[var(--accent)] bg-[var(--accent)]/5' : 'border-[var(--border)] hover:border-border-active'}"
					on:click={() => (selectedPlan = plan.id)}
					role="radio"
					aria-checked={selectedPlan === plan.id}
					tabindex="0"
					on:keydown={(e) => e.key === 'Enter' && (selectedPlan = plan.id)}
				>
					{#if plan.is_popular}
						<span class="absolute -top-2.5 left-1/2 -translate-x-1/2 px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-accent-gradient text-white">
							Popular
						</span>
					{/if}

					<div class="flex items-center justify-between mb-3">
						<h3 class="text-base font-bold text-text-primary">{plan.name}</h3>
						{#if selectedPlan === plan.id}
							<div class="w-5 h-5 rounded-full bg-[var(--accent)] flex items-center justify-center">
								<svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
								</svg>
							</div>
						{/if}
					</div>

					<div class="mb-3">
						<span class="text-2xl font-bold text-text-primary">${plan.price.toFixed(2)}</span>
						<span class="text-xs text-text-muted">/month</span>
					</div>

					<p class="text-xs text-text-secondary mb-3">{plan.generation_limit} generations/month</p>

					<ul class="space-y-1.5">
						{#each plan.features as feature}
							<li class="flex items-center gap-1.5 text-xs text-text-secondary">
								<svg class="w-3 h-3 text-[var(--success)] flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
								</svg>
								{feature}
							</li>
						{/each}
					</ul>
				</div>
			{/each}
		</div>

		<div class="flex items-center justify-between pt-4 border-t border-[var(--border)]">
			<p class="text-xs text-text-muted">Cancel anytime. No commitment.</p>
			<Button loading={purchasing} on:click={handleSubscribe}>
				Subscribe
			</Button>
		</div>
	{:else}
		<!-- Credit packs -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-6">
			{#each creditPacks as pack}
				<button
					class="card text-center cursor-pointer hover:border-border-active transition-all"
					on:click={() => handlePurchasePack(pack.id)}
				>
					<p class="text-lg font-bold text-text-primary">{pack.label}</p>
					<p class="text-2xl font-bold text-text-primary mt-2">
						${pack.price.toFixed(2)}
					</p>
					<p class="text-xs text-text-muted mt-1">
						${(pack.price / pack.credits).toFixed(3)}/credit
					</p>
				</button>
			{/each}
		</div>

		<!-- Custom amount -->
		<div class="border-t border-[var(--border)] pt-4 mt-4">
			<p class="text-sm font-medium text-text-primary mb-3">Custom Amount</p>
			<div class="flex items-center gap-3">
				<div class="flex-1">
					<Input
						type="number"
						placeholder="Enter number of credits"
						bind:value={customCredits}
						min={1}
					/>
				</div>
				<Button loading={purchasing} on:click={handleCustomPurchase}>
					Purchase
				</Button>
			</div>
			{#if customCredits > 0}
				<p class="text-xs text-text-muted mt-2">
					Estimated cost: ${(customCredits * 0.05).toFixed(2)}
				</p>
			{/if}
		</div>

		<!-- Current subscription info -->
		<div class="border-t border-[var(--border)] pt-4 mt-4">
			<p class="text-xs text-text-muted">
				Current balance: <span class="text-text-secondary font-medium">{creditBalance} credits</span>
				{#if subscriptionTier !== 'free'}
					<span class="mx-2">|</span>
					Plan: <span class="text-text-secondary font-medium capitalize">{subscriptionTier}</span>
				{/if}
			</p>
		</div>
	{/if}
</Modal>
