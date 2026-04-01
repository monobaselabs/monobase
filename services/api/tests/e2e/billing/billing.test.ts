import { describe, test, expect } from "bun:test";
import { ApiError, createClient } from "../helpers/client";
import { createAuthenticatedClient } from "../helpers/auth";
import { createTestPerson, createTestInvoice } from "../helpers/fixtures";

describe("Billing", () => {
  test("create invoice with line items", async () => {
    const client = await createAuthenticatedClient();
    const customer = await createTestPerson(client);
    const merchant = await createTestPerson(client);
    const invoice = await createTestInvoice(client, customer.id, merchant.id);

    expect(invoice.id).toBeDefined();
    expect(invoice.invoice_number || invoice.invoiceNumber).toBeDefined();
    expect(invoice.status).toBe("draft");
    expect(invoice.total).toBe(5000);
  });

  test("get invoice by ID", async () => {
    const client = await createAuthenticatedClient();
    const customer = await createTestPerson(client);
    const merchant = await createTestPerson(client);
    const invoice = await createTestInvoice(client, customer.id, merchant.id);

    const fetched = await client.get(`/billing/invoices/${invoice.id}`);
    expect(fetched.id).toBe(invoice.id);
  });

  test("finalize invoice (draft → open)", async () => {
    const client = await createAuthenticatedClient();
    const customer = await createTestPerson(client);
    const merchant = await createTestPerson(client);
    const invoice = await createTestInvoice(client, customer.id, merchant.id);

    const finalized = await client.post(
      `/billing/invoices/${invoice.id}/finalize`,
    );
    expect(finalized.status).toBe("open");
  });

  test("void invoice", async () => {
    const client = await createAuthenticatedClient();
    const customer = await createTestPerson(client);
    const merchant = await createTestPerson(client);
    const invoice = await createTestInvoice(client, customer.id, merchant.id);

    const voided = await client.post(`/billing/invoices/${invoice.id}/void`);
    expect(voided.status).toBe("void");
  });

  test("create merchant account", async () => {
    const client = await createAuthenticatedClient();
    const person = await createTestPerson(client);
    const merchant = await client.post("/billing/merchant-accounts", {
      personId: person.id,
      metadata: { businessName: "Test Business" },
    });

    expect(merchant.id).toBeDefined();
    expect(merchant.active).toBe(true);
  });

  test("stripe webhook endpoint returns 200", async () => {
    const client = createClient();
    // Stripe webhooks are public (no auth)
    const res = await client.postRaw("/billing/webhooks/stripe", {
      type: "payment_intent.succeeded",
      data: { object: { id: "pi_test" } },
    });
    expect(res.status).toBe(200);
  });
});
