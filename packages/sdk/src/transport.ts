/**
 * Transport abstraction for API communication
 *
 * Provides a unified interface for making API requests that can switch between:
 * - HttpTransport: Standard fetch-based HTTP requests (browser/Node)
 * - IpcTransport: Tauri IPC for embedded backend communication
 *
 * The transport is auto-detected based on the environment:
 * - If running in Tauri and embedded mode is enabled → IpcTransport
 * - Otherwise → HttpTransport
 */

/**
 * Response from transport layer
 */
export interface TransportResponse {
  status: number;
  body: string;
  headers: Record<string, string>;
}

/**
 * Request options for transport
 */
export interface TransportRequest {
  method: string;
  url: string;
  body?: string;
  headers?: Record<string, string>;
}

/**
 * Transport interface - implemented by HTTP and IPC transports
 */
export interface Transport {
  request(req: TransportRequest): Promise<TransportResponse>;
}

/**
 * HTTP Transport - uses fetch for standard HTTP requests
 */
export class HttpTransport implements Transport {
  constructor(private baseUrl: string) {}

  async request(req: TransportRequest): Promise<TransportResponse> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), 30000);

    try {
      const response = await fetch(`${this.baseUrl}${req.url}`, {
        method: req.method,
        body: req.body,
        headers: {
          'Content-Type': 'application/json',
          ...req.headers,
        },
        credentials: 'include',
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      const headers: Record<string, string> = {};
      response.headers.forEach((value, key) => {
        headers[key] = value;
      });

      return {
        status: response.status,
        body: await response.text(),
        headers,
      };
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error && error.name === 'AbortError') {
        return {
          status: 408,
          body: JSON.stringify({ message: 'Request timeout' }),
          headers: { 'content-type': 'application/json' },
        };
      }

      throw error;
    }
  }
}

/**
 * IPC Transport - uses Tauri invoke for embedded backend
 */
export class IpcTransport implements Transport {
  private invoke: ((cmd: string, args: any) => Promise<any>) | null = null;

  constructor() {
    // Dynamically import Tauri API
    this.initTauri();
  }

  private async initTauri() {
    try {
      // Dynamic import to avoid bundling Tauri in web builds
      const tauri = await import('@tauri-apps/api/core');
      this.invoke = tauri.invoke;
    } catch {
      console.warn('[IpcTransport] Tauri API not available');
    }
  }

  async request(req: TransportRequest): Promise<TransportResponse> {
    if (!this.invoke) {
      throw new Error('Tauri IPC not available');
    }

    try {
      const result = await this.invoke('api_request', {
        method: req.method,
        url: req.url,
        body: req.body || null,
        headers: req.headers || {},
      });

      return {
        status: result.status,
        body: result.body,
        headers: result.headers || {},
      };
    } catch (error) {
      // Handle IPC errors
      return {
        status: 500,
        body: JSON.stringify({
          message: 'IPC request failed',
          error: String(error),
        }),
        headers: { 'content-type': 'application/json' },
      };
    }
  }
}

// Transport state
let currentTransport: Transport | null = null;
let transportMode: 'http' | 'ipc' | 'auto' = 'auto';
let httpBaseUrl = 'http://localhost:7213';

/**
 * Check if running in Tauri environment
 */
export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Check if embedded mode is enabled
 */
export function isEmbeddedMode(): boolean {
  if (typeof window !== 'undefined') {
    // Check for explicit embedded mode flag
    return (window as any).__MONOBASE_EMBEDDED__ === true;
  }
  return false;
}

/**
 * Set transport mode
 * - 'http': Always use HTTP transport
 * - 'ipc': Always use IPC transport (requires Tauri)
 * - 'auto': Auto-detect based on environment
 */
export function setTransportMode(mode: 'http' | 'ipc' | 'auto') {
  transportMode = mode;
  currentTransport = null; // Reset to force re-initialization
}

/**
 * Set base URL for HTTP transport
 */
export function setHttpBaseUrl(url: string) {
  httpBaseUrl = url;
  if (currentTransport instanceof HttpTransport) {
    currentTransport = new HttpTransport(url);
  }
}

/**
 * Get the current transport instance
 */
export function getTransport(): Transport {
  if (currentTransport) {
    return currentTransport;
  }

  // Determine transport based on mode
  if (transportMode === 'ipc' || (transportMode === 'auto' && isTauriEnvironment() && isEmbeddedMode())) {
    currentTransport = new IpcTransport();
  } else {
    currentTransport = new HttpTransport(httpBaseUrl);
  }

  return currentTransport;
}

/**
 * Get transport info for debugging
 */
export function getTransportInfo(): {
  mode: 'http' | 'ipc' | 'auto';
  active: 'http' | 'ipc';
  baseUrl: string;
  isTauri: boolean;
  isEmbedded: boolean;
} {
  const transport = getTransport();
  return {
    mode: transportMode,
    active: transport instanceof IpcTransport ? 'ipc' : 'http',
    baseUrl: httpBaseUrl,
    isTauri: isTauriEnvironment(),
    isEmbedded: isEmbeddedMode(),
  };
}
