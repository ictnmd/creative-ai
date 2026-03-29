/**
 * Presets Store
 * Manages generation presets (both built-in and user-created).
 */

import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';
import { api } from '$lib/api/client';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface Preset {
	id: string;
	name: string;
	description?: string;
	style?: string;
	aspect_ratio?: string;
	default_model?: string;
	seed?: number;
	is_builtin?: boolean;
	is_public?: boolean;
	created_at: string;
	updated_at?: string;
}

export interface PresetsState {
	presets: Preset[];
	loading: boolean;
	error: string | null;
}

export interface CreatePresetInput {
	name: string;
	description?: string;
	style?: string;
	aspect_ratio?: string;
	default_model?: string;
	seed?: number;
	is_public?: boolean;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const initialState: PresetsState = {
	presets: [],
	loading: false,
	error: null
};

function createPresetsStore() {
	const store: Writable<PresetsState> = writable(initialState);

	return {
		subscribe: store.subscribe,

		/** Fetch all presets (built-in + custom). */
		async fetchAll() {
			store.update((s) => ({ ...s, loading: true, error: null }));
			try {
				const response = await api.get<{ presets: Preset[] }>('/api/v1/presets');
				store.update((s) => ({ ...s, presets: response.data.presets, loading: false }));
			} catch (err: unknown) {
				const apiErr = err as { message?: string };
				store.update((s) => ({
					...s,
					loading: false,
					error: apiErr.message || 'Failed to fetch presets'
				}));
			}
		},

		/** Create a new custom preset. */
		async create(input: CreatePresetInput): Promise<Preset | null> {
			store.update((s) => ({ ...s, loading: true, error: null }));
			try {
				const response = await api.post<{ preset: Preset }>('/api/v1/presets', input);
				store.update((s) => ({
					...s,
					presets: [...s.presets, response.data.preset],
					loading: false
				}));
				return response.data.preset;
			} catch (err: unknown) {
				const apiErr = err as { message?: string };
				store.update((s) => ({
					...s,
					loading: false,
					error: apiErr.message || 'Failed to create preset'
				}));
				return null;
			}
		},

		/** Update an existing custom preset. */
		async update(id: string, input: Partial<CreatePresetInput>): Promise<boolean> {
			store.update((s) => ({ ...s, loading: true, error: null }));
			try {
				const response = await api.put<{ preset: Preset }>(`/api/v1/presets/${id}`, input);
				store.update((s) => ({
					...s,
					presets: s.presets.map((p) => (p.id === id ? response.data.preset : p)),
					loading: false
				}));
				return true;
			} catch (err: unknown) {
				const apiErr = err as { message?: string };
				store.update((s) => ({
					...s,
					loading: false,
					error: apiErr.message || 'Failed to update preset'
				}));
				return false;
			}
		},

		/** Delete a custom preset. */
		async delete(id: string): Promise<boolean> {
			store.update((s) => ({ ...s, loading: true, error: null }));
			try {
				await api.delete(`/api/v1/presets/${id}`);
				store.update((s) => ({
					...s,
					presets: s.presets.filter((p) => p.id !== id),
					loading: false
				}));
				return true;
			} catch (err: unknown) {
				const apiErr = err as { message?: string };
				store.update((s) => ({
					...s,
					loading: false,
					error: apiErr.message || 'Failed to delete preset'
				}));
				return false;
			}
		},

		/** Reset store to initial state. */
		reset() {
			store.set(initialState);
		}
	};
}

export const presets = createPresetsStore();
