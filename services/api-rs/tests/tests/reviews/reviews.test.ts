import { describe, test, expect } from "bun:test";
import { ApiError } from "../../src/client";
import { createAuthenticatedClient } from "../../src/auth";
import { createTestPerson, createTestReview } from "../../src/fixtures";
import { faker } from "@faker-js/faker";

describe("Reviews", () => {
  test("create review with NPS score", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const review = await createTestReview(client, {
      reviewerId: person.id,
      npsScore: 9,
    });

    expect(review.id).toBeDefined();
    expect(review.nps_score ?? review.npsScore).toBe(9);
  });

  test("list reviews", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    await createTestReview(client, { reviewerId: person.id });

    const result = await client.get("/reviews");
    expect(result.data).toBeDefined();
    expect(Array.isArray(result.data)).toBe(true);
  });

  test("get review by ID", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const review = await createTestReview(client, { reviewerId: person.id });

    const fetched = await client.get(`/reviews/${review.id}`);
    expect(fetched.id).toBe(review.id);
  });

  test("delete review", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const review = await createTestReview(client, { reviewerId: person.id });

    await client.delete(`/reviews/${review.id}`);

    try {
      await client.get(`/reviews/${review.id}`);
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBe(404);
    }
  });

  test("invalid NPS score (>10) returns 400", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    try {
      await createTestReview(client, {
        reviewerId: person.id,
        npsScore: 15,
      });
      expect(true).toBe(false);
    } catch (e) {
      expect(e).toBeInstanceOf(ApiError);
      expect((e as ApiError).status).toBeLessThanOrEqual(422);
      expect((e as ApiError).status).toBeGreaterThanOrEqual(400);
    }
  });
});
