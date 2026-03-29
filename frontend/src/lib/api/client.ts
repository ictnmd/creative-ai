/**
 * API Client
 * Central HTTP client for communicating with the backend API.
 * Uses PUBLIC_API_URL environment variable for the base URL.
 */

const BASE_URL = import.meta.env.PUBLIC_API_URL || 'http://localhost:3001';

export interface ApiError {
	message: string;
	code?: string;
	status?: number;
}

export interface ApiResponse<T> {
	data: T;
	status: number;
}

export class ApiClient {
	private baseUrl: string;

	constructor(baseUrl: string = BASE_URL) {
		this.baseUrl = baseUrl;
	}

	private async request<T>(
		method: string,
		path: string,
		options: RequestInit = {}
	): Promise<ApiResponse<T>> {
		const url = `${this.baseUrl}${path}`;

		const config: RequestInit = {
			...options,
			method,
			headers: {
				'Content-Type': 'application/json',
				...options.headers
			},
			credentials: 'include'
		};

		try {
			const response = await fetch(url, config);

			if (!response.ok) {
				let errorMessage = `HTTP ${response.status}: ${response.statusText}`;
				try {
					const errorData = await response.json();
					errorMessage = errorData.message || errorMessage;
				} catch {
					// Response body is not JSON, use status text
				}
				const error: ApiError = {
					message: errorMessage,
					status: response.status
				};
				throw error;
			}

			// Handle 204 No Content
			if (response.status === 204) {
				return { data: undefined as T, status: response.status };
			}

			const data = await response.json();
			return { data, status: response.status };
		} catch (err) {
			if (err instanceof Error && !(err as ApiError).status) {
				// Network error or other non-HTTP error
				throw {
					message: err.message || 'Network error',
					status: 0
				} as ApiError;
			}
			throw err;
		}
	}

	async get<T>(path: string, options?: RequestInit): Promise<ApiResponse<T>> {
		return this.request<T>('GET', path, options);
	}

	async post<T>(path: string, body?: unknown, options?: RequestInit): Promise<ApiResponse<T>> {
		return this.request<T>('POST', path, {
			...options,
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async put<T>(path: string, body?: unknown, options?: RequestInit): Promise<ApiResponse<T>> {
		return this.request<T>('PUT', path, {
			...options,
			body: body ? JSON.stringify(body) : undefined
		});
	}

	async delete<T>(path: string, options?: RequestInit): Promise<ApiResponse<T>> {
		return this.request<T>('DELETE', path, options);
	}
}

// Singleton instance
export const api = new ApiClient();
