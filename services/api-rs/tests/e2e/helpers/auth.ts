/**
 * Authentication helpers for E2E tests.
 */

import { faker } from "@faker-js/faker";
import { ApiClient, createClient } from "./client";

let counter = 0;

/** Generate a unique test email that won't collide across parallel tests. */
export function generateTestEmail(): string {
  counter++;
  return `test-${Date.now()}-${counter}-${faker.string.nanoid(6)}@test.local`;
}

/** Sign up a new random user and return an authenticated client. */
export async function createAuthenticatedClient(
  baseUrl?: string,
): Promise<ApiClient> {
  const client = createClient(baseUrl);
  const email = generateTestEmail();
  const password = "TestPass123!";
  const name = `${faker.person.firstName()} ${faker.person.lastName()}`;
  await client.signup(name, email, password);
  return client;
}

/** Create an admin client. Requires AUTH_ADMIN_EMAILS to include this email. */
export async function createAdminClient(
  baseUrl?: string,
): Promise<ApiClient> {
  // Admin emails are configured on the server.
  // For tests, we sign up with a known admin email.
  // The server should auto-promote users whose email is in AUTH_ADMIN_EMAILS.
  const client = createClient(baseUrl);
  const email = "admin-test@test.local";
  const password = "AdminPass123!";
  const name = "Test Admin";
  try {
    await client.signup(name, email, password);
  } catch (e: any) {
    // Already exists — sign in instead
    if (e.status === 409) {
      await client.signin(email, password);
    } else {
      throw e;
    }
  }
  return client;
}
