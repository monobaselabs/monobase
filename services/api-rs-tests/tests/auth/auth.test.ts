import { describe, test, expect } from "bun:test";
import { createClient, ApiError } from "../../src/client";
import { generateTestEmail } from "../../src/auth";

describe("Authentication", () => {
  test("sign up creates user and returns token", async () => {
    const client = createClient();
    const email = generateTestEmail();
    const res = await client.signup("Test User", email, "TestPass123!");

    expect(res.user).toBeDefined();
    expect(res.user.id).toBeDefined();
    expect(res.user.email).toBe(email);
    expect(res.user.name).toBe("Test User");
    expect(res.token).toBeDefined();
    expect(typeof res.token).toBe("string");
  });

  test("sign in with valid credentials", async () => {
    const client = createClient();
    const email = generateTestEmail();
    const password = "TestPass123!";
    await client.signup("Test User", email, password);

    // Sign in with a fresh client
    const client2 = createClient();
    const res = await client2.signin(email, password);

    expect(res.user).toBeDefined();
    expect(res.user.email).toBe(email);
    expect(res.token).toBeDefined();
  });

  test("sign in with wrong password returns 401", async () => {
    const client = createClient();
    const email = generateTestEmail();
    await client.signup("Test User", email, "TestPass123!");

    const client2 = createClient();
    try {
      await client2.signin(email, "WrongPassword!");
      expect(true).toBe(false); // Should not reach here
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(401);
    }
  });

  test("sign up with duplicate email returns 409", async () => {
    const client = createClient();
    const email = generateTestEmail();
    await client.signup("Test User", email, "TestPass123!");

    const client2 = createClient();
    try {
      await client2.signup("Another User", email, "TestPass456!");
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(409);
    }
  });

  test("get session returns current user", async () => {
    const client = createClient();
    const email = generateTestEmail();
    await client.signup("Test User", email, "TestPass123!");

    const session = await client.getSession();
    expect(session.user).toBeDefined();
    expect(session.user.email).toBe(email);
  });

  test("get session without auth returns 401", async () => {
    const client = createClient();
    try {
      await client.getSession();
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(401);
    }
  });

  test("sign out invalidates session", async () => {
    const client = createClient();
    const email = generateTestEmail();
    await client.signup("Test User", email, "TestPass123!");

    await client.signout();

    // Session should be invalid now
    try {
      await client.getSession();
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(401);
    }
  });
});
