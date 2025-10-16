#!/usr/bin/env bun

/**
 * Monobase API Build Script
 *
 * Handles standalone binary compilation with build-time constants injection
 * Based on the pattern from monobe/services/hapihub
 */

import { $ } from 'bun';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

// =====================================
// BUILD CONSTANTS
// =====================================

export interface BuildConstants {
  BUILD_VERSION: string;
  BUILD_TIME: string;
  BUILD_TIMESTAMP: string;
  GIT_COMMIT: string;
  GIT_BRANCH: string;
  COMPILER_VERSION: string;
}

/**
 * Get current timestamp in ISO format
 */
function getCurrentTimestamp(): string {
  return new Date().toISOString();
}

/**
 * Get human-readable build time
 */
function getBuildTime(): string {
  return new Date().toLocaleString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    timeZoneName: 'short',
  });
}

/**
 * Get Git commit hash
 */
async function getGitCommit(): Promise<string> {
  try {
    if (!existsSync('.git')) {
      return 'unknown';
    }
    const result = await $`git rev-parse HEAD`.text();
    return result.trim();
  } catch (error) {
    console.warn('Warning: Could not get git commit hash:', error);
    return 'unknown';
  }
}

/**
 * Get Git branch name
 */
async function getGitBranch(): Promise<string> {
  try {
    if (!existsSync('.git')) {
      return 'unknown';
    }
    const result = await $`git rev-parse --abbrev-ref HEAD`.text();
    return result.trim();
  } catch (error) {
    console.warn('Warning: Could not get git branch:', error);
    return 'unknown';
  }
}

/**
 * Get package version from package.json
 */
function getPackageVersion(): string {
  try {
    const packagePath = join(process.cwd(), 'package.json');
    const packageJson = JSON.parse(readFileSync(packagePath, 'utf-8'));
    return packageJson.version || '0.0.0';
  } catch (error) {
    console.warn('Warning: Could not read package.json version:', error);
    return '0.0.0';
  }
}

/**
 * Get Bun version
 */
function getCompilerVersion(): string {
  return Bun.version;
}

/**
 * Generate all build constants
 */
export async function generateBuildConstants(): Promise<BuildConstants> {
  console.log('📦 Generating build constants...');

  const [gitCommit, gitBranch] = await Promise.all([getGitCommit(), getGitBranch()]);

  const constants: BuildConstants = {
    BUILD_VERSION: getPackageVersion(),
    BUILD_TIME: getBuildTime(),
    BUILD_TIMESTAMP: getCurrentTimestamp(),
    GIT_COMMIT: gitCommit,
    GIT_BRANCH: gitBranch,
    COMPILER_VERSION: getCompilerVersion(),
  };

  console.log('✅ Build constants generated:');
  console.log(`   Version: ${constants.BUILD_VERSION}`);
  console.log(`   Git: ${constants.GIT_COMMIT.substring(0, 7)} (${constants.GIT_BRANCH})`);
  console.log(`   Compiler: Bun v${constants.COMPILER_VERSION}`);
  console.log(`   Built: ${constants.BUILD_TIME}`);

  return constants;
}

/**
 * Convert build constants to Bun --define arguments
 */
export function constantsToDefineArgs(constants: BuildConstants): string[] {
  const args: string[] = [];

  Object.entries(constants).forEach(([key, value]) => {
    // Each constant needs to be JSON-encoded
    args.push('--define', `${key}=${JSON.stringify(value)}`);
  });

  return args;
}

/**
 * Build standalone binary
 */
async function buildBinary(constants: BuildConstants): Promise<boolean> {
  console.log('\n🔨 Building standalone binary...');

  try {
    // Prepare build command arguments
    const buildArgs = [
      'build',
      '--compile',
      '--minify',
      '--sourcemap',
      '--define',
      'process.env.NODE_ENV="production"',
      '--external',
      'ajv',
      '--external',
      'ajv-draft-04',
      'src/index.ts',
      '--outfile',
      'dist/server',
    ];

    // Add build-time constant defines
    const defineArgs = constantsToDefineArgs(constants);
    buildArgs.push(...defineArgs);

    console.log(`🔧 Bun version: ${Bun.version}`);
    console.log(`📝 Build constants: ${defineArgs.length / 2} defined`);

    // Execute build
    const buildProcess = Bun.spawn(['bun', ...buildArgs], {
      stdout: 'pipe',
      stderr: 'pipe',
      cwd: process.cwd(),
    });

    const output = await new Response(buildProcess.stdout).text();
    const errorOutput = await new Response(buildProcess.stderr).text();
    const exitCode = await buildProcess.exited;

    // Print build output
    if (output) console.log(output);
    if (errorOutput) console.error(errorOutput);

    // Check build status
    if (exitCode === 0) {
      console.log('✅ Build successful!');
      console.log(`📦 Output: dist/server`);
      console.log(`🏷️  Version: ${constants.BUILD_VERSION}`);
      console.log(`🔖 Git: ${constants.GIT_COMMIT.substring(0, 7)} (${constants.GIT_BRANCH})`);
      return true;
    } else {
      console.log('❌ Build failed');
      return false;
    }
  } catch (error) {
    console.log(`❌ Build failed: ${error}`);
    return false;
  }
}

/**
 * Main execution
 */
async function main(): Promise<void> {
  console.log('🚀 Monobase API Build Script\n');
  console.log('═══════════════════════════════════════════════════════════\n');

  // Generate build constants
  const buildConstants = await generateBuildConstants();

  // Build binary
  const success = await buildBinary(buildConstants);

  console.log('\n═══════════════════════════════════════════════════════════');

  if (!success) {
    process.exit(1);
  }

  console.log('✨ Build complete!\n');
}

// Run if this is the main module
if (import.meta.main) {
  try {
    await main();
  } catch (error) {
    console.error(`❌ Fatal error: ${error}`);
    process.exit(1);
  }
}
