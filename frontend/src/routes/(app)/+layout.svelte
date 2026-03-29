<script lang="ts">
	import { page } from '$app/stores';
	import { auth } from '$stores/auth';

	let sidebarCollapsed = false;
	let mobileSidebarOpen = false;

	// Navigation items
	const navItems = [
		{
			href: '/app',
			label: 'Create',
			icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="20" height="20">
				<path d="M15.5 2A1.5 1.5 0 0014 3.5v13A1.5 1.5 0 0015.5 18h0a1.5 1.5 0 001.5-1.5v-13A1.5 1.5 0 0015.5 2z"/>
				<path d="M10 18v-3"/>
				<path d="M8 18v-5"/>
				<path d="M12 18v-7"/>
				<path d="M3.5 7.5L8 6l1 4-3 1.5"/>
				<path d="M3.5 7.5h5l-1 4h-4"/>
				<path d="M16.5 10.5l-5 1.5"/>
			</svg>`
		},
		{
			href: '/app/gallery',
			label: 'Gallery',
			icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="20" height="20">
				<rect x="2" y="2" width="7" height="7" rx="1"/>
				<rect x="11" y="2" width="7" height="7" rx="1"/>
				<rect x="2" y="11" width="7" height="7" rx="1"/>
				<rect x="11" y="11" width="7" height="7" rx="1"/>
			</svg>`
		},
		{
			href: '/app/dashboard',
			label: 'Dashboard',
			icon: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="20" height="20">
				<path d="M3 3v14h14"/>
				<path d="M7 14l4-5 4 3 3-6"/>
			</svg>`
		}
	];

	// Settings link
	const settingsHref = '/app/settings';
	const settingsIcon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="20" height="20">
		<path d="M10 13a3 3 0 100-6 3 3 0 000 6z"/>
		<path d="M17.4 9.6a1.6 1.6 0 010 2.8l-1.2.7a8.1 8.1 0 01-.9 2.3l.3.6a1.6 1.6 0 01-2 2l-.6-.3a9.2 9.2 0 01-2.3.9l-.7 1.2a1.6 1.6 0 01-2.8 0l-.7-1.2a8.2 8.2 0 01-2.3-.9l-.6.3a1.6 1.6 0 01-2-2l.3-.6a8.2 8.2 0 01-.9-2.3l-1.2-.7a1.6 1.6 0 010-2.8l1.2-.7a8.2 8.2 0 01.9-2.3l-.3-.6a1.6 1.6 0 012-2l.6.3c.8-.4 1.6-.6 2.3-.9l.7-1.2a1.6 1.6 0 012.8 0l.7 1.2c.7.3 1.5.5 2.3.9l.6-.3a1.6 1.6 0 012 2l-.3.6a8.2 8.2 0 01.9 2.3l1.2.7z"/>
	</svg>`;

	// Profile link
	const profileHref = '/app/profile';
	const profileIcon = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="20" height="20">
		<path d="M10 4a4 4 0 100 8 4 4 0 000-8z"/>
		<path d="M3 18a8 8 0 0114 0"/>
	</svg>`;

	function isActive(href: string): boolean {
		return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
	}

	function getInitials(username: string): string {
		return username.slice(0, 2).toUpperCase();
	}

	function closeMobileSidebar() {
		mobileSidebarOpen = false;
	}

	function handleBackdropClick() {
		mobileSidebarOpen = false;
	}
</script>

<div class="flex h-screen overflow-hidden bg-bg-primary">
	<!-- Desktop Sidebar -->
	<aside
		class="hidden md:flex flex-col h-full border-r border-[var(--border)] transition-all duration-300 flex-shrink-0"
		style="background-color: var(--bg-secondary); width: {sidebarCollapsed ? '64px' : '240px'};"
	>
		<!-- Logo -->
		<div class="flex items-center h-16 px-4 border-b border-[var(--border)]">
			<a href="/app" class="flex items-center gap-3 overflow-hidden">
				<!-- Logo mark -->
				<div
					class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
					style="background: var(--accent-gradient);"
				>
					<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="white" width="16" height="16">
						<path d="M8 1.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13zM0 8a8 8 0 1116 0A8 8 0 010 8z"/>
						<path d="M8 4a.75.75 0 01.75.75v2.5h2.5a.75.75 0 010 1.5h-2.5v2.5a.75.75 0 01-1.5 0v-2.5h-2.5a.75.75 0 010-1.5h2.5v-2.5A.75.75 0 018 4z"/>
					</svg>
				</div>
				{#if !sidebarCollapsed}
					<span class="text-sm font-bold text-text-primary whitespace-nowrap">Creative AI</span>
				{/if}
			</a>
		</div>

		<!-- Navigation -->
		<nav class="flex-1 py-4 px-2 space-y-1 overflow-y-auto">
			{#each navItems as item}
				<a
					href={item.href}
					class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150
						{isActive(item.href)
							? 'text-white'
							: 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'}"
					style={isActive(item.href) ? 'background: var(--accent-gradient);' : ''}
					title={sidebarCollapsed ? item.label : undefined}
				>
					<span class="flex-shrink-0">{@html item.icon}</span>
					{#if !sidebarCollapsed}
						<span>{item.label}</span>
					{/if}
				</a>
			{/each}
		</nav>

		<!-- Bottom section -->
		<div class="py-4 px-2 border-t border-[var(--border)] space-y-1">
			<!-- Settings -->
			<a
				href={settingsHref}
				class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150
					text-text-secondary hover:text-text-primary hover:bg-bg-hover"
				title={sidebarCollapsed ? 'Settings' : undefined}
			>
				<span class="flex-shrink-0">{@html settingsIcon}</span>
				{#if !sidebarCollapsed}
					<span>Settings</span>
				{/if}
			</a>

			<!-- User profile -->
			<a
				href={profileHref}
				class="flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-150
					{isActive(profileHref) ? 'bg-bg-hover' : 'hover:bg-bg-hover'}"
				title={sidebarCollapsed ? ($auth.user?.username ?? 'User') : undefined}
			>
				{#if $auth.user?.avatar_url}
					<img
						src={$auth.user.avatar_url}
						alt={$auth.user.username}
						class="w-8 h-8 rounded-full flex-shrink-0 object-cover border border-[var(--border)]"
					/>
				{:else}
					<div
						class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold text-white flex-shrink-0"
						style="background: var(--accent-gradient);"
					>
						{getInitials($auth.user?.username ?? 'U')}
					</div>
				{/if}
				{#if !sidebarCollapsed}
					<div class="flex-1 min-w-0">
						<p class="text-sm font-medium text-text-primary truncate">
							{$auth.user?.username ?? 'User'}
						</p>
					</div>
				{/if}
			</a>
		</div>

		<!-- Collapse toggle -->
		<button
			type="button"
			class="hidden md:flex items-center justify-center h-8 mx-2 mb-2 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors"
			on:click={() => (sidebarCollapsed = !sidebarCollapsed)}
			aria-label={sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				viewBox="0 0 20 20"
				fill="currentColor"
				width="16"
				height="16"
				class="transition-transform duration-300"
				class:rotate-180={sidebarCollapsed}
			>
				<path fill-rule="evenodd" d="M12.79 5.23a.75.75 0 01-.02 1.06L8.832 10l3.938 3.71a.75.75 0 11-1.04 1.08l-4.5-4.25a.75.75 0 010-1.08l4.5-4.25a.75.75 0 011.06.02z" clip-rule="evenodd" />
			</svg>
		</button>
	</aside>

	<!-- Mobile Sidebar Backdrop -->
	{#if mobileSidebarOpen}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			class="md:hidden fixed inset-0 z-40 bg-black/60 backdrop-blur-sm"
			on:click={handleBackdropClick}
		></div>

		<!-- Mobile Sidebar -->
		<aside
			class="md:hidden fixed inset-y-0 left-0 z-50 flex flex-col h-full border-r border-[var(--border)] transition-transform duration-300"
			style="background-color: var(--bg-secondary); width: 240px;"
		>
			<!-- Logo -->
			<div class="flex items-center h-16 px-4 border-b border-[var(--border)]">
				<a href="/app" class="flex items-center gap-3" on:click={closeMobileSidebar}>
					<div
						class="w-8 h-8 rounded-lg flex items-center justify-center flex-shrink-0"
						style="background: var(--accent-gradient);"
					>
						<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="white" width="16" height="16">
							<path d="M8 1.5a6.5 6.5 0 100 13 6.5 6.5 0 000-13zM0 8a8 8 0 1116 0A8 8 0 010 8z"/>
							<path d="M8 4a.75.75 0 01.75.75v2.5h2.5a.75.75 0 010 1.5h-2.5v2.5a.75.75 0 01-1.5 0v-2.5h-2.5a.75.75 0 010-1.5h2.5v-2.5A.75.75 0 018 4z"/>
						</svg>
					</div>
					<span class="text-sm font-bold text-text-primary">Creative AI</span>
				</a>
			</div>

			<!-- Navigation -->
			<nav class="flex-1 py-4 px-2 space-y-1 overflow-y-auto">
				{#each navItems as item}
					<a
						href={item.href}
						class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150
							{isActive(item.href)
								? 'text-white'
								: 'text-text-secondary hover:text-text-primary hover:bg-bg-hover'}"
						style={isActive(item.href) ? 'background: var(--accent-gradient);' : ''}
						on:click={closeMobileSidebar}
					>
						<span class="flex-shrink-0">{@html item.icon}</span>
						<span>{item.label}</span>
					</a>
				{/each}
			</nav>

			<!-- Bottom section -->
			<div class="py-4 px-2 border-t border-[var(--border)] space-y-1">
				<a
					href={settingsHref}
					class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150
						text-text-secondary hover:text-text-primary hover:bg-bg-hover"
					on:click={closeMobileSidebar}
				>
					<span class="flex-shrink-0">{@html settingsIcon}</span>
					<span>Settings</span>
				</a>

				<a
					href={profileHref}
					class="flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-150 hover:bg-bg-hover"
					on:click={closeMobileSidebar}
				>
					{#if $auth.user?.avatar_url}
						<img
							src={$auth.user.avatar_url}
							alt={$auth.user.username}
							class="w-8 h-8 rounded-full flex-shrink-0 object-cover border border-[var(--border)]"
						/>
					{:else}
						<div
							class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold text-white flex-shrink-0"
							style="background: var(--accent-gradient);"
						>
							{getInitials($auth.user?.username ?? 'U')}
						</div>
					{/if}
					<div class="flex-1 min-w-0">
						<p class="text-sm font-medium text-text-primary truncate">
							{$auth.user?.username ?? 'User'}
						</p>
					</div>
				</a>
			</div>
		</aside>
	{/if}

	<!-- Main content area -->
	<div class="flex-1 flex flex-col min-w-0 overflow-hidden">
		<!-- Mobile top bar -->
		<header class="md:hidden flex items-center h-14 px-4 border-b border-[var(--border)]" style="background-color: var(--bg-secondary);">
			<button
				type="button"
				class="p-2 -ml-2 rounded-md text-text-secondary hover:text-text-primary hover:bg-bg-hover transition-colors"
				on:click={() => (mobileSidebarOpen = true)}
				aria-label="Open menu"
			>
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" width="20" height="20">
					<path fill-rule="evenodd" d="M2 4.75A.75.75 0 012.75 4h14.5a.75.75 0 010 1.5H2.75A.75.75 0 012 4.75zm0 10.5a.75.75 0 01.75-.75h14.5a.75.75 0 010 1.5H2.75a.75.75 0 01-.75-.75zM2 10a.75.75 0 01.75-.75h7.5a.75.75 0 010 1.5h-7.5A.75.75 0 012 10z" clip-rule="evenodd" />
				</svg>
			</button>
		</header>

		<!-- Page content -->
		<main class="flex-1 overflow-y-auto">
			<slot />
		</main>
	</div>
</div>
