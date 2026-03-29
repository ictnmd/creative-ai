import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export interface PublicProfile {
	user_id: string;
	username: string;
	name: string;
	avatar_url: string | null;
	bio: string | null;
	follower_count: number;
	following_count: number;
	generation_count: number;
	public_generations: PublicGeneration[];
}

export interface PublicGeneration {
	id: string;
	share_token: string;
	prompt: string;
	thumbnail_url: string | null;
	view_count: number;
	created_at: string;
}

export const load: PageLoad = async ({ params, fetch }) => {
	const username = params.username;

	try {
		const baseUrl = import.meta.env.VITE_API_URL || 'http://localhost:8080';
		const response = await fetch(`${baseUrl}/api/v1/public/${username}`);

		if (!response.ok) {
			if (response.status === 404) {
				throw error(404, `User "${username}" not found`);
			}
			throw error(response.status, 'Failed to load profile');
		}

		const profile: PublicProfile = await response.json();
		return { profile };
	} catch (err) {
		throw err;
	}
};
