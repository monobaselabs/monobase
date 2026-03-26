/**
 * Web API Shim for Boa JS Engine
 *
 * Provides polyfills for Web APIs that Boa doesn't have natively:
 * - TextEncoder / TextDecoder
 * - Headers
 * - Request / Response
 * - URL / URLSearchParams
 * - crypto (bridges to __crypto native)
 * - btoa / atob
 *
 * The __db, __crypto, and __bcrypt globals are provided by the Rust side.
 */

// Skip if already defined (running in Node/Bun)
if (typeof globalThis.Response !== 'undefined') {
  // Web APIs already available
} else {
  // ===== TextEncoder / TextDecoder =====
  (globalThis as any).TextEncoder = class TextEncoder {
    encode(str: string): Uint8Array {
      const bytes: number[] = [];
      for (let i = 0; i < str.length; i++) {
        let c = str.charCodeAt(i);
        if (c < 0x80) {
          bytes.push(c);
        } else if (c < 0x800) {
          bytes.push(0xc0 | (c >> 6), 0x80 | (c & 0x3f));
        } else if (c < 0xd800 || c >= 0xe000) {
          bytes.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 0x3f), 0x80 | (c & 0x3f));
        } else {
          i++;
          c = 0x10000 + (((c & 0x3ff) << 10) | (str.charCodeAt(i) & 0x3ff));
          bytes.push(
            0xf0 | (c >> 18),
            0x80 | ((c >> 12) & 0x3f),
            0x80 | ((c >> 6) & 0x3f),
            0x80 | (c & 0x3f)
          );
        }
      }
      return new Uint8Array(bytes);
    }
  };

  (globalThis as any).TextDecoder = class TextDecoder {
    decode(bytes?: Uint8Array): string {
      if (!bytes) return '';
      let result = '';
      let i = 0;
      while (i < bytes.length) {
        const byte = bytes[i];
        if (byte < 0x80) {
          result += String.fromCharCode(byte);
          i++;
        } else if ((byte & 0xe0) === 0xc0) {
          result += String.fromCharCode(((byte & 0x1f) << 6) | (bytes[i + 1] & 0x3f));
          i += 2;
        } else if ((byte & 0xf0) === 0xe0) {
          result += String.fromCharCode(
            ((byte & 0x0f) << 12) | ((bytes[i + 1] & 0x3f) << 6) | (bytes[i + 2] & 0x3f)
          );
          i += 3;
        } else {
          const codePoint =
            ((byte & 0x07) << 18) |
            ((bytes[i + 1] & 0x3f) << 12) |
            ((bytes[i + 2] & 0x3f) << 6) |
            (bytes[i + 3] & 0x3f);
          result += String.fromCodePoint(codePoint);
          i += 4;
        }
      }
      return result;
    }
  };

  // ===== Headers =====
  (globalThis as any).Headers = class Headers {
    private _headers: Map<string, string> = new Map();

    constructor(init?: Record<string, string> | [string, string][]) {
      if (init) {
        if (Array.isArray(init)) {
          for (const [key, value] of init) {
            this.set(key, value);
          }
        } else {
          for (const key of Object.keys(init)) {
            this.set(key, init[key]);
          }
        }
      }
    }

    get(name: string): string | null {
      return this._headers.get(name.toLowerCase()) ?? null;
    }

    set(name: string, value: string): void {
      this._headers.set(name.toLowerCase(), value);
    }

    has(name: string): boolean {
      return this._headers.has(name.toLowerCase());
    }

    delete(name: string): void {
      this._headers.delete(name.toLowerCase());
    }

    forEach(callback: (value: string, key: string) => void): void {
      this._headers.forEach((value, key) => callback(value, key));
    }

    entries(): IterableIterator<[string, string]> {
      return this._headers.entries();
    }
  };

  // ===== Request =====
  (globalThis as any).Request = class Request {
    method: string;
    url: string;
    headers: Headers;
    body: string | null;

    constructor(input: string | Request, init?: RequestInit) {
      if (typeof input === 'string') {
        this.url = input;
        this.method = init?.method || 'GET';
        this.body = init?.body as string || null;
        this.headers = new Headers(init?.headers as any);
      } else {
        this.url = input.url;
        this.method = init?.method || input.method;
        this.body = init?.body as string || input.body;
        this.headers = new Headers(init?.headers as any || input.headers);
      }
    }

    async text(): Promise<string> {
      return this.body || '';
    }

    async json(): Promise<any> {
      return JSON.parse(this.body || 'null');
    }

    clone(): Request {
      return new Request(this.url, {
        method: this.method,
        headers: this.headers,
        body: this.body,
      });
    }
  };

  // ===== Response =====
  (globalThis as any).Response = class Response {
    status: number;
    statusText: string;
    headers: Headers;
    body: string | null;
    ok: boolean;

    constructor(body?: string | null, init?: ResponseInit) {
      this.body = body ?? null;
      this.status = init?.status ?? 200;
      this.statusText = init?.statusText ?? 'OK';
      this.headers = new Headers(init?.headers as any);
      this.ok = this.status >= 200 && this.status < 300;
    }

    async text(): Promise<string> {
      return this.body || '';
    }

    async json(): Promise<any> {
      return JSON.parse(this.body || 'null');
    }

    clone(): Response {
      return new Response(this.body, {
        status: this.status,
        statusText: this.statusText,
        headers: this.headers,
      });
    }

    static json(data: any, init?: ResponseInit): Response {
      const headers = new Headers(init?.headers as any);
      headers.set('content-type', 'application/json');
      return new Response(JSON.stringify(data), {
        ...init,
        headers,
      });
    }

    static redirect(url: string, status = 302): Response {
      const headers = new Headers();
      headers.set('location', url);
      return new Response(null, { status, headers });
    }
  };

  // ===== URLSearchParams =====
  (globalThis as any).URLSearchParams = class URLSearchParams {
    private _params: Map<string, string[]> = new Map();

    constructor(init?: string | Record<string, string> | [string, string][]) {
      if (typeof init === 'string') {
        const query = init.startsWith('?') ? init.slice(1) : init;
        for (const pair of query.split('&')) {
          const [key, value] = pair.split('=').map(decodeURIComponent);
          this.append(key, value || '');
        }
      } else if (Array.isArray(init)) {
        for (const [key, value] of init) {
          this.append(key, value);
        }
      } else if (init) {
        for (const key of Object.keys(init)) {
          this.append(key, init[key]);
        }
      }
    }

    get(name: string): string | null {
      const values = this._params.get(name);
      return values?.[0] ?? null;
    }

    getAll(name: string): string[] {
      return this._params.get(name) || [];
    }

    set(name: string, value: string): void {
      this._params.set(name, [value]);
    }

    append(name: string, value: string): void {
      const existing = this._params.get(name) || [];
      existing.push(value);
      this._params.set(name, existing);
    }

    has(name: string): boolean {
      return this._params.has(name);
    }

    delete(name: string): void {
      this._params.delete(name);
    }

    toString(): string {
      const pairs: string[] = [];
      this._params.forEach((values, key) => {
        for (const value of values) {
          pairs.push(`${encodeURIComponent(key)}=${encodeURIComponent(value)}`);
        }
      });
      return pairs.join('&');
    }

    forEach(callback: (value: string, key: string) => void): void {
      this._params.forEach((values, key) => {
        for (const value of values) {
          callback(value, key);
        }
      });
    }
  };

  // ===== URL =====
  (globalThis as any).URL = class URL {
    protocol: string = '';
    hostname: string = '';
    port: string = '';
    pathname: string = '/';
    search: string = '';
    hash: string = '';
    searchParams: URLSearchParams;

    constructor(url: string, base?: string) {
      let fullUrl = url;
      if (base && !url.match(/^[a-z]+:\/\//i)) {
        fullUrl = base.replace(/\/$/, '') + '/' + url.replace(/^\//, '');
      }

      const match = fullUrl.match(/^([a-z]+):\/\/([^/:]+)(?::(\d+))?(\/[^?#]*)?(\?[^#]*)?(#.*)?$/i);
      if (match) {
        this.protocol = match[1] + ':';
        this.hostname = match[2];
        this.port = match[3] || '';
        this.pathname = match[4] || '/';
        this.search = match[5] || '';
        this.hash = match[6] || '';
      }

      this.searchParams = new URLSearchParams(this.search);
    }

    get host(): string {
      return this.port ? `${this.hostname}:${this.port}` : this.hostname;
    }

    get origin(): string {
      return `${this.protocol}//${this.host}`;
    }

    get href(): string {
      return `${this.origin}${this.pathname}${this.search}${this.hash}`;
    }

    toString(): string {
      return this.href;
    }
  };

  // ===== crypto (bridges to __crypto native) =====
  const __crypto = (globalThis as any).__crypto;
  (globalThis as any).crypto = {
    getRandomValues(arr: Uint8Array): Uint8Array {
      if (__crypto) {
        const bytes = __crypto.getRandomValues(arr.length);
        for (let i = 0; i < arr.length; i++) {
          arr[i] = bytes[i];
        }
      }
      return arr;
    },
    randomUUID(): string {
      if (__crypto) {
        return __crypto.randomUUID();
      }
      // Fallback UUID v4 generation
      return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
        const r = (Math.random() * 16) | 0;
        const v = c === 'x' ? r : (r & 0x3) | 0x8;
        return v.toString(16);
      });
    },
    subtle: {
      async digest(algorithm: string, data: ArrayBuffer): Promise<ArrayBuffer> {
        if (__crypto && algorithm === 'SHA-256') {
          const bytes = __crypto.sha256(Array.from(new Uint8Array(data)));
          return new Uint8Array(bytes).buffer;
        }
        throw new Error(`Unsupported algorithm: ${algorithm}`);
      },
    },
  };

  // ===== btoa / atob =====
  const base64Chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

  (globalThis as any).btoa = function(str: string): string {
    let result = '';
    let i = 0;
    while (i < str.length) {
      const a = str.charCodeAt(i++);
      const b = i < str.length ? str.charCodeAt(i++) : 0;
      const c = i < str.length ? str.charCodeAt(i++) : 0;
      const triplet = (a << 16) | (b << 8) | c;
      result +=
        base64Chars[(triplet >> 18) & 0x3f] +
        base64Chars[(triplet >> 12) & 0x3f] +
        (i > str.length + 1 ? '=' : base64Chars[(triplet >> 6) & 0x3f]) +
        (i > str.length ? '=' : base64Chars[triplet & 0x3f]);
    }
    return result;
  };

  (globalThis as any).atob = function(str: string): string {
    str = str.replace(/=+$/, '');
    let result = '';
    for (let i = 0; i < str.length; i += 4) {
      const a = base64Chars.indexOf(str[i]);
      const b = base64Chars.indexOf(str[i + 1]);
      const c = base64Chars.indexOf(str[i + 2]);
      const d = base64Chars.indexOf(str[i + 3]);
      const triplet = (a << 18) | (b << 12) | (c << 6) | d;
      result += String.fromCharCode((triplet >> 16) & 0xff);
      if (c !== -1) result += String.fromCharCode((triplet >> 8) & 0xff);
      if (d !== -1) result += String.fromCharCode(triplet & 0xff);
    }
    return result;
  };

  console.log('[Shim] Web APIs initialized for Boa');
}

export {};
