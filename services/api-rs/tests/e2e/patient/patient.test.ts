import { describe, test, expect } from "bun:test";
import { ApiError } from "../helpers/client";
import { createAuthenticatedClient } from "../helpers/auth";
import { createTestPerson, createTestPatient } from "../helpers/fixtures";

describe("Patient", () => {
  test("create patient with person_id", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);

    expect(patient.id).toBeDefined();
    expect(patient.person_id || patient.personId).toBe(person.id);
  });

  test("get patient by ID", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);

    const fetched = await client.get(`/patients/${patient.id}`);
    expect(fetched.id).toBe(patient.id);
  });

  test("update patient", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);

    const updated = await client.patch(`/patients/${patient.id}`, {
      primaryPharmacy: { name: "Test Pharmacy" },
    });
    expect(updated.id).toBe(patient.id);
  });

  test("delete patient", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);

    await client.delete(`/patients/${patient.id}`);

    try {
      await client.get(`/patients/${patient.id}`);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });

  test("duplicate person_id returns 409", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    await createTestPatient(client, person.id);

    try {
      await createTestPatient(client, person.id);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(409);
    }
  });
});
