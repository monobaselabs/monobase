import { describe, test, expect } from "bun:test";
import { ApiError } from "../../src/client";
import { createAuthenticatedClient } from "../../src/auth";
import { createTestPerson, createTestProvider } from "../../src/fixtures";

describe("Provider", () => {
  test("create provider", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const provider = await createTestProvider(client, person.id, "physician");

    expect(provider.id).toBeDefined();
    expect(provider.provider_type || provider.providerType).toBe("physician");
  });

  test("list providers (any authenticated user)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    await createTestProvider(client, person.id);

    const result = await client.get("/providers");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("get provider by ID", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const provider = await createTestProvider(client, person.id);

    const fetched = await client.get(`/providers/${provider.id}`);
    expect(fetched.id).toBe(provider.id);
  });

  test("update provider (owner only)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const provider = await createTestProvider(client, person.id);

    const updated = await client.patch(`/providers/${provider.id}`, {
      biography: "Updated bio",
    });
    expect(updated.biography).toBe("Updated bio");
  });

  test("delete provider (owner only)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const provider = await createTestProvider(client, person.id);

    await client.delete(`/providers/${provider.id}`);

    try {
      await client.get(`/providers/${provider.id}`);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });

  test("unauthenticated list returns 401", async () => {
    const client = (await createAuthenticatedClient()).anonymous();
    try {
      await client.get("/providers");
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(401);
    }
  });
});
