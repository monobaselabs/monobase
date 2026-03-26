#!/usr/bin/env node
/**
 * Build script for Boa JS Engine bundle
 *
 * Creates an IIFE bundle of the Hono API for embedding in Tauri apps.
 * The bundle is loaded by the Boa JS engine and provides a complete
 * backend API accessible via the __dispatch global function.
 *
 * Usage: bun run build:boa [--watch]
 */

import * as esbuild from 'esbuild';
import { gzipSync } from 'zlib';
import { writeFileSync, mkdirSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..');
const OUTPUT_DIR = join(ROOT, '../../apps/patient/src-tauri/js');
const OUTPUT_FILE = join(OUTPUT_DIR, 'bundle.js');
const OUTPUT_GZ = join(OUTPUT_DIR, 'bundle.js.gz');

// Ensure output directory exists
mkdirSync(OUTPUT_DIR, { recursive: true });

const isWatch = process.argv.includes('--watch');

/** @type {esbuild.BuildOptions} */
const buildOptions = {
  entryPoints: [join(ROOT, 'src/boa-entry.ts')],
  bundle: true,
  format: 'iife',
  target: 'es2020',
  platform: 'neutral', // Boa is neither node nor browser
  outfile: OUTPUT_FILE,
  minify: !isWatch,
  sourcemap: isWatch ? 'inline' : false,

  // Tree-shaking
  treeShaking: true,

  // Define globals for Boa environment
  define: {
    'process.env.NODE_ENV': '"production"',
    'globalThis.Bun': 'undefined', // Not running in Bun
  },

  // External modules that don't work in Boa
  external: [
    'pg',
    'pg-boss',
    'nodemailer',
    'stripe',
    '@aws-sdk/*',
    '@onesignal/*',
    'postmark',
    'drizzle-orm/node-postgres',
  ],

  // Replace node modules with shims
  alias: {
    // Use our web-api shims instead of node polyfills
    'node:crypto': join(ROOT, 'src/shim/empty.ts'),
    'crypto': join(ROOT, 'src/shim/empty.ts'),
  },

  // Banner to mark the bundle
  banner: {
    js: '// Monobase API Bundle for Boa JS Engine\n// Generated: ' + new Date().toISOString() + '\n',
  },

  // Log level
  logLevel: 'info',
};

async function build() {
  console.log('[build-boa] Building Boa bundle...');

  try {
    const result = await esbuild.build(buildOptions);

    if (result.errors.length > 0) {
      console.error('[build-boa] Build errors:', result.errors);
      process.exit(1);
    }

    // Read the built file and gzip it
    const bundle = Bun.file(OUTPUT_FILE);
    const content = await bundle.text();
    const gzipped = gzipSync(content, { level: 9 });

    writeFileSync(OUTPUT_GZ, gzipped);

    const sizeKb = (content.length / 1024).toFixed(1);
    const gzSizeKb = (gzipped.length / 1024).toFixed(1);

    console.log(`[build-boa] Bundle: ${sizeKb} KB`);
    console.log(`[build-boa] Gzipped: ${gzSizeKb} KB (${((1 - gzipped.length / content.length) * 100).toFixed(0)}% reduction)`);
    console.log(`[build-boa] Output: ${OUTPUT_GZ}`);

  } catch (error) {
    console.error('[build-boa] Build failed:', error);
    process.exit(1);
  }
}

async function watch() {
  console.log('[build-boa] Starting watch mode...');

  const ctx = await esbuild.context({
    ...buildOptions,
    plugins: [
      {
        name: 'gzip-on-end',
        setup(build) {
          build.onEnd(async (result) => {
            if (result.errors.length === 0) {
              const bundle = Bun.file(OUTPUT_FILE);
              const content = await bundle.text();
              const gzipped = gzipSync(content, { level: 9 });
              writeFileSync(OUTPUT_GZ, gzipped);
              console.log(`[build-boa] Rebuilt (${(gzipped.length / 1024).toFixed(1)} KB gzipped)`);
            }
          });
        },
      },
    ],
  });

  await ctx.watch();
  console.log('[build-boa] Watching for changes...');
}

if (isWatch) {
  watch();
} else {
  build();
}
