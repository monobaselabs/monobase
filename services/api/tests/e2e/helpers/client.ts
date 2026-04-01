/**
 * Language-agnostic HTTP API client for E2E testing.
 * Uses real fetch() — works against any server implementing the Monobase API.
 */

export const API_URL = process.env.API_URL || "http://localhost:7213";

export interface ApiResponse<T = any> {
  status: number;
  data: T;
  headers: Headers;
}

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: any,
    public path: string,
  ) {
    const msg = body?.message || body?.code || `HTTP ${status}`;
    super(`${path}: ${msg}`);
    this.name = "ApiError";
  }
}

export class ApiClient {
  private baseUrl: string;
  private token: string | null = null;
  public currentUser: any = null;

  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl || API_URL;
  }

  // -- Auth state --

  setToken(token: string) {
    this.token = token;
  }

  getToken(): string | null {
    return this.token;
  }

  clearAuth() {
    this.token = null;
    this.currentUser = null;
  }

  private headers(): Record<string, string> {
    const h: Record<string, string> = { "Content-Type": "application/json" };
    if (this.token) {
      h["Authorization"] = `Bearer ${this.token}`;
    }
    return h;
  }

  // -- Core HTTP methods --

  async request<T = any>(
    method: string,
    path: string,
    body?: any,
  ): Promise<ApiResponse<T>> {
    const url = `${this.baseUrl}${path}`;
    const res = await fetch(url, {
      method,
      headers: this.headers(),
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    let data: any;
    const contentType = res.headers.get("content-type") || "";
    if (contentType.includes("application/json")) {
      data = await res.json();
    } else {
      data = await res.text();
    }

    if (!res.ok) {
      throw new ApiError(res.status, data, `${method} ${path}`);
    }

    return { status: res.status, data, headers: res.headers };
  }

  async get<T = any>(path: string, query?: Record<string, any>): Promise<T> {
    let fullPath = path;
    if (query) {
      const params = new URLSearchParams();
      for (const [k, v] of Object.entries(query)) {
        if (v !== undefined && v !== null) params.set(k, String(v));
      }
      const qs = params.toString();
      if (qs) fullPath += `?${qs}`;
    }
    const res = await this.request<T>("GET", fullPath);
    return res.data;
  }

  async post<T = any>(path: string, body?: any): Promise<T> {
    const res = await this.request<T>("POST", path, body);
    return res.data;
  }

  async patch<T = any>(path: string, body?: any): Promise<T> {
    const res = await this.request<T>("PATCH", path, body);
    return res.data;
  }

  async delete<T = any>(path: string): Promise<T> {
    const res = await this.request<T>("DELETE", path);
    return res.data;
  }

  /** POST with raw response (for checking status codes). */
  async postRaw(path: string, body?: any): Promise<ApiResponse> {
    return this.request("POST", path, body);
  }

  /** GET with raw response. */
  async getRaw(path: string, query?: Record<string, any>): Promise<ApiResponse> {
    let fullPath = path;
    if (query) {
      const params = new URLSearchParams();
      for (const [k, v] of Object.entries(query)) {
        if (v !== undefined && v !== null) params.set(k, String(v));
      }
      const qs = params.toString();
      if (qs) fullPath += `?${qs}`;
    }
    return this.request("GET", fullPath);
  }

  /** DELETE with raw response. */
  async deleteRaw(path: string): Promise<ApiResponse> {
    return this.request("DELETE", path);
  }

  // -- Auth convenience --

  async signup(
    name: string,
    email: string,
    password: string,
  ): Promise<{ user: any; token: string }> {
    const res = await this.post<{ user: any; token: string }>(
      "/auth/sign-up/email",
      { name, email, password },
    );
    this.token = res.token;
    this.currentUser = res.user;
    return res;
  }

  async signin(
    email: string,
    password: string,
  ): Promise<{ user: any; token: string }> {
    const res = await this.post<{ user: any; token: string }>(
      "/auth/sign-in/email",
      { email, password },
    );
    this.token = res.token;
    this.currentUser = res.user;
    return res;
  }

  async signout(): Promise<void> {
    await this.post("/auth/sign-out");
    this.clearAuth();
  }

  async getSession(): Promise<{ user: any }> {
    return this.get("/auth/get-session");
  }

  /** Clone this client (same auth state). */
  clone(): ApiClient {
    const c = new ApiClient(this.baseUrl);
    c.token = this.token;
    c.currentUser = this.currentUser;
    return c;
  }

  /** Create a new anonymous client (no auth). */
  anonymous(): ApiClient {
    return new ApiClient(this.baseUrl);
  }
}

/** Create a fresh anonymous client. */
export function createClient(baseUrl?: string): ApiClient {
  return new ApiClient(baseUrl);
}
