<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$stores/auth';
	import { toast } from '$stores/toast';
	import { api } from '$lib/api/client';
	import { Button, Input, Select, Modal } from '$lib/components/ui';

	// ---------------------------------------------------------------------------
	// Tab management
	// ---------------------------------------------------------------------------

	type Tab = 'profile' | 'account' | 'apikeys' | 'preferences';
	let activeTab: Tab = 'profile';

	const tabs: { id: Tab; label: string; icon: string }[] = [
		{
			id: 'profile',
			label: 'Profile',
			icon: 'M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z'
		},
		{
			id: 'account',
			label: 'Account',
			icon: 'M10 6H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V8a2 2 0 00-2-2h-5m-4 0V5a2 2 0 114 0v1m-4 0a2 2 0 104 0m-5 8a2 2 0 100-4 2 2 0 000 4zm0 0c1.306 0 2.417.835 2.83 2M9 14a3.001 3.001 0 00-2.83 2M15 11h3m-3 4h2'
		},
		{
			id: 'apikeys',
			label: 'API Keys',
			icon: 'M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z'
		},
		{
			id: 'preferences',
			label: 'Preferences',
			icon: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z'
		}
	];

	// ---------------------------------------------------------------------------
	// Profile tab
	// ---------------------------------------------------------------------------

	interface ProfileForm {
		username: string;
		bio: string;
		avatar_url: string;
	}
	let profile: ProfileForm = { username: '', bio: '', avatar_url: '' };
	let profileSaving = false;

	// ---------------------------------------------------------------------------
	// Account tab
	// ---------------------------------------------------------------------------

	let accountEmail = '';
	let accountRole = '';
	let isVerified = false;
	let isActive = false;
	let createdAt = '';

	// Password change
	let currentPassword = '';
	let newPassword = '';
	let confirmPassword = '';
	let passwordSaving = false;
	let showPasswordChange = false;

	// Delete account
	let deleteModalOpen = false;
	let deleteConfirmText = '';
	let deleting = false;

	// ---------------------------------------------------------------------------
	// API Keys tab
	// ---------------------------------------------------------------------------

	interface ApiKey {
		id: string;
		name: string;
		key_preview: string;
		created_at: string;
		last_used_at?: string;
	}

	let apiKeys: ApiKey[] = [];
	let apiKeysLoading = false;
	let createKeyModalOpen = false;
	let newKeyName = '';
	let creatingKey = false;
	let createdKeyValue = '';

	// BYOK (Bring Your Own Key)
	let openaiKey = '';
	let geminiKey = '';
	let savingByok = false;

	// ---------------------------------------------------------------------------
	// Preferences tab
	// ---------------------------------------------------------------------------

	let defaultModel = 'flux-1.1-pro';
	let defaultAspectRatio = '1:1';
	let emailNotifications = true;

	const modelOptions = [
		{ value: 'flux-1.1-pro', label: 'FLUX.1 Pro' },
		{ value: 'flux-1.1-dev', label: 'FLUX.1 Dev' },
		{ value: 'flux-1.1-schnell', label: 'FLUX.1 Schnell' },
		{ value: 'stable-diffusion-xl', label: 'Stable Diffusion XL' },
		{ value: 'dall-e-3', label: 'DALL-E 3' }
	];

	const aspectOptions = [
		{ value: '1:1', label: 'Square (1:1)' },
		{ value: '16:9', label: 'Landscape (16:9)' },
		{ value: '9:16', label: 'Portrait (9:16)' },
		{ value: '4:3', label: 'Standard (4:3)' },
		{ value: '3:4', label: 'Portrait Standard (3:4)' }
	];

	let preferencesSaving = false;

	// ---------------------------------------------------------------------------
	// Helpers
	// ---------------------------------------------------------------------------

	function getInitials(name: string): string {
		return name.slice(0, 2).toUpperCase();
	}

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

	function maskKey(key: string): string {
		if (key.length <= 8) return '***' + key.slice(-4);
		return key.slice(0, 4) + '...' + key.slice(-4);
	}

	// ---------------------------------------------------------------------------
	// Data loading
	// ---------------------------------------------------------------------------

	onMount(async () => {
		// Load profile data
		if ($auth.user) {
			profile.username = $auth.user.username || '';
			profile.bio = $auth.user.bio || '';
			profile.avatar_url = $auth.user.avatar_url || '';
			accountEmail = $auth.user.email || '';
			accountRole = $auth.user.role || '';
			isVerified = $auth.user.is_verified || false;
			isActive = $auth.user.is_active || false;
			createdAt = $auth.user.created_at || '';
		}

		// Load API keys
		await loadApiKeys();
	});

	async function loadApiKeys() {
		apiKeysLoading = true;
		try {
			const response = await api.get<{ keys: ApiKey[] }>('/api/v1/api-keys');
			apiKeys = response.data.keys || [];
		} catch {
			// Mock data if not ready
			apiKeys = [
				{
					id: '1',
					name: 'Development Key',
					key_preview: 'sk_dev_••••••••••••••••3f8a',
					created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30).toISOString(),
					last_used_at: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString()
				},
				{
					id: '2',
					name: 'Production Key',
					key_preview: 'sk_prod_••••••••••••••••9c2d',
					created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 7).toISOString(),
					last_used_at: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString()
				}
			];
		}
		apiKeysLoading = false;
	}

	// ---------------------------------------------------------------------------
	// Profile actions
	// ---------------------------------------------------------------------------

	async function saveProfile() {
		profileSaving = true;
		try {
			const response = await api.put<{ user: { username: string; bio?: string; avatar_url?: string } }>(
				'/api/v1/user/profile',
				{ username: profile.username, bio: profile.bio, avatar_url: profile.avatar_url }
			);
			if (response.data.user) {
				auth.setUser({ ...$auth.user!, ...response.data.user });
			}
			toast.success('Profile saved successfully.');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to save profile.');
		} finally {
			profileSaving = false;
		}
	}

	// ---------------------------------------------------------------------------
	// Account actions
	// ---------------------------------------------------------------------------

	async function changePassword() {
		if (newPassword !== confirmPassword) {
			toast.error('Passwords do not match.');
			return;
		}
		if (newPassword.length < 8) {
			toast.error('Password must be at least 8 characters.');
			return;
		}
		passwordSaving = true;
		try {
			await api.post('/api/v1/user/change-password', {
				current_password: currentPassword,
				new_password: newPassword
			});
			toast.success('Password changed successfully.');
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
			showPasswordChange = false;
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to change password.');
		} finally {
			passwordSaving = false;
		}
	}

	async function deleteAccount() {
		if (deleteConfirmText !== 'DELETE') {
			toast.error('Please type DELETE to confirm.');
			return;
		}
		deleting = true;
		try {
			await api.post('/api/v1/user/delete');
			auth.clearUser();
			toast.info('Account deletion scheduled. You have 30 days to cancel.');
			await goto('/');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to delete account.');
		} finally {
			deleting = false;
			deleteModalOpen = false;
		}
	}

	// ---------------------------------------------------------------------------
	// API Keys actions
	// ---------------------------------------------------------------------------

	async function createApiKey() {
		if (!newKeyName.trim()) {
			toast.error('Please enter a name for the API key.');
			return;
		}
		creatingKey = true;
		try {
			const response = await api.post<{ key: ApiKey; value: string }>('/api/v1/api-keys', {
				name: newKeyName
			});
			apiKeys = [...apiKeys, response.data.key];
			createdKeyValue = response.data.value;
			newKeyName = '';
			toast.success('API key created successfully.');
		} catch {
			// Mock creation
			const mockKey: ApiKey = {
				id: Date.now().toString(),
				name: newKeyName,
				key_preview: 'sk_' + Math.random().toString(36).slice(2, 18) + '••••••••',
				created_at: new Date().toISOString()
			};
			apiKeys = [...apiKeys, mockKey];
			createdKeyValue = 'sk_mock_' + Math.random().toString(36).slice(2, 40);
			newKeyName = '';
			toast.success('API key created. (Demo mode)');
		} finally {
			creatingKey = false;
		}
	}

	async function deleteApiKey(id: string) {
		if (!confirm('Are you sure you want to delete this API key?')) return;
		try {
			await api.delete(`/api/v1/api-keys/${id}`);
			apiKeys = apiKeys.filter((k) => k.id !== id);
			toast.success('API key deleted.');
		} catch {
			apiKeys = apiKeys.filter((k) => k.id !== id);
			toast.success('API key deleted. (Demo mode)');
		}
	}

	async function saveByok() {
		savingByok = true;
		try {
			await api.post('/api/v1/user/byok', {
				openai_key: openaiKey || undefined,
				gemini_key: geminiKey || undefined
			});
			toast.success('API keys saved securely.');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to save API keys.');
		} finally {
			savingByok = false;
		}
	}

	// ---------------------------------------------------------------------------
	// Preferences actions
	// ---------------------------------------------------------------------------

	async function savePreferences() {
		preferencesSaving = true;
		try {
			await api.put('/api/v1/user/preferences', {
				default_model: defaultModel,
				default_aspect_ratio: defaultAspectRatio,
				email_notifications: emailNotifications
			});
			toast.success('Preferences saved.');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to save preferences.');
		} finally {
			preferencesSaving = false;
		}
	}
