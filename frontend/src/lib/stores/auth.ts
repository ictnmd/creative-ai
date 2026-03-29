/**
 * Auth Store
 * Manages authentication state including user info, loading state,
 * and initialization status. Provides derived stores for common checks.
 */

import { writable, derived } from 'svelte/store';
import type { Writable } from 'svelte/store';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface User {
	id: string;
	email: string;
	username: string;
	role: string;
	is_verified: boolean;
	is_active: boolean;
	avatar_url?: string;
	bio?: string;
	created_at?: string;
}

export interface AuthState {
	user: User | null;
	loading: boolean;
	initialized: boolean;
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

const initialState: AuthState = {
	user: null,
	loading: false,
	initialized: false
};

function createAuthStore() {
	const store: Writable<AuthState> = writable(initialState);

	return {
		subscribe: store.subscribe,

		/** Set the authenticated user. */
		setUser(user: User | null) {
			store.update((s) => ({ ...s, user, loading: false, initialized: true }));
		},

		/** Clear user and reset to unauthenticated state. */
		clearUser() {
			store.set({ user: null, loading: false, initialized: true });
		},

		/** Set loading state (e.g. during a profile fetch). */
		setLoading(loading: boolean) {
			store.update((s) => ({ ...s, loading }));
		},

		/** Mark the store as initialized after first mount. */
		setInitialized(initialized: boolean) {
			store.update((s) => ({ ...s, initialized }));
		},

		/** Reset to initial state. */
		reset() {
			store.set(initialState);
		}
	};
}

export const auth = createAuthStore();

// ---------------------------------------------------------------------------
// Derived stores
// ---------------------------------------------------------------------------

/** True when there is a valid, non-null user. */
export const isAuthenticated = derived(auth, ($auth) => $auth.user !== null);

/** True when the authenticated user has the admin role. */
export const isAdmin = derived(auth, ($auth) => $auth.user?.role === 'admin');

/** The current user, or null. */
export const currentUser = derived(auth, ($auth) => $auth.user);
