/**
 * Boa JS Engine Entry Point
 *
 * This file is bundled as an IIFE for the embedded Boa JS engine in Tauri apps.
 * It sets up the Hono app with SQLite backend and exposes a global __dispatch function.
 *
 * The Rust side calls __dispatch(method, url, body, headersJson) and reads __res.
 */

// Web API shim for Boa (Request, Response, Headers, URL, crypto)
import './shim/web-api';

import { createApp } from '@/app';
import type { Config } from '@/core/config';

// Create config for SQLite embedded mode
// Note: In Boa, the __db global provides execute/select functions
// that bridge to the Rust SQLite wrapper
const config: Config = {
  server: {
    host: 'localhost',
    port: 0, // Not used in embedded mode
  },
  database: {
    // Use sqlite:// URL - dialect is auto-detected
    // In Boa, the __db global provides execute/select that bridge to Rust SQLite
    url: 'sqlite://:memory:',
    logging: false,
  },
  cors: {
    origins: ['*'],
    credentials: true,
    allowLocalNetwork: true,
    allowTunneling: false,
    strict: false,
  },
  logging: {
    level: 'info',
    pretty: false,
  },
  auth: {
    baseUrl: 'http://localhost',
    secret: 'boa-embedded-secret-' + Date.now(),
    sessionExpiresIn: 60 * 60 * 24 * 7, // 7 days
    rateLimitEnabled: false,
    rateLimitWindow: 60,
    rateLimitMax: 100,
    adminEmails: [],
    socialProviders: {},
  },
  rateLimit: {
    enabled: false,
    max: 1000,
  },
  storage: {
    provider: 'minio',
    endpoint: '',
    publicEndpoint: '',
    bucket: 'local',
    region: 'us-east-1',
    credentials: {
      accessKeyId: '',
      secretAccessKey: '',
    },
    uploadUrlExpiry: 300,
    downloadUrlExpiry: 900,
  },
  email: {
    provider: 'smtp',
    from: {
      name: 'Monobase',
      email: 'noreply@localhost',
    },
    smtp: {
      host: 'localhost',
      port: 1025,
      secure: false,
      auth: { user: '', pass: '' },
    },
  },
  notifs: {
    provider: 'onesignal',
  },
  billing: {
    provider: 'stripe',
    stripe: {},
  },
  webrtc: {
    iceServers: [],
  },
};

// Create the Hono app
const app = createApp(config);

// Global dispatcher for Boa engine
// Called by Rust: __dispatch(method, url, body, headersJson)
// Result stored in: __res (JSON string)
(globalThis as any).__dispatch = async function(
  method: string,
  url: string,
  body: string | null,
  headersJson: string
) {
  try {
    const headers = JSON.parse(headersJson);
    const req = new Request(url, {
      method,
      body: body === null ? undefined : body,
      headers,
    });

    const res = await app.fetch(req);
    const headersObj: Record<string, string> = {};
    res.headers.forEach((v, k) => { headersObj[k] = v; });
    const bodyText = await res.text();

    (globalThis as any).__res = JSON.stringify({
      s: res.status,
      b: bodyText,
      h: headersObj,
    });
  } catch (error) {
    console.error('[Boa] Dispatch error:', error);
    (globalThis as any).__res = JSON.stringify({
      s: 500,
      b: JSON.stringify({ error: 'Internal Server Error', message: String(error) }),
      h: { 'content-type': 'application/json' },
    });
  }
};

console.log('[Boa] Monobase API bundle loaded');