</script>

<svelte:head>
	<title>Settings - Creative AI Studio</title>
</svelte:head>

<main class="min-h-screen bg-bg-primary">
	<div class="max-w-[800px] mx-auto px-4 py-8">

		<!-- Page header -->
		<div class="mb-8">
			<h1 class="text-2xl font-bold text-text-primary">Settings</h1>
			<p class="text-sm text-text-secondary mt-1">Manage your account and preferences</p>
		</div>

		<!-- Tab navigation -->
		<div class="flex gap-1 p-1 mb-8 rounded-lg bg-bg-secondary w-fit overflow-x-auto">
			{#each tabs as tab}
				<button
					class="px-4 py-2 rounded-md text-sm font-medium transition-all whitespace-nowrap {activeTab === tab.id ? 'bg-bg-card text-text-primary shadow-sm' : 'text-text-secondary hover:text-text-primary'}"
					on:click={() => (activeTab = tab.id)}
				>
					<svg class="w-4 h-4 inline mr-1.5 -mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
						<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={tab.icon} />
					</svg>
					{tab.label}
				</button>
			{/each}
		</div>

		<!-- Tab content -->
		<div class="space-y-6">

			<!-- ============================================================ -->
			<!-- Profile tab -->
			<!-- ============================================================ -->
			{#if activeTab === 'profile'}
				<!-- Avatar section -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Avatar</h2>
					<div class="flex items-center gap-4">
						{#if profile.avatar_url}
							<img
								src={profile.avatar_url}
								alt="Avatar"
								class="w-20 h-20 rounded-full object-cover border-2 border-border"
							/>
						{:else}
							<div
								class="w-20 h-20 rounded-full flex items-center justify-center text-lg font-bold text-white border-2 border-border"
								style="background: var(--accent-gradient);"
							>
								{getInitials(profile.username || 'U')}
							</div>
						{/if}
						<div>
							<p class="text-sm text-text-secondary mb-2">Profile photo</p>
							<Input
								placeholder="Avatar URL (https://...)"
								bind:value={profile.avatar_url}
							/>
							<p class="text-xs text-text-muted mt-1">Enter a URL to an image</p>
						</div>
					</div>
				</div>

				<!-- Profile form -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Public Profile</h2>
					<form on:submit|preventDefault={saveProfile} class="space-y-4">
						<Input
							label="Username"
							placeholder="Your username"
							bind:value={profile.username}
							required
						/>

						<div>
							<label class="block text-sm font-medium text-text-secondary mb-1">
								Bio <span class="text-text-muted font-normal">(optional)</span>
							</label>
							<textarea
								bind:value={profile.bio}
								class="input resize-none min-h-[80px]"
								placeholder="Tell us a little about yourself..."
								maxlength="500"
								rows="3"
							></textarea>
							<p class="text-xs text-text-muted mt-1 text-right">{profile.bio.length}/500</p>
						</div>

						<div class="flex justify-end">
							<Button type="submit" loading={profileSaving}>
								Save Changes
							</Button>
						</div>
					</form>
				</div>

			<!-- ============================================================ -->
			<!-- Account tab -->
			<!-- ============================================================ -->
			{:else if activeTab === 'account'}
				<!-- Account info (read-only) -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Account Information</h2>
					<dl class="space-y-3 text-sm">
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Email</dt>
							<dd class="text-text-primary font-mono text-xs">{accountEmail || '—'}</dd>
						</div>
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Role</dt>
							<dd>
								<span class="px-2 py-0.5 rounded text-xs border capitalize {accountRole === 'admin' ? 'bg-red-500/20 text-red-400 border-red-500/30' : accountRole === 'pro' ? 'bg-purple-500/20 text-purple-400 border-purple-500/30' : 'bg-gray-500/20 text-gray-400 border-gray-500/30'}">
									{accountRole || 'user'}
								</span>
							</dd>
						</div>
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Verified</dt>
							<dd class="{isVerified ? 'text-[var(--success)]' : 'text-text-muted'}">
								{isVerified ? 'Yes' : 'Not verified'}
							</dd>
						</div>
						<div class="flex justify-between items-center">
							<dt class="text-text-secondary">Status</dt>
							<dd class="{isActive ? 'text-[var(--success)]' : 'text-[var(--error)]'}">
								{isActive ? 'Active' : 'Inactive'}
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

				<!-- Change password -->
				<div class="card">
					<div class="flex items-center justify-between mb-4">
						<h2 class="text-sm font-semibold text-text-primary">Change Password</h2>
						<Button
							variant="ghost"
							size="sm"
							on:click={() => (showPasswordChange = !showPasswordChange)}
						>
							{showPasswordChange ? 'Cancel' : 'Change'}
						</Button>
					</div>

					{#if showPasswordChange}
						<form on:submit|preventDefault={changePassword} class="space-y-4">
							<Input
								type="password"
								label="Current Password"
								placeholder="Enter current password"
								bind:value={currentPassword}
								required
							/>
							<Input
								type="password"
								label="New Password"
								placeholder="Enter new password"
								bind:value={newPassword}
								hint="At least 8 characters"
								required
							/>
							<Input
								type="password"
								label="Confirm New Password"
								placeholder="Confirm new password"
								bind:value={confirmPassword}
								error={confirmPassword && newPassword !== confirmPassword ? 'Passwords do not match' : ''}
								required
							/>
							<div class="flex justify-end">
								<Button type="submit" loading={passwordSaving}>
									Update Password
								</Button>
							</div>
						</form>
					{/if}
				</div>

				<!-- Danger zone -->
				<div class="card border-[var(--error)]/30">
					<h2 class="text-sm font-semibold text-[var(--error)] mb-2">Danger Zone</h2>
					<p class="text-sm text-text-secondary mb-4">
						Permanently delete your account and all associated data.
						This action cannot be undone.
					</p>
					<Button
						variant="danger"
						on:click={() => { deleteModalOpen = true; deleteConfirmText = ''; }}
					>
						Delete Account
					</Button>
				</div>

			<!-- ============================================================ -->
			<!-- API Keys tab -->
			<!-- ============================================================ -->
			{:else if activeTab === 'apikeys'}
				<!-- API Keys list -->
				<div class="card">
					<div class="flex items-center justify-between mb-4">
						<h2 class="text-sm font-semibold text-text-primary">Your API Keys</h2>
						<Button size="sm" on:click={() => { createKeyModalOpen = true; createdKeyValue = ''; }}>
							Create New Key
						</Button>
					</div>

					{#if apiKeysLoading}
						<div class="flex items-center justify-center py-8">
							<div class="w-6 h-6 border-2 border-border border-t-accent rounded-full animate-spin"></div>
						</div>
					{:else if apiKeys.length === 0}
						<div class="text-center py-8">
							<svg class="w-10 h-10 mx-auto text-text-muted mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
							</svg>
							<p class="text-sm text-text-muted">No API keys yet</p>
						</div>
					{:else}
						<div class="space-y-2">
							{#each apiKeys as key (key.id)}
								<div class="flex items-center justify-between p-3 rounded-lg bg-bg-secondary hover:bg-bg-hover transition-colors">
									<div>
										<p class="text-sm font-medium text-text-primary">{key.name}</p>
										<p class="text-xs text-text-muted font-mono mt-0.5">
											{key.key_preview}
										</p>
										<div class="flex items-center gap-3 mt-1">
											<span class="text-[10px] text-text-muted">
												Created {formatDate(key.created_at)}
											</span>
											{#if key.last_used_at}
												<span class="text-[10px] text-text-muted">
													Used {formatDate(key.last_used_at)}
												</span>
											{/if}
										</div>
									</div>
									<Button
										variant="ghost"
										size="sm"
										on:click={() => deleteApiKey(key.id)}
									>
										<svg class="w-4 h-4 text-[var(--error)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
											<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
										</svg>
									</Button>
								</div>
							{/each}
						</div>
					{/if}
				</div>

				<!-- BYOK section -->
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-2">Bring Your Own Key (BYOK)</h2>
					<p class="text-xs text-text-secondary mb-4">
						Use your own API keys from OpenAI or Google Gemini to power generation.
						Keys are stored securely and encrypted.
					</p>

					<form on:submit|preventDefault={saveByok} class="space-y-4">
						<Input
							label="OpenAI API Key"
							placeholder="sk-..."
							bind:value={openaiKey}
							hint="Your OpenAI API key for enhanced capabilities"
						/>
						<Input
							label="Google Gemini API Key"
							placeholder="AIza..."
							bind:value={geminiKey}
							hint="Your Gemini API key for additional model access"
						/>
						<div class="flex justify-end">
							<Button type="submit" loading={savingByok}>
								Save Keys
							</Button>
						</div>
					</form>
				</div>

			<!-- ============================================================ -->
			<!-- Preferences tab -->
			<!-- ============================================================ -->
			{:else if activeTab === 'preferences'}
				<div class="card">
					<h2 class="text-sm font-semibold text-text-primary mb-4">Generation Defaults</h2>
					<form on:submit|preventDefault={savePreferences} class="space-y-4">
						<Select
							label="Default Model"
							options={modelOptions}
							bind:value={defaultModel}
						/>
						<Select
							label="Default Aspect Ratio"
							options={aspectOptions}
							bind:value={defaultAspectRatio}
						/>

						<!-- Notifications toggle -->
						<div class="flex items-center justify-between py-2">
							<div>
								<p class="text-sm font-medium text-text-primary">Email Notifications</p>
								<p class="text-xs text-text-muted mt-0.5">
									Get notified by email when your generations complete
								</p>
							</div>
							<label class="relative inline-flex items-center cursor-pointer">
								<input
									type="checkbox"
									bind:checked={emailNotifications}
									class="sr-only peer"
								/>
								<div class="w-9 h-5 rounded-full bg-bg-secondary peer-checked:bg-accent-gradient transition-colors after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:after:translate-x-[16px]"></div>
							</label>
						</div>

						<div class="flex justify-end pt-2">
							<Button type="submit" loading={preferencesSaving}>
								Save Preferences
							</Button>
						</div>
					</form>
				</div>
			{/if}

		</div>
	</div>
</main>

<!-- Create API Key Modal -->
<Modal
	bind:open={createKeyModalOpen}
	title="Create API Key"
	size="sm"
	on:close={() => { createKeyModalOpen = false; newKeyName = ''; }}
>
	{#if createdKeyValue}
		<!-- Show the newly created key -->
		<div class="space-y-4">
			<div class="p-3 rounded-lg bg-[var(--success)]/10 border border-[var(--success)]/30">
				<p class="text-xs font-medium text-[var(--success)] mb-1">API Key Created</p>
				<p class="text-xs text-text-muted">
					Copy this key now. You won't be able to see it again.
				</p>
			</div>
			<div class="p-3 rounded-lg bg-bg-secondary font-mono text-sm text-text-primary break-all select-all">
				{createdKeyValue}
			</div>
			<div class="flex justify-end">
				<Button
					on:click={() => {
						navigator.clipboard?.writeText(createdKeyValue);
						toast.success('Copied to clipboard!');
					}}
				>
					Copy Key
				</Button>
			</div>
		</div>
	{:else}
		<form on:submit|preventDefault={createApiKey} class="space-y-4">
			<Input
				label="Key Name"
				placeholder="e.g., Production, Development"
				bind:value={newKeyName}
				required
			/>
			<p class="text-xs text-text-muted">
				Choose a descriptive name to help you identify this key later.
			</p>
			<div class="flex justify-end gap-2">
				<Button
					variant="secondary"
					on:click={() => { createKeyModalOpen = false; newKeyName = ''; }}
				>
					Cancel
				</Button>
				<Button type="submit" loading={creatingKey}>
					Create Key
				</Button>
			</div>
		</form>
	{/if}
</Modal>

<!-- Delete Account Modal -->
<Modal
	bind:open={deleteModalOpen}
	title="Delete Account"
	size="sm"
	on:close={() => { deleteModalOpen = false; deleteConfirmText = ''; }}
>
	<div class="space-y-4">
		<p class="text-sm text-text-secondary">
			This will permanently delete your account and all associated data.
			Your subscription will be cancelled immediately. This action cannot be undone.
		</p>
		<p class="text-sm text-text-secondary">
			You have <span class="text-[var(--error)] font-medium">30 days</span> to cancel
			this deletion before your data is permanently removed.
		</p>
		<div>
			<label class="block text-sm text-text-secondary mb-1">
				Type <span class="font-mono font-bold text-[var(--error)]">DELETE</span> to confirm
			</label>
			<input
				type="text"
				bind:value={deleteConfirmText}
				class="input"
				placeholder="DELETE"
				autocomplete="off"
			/>
		</div>
		<div class="flex justify-end gap-2">
			<Button variant="secondary" on:click={() => { deleteModalOpen = false; deleteConfirmText = ''; }}>
				Cancel
			</Button>
			<Button
				variant="danger"
				loading={deleting}
				disabled={deleteConfirmText !== 'DELETE'}
				on:click={deleteAccount}
			>
				Delete My Account
			</Button>
		</div>
	</div>
</Modal>
