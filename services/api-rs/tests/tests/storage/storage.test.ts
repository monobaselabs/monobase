import { describe, test, expect } from "bun:test";
import { ApiError } from "../../src/client";
import { createAuthenticatedClient } from "../../src/auth";

describe("Storage", () => {
  test("upload file (get upload URL)", async () => {
    const client = await createAuthenticatedClient();
    const result = await client.post("/storage/files/upload", {
      filename: "test-doc.pdf",
      mimeType: "application/pdf",
      size: 1024,
    });

    expect(result.file).toBeDefined();
    expect(result.file.id).toBeDefined();
    expect(result.file.status).toBe("uploading");
    expect(result.uploadUrl).toBeDefined();
  });

  test("complete upload", async () => {
    const client = await createAuthenticatedClient();
    const upload = await client.post("/storage/files/upload", {
      filename: "test.txt",
      mimeType: "text/plain",
      size: 100,
    });

    const completed = await client.post(
      `/storage/files/${upload.file.id}/complete`,
    );
    expect(completed.status).toBe("available");
  });

  test("get file download URL", async () => {
    const client = await createAuthenticatedClient();
    const upload = await client.post("/storage/files/upload", {
      filename: "download-test.txt",
      mimeType: "text/plain",
      size: 100,
    });
    await client.post(`/storage/files/${upload.file.id}/complete`);

    const download = await client.get(
      `/storage/files/${upload.file.id}/download`,
    );
    expect(download.downloadUrl || download.url).toBeDefined();
  });

  test("delete file (owner only)", async () => {
    const client = await createAuthenticatedClient();
    const upload = await client.post("/storage/files/upload", {
      filename: "to-delete.txt",
      mimeType: "text/plain",
      size: 50,
    });

    await client.delete(`/storage/files/${upload.file.id}`);

    try {
      await client.get(`/storage/files/${upload.file.id}`);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });

  test("list files", async () => {
    const client = await createAuthenticatedClient();
    await client.post("/storage/files/upload", {
      filename: "list-test.txt",
      mimeType: "text/plain",
      size: 100,
    });

    const result = await client.get("/storage/files");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });
});
