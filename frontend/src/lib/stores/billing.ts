/**
 * Billing Store
 * Manages billing-related state: credits, subscriptions, and transactions.
 */

import { writable, derived } from 'svelte/store';
import type { Readable } from 'svelte/store';
import { api } from '$lib/api/client';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface CreditBalance {
	credit_balance: number;
	subscription_tier: string;
	quota_remaining: number;
	quota_limit: number;
	generations_used: number;
}

export interface SubscriptionPlan {
	id: string;
	name: string;
	price: number;
	billing_cycle: 'monthly' | 'yearly';
	generation_limit: number;
	features: string[];
	is_popular?: boolean;
}

export interface CreditPack {
	id: string;
	credits: number;
	price: number;
	price_per_credit: number;
}

export interface Transaction {
	id: string;
	type: 'generation' | 'purchase' | 'refund' | 'subscription';
	amount: number;
	credits?: number;
	description: string;
	created_at: string;
}

export interface BillingState {
	balance: CreditBalance | null;
	plans: SubscriptionPlan[];
	creditPacks: CreditPack[];
	transactions: Transaction[];
	loading: boolean;
	error: string | null;
}

export interface UserApiKey {
	id: string;
	name: string;
	key_preview: string;
	created_at: string;
	last_used_at?: string;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const initialState: BillingState = {
	balance: null,
	plans: [],
	creditPacks: [],
	transactions: [],
	loading: false,
	error: null
};

// Create the underlying writable store
const _store = writable<BillingState>(initialState);

// Export the store as a combined object that Svelte treats as a store
export const billing = {
	subscribe: _store.subscribe,

	async fetchBalance() {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.get<CreditBalance>('/api/v1/credits/balance');
			_store.update((s) => ({ ...s, balance: response.data, loading: false }));
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to fetch balance'
			}));
		}
	},

	async fetchPlans() {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.get<{ plans: SubscriptionPlan[] }>('/api/v1/subscriptions');
			_store.update((s) => ({ ...s, plans: response.data.plans, loading: false }));
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to fetch plans'
			}));
		}
	},

	async fetchCreditPacks() {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.get<{ packs: CreditPack[] }>('/api/v1/credits/packs');
			_store.update((s) => ({ ...s, creditPacks: response.data.packs, loading: false }));
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to fetch credit packs'
			}));
		}
	},

	async subscribeToPlan(planId: string): Promise<string | null> {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.post<{ checkout_url: string }>('/api/v1/subscriptions/subscribe', {
				plan_id: planId
			});
			_store.update((s) => ({ ...s, loading: false }));
			return response.data.checkout_url;
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to subscribe'
			}));
			return null;
		}
	},

	async purchaseCredits(packId: string): Promise<string | null> {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.post<{ checkout_url: string }>('/api/v1/credits/purchase', {
				pack_id: packId
			});
			_store.update((s) => ({ ...s, loading: false }));
			return response.data.checkout_url;
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to purchase credits'
			}));
			return null;
		}
	},

	async purchaseCustomCredits(amount: number): Promise<string | null> {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.post<{ checkout_url: string }>('/api/v1/credits/purchase', {
				amount
			});
			_store.update((s) => ({ ...s, loading: false }));
			return response.data.checkout_url;
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to purchase credits'
			}));
			return null;
		}
	},

	async fetchHistory(page = 1, limit = 20) {
		_store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.get<{ transactions: Transaction[] }>(
				`/api/v1/credits/history?page=${page}&limit=${limit}`
			);
			_store.update((s) => ({
				...s,
				transactions: response.data.transactions,
				loading: false
			}));
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			_store.update((s) => ({
				...s,
				loading: false,
				error: apiErr.message || 'Failed to fetch history'
			}));
		}
	},

	updateBalance(balance: Partial<CreditBalance>) {
		_store.update((s) => ({
			...s,
			balance: s.balance ? { ...s.balance, ...balance } : null
		}));
	},

	reset() {
		_store.set(initialState);
	}
};

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

export const quotaPercentage: Readable<number> = derived(_store, ($billing) => {
	if (!$billing.balance) return 0;
	const { generations_used, quota_limit } = $billing.balance;
	if (quota_limit === 0) return 100;
	return Math.round((generations_used / quota_limit) * 100);
});

export const quotaStatus: Readable<'critical' | 'warning' | 'ok'> = derived(
	quotaPercentage,
	($pct) => {
		if ($pct >= 95) return 'critical';
		if ($pct >= 80) return 'warning';
		return 'ok';
	}
);
