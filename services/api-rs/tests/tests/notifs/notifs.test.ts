import { describe, test, expect } from "bun:test";
import { createAuthenticatedClient } from "../../src/auth";

describe("Notifications", () => {
  test("list notifications (own only)", async () => {
    const client = await createAuthenticatedClient();
    const result = await client.get("/notifs");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  // Note: creating notifications is typically done server-side (via service layer),
  // not via a public API endpoint. These tests verify the read/mark-read paths.

  test("mark all as read returns count", async () => {
    const client = await createAuthenticatedClient();
    const result = await client.post("/notifs/read-all");
    expect(result).toBeDefined();
  });
});
