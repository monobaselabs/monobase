/**
 * Test data generators — create resources via API calls.
 * All functions are implementation-agnostic (pure HTTP).
 */

import { faker } from "@faker-js/faker";
import type { ApiClient } from "./client";

export function personData(overrides?: Record<string, any>) {
  return {
    firstName: faker.person.firstName(),
    lastName: faker.person.lastName(),
    dateOfBirth: "1990-01-15",
    gender: "male",
    timezone: "America/New_York",
    ...overrides,
  };
}

export async function createTestPerson(
  client: ApiClient,
  overrides?: Record<string, any>,
) {
  return client.post("/persons", personData(overrides));
}

export async function createTestPatient(
  client: ApiClient,
  personId: string,
  overrides?: Record<string, any>,
) {
  return client.post("/patients", { personId, ...overrides });
}

export async function createTestProvider(
  client: ApiClient,
  personId: string,
  providerType = "physician",
  overrides?: Record<string, any>,
) {
  return client.post("/providers", {
    personId,
    providerType,
    yearsOfExperience: 5,
    biography: "Test provider",
    ...overrides,
  });
}

export async function createTestBookingEvent(
  client: ApiClient,
  ownerId: string,
  overrides?: Record<string, any>,
) {
  return client.post("/booking/events", {
    ownerId,
    title: `Event ${faker.string.nanoid(6)}`,
    timezone: "America/New_York",
    locationTypes: ["video"],
    maxBookingDays: 30,
    minBookingMinutes: 60,
    status: "active",
    dailyConfigs: {},
    ...overrides,
  });
}

export async function createTestInvoice(
  client: ApiClient,
  customerId: string,
  merchantId: string,
  overrides?: Record<string, any>,
) {
  return client.post("/billing/invoices", {
    customerId,
    merchantId,
    subtotal: 5000,
    total: 5000,
    currency: "USD",
    lineItems: [
      {
        description: "Test service",
        quantity: 1,
        unitPrice: 5000,
        amount: 5000,
      },
    ],
    ...overrides,
  });
}

export async function createTestChatRoom(
  client: ApiClient,
  participants: string[],
  overrides?: Record<string, any>,
) {
  return client.post("/comms/chat-rooms", {
    participants,
    admins: [participants[0]],
    ...overrides,
  });
}

export async function createTestReview(
  client: ApiClient,
  overrides?: Record<string, any>,
) {
  return client.post("/reviews", {
    contextId: faker.string.uuid(),
    reviewerId: client.currentUser?.id || faker.string.uuid(),
    reviewType: "consultation",
    npsScore: 8,
    comment: "Good experience",
    ...overrides,
  });
}

export async function createTestEmailTemplate(
  client: ApiClient,
  overrides?: Record<string, any>,
) {
  return client.post("/email/templates", {
    name: `template-${faker.string.nanoid(6)}`,
    subject: "Test subject",
    bodyHtml: "<p>Hello {{name}}</p>",
    variables: [{ name: "name", type: "string" }],
    status: "draft",
    ...overrides,
  });
}

export async function createTestConsultation(
  client: ApiClient,
  patientId: string,
  providerId: string,
  overrides?: Record<string, any>,
) {
  return client.post("/emr/consultations", {
    patientId,
    providerId,
    chiefComplaint: "Test complaint",
    status: "draft",
    ...overrides,
  });
}
