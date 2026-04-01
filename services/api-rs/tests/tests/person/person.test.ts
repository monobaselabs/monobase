import { describe, test, expect } from "bun:test";
import { ApiError } from "../../src/client";
import { createAuthenticatedClient } from "../../src/auth";
import { createTestPerson, personData } from "../../src/fixtures";

describe("Person", () => {
  test("create person", async () => {
    const client = await createAuthenticatedClient();
    const data = personData();
    const person = await createTestPerson(client, data);

    expect(person.id).toBeDefined();
    expect(person.first_name || person.firstName).toBe(data.firstName);
    expect(person.last_name || person.lastName).toBe(data.lastName);
  });

  test("get person by ID", async () => {
    const client = await createAuthenticatedClient();
    const created = await createTestPerson(client);

    const fetched = await client.get(`/persons/${created.id}`);
    expect(fetched.id).toBe(created.id);
  });

  test("update person (owner)", async () => {
    const client = await createAuthenticatedClient();
    const created = await createTestPerson(client);

    const updated = await client.patch(`/persons/${created.id}`, {
      firstName: "Updated",
    });
    expect(updated.first_name || updated.firstName).toBe("Updated");
  });

  test("unauthenticated create returns 401", async () => {
    const client = (await createAuthenticatedClient()).anonymous();
    try {
      await createTestPerson(client);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(401);
    }
  });

  test("get nonexistent person returns 404", async () => {
    const client = await createAuthenticatedClient();
    try {
      await client.get("/persons/00000000-0000-0000-0000-000000000000");
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });
});
