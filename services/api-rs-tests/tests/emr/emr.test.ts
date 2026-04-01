import { describe, test, expect } from "bun:test";
import { ApiError } from "../../src/client";
import { createAuthenticatedClient } from "../../src/auth";
import {
  createTestPerson,
  createTestPatient,
  createTestProvider,
  createTestConsultation,
} from "../../src/fixtures";

describe("EMR", () => {
  test("create consultation", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);
    const providerPerson = await createTestPerson(client);
    const provider = await createTestProvider(client, providerPerson.id);

    const consultation = await createTestConsultation(
      client,
      patient.id,
      provider.id,
    );

    expect(consultation.id).toBeDefined();
    expect(consultation.status).toBe("draft");
  });

  test("update consultation (draft only)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);
    const providerPerson = await createTestPerson(client);
    const provider = await createTestProvider(client, providerPerson.id);
    const consultation = await createTestConsultation(
      client,
      patient.id,
      provider.id,
    );

    const updated = await client.patch(
      `/emr/consultations/${consultation.id}`,
      {
        assessment: "Patient shows improvement",
        plan: "Continue current treatment",
      },
    );

    expect(updated.assessment).toBe("Patient shows improvement");
    expect(updated.plan).toBe("Continue current treatment");
  });

  test("finalize consultation", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);
    const providerPerson = await createTestPerson(client);
    const provider = await createTestProvider(client, providerPerson.id);
    const consultation = await createTestConsultation(
      client,
      patient.id,
      provider.id,
    );

    const finalized = await client.post(
      `/emr/consultations/${consultation.id}/finalize`,
    );

    expect(finalized.status).toBe("finalized");
    expect(finalized.finalized_at ?? finalized.finalizedAt).toBeDefined();
  });

  test("cannot update finalized consultation", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const patient = await createTestPatient(client, person.id);
    const providerPerson = await createTestPerson(client);
    const provider = await createTestProvider(client, providerPerson.id);
    const consultation = await createTestConsultation(
      client,
      patient.id,
      provider.id,
    );

    await client.post(`/emr/consultations/${consultation.id}/finalize`);

    try {
      await client.patch(`/emr/consultations/${consultation.id}`, {
        assessment: "New assessment",
      });
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(422);
    }
  });

  test("list consultations", async () => {
    const client = await createAuthenticatedClient();
    const result = await client.get("/emr/consultations");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("list EMR patients", async () => {
    const client = await createAuthenticatedClient();
    const result = await client.get("/emr/patients");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });
});
