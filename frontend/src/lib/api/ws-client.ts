/**
 * WebSocket Client
 * Real-time generation status updates with JWT auth, auto-reconnect, and exponential backoff.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface GenerationUpdate {
	generation_id: string;
	status: 'pending' | 'processing' | 'completed' | 'failed';
	message: string;
	timestamp: string;
	image_url?: string;
}

export interface WsServerMessage {
	type: 'update' | 'notice' | 'auth_success' | 'error';
	payload?: GenerationUpdate;
	message?: string;
}

// ---------------------------------------------------------------------------
// WebSocket Client
// ---------------------------------------------------------------------------

const WS_URL = (typeof import.meta !== 'undefined' && import.meta.env?.VITE_WS_URL) || 'ws://localhost:8080';
const WS_PATH = '/ws/generations';

export class WsClient {
	private ws: WebSocket | null = null;
	private readonly url: string;
	private reconnectDelay = 1000;
	private readonly maxReconnectDelay = 30000;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private currentToken: string | null = null;
	private subscribedIds = new Set<string>();
	private updateCallbacks = new Set<(update: GenerationUpdate) => void>();
	private noticeCallbacks = new Set<(message: string) => void>();
	private isManualDisconnect = false;

	constructor() {
		this.url = `${WS_URL}${WS_PATH}`;
	}

	/**
	 * Connect to the WebSocket server with JWT authentication.
	 * Sends an authenticate message immediately after connection opens.
	 */
	connect(token: string): void {
		if (this.ws?.readyState === WebSocket.OPEN || this.ws?.readyState === WebSocket.CONNECTING) {
			return;
		}

		this.isManualDisconnect = false;
		this.currentToken = token;

		try {
			this.ws = new WebSocket(this.url);

			this.ws.onopen = () => {
				if (this.ws && this.currentToken) {
					this.ws.send(
						JSON.stringify({
							action: 'authenticate',
							token: this.currentToken
						})
					);
				}
			};

			this.ws.onmessage = (event: MessageEvent) => {
				try {
					const msg: WsServerMessage = JSON.parse(event.data as string);

					switch (msg.type) {
						case 'update':
							if (msg.payload) {
								this.updateCallbacks.forEach((cb) => cb(msg.payload as GenerationUpdate));
							}
							break;
						case 'notice':
							if (msg.message) {
								this.noticeCallbacks.forEach((cb) => cb(msg.message as string));
							}
							break;
						case 'auth_success':
							// Re-subscribe to previously subscribed generation IDs after reconnect
							this.subscribedIds.forEach((id) => {
								this.sendRaw({ action: 'subscribe', generation_id: id });
							});
							break;
					}
				} catch {
					// Ignore malformed messages
				}
			};

			this.ws.onerror = () => {
				// Errors are handled in onclose
			};

			this.ws.onclose = () => {
				if (!this.isManualDisconnect) {
					this.scheduleReconnect();
				}
			};
		} catch {
			this.scheduleReconnect();
		}
	}

	/**
	 * Subscribe to real-time updates for a specific generation ID.
	 */
	subscribe(generationId: string): void {
		this.subscribedIds.add(generationId);
		this.sendRaw({ action: 'subscribe', generation_id: generationId });
	}

	/**
	 * Unsubscribe from updates for a specific generation ID.
	 */
	unsubscribe(generationId: string): void {
		this.subscribedIds.delete(generationId);
		this.sendRaw({ action: 'unsubscribe', generation_id: generationId });
	}

	/**
	 * Disconnect from the WebSocket server.
	 */
	disconnect(): void {
		this.isManualDisconnect = true;
		if (this.reconnectTimer !== null) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}
		this.subscribedIds.clear();
	}

	/**
	 * Register a callback for generation update messages.
	 */
	onUpdate(callback: (update: GenerationUpdate) => void): () => void {
		this.updateCallbacks.add(callback);
		return () => {
			this.updateCallbacks.delete(callback);
		};
	}

	/**
	 * Register a callback for notice messages.
	 */
	onNotice(callback: (message: string) => void): () => void {
		this.noticeCallbacks.add(callback);
		return () => {
			this.noticeCallbacks.delete(callback);
		};
	}

	private sendRaw(data: Record<string, unknown>): void {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(data));
		}
	}

	private scheduleReconnect(): void {
		if (this.reconnectTimer !== null) {
			return;
		}

		this.reconnectTimer = setTimeout(() => {
			this.reconnectTimer = null;
			if (this.currentToken && !this.isManualDisconnect) {
				this.connect(this.currentToken);
			}
			// Double delay up to maxReconnectDelay
			this.reconnectDelay = Math.min(this.reconnectDelay * 2, this.maxReconnectDelay);
		}, this.reconnectDelay);
	}
}

// Singleton instance
export const wsClient = new WsClient();
