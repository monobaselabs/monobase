import { describe, test, expect } from "bun:test";
import { ApiError } from "../helpers/client";
import { createAuthenticatedClient, createAdminClient } from "../helpers/auth";

describe("Audit", () => {
  test("list audit logs (admin only)", async () => {
    const client = await createAdminClient();
    const result = await client.get("/audit/logs");

    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
    expect(result.pagination).toBeDefined();
  });

  test("list audit logs with filters", async () => {
    const client = await createAdminClient();
    const result = await client.get("/audit/logs", {
      eventType: "authentication",
      limit: 5,
    });

    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("non-admin returns 403", async () => {
    const client = await createAuthenticatedClient();
    try {
      await client.get("/audit/logs");
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(403);
    }
  });
});
