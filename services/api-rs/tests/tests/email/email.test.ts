import { describe, test, expect } from "bun:test";
import { ApiError } from "../../src/client";
import { createAuthenticatedClient, createAdminClient } from "../../src/auth";
import { createTestEmailTemplate } from "../../src/fixtures";

describe("Email", () => {
  // Note: email endpoints require admin role.
  // These tests may fail if the server doesn't have AUTH_ADMIN_EMAILS configured.
  // Run with: AUTH_ADMIN_EMAILS=admin-test@test.local

  test("create email template (admin)", async () => {
    const client = await createAdminClient();
    const template = await createTestEmailTemplate(client);

    expect(template.id).toBeDefined();
    expect(template.name).toBeDefined();
    expect(template.status).toBe("draft");
  });

  test("list email templates (admin)", async () => {
    const client = await createAdminClient();
    await createTestEmailTemplate(client);

    const result = await client.get("/email/templates");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("get email template by ID (admin)", async () => {
    const client = await createAdminClient();
    const template = await createTestEmailTemplate(client);

    const fetched = await client.get(`/email/templates/${template.id}`);
    expect(fetched.id).toBe(template.id);
  });

  test("update email template (admin)", async () => {
    const client = await createAdminClient();
    const template = await createTestEmailTemplate(client);

    const updated = await client.patch(`/email/templates/${template.id}`, {
      subject: "Updated subject",
      status: "active",
    });
    expect(updated.subject).toBe("Updated subject");
    expect(updated.status).toBe("active");
  });

  test("test email template preview (admin)", async () => {
    const client = await createAdminClient();
    const template = await createTestEmailTemplate(client, {
      bodyHtml: "<p>Hello {{name}}</p>",
      variables: [{ name: "name", type: "string" }],
    });

    const preview = await client.post(
      `/email/templates/${template.id}/test`,
      {
        variables: { name: "World" },
      },
    );
    expect(preview).toBeDefined();
  });

  test("non-admin cannot access email templates", async () => {
    const client = await createAuthenticatedClient();
    try {
      await client.get("/email/templates");
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(403);
    }
  });
});
