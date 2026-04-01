import { describe, test, expect } from "bun:test";
import { createAuthenticatedClient } from "../helpers/auth";
import { createTestPerson, createTestChatRoom } from "../helpers/fixtures";

describe("Communications", () => {
  test("create chat room", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const room = await createTestChatRoom(client, [person.id]);

    expect(room.id).toBeDefined();
    expect(room.status).toBe("active");
    expect(room.message_count ?? room.messageCount).toBe(0);
  });

  test("list chat rooms (participant only)", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    await createTestChatRoom(client, [person.id]);

    const result = await client.get("/comms/chat-rooms");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("get chat room by ID", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const room = await createTestChatRoom(client, [person.id]);

    const fetched = await client.get(`/comms/chat-rooms/${room.id}`);
    expect(fetched.id).toBe(room.id);
  });

  test("send message to chat room", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const room = await createTestChatRoom(client, [person.id]);

    const message = await client.post(
      `/comms/chat-rooms/${room.id}/messages`,
      {
        messageType: "text",
        message: "Hello, world!",
      },
    );

    expect(message.id).toBeDefined();
    expect(message.message).toBe("Hello, world!");
  });

  test("get chat messages", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const room = await createTestChatRoom(client, [person.id]);

    await client.post(`/comms/chat-rooms/${room.id}/messages`, {
      messageType: "text",
      message: "Test message",
    });

    const result = await client.get(`/comms/chat-rooms/${room.id}/messages`);
    expect(result.data).toBeDefined();
    expect(result.data.length).toBeGreaterThanOrEqual(1);
  });

  test("get ICE servers", async () => {
    const client = await createAuthenticatedClient();
    const result = await client.get("/comms/ice-servers");

    expect(result.iceServers || result.ice_servers).toBeDefined();
    expect(Array.isArray(result.iceServers || result.ice_servers)).toBe(true);
  });
});
