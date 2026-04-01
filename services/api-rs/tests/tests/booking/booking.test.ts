import { describe, test, expect } from "bun:test";
import { ApiError, createClient } from "../../src/client";
import { createAuthenticatedClient } from "../../src/auth";
import { createTestPerson, createTestBookingEvent } from "../../src/fixtures";

describe("Booking", () => {
  test("create booking event", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const event = await createTestBookingEvent(client, person.id);

    expect(event.id).toBeDefined();
    expect(event.title).toBeDefined();
    expect(event.status || event.status).toBe("active");
  });

  test("list booking events (public / optional auth)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    await createTestBookingEvent(client, person.id);

    // Should work even with a fresh unauthenticated client
    const anonClient = createClient();
    const result = await anonClient.get("/booking/events");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("get booking event by ID", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const event = await createTestBookingEvent(client, person.id);

    const fetched = await client.get(`/booking/events/${event.id}`);
    expect(fetched.id).toBe(event.id);
  });

  test("update booking event (owner)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const event = await createTestBookingEvent(client, person.id);

    const updated = await client.patch(`/booking/events/${event.id}`, {
      title: "Updated Title",
    });
    expect(updated.title).toBe("Updated Title");
  });

  test("delete booking event", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const event = await createTestBookingEvent(client, person.id);

    await client.delete(`/booking/events/${event.id}`);

    try {
      await client.get(`/booking/events/${event.id}`);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });
});
