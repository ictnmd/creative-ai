/**
 * Toast Store
 * Manages transient toast notifications with auto-dismiss after 4 seconds.
 */

import { writable, derived } from 'svelte/store';
import type { Writable, Readable } from 'svelte/store';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
	id: string;
	type: ToastType;
	message: string;
	/** Unix timestamp when the toast was created, for ordering. */
	createdAt: number;
}

interface ToastState {
	toasts: Toast[];
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const DISMISS_TIMEOUT_MS = 4000;

function createToastStore() {
	const store: Writable<ToastState> = writable({ toasts: [] });

	function generateId(): string {
		return `toast-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
	}

	function addToast(type: ToastType, message: string) {
		const id = generateId();
		const toast: Toast = {
			id,
			type,
			message,
			createdAt: Date.now()
		};

		store.update((s) => ({
			toasts: [...s.toasts, toast]
		}));

		// Auto-dismiss after 4 seconds
		setTimeout(() => {
			store.update((s) => ({
				toasts: s.toasts.filter((t) => t.id !== id)
			}));
		}, DISMISS_TIMEOUT_MS);
	}

	return {
		subscribe: store.subscribe,

		/** Show a success toast (green). */
		success(message: string) {
			addToast('success', message);
		},

		/** Show an error toast (red). */
		error(message: string) {
			addToast('error', message);
		},

		/** Show an info toast (blue). */
		info(message: string) {
			addToast('info', message);
		},

		/** Show a warning toast (amber). */
		warning(message: string) {
			addToast('warning', message);
		},

		/** Remove a toast by its id. */
		remove(id: string) {
			store.update((s) => ({
				toasts: s.toasts.filter((t) => t.id !== id)
			}));
		},

		/** Remove all toasts. */
		clear() {
			store.set({ toasts: [] });
		}
	};
}

export const toast = createToastStore();

// Separate readable store that exposes just the toasts array, for use in templates
export const toasts: Readable<Toast[]> = derived(
	toast as unknown as Readable<{ toasts: Toast[] }>,
	($t) => $t.toasts
);
