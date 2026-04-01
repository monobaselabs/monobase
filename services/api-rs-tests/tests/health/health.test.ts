import { describe, test, expect } from "bun:test";
import { createClient } from "../../src/client";

describe("Health endpoints", () => {
  const client = createClient();

  test("GET /health returns pass", async () => {
    const data = await client.get("/health");
    expect(data.status).toBe("pass");
  });

  test("GET /livez returns pass", async () => {
    const data = await client.get("/livez");
    expect(data.status).toBe("pass");
  });

  test("GET /readyz returns pass", async () => {
    const data = await client.get("/readyz");
    expect(data.status).toBe("pass");
  });
});
