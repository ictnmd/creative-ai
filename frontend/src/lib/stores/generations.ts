/**
 * Generations Store
 * Manages the list of AI image generations with WebSocket real-time updates.
 */

import { writable, get } from 'svelte/store';
import { api } from '$lib/api/client';
import { wsClient } from '$lib/api/ws-client';
import { auth } from './auth';
import { toast } from './toast';
import type { Writable } from 'svelte/store';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type GenerationStatus = 'pending' | 'processing' | 'completed' | 'failed';

export interface Generation {
	id: string;
	prompt: string;
	negative_prompt?: string;
	model: string;
	width: number;
	height: number;
	status: GenerationStatus;
	image_url?: string;
	thumbnail_url?: string;
	generation_time?: number;
	created_at: string;
	error_message?: string;
}

interface GenerationsState {
	items: Generation[];
	loading: boolean;
	error: string | null;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

function createGenerationsStore() {
	const store: Writable<GenerationsState> = writable({
		items: [],
		loading: false,
		error: null
	});

	let wsUnsubscribers: (() => void)[] = [];

	/** Fetch all generations from the API. */
	async function fetchAll(): Promise<void> {
		store.update((s) => ({ ...s, loading: true, error: null }));
		try {
			const response = await api.get<{ generations: Generation[] }>('/api/v1/generations');
			store.update((s) => ({ ...s, items: response.data.generations, loading: false }));
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			store.update((s) => ({ ...s, loading: false, error: apiErr.message || 'Failed to load generations' }));
		}
	}

	/** Add a new generation to the store. */
	function addGeneration(generation: Generation): void {
		store.update((s) => {
			// Avoid duplicates
			if (s.items.some((g) => g.id === generation.id)) {
				return s;
			}
			return { ...s, items: [generation, ...s.items] };
		});
		// Subscribe to WebSocket updates for this generation
		wsClient.subscribe(generation.id);
	}

	/** Update an existing generation in the store. */
	function updateGeneration(update: {
		id: string;
		status?: GenerationStatus;
		image_url?: string;
		thumbnail_url?: string;
		generation_time?: number;
		error_message?: string;
	}): void {
		store.update((s) => ({
			...s,
			items: s.items.map((g) =>
				g.id === update.id
					? {
							...g,
							status: update.status ?? g.status,
							image_url: update.image_url ?? g.image_url,
							thumbnail_url: update.thumbnail_url ?? g.thumbnail_url,
							generation_time: update.generation_time ?? g.generation_time,
							error_message: update.error_message ?? g.error_message
					  }
					: g
			)
		}));

		// When a generation completes or fails, unsubscribe from its WebSocket feed
		if (update.status === 'completed' || update.status === 'failed') {
			wsClient.unsubscribe(update.id);
		}
	}

	/** Remove a generation from the store. */
	function removeGeneration(id: string): void {
		wsClient.unsubscribe(id);
		store.update((s) => ({
			...s,
			items: s.items.filter((g) => g.id !== id)
		}));
	}

	/** Initialize the store: fetch existing generations and connect WebSocket. */
	async function initialize(): Promise<void> {
		// Set up WebSocket update handler
		const unsubUpdate = wsClient.onUpdate((update) => {
			// Find if this generation already exists in the store
			const state = get(store);
			const exists = state.items.some((g) => g.id === update.generation_id);

			if (exists) {
				updateGeneration({
					id: update.generation_id,
					status: update.status,
					image_url: update.image_url,
					error_message: update.status === 'failed' ? update.message : undefined
				});

				if (update.status === 'completed') {
					toast.success('Generation completed!');
				} else if (update.status === 'failed') {
					toast.error(`Generation failed: ${update.message}`);
				}
			} else {
				// New generation created externally — add it
				const newGen: Generation = {
					id: update.generation_id,
					prompt: '',
					model: '',
					width: 0,
					height: 0,
					status: update.status,
					image_url: update.image_url,
					created_at: update.timestamp,
					error_message: update.status === 'failed' ? update.message : undefined
				};
				addGeneration(newGen);
			}
		});

		wsUnsubscribers.push(unsubUpdate);

		// Connect WebSocket if authenticated
		const currentAuth = get(auth);
		if (currentAuth.user) {
			wsClient.connect(currentAuth.user.id); // Using user id as token for now
		}

		// Fetch existing generations
		await fetchAll();

		// Subscribe to WebSocket updates for all in-progress generations
		const state = get(store);
		state.items
			.filter((g) => g.status === 'pending' || g.status === 'processing')
			.forEach((g) => wsClient.subscribe(g.id));
	}

	/** Clean up: disconnect WebSocket and remove all listeners. */
	function destroy(): void {
		wsUnsubscribers.forEach((unsub) => unsub());
		wsUnsubscribers = [];
		wsClient.disconnect();
	}

	/** Delete a generation via the API. */
	async function deleteGeneration(id: string): Promise<void> {
		try {
			await api.delete(`/api/v1/generations/${id}`);
			removeGeneration(id);
			toast.success('Generation deleted.');
		} catch (err: unknown) {
			const apiErr = err as { message?: string };
			toast.error(apiErr.message || 'Failed to delete generation.');
			throw err;
		}
	}

	return {
		subscribe: store.subscribe,

		/** The list of generations (read-only derived). */
		get generations() {
			return { subscribe: store.subscribe };
		},

		fetchAll,
		addGeneration,
		updateGeneration,
		removeGeneration,
		initialize,
		destroy,
		deleteGeneration
	};
}

export const generations = createGenerationsStore();
