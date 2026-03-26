// Placeholder bundle - will be replaced with actual Hono + Better Auth bundle
// Built from services/api/src/boa-entry.ts

(function() {
  'use strict';

  // Minimal dispatcher for testing - responds with 501 Not Implemented
  globalThis.__dispatch = function(method, url, body, headersJson) {
    console.log('[Bundle] Request:', method, url);

    // Placeholder response - actual bundle will use Hono app
    globalThis.__res = JSON.stringify({
      s: 501,
      b: JSON.stringify({
        error: 'Not Implemented',
        message: 'Bundle not yet built. Run: cd services/api && bun run build:boa'
      }),
      h: { 'content-type': 'application/json' }
    });
  };

  console.log('[Bundle] Placeholder bundle loaded - run build:boa to generate real bundle');
})();
