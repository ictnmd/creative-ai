import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export interface SharedGeneration {
	generation_id: string;
	share_token: string;
	prompt: string;
	enhanced_prompt: string | null;
	output_urls: string[];
	provider: string;
	model: string;
	style_preset_name: string | null;
	view_count: number;
	created_at: string;
	creator_username: string;
	creator_avatar_url: string | null;
}

export const load: PageLoad = async ({ params, fetch }) => {
	const token = params.token;

	const baseUrl = import.meta.env.VITE_API_URL || 'http://localhost:8080';
	const response = await fetch(`${baseUrl}/api/v1/shared/${token}`);

	if (!response.ok) {
		if (response.status === 404) {
			throw error(404, 'Shared generation not found');
		}
		throw error(response.status, 'Failed to load shared generation');
	}

	const generation: SharedGeneration = await response.json();
	return { generation };
};
