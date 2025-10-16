import { z } from 'zod';
import ISO6391 from 'iso-639-1';
// Import browser version to bypass entry-node.js dynamic requires
import countries from 'i18n-iso-countries/index';
import enLocale from 'i18n-iso-countries/langs/en.json';
import { getTimeZones } from '@vvo/tzdb';
import { isValidPhoneNumber } from 'libphonenumber-js';

// Register English locale for country code validation
countries.registerLocale(enLocale);

// Generated Zod validators from OpenAPI spec

// Healthcare validation helpers
const validateNPI = (npi: string): boolean => {
  // NPI validation algorithm (Luhn algorithm)
  const digits = npi.split("").map(Number);
  let sum = 0;
  for (let i = 0; i < 9; i++) {
    let digit = digits[i];
    if (digit === undefined) return false; // Type guard for undefined
    if (i % 2 === 0) {
      digit *= 2;
      if (digit > 9) digit -= 9;
    }
    sum += digit;
  }
  const lastDigit = digits[9];
  if (lastDigit === undefined) return false; // Type guard for undefined
  return (10 - (sum % 10)) % 10 === lastDigit;
};

const containsPHI = (value: string): boolean => {
  // Basic PHI detection patterns
  const phiPatterns = [
    /\b\d{3}-\d{2}-\d{4}\b/, // SSN
    /\b\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b/, // Credit card
    /\b[A-Z]{2}\d{6}[A-Z]\b/, // Medical record patterns
  ];
  return phiPatterns.some(pattern => pattern.test(value));
};

// International data validation helpers
const validateLanguageCode = (code: string): boolean => {
  return ISO6391.validate(code);
};

const validateCountryCode = (code: string): boolean => {
  return countries.isValid(code);
};

const validatePhoneNumber = (phone: string): boolean => {
  try {
    // libphonenumber-js validates E.164 format and country-specific rules
    return isValidPhoneNumber(phone);
  } catch {
    return false;
  }
};

const timezoneNames = getTimeZones().map(tz => tz.name);
const validateTimezone = (tz: string): boolean => {
  return timezoneNames.includes(tz);
};

export const AddressSchema = z.object({
  street1: z.string().min(1).max(100),
  street2: z.string().max(100).optional(),
  city: z.string().min(1).max(50),
  state: z.string().min(1).max(50),
  postalCode: z.string().min(1).max(20),
  country: z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" }),
  coordinates: z.object({
  latitude: z.number().gte(-90).lte(90),
  longitude: z.number().gte(-180).lte(180),
  accuracy: z.number().gte(0).optional()
}).optional()
});

export const AddressUpdateSchema = z.object({
  street1: z.string().min(1).max(100).optional(),
  street2: z.union([z.string().max(100), z.null()]).optional(),
  city: z.string().min(1).max(50).optional(),
  state: z.string().min(1).max(50).optional(),
  postalCode: z.string().min(1).max(20).optional(),
  country: z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" }).optional(),
  coordinates: z.union([z.object({
  latitude: z.number().gte(-90).lte(90).optional(),
  longitude: z.number().gte(-180).lte(180).optional(),
  accuracy: z.number().gte(0).optional()
}), z.null()]).optional()
});

export const AuditActionSchema = z.enum(["create", "read", "update", "delete", "login", "logout"]);

export const AuditCategorySchema = z.enum(["regulatory", "security", "privacy", "administrative", "domain", "financial"]);

export const AuditEventTypeSchema = z.enum(["authentication", "data-access", "data-modification", "system-config", "security", "compliance"]);

export const AuditLogEntrySchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  eventType: z.enum(["authentication", "data-access", "data-modification", "system-config", "security", "compliance"]),
  category: z.enum(["regulatory", "security", "privacy", "administrative", "domain", "financial"]),
  user: z.string().uuid().optional(),
  userType: z.enum(["client", "service_provider", "admin", "system"]).optional(),
  resourceType: z.string(),
  resource: z.string(),
  action: z.enum(["create", "read", "update", "delete", "login", "logout"]),
  outcome: z.enum(["success", "failure", "partial", "denied"]),
  description: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  ipAddress: z.string().optional(),
  userAgent: z.string().optional(),
  session: z.string().optional(),
  request: z.string().optional(),
  integrityHash: z.string().optional(),
  retentionStatus: z.enum(["active", "archived", "pending-purge"]),
  archivedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  archivedBy: z.string().uuid().optional(),
  purgeAfter: z.string().datetime().transform((str) => new Date(str)).optional()
});

export const AuditOutcomeSchema = z.enum(["success", "failure", "partial", "denied"]);

export const AuditRetentionStatusSchema = z.enum(["active", "archived", "pending-purge"]);

export const AuthenticationErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  scheme: z.enum(["bearer", "api-key", "oauth2"]).optional(),
  supportedSchemes: z.array(z.string()).optional()
});

export const AuthorizationErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  requiredPermission: z.string().optional(),
  userPermissions: z.array(z.string()).optional(),
  resource: z.string().optional()
});

export const BaseEntitySchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional()
});

export const BillingConfigSchema = z.object({
  price: z.number().int().gte(0),
  currency: z.string(),
  cancellationThresholdMinutes: z.number().int().gte(0).lte(10080)
});

export const BillingConfigUpdateSchema = z.object({
  price: z.number().int().gte(0).optional(),
  currency: z.string().optional(),
  cancellationThresholdMinutes: z.number().int().gte(0).lte(10080).optional()
});

export const LanguageCodeSchema = z.string().regex(/^[a-z]{2}$/).refine(val => validateLanguageCode(val), { message: "Invalid ISO 639-1 language code" });

export const PersonSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  firstName: z.string().min(1).max(50),
  lastName: z.string().min(1).max(50).optional(),
  middleName: z.string().max(50).optional(),
  dateOfBirth: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" }).optional(),
  gender: z.enum(["male", "female", "non-binary", "other", "prefer-not-to-say"]).optional(),
  primaryAddress: z.object({
  street1: z.string().min(1).max(100),
  street2: z.string().max(100).optional(),
  city: z.string().min(1).max(50),
  state: z.string().min(1).max(50),
  postalCode: z.string().min(1).max(20),
  country: z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" }),
  coordinates: z.object({
  latitude: z.number().gte(-90).lte(90),
  longitude: z.number().gte(-180).lte(180),
  accuracy: z.number().gte(0).optional()
}).optional()
}).optional(),
  contactInfo: z.object({
  email: z.string().email().optional(),
  phone: z.string().regex(/^\+[1-9]\d{1,14}$/).refine(val => validatePhoneNumber(val), { message: "Invalid phone number in E.164 format" }).optional()
}).optional(),
  avatar: z.object({
  file: z.string().uuid().optional(),
  url: z.string().url()
}).optional(),
  languagesSpoken: z.array(LanguageCodeSchema).optional(),
  timezone: z.string().regex(/^[A-Za-z_]+\/[A-Za-z_]+$/).refine(val => validateTimezone(val), { message: "Invalid IANA timezone identifier" }).optional()
});

export const LocationTypeSchema = z.enum(["video", "phone", "in-person"]);

export const FormFieldOptionSchema = z.object({
  label: z.string(),
  value: z.string()
});

export const FormFieldConfigSchema = z.object({
  name: z.string(),
  type: z.enum(["text", "textarea", "email", "phone", "number", "date", "datetime", "url", "select", "multiselect", "checkbox", "display"]),
  label: z.string(),
  required: z.boolean().optional(),
  options: z.array(FormFieldOptionSchema).optional(),
  validation: z.object({
  minLength: z.number().int().optional(),
  maxLength: z.number().int().optional(),
  min: z.union([z.number(), z.string()]).optional(),
  max: z.union([z.number(), z.string()]).optional(),
  pattern: z.string().optional()
}).optional(),
  placeholder: z.string().optional(),
  helpText: z.string().optional()
});

export const TimeBlockSchema = z.object({
  startTime: z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/),
  endTime: z.string().regex(/^([01]\d|2[0-3]):[0-5]\d$/),
  slotDuration: z.number().int().gte(15).lte(480).optional(),
  bufferTime: z.number().int().gte(0).lte(120).optional()
});

export const DailyConfigSchema = z.object({
  enabled: z.boolean(),
  timeBlocks: z.array(TimeBlockSchema)
});

export const BookingEventSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  owner: z.union([z.string(), PersonSchema]),
  context: z.string().optional(),
  title: z.string(),
  description: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  tags: z.array(z.string()).optional(),
  timezone: z.string(),
  locationTypes: z.array(LocationTypeSchema),
  maxBookingDays: z.number().int().gte(0).lte(365),
  minBookingMinutes: z.number().int().gte(0).lte(4320),
  formConfig: z.object({
  fields: z.array(FormFieldConfigSchema).optional()
}).optional(),
  billingConfig: z.object({
  price: z.number().int().gte(0),
  currency: z.string(),
  cancellationThresholdMinutes: z.number().int().gte(0).lte(10080)
}).optional(),
  status: z.enum(["draft", "active", "paused", "archived"]),
  effectiveFrom: z.string().datetime().transform((str) => new Date(str)),
  effectiveTo: z.string().datetime().transform((str) => new Date(str)).optional(),
  dailyConfigs: z.record(z.string(), z.unknown())
});

export const TimeSlotSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  owner: z.string().uuid(),
  event: z.union([z.string(), BookingEventSchema]),
  context: z.string().optional(),
  startTime: z.string().datetime().transform((str) => new Date(str)),
  endTime: z.string().datetime().transform((str) => new Date(str)),
  locationTypes: z.array(LocationTypeSchema),
  status: z.enum(["available", "booked", "blocked"]),
  billingConfig: z.object({
  price: z.number().int().gte(0),
  currency: z.string(),
  cancellationThresholdMinutes: z.number().int().gte(0).lte(10080)
}).optional(),
  booking: z.string().uuid().optional()
});

export const BookingSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  client: z.union([z.string(), PersonSchema]),
  provider: z.union([z.string(), PersonSchema]),
  slot: z.union([z.string(), TimeSlotSchema]),
  locationType: z.enum(["video", "phone", "in-person"]),
  reason: z.string().max(500),
  status: z.enum(["pending", "confirmed", "rejected", "cancelled", "completed", "no_show_client", "no_show_provider"]),
  bookedAt: z.string().datetime().transform((str) => new Date(str)),
  confirmationTimestamp: z.string().datetime().transform((str) => new Date(str)).optional(),
  scheduledAt: z.string().datetime().transform((str) => new Date(str)),
  durationMinutes: z.number().int().gte(15).lte(480),
  cancellationReason: z.string().optional(),
  cancelledBy: z.string().optional(),
  cancelledAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  noShowMarkedBy: z.string().optional(),
  noShowMarkedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  formResponses: z.object({
  data: z.record(z.string(), z.unknown()),
  metadata: z.object({
  submittedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  completionTimeSeconds: z.number().int().optional(),
  ipAddress: z.string().optional()
}).optional()
}).optional(),
  invoice: z.string().uuid().optional()
});

export const BookingActionRequestSchema = z.object({
  reason: z.string().max(500)
});

export const BookingCreateRequestSchema = z.object({
  slot: z.string().uuid(),
  locationType: z.enum(["video", "phone", "in-person"]).optional(),
  reason: z.string().max(500).optional(),
  formResponses: z.object({
  data: z.record(z.string(), z.unknown())
}).optional()
});

export const BookingEventCreateRequestSchema = z.object({
  title: z.string(),
  description: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  tags: z.array(z.string()).optional(),
  context: z.string().optional(),
  timezone: z.string().optional(),
  locationTypes: z.array(LocationTypeSchema).optional(),
  maxBookingDays: z.number().int().gte(0).lte(365).optional(),
  minBookingMinutes: z.number().int().gte(0).lte(4320).optional(),
  formConfig: z.object({
  fields: z.array(FormFieldConfigSchema).optional()
}).optional(),
  billingConfig: z.object({
  price: z.number().int().gte(0),
  currency: z.string(),
  cancellationThresholdMinutes: z.number().int().gte(0).lte(10080)
}).optional(),
  status: z.enum(["draft", "active", "paused", "archived"]).optional(),
  effectiveFrom: z.string().datetime().transform((str) => new Date(str)).optional(),
  effectiveTo: z.string().datetime().transform((str) => new Date(str)).optional(),
  dailyConfigs: z.record(z.string(), z.unknown())
});

export const BookingEventStatusSchema = z.enum(["draft", "active", "paused", "archived"]);

export const DailyConfigUpdateSchema = z.object({
  enabled: z.boolean().optional(),
  timeBlocks: z.array(TimeBlockSchema).optional()
});

export const BookingEventUpdateRequestSchema = z.object({
  title: z.string().optional(),
  description: z.union([z.string(), z.null()]).optional(),
  keywords: z.union([z.array(z.string()), z.null()]).optional(),
  tags: z.union([z.array(z.string()), z.null()]).optional(),
  timezone: z.string().optional(),
  locationTypes: z.array(LocationTypeSchema).optional(),
  maxBookingDays: z.number().int().optional(),
  minBookingMinutes: z.number().int().optional(),
  formConfig: z.union([z.object({
  fields: z.array(FormFieldConfigSchema).optional()
}), z.null()]).optional(),
  billingConfig: z.union([z.object({
  price: z.number().int().gte(0).optional(),
  currency: z.string().optional(),
  cancellationThresholdMinutes: z.number().int().gte(0).lte(10080).optional()
}), z.null()]).optional(),
  status: z.enum(["draft", "active", "paused", "archived"]).optional(),
  effectiveTo: z.union([z.string().datetime().transform((str) => new Date(str)), z.null()]).optional(),
  dailyConfigs: z.record(z.string(), z.unknown()).optional()
});

export const BookingStatusSchema = z.enum(["pending", "confirmed", "rejected", "cancelled", "completed", "no_show_client", "no_show_provider"]);

export const CallParticipantSchema = z.object({
  user: z.string().uuid(),
  displayName: z.string().max(100),
  joinedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  leftAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  audioEnabled: z.boolean(),
  videoEnabled: z.boolean()
});

export const CancelEmailRequestSchema = z.object({
  reason: z.string().max(500)
});

export const CaptureMethodSchema = z.enum(["automatic", "manual"]);

export const ChatMessageSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  chatRoom: z.string().uuid(),
  sender: z.string().uuid(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  messageType: z.enum(["text", "system", "video_call"]),
  message: z.string().max(5000).optional(),
  videoCallData: z.object({
  status: z.enum(["starting", "active", "ended", "cancelled"]),
  roomUrl: z.string().optional(),
  token: z.string().optional(),
  startedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  startedBy: z.string().uuid().optional(),
  endedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  endedBy: z.string().uuid().optional(),
  durationMinutes: z.number().int().optional(),
  participants: z.array(CallParticipantSchema)
}).optional()
});

export const UUIDSchema = z.string().uuid();

export const ChatRoomSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  participants: z.array(UUIDSchema),
  admins: z.array(UUIDSchema),
  context: z.string().uuid().optional(),
  status: z.enum(["active", "archived"]),
  lastMessageAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  messageCount: z.number().int(),
  activeVideoCallMessage: z.string().uuid().optional()
});

export const ChatRoomStatusSchema = z.enum(["active", "archived"]);

export const ConflictErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  conflictingResource: z.string().optional(),
  reason: z.enum(["duplicate", "version-mismatch", "state-conflict", "dependency"]).optional(),
  currentState: z.record(z.string(), z.unknown()).optional(),
  resolution: z.array(z.string()).optional()
});

export const ContactInfoSchema = z.object({
  email: z.string().email().optional(),
  phone: z.string().regex(/^\+[1-9]\d{1,14}$/).refine(val => validatePhoneNumber(val), { message: "Invalid phone number in E.164 format" }).optional()
});

export const CountryCodeSchema = z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" });

export const CreateChatRoomRequestSchema = z.object({
  participants: z.array(UUIDSchema),
  admins: z.array(UUIDSchema).optional(),
  context: z.string().uuid().optional(),
  upsert: z.boolean().optional()
});

export const CreateLineItemRequestSchema = z.object({
  description: z.string().max(500),
  quantity: z.number().int().gte(1).optional(),
  unitPrice: z.number().int().gte(0),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const CreateInvoiceRequestSchema = z.object({
  customer: z.string().uuid(),
  merchant: z.string().uuid(),
  context: z.string().max(255).optional(),
  currency: z.string().regex(/^[A-Z]{3}$/).optional(),
  paymentCaptureMethod: z.enum(["automatic", "manual"]).optional(),
  paymentDueAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  voidThresholdMinutes: z.number().int().optional(),
  lineItems: z.array(CreateLineItemRequestSchema),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const CreateMerchantAccountRequestSchema = z.object({
  person: z.string().uuid().optional(),
  refreshUrl: z.string().url(),
  returnUrl: z.string().url(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const CreateReviewRequestSchema = z.object({
  context: z.string().uuid(),
  reviewType: z.string().max(50),
  reviewedEntity: z.string().uuid().optional(),
  npsScore: z.number().int().gte(0).lte(10),
  comment: z.string().max(1000).optional()
});

export const TemplateVariableSchema = z.object({
  id: z.string().max(100),
  type: z.enum(["string", "number", "boolean", "date", "datetime", "url", "email", "array"]),
  label: z.string().max(255).optional(),
  required: z.boolean().optional(),
  defaultValue: z.string().optional(),
  minLength: z.number().int().gte(0).optional(),
  maxLength: z.number().int().lte(10000).optional(),
  min: z.number().optional(),
  max: z.number().optional(),
  pattern: z.string().max(500).optional(),
  options: z.array(z.string()).optional()
});

export const CreateTemplateRequestSchema = z.object({
  tags: z.array(z.string()).optional(),
  name: z.string().max(255),
  description: z.string().max(500).optional(),
  subject: z.string().max(500),
  bodyHtml: z.string(),
  bodyText: z.string().optional(),
  variables: z.array(TemplateVariableSchema).optional(),
  fromName: z.string().optional(),
  fromEmail: z.string().email().optional(),
  replyToEmail: z.string().email().optional(),
  replyToName: z.string().optional(),
  status: z.enum(["draft", "active", "archived"]).optional()
});

export const CurrencyAmountSchema = z.number().int().gte(0);

export const CurrencyCodeSchema = z.string().regex(/^[A-Z]{3}$/);

export const DashboardResponseSchema = z.object({
  dashboardUrl: z.string().url(),
  expiresAt: z.string().datetime().transform((str) => new Date(str))
});

export const EmailSchema = z.string().email();

export const EmailProviderSchema = z.enum(["smtp", "postmark"]);

export const EmailQueueItemSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  template: z.string().uuid().optional(),
  templateTags: z.array(z.string()).optional(),
  recipientEmail: z.string().email(),
  recipientName: z.string().max(255).optional(),
  variables: z.record(z.string(), z.unknown()).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
  status: z.enum(["pending", "processing", "sent", "failed", "cancelled"]),
  priority: z.number().int().gte(1).lte(10),
  scheduledAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  attempts: z.number().int().gte(0),
  lastAttemptAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  nextRetryAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  lastError: z.string().optional(),
  sentAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  provider: z.enum(["smtp", "postmark"]).optional(),
  providerMessageId: z.string().max(255).optional(),
  cancelledAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  cancelledBy: z.string().uuid().optional(),
  cancellationReason: z.string().max(500).optional()
});

export const EmailQueueStatusSchema = z.enum(["pending", "processing", "sent", "failed", "cancelled"]);

export const EmailTemplateSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int().gte(1),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  tags: z.array(z.string()).optional(),
  name: z.string().max(255),
  description: z.string().max(500).optional(),
  subject: z.string().max(500),
  bodyHtml: z.string(),
  bodyText: z.string().optional(),
  variables: z.array(TemplateVariableSchema).optional(),
  fromName: z.string().max(255).optional(),
  fromEmail: z.string().email().optional(),
  replyToEmail: z.string().email().optional(),
  replyToName: z.string().max(255).optional(),
  status: z.enum(["draft", "active", "archived"])
});

export const ErrorDetailSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional()
});

export const FieldErrorSchema = z.object({
  field: z.string(),
  value: z.unknown().optional(),
  code: z.string(),
  message: z.string(),
  context: z.record(z.string(), z.unknown()).optional()
});

export const FileDownloadResponseSchema = z.object({
  downloadUrl: z.string().url(),
  expiresAt: z.string().datetime().transform((str) => new Date(str)),
  file: z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  filename: z.string().max(255),
  mimeType: z.string().max(100),
  size: z.number().int().gte(0),
  status: z.enum(["uploading", "processing", "available", "failed"]),
  owner: z.string().uuid(),
  uploadedAt: z.string().datetime().transform((str) => new Date(str))
})
});

export const FileStatusSchema = z.enum(["uploading", "processing", "available", "failed"]);

export const FileUploadRequestSchema = z.object({
  filename: z.string().max(255),
  size: z.number().int().gte(1),
  mimeType: z.string().max(100)
});

export const FileUploadResponseSchema = z.object({
  file: z.string().uuid(),
  uploadUrl: z.string().url(),
  uploadMethod: z.enum(["PUT"]),
  expiresAt: z.string().datetime().transform((str) => new Date(str))
});

export const FormConfigSchema = z.object({
  fields: z.array(FormFieldConfigSchema).optional()
});

export const FormFieldTypeSchema = z.enum(["text", "textarea", "email", "phone", "number", "date", "datetime", "url", "select", "multiselect", "checkbox", "display"]);

export const FormFieldValidationSchema = z.object({
  minLength: z.number().int().optional(),
  maxLength: z.number().int().optional(),
  min: z.union([z.number(), z.string()]).optional(),
  max: z.union([z.number(), z.string()]).optional(),
  pattern: z.string().optional()
});

export const FormResponseDataSchema = z.object({
  data: z.record(z.string(), z.unknown())
});

export const FormResponseMetaDataSchema = z.object({
  submittedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  completionTimeSeconds: z.number().int().optional(),
  ipAddress: z.string().optional()
});

export const FormResponsesSchema = z.object({
  data: z.record(z.string(), z.unknown()),
  metadata: z.object({
  submittedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  completionTimeSeconds: z.number().int().optional(),
  ipAddress: z.string().optional()
}).optional()
});

export const GenderSchema = z.enum(["male", "female", "non-binary", "other", "prefer-not-to-say"]);

export const GeoCoordinatesSchema = z.object({
  latitude: z.number().gte(-90).lte(90),
  longitude: z.number().gte(-180).lte(180),
  accuracy: z.number().gte(0).optional()
});

export const GeoCoordinatesUpdateSchema = z.object({
  latitude: z.number().gte(-90).lte(90).optional(),
  longitude: z.number().gte(-180).lte(180).optional(),
  accuracy: z.number().gte(0).optional()
});

export const IceServerSchema = z.object({
  urls: z.union([z.string(), z.array(z.string())]),
  username: z.string().optional(),
  credential: z.string().optional()
});

export const IceServersResponseSchema = z.object({
  iceServers: z.array(IceServerSchema)
});

export const InternalServerErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  trackingId: z.string().optional(),
  reported: z.boolean().optional()
});

export const MerchantAccountSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  person: z.union([z.string(), PersonSchema]),
  active: z.boolean(),
  metadata: z.record(z.string(), z.unknown())
});

export const InvoiceLineItemSchema = z.object({
  description: z.string().max(500),
  quantity: z.number().int().gte(1),
  unitPrice: z.number().int().gte(0),
  amount: z.number().int().gte(0),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const InvoiceSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  invoiceNumber: z.string().max(50),
  customer: z.union([z.string(), PersonSchema]),
  merchant: z.union([z.string(), PersonSchema]),
  merchantAccount: z.union([z.string(), MerchantAccountSchema]).optional(),
  context: z.string().max(255).optional(),
  status: z.enum(["draft", "open", "paid", "void", "uncollectible"]),
  subtotal: z.number().int().gte(0),
  tax: z.number().int().gte(0).optional(),
  total: z.number().int().gte(0),
  currency: z.string().regex(/^[A-Z]{3}$/),
  paymentCaptureMethod: z.enum(["automatic", "manual"]),
  issuedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  paymentDueAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  lineItems: z.array(InvoiceLineItemSchema),
  paymentStatus: z.enum(["pending", "requires_capture", "processing", "succeeded", "failed", "canceled"]).optional(),
  paidAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  paidBy: z.string().uuid().optional(),
  voidedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  voidedBy: z.string().uuid().optional(),
  voidThresholdMinutes: z.number().int().optional(),
  authorizedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  authorizedBy: z.string().uuid().optional(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const InvoiceStatusSchema = z.enum(["draft", "open", "paid", "void", "uncollectible"]);

export const JoinVideoCallRequestSchema = z.object({
  displayName: z.string().max(100),
  audioEnabled: z.boolean(),
  videoEnabled: z.boolean()
});

export const LeaveVideoCallResponseSchema = z.object({
  message: z.string(),
  callStillActive: z.boolean(),
  remainingParticipants: z.number().int()
});

export const MaybeStoredFileSchema = z.object({
  file: z.string().uuid().optional(),
  url: z.string().url()
});

export const MaybeStoredFileUpdateSchema = z.object({
  file: z.union([z.string().uuid(), z.null()]).optional(),
  url: z.string().url().optional()
});

export const MessageTypeSchema = z.enum(["text", "system", "video_call"]);

export const NotFoundErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  resourceType: z.string().optional(),
  resource: z.string().optional(),
  suggestions: z.array(z.string()).optional()
});

export const NotificationSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  recipient: z.string().uuid(),
  type: z.enum(["billing", "security", "system", "booking.created", "booking.confirmed", "booking.rejected", "booking.cancelled", "booking.no-show-client", "booking.no-show-provider", "comms.video-call-started", "comms.video-call-joined", "comms.video-call-left", "comms.video-call-ended", "comms.chat-message"]),
  channel: z.enum(["email", "push", "in-app"]),
  title: z.string().max(200),
  message: z.string().max(1000),
  scheduledAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  relatedEntityType: z.string().optional(),
  relatedEntity: z.string().uuid().optional(),
  status: z.enum(["queued", "sent", "delivered", "read", "failed", "expired", "unread"]),
  sentAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  deliveredAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  readAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
  consentValidated: z.boolean()
});

export const NotificationChannelSchema = z.enum(["email", "push", "in-app"]);

export const NotificationStatusSchema = z.enum(["queued", "sent", "delivered", "read", "failed", "expired", "unread"]);

export const NotificationTypeSchema = z.enum(["billing", "security", "system", "booking.created", "booking.confirmed", "booking.rejected", "booking.cancelled", "booking.no-show-client", "booking.no-show-provider", "comms.video-call-started", "comms.video-call-joined", "comms.video-call-left", "comms.video-call-ended", "comms.chat-message"]);

export const OffsetPaginationMetaSchema = z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
});

export const OnboardingRequestSchema = z.object({
  refreshUrl: z.string().url(),
  returnUrl: z.string().url()
});

export const OnboardingResponseSchema = z.object({
  onboardingUrl: z.string(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const PaymentRequestSchema = z.object({
  paymentMethod: z.string().max(255).optional(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const PaymentResponseSchema = z.object({
  checkoutUrl: z.string(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const PaymentStatusSchema = z.enum(["pending", "requires_capture", "processing", "succeeded", "failed", "canceled"]);

export const PersonCreateRequestSchema = z.object({
  firstName: z.string().min(1).max(50),
  lastName: z.string().min(1).max(50).optional(),
  middleName: z.string().max(50).optional(),
  dateOfBirth: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" }).optional(),
  gender: z.enum(["male", "female", "non-binary", "other", "prefer-not-to-say"]).optional(),
  primaryAddress: z.object({
  street1: z.string().min(1).max(100),
  street2: z.string().max(100).optional(),
  city: z.string().min(1).max(50),
  state: z.string().min(1).max(50),
  postalCode: z.string().min(1).max(20),
  country: z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" }),
  coordinates: z.object({
  latitude: z.number().gte(-90).lte(90),
  longitude: z.number().gte(-180).lte(180),
  accuracy: z.number().gte(0).optional()
}).optional()
}).optional(),
  contactInfo: z.object({
  email: z.string().email().optional(),
  phone: z.string().regex(/^\+[1-9]\d{1,14}$/).refine(val => validatePhoneNumber(val), { message: "Invalid phone number in E.164 format" }).optional()
}).optional(),
  avatar: z.object({
  file: z.string().uuid().optional(),
  url: z.string().url()
}).optional(),
  languagesSpoken: z.array(LanguageCodeSchema).optional(),
  timezone: z.string().regex(/^[A-Za-z_]+\/[A-Za-z_]+$/).refine(val => validateTimezone(val), { message: "Invalid IANA timezone identifier" }).optional()
});

export const PersonUpdateRequestSchema = z.object({
  firstName: z.string().min(1).max(50).optional(),
  lastName: z.union([z.string().min(1).max(50), z.null()]).optional(),
  middleName: z.union([z.string().max(50), z.null()]).optional(),
  dateOfBirth: z.union([z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" }), z.null()]).optional(),
  gender: z.union([z.enum(["male", "female", "non-binary", "other", "prefer-not-to-say"]), z.null()]).optional(),
  primaryAddress: z.union([z.object({
  street1: z.string().min(1).max(100).optional(),
  street2: z.union([z.string().max(100), z.null()]).optional(),
  city: z.string().min(1).max(50).optional(),
  state: z.string().min(1).max(50).optional(),
  postalCode: z.string().min(1).max(20).optional(),
  country: z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" }).optional(),
  coordinates: z.union([z.object({
  latitude: z.number().gte(-90).lte(90).optional(),
  longitude: z.number().gte(-180).lte(180).optional(),
  accuracy: z.number().gte(0).optional()
}), z.null()]).optional()
}), z.null()]).optional(),
  contactInfo: z.union([z.object({
  email: z.string().email().optional(),
  phone: z.string().regex(/^\+[1-9]\d{1,14}$/).refine(val => validatePhoneNumber(val), { message: "Invalid phone number in E.164 format" }).optional()
}), z.null()]).optional(),
  avatar: z.union([z.object({
  file: z.union([z.string().uuid(), z.null()]).optional(),
  url: z.string().url().optional()
}), z.null()]).optional(),
  languagesSpoken: z.union([z.array(LanguageCodeSchema), z.null()]).optional(),
  timezone: z.union([z.string().regex(/^[A-Za-z_]+\/[A-Za-z_]+$/).refine(val => validateTimezone(val), { message: "Invalid IANA timezone identifier" }), z.null()]).optional()
});

export const PhoneNumberSchema = z.string().regex(/^\+[1-9]\d{1,14}$/).refine(val => validatePhoneNumber(val), { message: "Invalid phone number in E.164 format" });

export const RateLimitErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  limitType: z.enum(["requests", "bandwidth", "concurrent"]),
  limit: z.number().int(),
  usage: z.number().int(),
  resetTime: z.number().int(),
  windowSize: z.number().int()
});

export const RecurrencePatternSchema = z.object({
  type: z.enum(["daily", "weekly", "monthly", "yearly"]),
  interval: z.number().int().gte(1).optional(),
  daysOfWeek: z.array(z.number().int()).optional(),
  dayOfMonth: z.number().int().gte(1).lte(31).optional(),
  monthOfYear: z.number().int().gte(1).lte(12).optional(),
  endDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" }).optional(),
  maxOccurrences: z.number().int().gte(1).optional()
});

export const RecurrenceTypeSchema = z.enum(["daily", "weekly", "monthly", "yearly"]);

export const RefundRequestSchema = z.object({
  amount: z.number().int().gte(0).optional(),
  reason: z.string().max(500).optional(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const RefundResponseSchema = z.object({
  refundedAmount: z.number().int().gte(0),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const ReviewSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  context: z.string().uuid(),
  reviewer: z.string().uuid(),
  reviewType: z.string().max(50),
  reviewedEntity: z.string().uuid().optional(),
  npsScore: z.number().int().gte(0).lte(10),
  comment: z.string().max(1000).optional()
});

export const ScheduleExceptionSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  event: z.string().uuid(),
  owner: z.string().uuid(),
  context: z.string().optional(),
  timezone: z.string(),
  startDatetime: z.string().datetime().transform((str) => new Date(str)),
  endDatetime: z.string().datetime().transform((str) => new Date(str)),
  reason: z.string().max(500),
  recurring: z.boolean(),
  recurrencePattern: z.object({
  type: z.enum(["daily", "weekly", "monthly", "yearly"]),
  interval: z.number().int().gte(1).optional(),
  daysOfWeek: z.array(z.number().int()).optional(),
  dayOfMonth: z.number().int().gte(1).lte(31).optional(),
  monthOfYear: z.number().int().gte(1).lte(12).optional(),
  endDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" }).optional(),
  maxOccurrences: z.number().int().gte(1).optional()
}).optional()
});

export const ScheduleExceptionCreateRequestSchema = z.object({
  timezone: z.string().optional(),
  startDatetime: z.string().datetime().transform((str) => new Date(str)),
  endDatetime: z.string().datetime().transform((str) => new Date(str)),
  reason: z.string().max(500),
  recurring: z.boolean().optional(),
  recurrencePattern: z.object({
  type: z.enum(["daily", "weekly", "monthly", "yearly"]),
  interval: z.number().int().gte(1).optional(),
  daysOfWeek: z.array(z.number().int()).optional(),
  dayOfMonth: z.number().int().gte(1).lte(31).optional(),
  monthOfYear: z.number().int().gte(1).lte(12).optional(),
  endDate: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" }).optional(),
  maxOccurrences: z.number().int().gte(1).optional()
}).optional()
});

export const SendTextMessageRequestSchema = z.object({
  messageType: z.enum(["text"]),
  message: z.string().max(5000)
});

export const SlotStatusSchema = z.enum(["available", "booked", "blocked"]);

export const StartVideoCallDataSchema = z.object({
  status: z.enum(["starting"]),
  participants: z.array(CallParticipantSchema)
});

export const StartVideoCallRequestSchema = z.object({
  messageType: z.enum(["video_call"]),
  videoCallData: z.object({
  status: z.enum(["starting"]),
  participants: z.array(CallParticipantSchema)
})
});

export const StoredFileSchema = z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  filename: z.string().max(255),
  mimeType: z.string().max(100),
  size: z.number().int().gte(0),
  status: z.enum(["uploading", "processing", "available", "failed"]),
  owner: z.string().uuid(),
  uploadedAt: z.string().datetime().transform((str) => new Date(str))
});

export const TemplateStatusSchema = z.enum(["draft", "active", "archived"]);

export const TestTemplateRequestSchema = z.object({
  recipientEmail: z.string().email(),
  recipientName: z.string().max(255).optional(),
  variables: z.record(z.string(), z.unknown()).optional()
});

export const TestTemplateResultSchema = z.object({
  queue: z.object({
  id: z.string().uuid(),
  version: z.number().int(),
  createdAt: z.string().datetime().transform((str) => new Date(str)),
  createdBy: z.string().uuid().optional(),
  updatedAt: z.string().datetime().transform((str) => new Date(str)),
  updatedBy: z.string().uuid().optional(),
  template: z.string().uuid().optional(),
  templateTags: z.array(z.string()).optional(),
  recipientEmail: z.string().email(),
  recipientName: z.string().max(255).optional(),
  variables: z.record(z.string(), z.unknown()).optional(),
  metadata: z.record(z.string(), z.unknown()).optional(),
  status: z.enum(["pending", "processing", "sent", "failed", "cancelled"]),
  priority: z.number().int().gte(1).lte(10),
  scheduledAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  attempts: z.number().int().gte(0),
  lastAttemptAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  nextRetryAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  lastError: z.string().optional(),
  sentAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  provider: z.enum(["smtp", "postmark"]).optional(),
  providerMessageId: z.string().max(255).optional(),
  cancelledAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  cancelledBy: z.string().uuid().optional(),
  cancellationReason: z.string().max(500).optional()
})
});

export const TimezoneIdSchema = z.string().regex(/^[A-Za-z_]+\/[A-Za-z_]+$/).refine(val => validateTimezone(val), { message: "Invalid IANA timezone identifier" });

export const UpdateInvoiceRequestSchema = z.object({
  paymentCaptureMethod: z.enum(["automatic", "manual"]).optional(),
  paymentDueAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  voidThresholdMinutes: z.number().int().optional(),
  lineItems: z.array(CreateLineItemRequestSchema).optional(),
  metadata: z.record(z.string(), z.unknown()).optional()
});

export const UpdateParticipantRequestSchema = z.object({
  audioEnabled: z.boolean().optional(),
  videoEnabled: z.boolean().optional()
});

export const UpdateTemplateRequestSchema = z.object({
  tags: z.array(z.string()).optional(),
  name: z.string().max(255).optional(),
  description: z.string().max(500).optional(),
  subject: z.string().max(500).optional(),
  bodyHtml: z.string().optional(),
  bodyText: z.string().optional(),
  variables: z.array(TemplateVariableSchema).optional(),
  fromName: z.string().optional(),
  fromEmail: z.string().email().optional(),
  replyToEmail: z.string().email().optional(),
  replyToName: z.string().optional(),
  status: z.enum(["draft", "active", "archived"]).optional()
});

export const UrlSchema = z.string().url();

export const ValidationErrorSchema = z.object({
  code: z.string(),
  message: z.string(),
  details: z.record(z.string(), z.unknown()).optional(),
  requestId: z.string(),
  timestamp: z.string().datetime().transform((str) => new Date(str)),
  path: z.string(),
  method: z.string(),
  statusCode: z.number().int(),
  helpUrl: z.string().url().optional(),
  fieldErrors: z.array(FieldErrorSchema).optional(),
  globalErrors: z.array(z.string()).optional()
});

export const VariableTypeSchema = z.enum(["string", "number", "boolean", "date", "datetime", "url", "email", "array"]);

export const VideoCallDataSchema = z.object({
  status: z.enum(["starting", "active", "ended", "cancelled"]),
  roomUrl: z.string().optional(),
  token: z.string().optional(),
  startedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  startedBy: z.string().uuid().optional(),
  endedAt: z.string().datetime().transform((str) => new Date(str)).optional(),
  endedBy: z.string().uuid().optional(),
  durationMinutes: z.number().int().optional(),
  participants: z.array(CallParticipantSchema)
});

export const VideoCallEndResponseSchema = z.object({
  message: z.string(),
  callDuration: z.number().int().optional()
});

export const VideoCallJoinResponseSchema = z.object({
  roomUrl: z.string(),
  token: z.string(),
  callStatus: z.enum(["starting", "active", "ended", "cancelled"]),
  participants: z.array(CallParticipantSchema)
});

export const VideoCallStatusSchema = z.enum(["starting", "active", "ended", "cancelled"]);

export const ListAuditLogsQuery = z.object({
  resourceType: z.string().optional(),
  resource: UUIDSchema.optional(),
  user: UUIDSchema.optional(),
  action: AuditActionSchema.optional(),
  startDate: z.string().datetime().transform((str) => new Date(str)).optional(),
  endDate: z.string().datetime().transform((str) => new Date(str)).optional(),
  orderBy: z.string().optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListAuditLogsQuery = z.infer<typeof ListAuditLogsQuery>;

export const ListAuditLogsResponse = z.object({
  data: z.array(AuditLogEntrySchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const CreateInvoiceBody = CreateInvoiceRequestSchema;
export type CreateInvoiceBody = z.infer<typeof CreateInvoiceBody>;

export const CreateInvoiceResponse = InvoiceSchema;

export const ListInvoicesQuery = z.object({
  customer: UUIDSchema.optional(),
  merchant: UUIDSchema.optional(),
  status: InvoiceStatusSchema.optional(),
  context: z.string().optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListInvoicesQuery = z.infer<typeof ListInvoicesQuery>;

export const ListInvoicesResponse = z.object({
  data: z.array(InvoiceSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetInvoiceParams = z.object({
  invoice: UUIDSchema,
});
export type GetInvoiceParams = z.infer<typeof GetInvoiceParams>;

export const GetInvoiceQuery = z.object({
  expand: z.string().optional(),
});
export type GetInvoiceQuery = z.infer<typeof GetInvoiceQuery>;

export const GetInvoiceResponse = InvoiceSchema;

export const UpdateInvoiceParams = z.object({
  invoice: UUIDSchema,
});
export type UpdateInvoiceParams = z.infer<typeof UpdateInvoiceParams>;

export const UpdateInvoiceBody = UpdateInvoiceRequestSchema;
export type UpdateInvoiceBody = z.infer<typeof UpdateInvoiceBody>;

export const UpdateInvoiceResponse = InvoiceSchema;

export const DeleteInvoiceParams = z.object({
  invoice: UUIDSchema,
});
export type DeleteInvoiceParams = z.infer<typeof DeleteInvoiceParams>;

export const DeleteInvoiceResponse = z.void();

export const CaptureInvoicePaymentParams = z.object({
  invoice: UUIDSchema,
});
export type CaptureInvoicePaymentParams = z.infer<typeof CaptureInvoicePaymentParams>;

export const CaptureInvoicePaymentResponse = InvoiceSchema;

export const FinalizeInvoiceParams = z.object({
  invoice: UUIDSchema,
});
export type FinalizeInvoiceParams = z.infer<typeof FinalizeInvoiceParams>;

export const FinalizeInvoiceResponse = InvoiceSchema;

export const MarkInvoiceUncollectibleParams = z.object({
  invoice: UUIDSchema,
});
export type MarkInvoiceUncollectibleParams = z.infer<typeof MarkInvoiceUncollectibleParams>;

export const MarkInvoiceUncollectibleResponse = InvoiceSchema;

export const PayInvoiceParams = z.object({
  invoice: UUIDSchema,
});
export type PayInvoiceParams = z.infer<typeof PayInvoiceParams>;

export const PayInvoiceBody = PaymentRequestSchema;
export type PayInvoiceBody = z.infer<typeof PayInvoiceBody>;

export const PayInvoiceResponse = PaymentResponseSchema;

export const RefundInvoicePaymentParams = z.object({
  invoice: UUIDSchema,
});
export type RefundInvoicePaymentParams = z.infer<typeof RefundInvoicePaymentParams>;

export const RefundInvoicePaymentBody = RefundRequestSchema;
export type RefundInvoicePaymentBody = z.infer<typeof RefundInvoicePaymentBody>;

export const RefundInvoicePaymentResponse = RefundResponseSchema;

export const VoidInvoiceParams = z.object({
  invoice: UUIDSchema,
});
export type VoidInvoiceParams = z.infer<typeof VoidInvoiceParams>;

export const VoidInvoiceResponse = InvoiceSchema;

export const CreateMerchantAccountBody = CreateMerchantAccountRequestSchema;
export type CreateMerchantAccountBody = z.infer<typeof CreateMerchantAccountBody>;

export const CreateMerchantAccountResponse = MerchantAccountSchema;

export const GetMerchantAccountParams = z.object({
  merchantAccount: z.union([UUIDSchema, z.enum(["me"])]),
});
export type GetMerchantAccountParams = z.infer<typeof GetMerchantAccountParams>;

export const GetMerchantAccountQuery = z.object({
  expand: z.string().optional(),
});
export type GetMerchantAccountQuery = z.infer<typeof GetMerchantAccountQuery>;

export const GetMerchantAccountResponse = MerchantAccountSchema;

export const GetMerchantDashboardParams = z.object({
  merchantAccount: z.union([UUIDSchema, z.enum(["me"])]),
});
export type GetMerchantDashboardParams = z.infer<typeof GetMerchantDashboardParams>;

export const GetMerchantDashboardResponse = DashboardResponseSchema;

export const OnboardMerchantAccountParams = z.object({
  merchantAccount: UUIDSchema,
});
export type OnboardMerchantAccountParams = z.infer<typeof OnboardMerchantAccountParams>;

export const OnboardMerchantAccountBody = OnboardingRequestSchema;
export type OnboardMerchantAccountBody = z.infer<typeof OnboardMerchantAccountBody>;

export const OnboardMerchantAccountResponse = OnboardingResponseSchema;

export const HandleStripeWebhookBody = z.unknown();
export type HandleStripeWebhookBody = z.infer<typeof HandleStripeWebhookBody>;

export const HandleStripeWebhookResponse = z.unknown();

export const CreateBookingBody = BookingCreateRequestSchema;
export type CreateBookingBody = z.infer<typeof CreateBookingBody>;

export const CreateBookingResponse = BookingSchema;

export const ListBookingsQuery = z.object({
  provider: UUIDSchema.optional(),
  client: UUIDSchema.optional(),
  status: BookingStatusSchema.optional(),
  startDate: z.string().datetime().transform((str) => new Date(str)).optional(),
  endDate: z.string().datetime().transform((str) => new Date(str)).optional(),
  expand: z.string().optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListBookingsQuery = z.infer<typeof ListBookingsQuery>;

export const ListBookingsResponse = z.object({
  data: z.array(BookingSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetBookingParams = z.object({
  booking: UUIDSchema,
});
export type GetBookingParams = z.infer<typeof GetBookingParams>;

export const GetBookingQuery = z.object({
  expand: z.string().optional(),
});
export type GetBookingQuery = z.infer<typeof GetBookingQuery>;

export const GetBookingResponse = BookingSchema;

export const CancelBookingParams = z.object({
  booking: UUIDSchema,
});
export type CancelBookingParams = z.infer<typeof CancelBookingParams>;

export const CancelBookingBody = BookingActionRequestSchema;
export type CancelBookingBody = z.infer<typeof CancelBookingBody>;

export const CancelBookingResponse = BookingSchema;

export const ConfirmBookingParams = z.object({
  booking: UUIDSchema,
});
export type ConfirmBookingParams = z.infer<typeof ConfirmBookingParams>;

export const ConfirmBookingBody = BookingActionRequestSchema;
export type ConfirmBookingBody = z.infer<typeof ConfirmBookingBody>;

export const ConfirmBookingResponse = BookingSchema;

export const MarkNoShowBookingParams = z.object({
  booking: UUIDSchema,
});
export type MarkNoShowBookingParams = z.infer<typeof MarkNoShowBookingParams>;

export const MarkNoShowBookingBody = BookingActionRequestSchema;
export type MarkNoShowBookingBody = z.infer<typeof MarkNoShowBookingBody>;

export const MarkNoShowBookingResponse = BookingSchema;

export const RejectBookingParams = z.object({
  booking: UUIDSchema,
});
export type RejectBookingParams = z.infer<typeof RejectBookingParams>;

export const RejectBookingBody = BookingActionRequestSchema;
export type RejectBookingBody = z.infer<typeof RejectBookingBody>;

export const RejectBookingResponse = BookingSchema;

export const ListBookingEventsQuery = z.object({
  owner: UUIDSchema.optional(),
  context: z.string().optional(),
  locationType: LocationTypeSchema.optional(),
  status: BookingEventStatusSchema.optional(),
  availableFrom: z.string().datetime().transform((str) => new Date(str)).optional(),
  availableTo: z.string().datetime().transform((str) => new Date(str)).optional(),
  tags: z.union([z.string(), z.array(z.string())]).optional(),
  expand: z.string().optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListBookingEventsQuery = z.infer<typeof ListBookingEventsQuery>;

export const ListBookingEventsResponse = z.object({
  data: z.array(BookingEventSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const CreateBookingEventBody = BookingEventCreateRequestSchema;
export type CreateBookingEventBody = z.infer<typeof CreateBookingEventBody>;

export const CreateBookingEventResponse = BookingEventSchema;

export const GetBookingEventParams = z.object({
  event: z.union([UUIDSchema, z.enum(["me"])]),
});
export type GetBookingEventParams = z.infer<typeof GetBookingEventParams>;

export const GetBookingEventQuery = z.object({
  expand: z.string().optional(),
});
export type GetBookingEventQuery = z.infer<typeof GetBookingEventQuery>;

export const GetBookingEventResponse = BookingEventSchema;

export const UpdateBookingEventParams = z.object({
  event: UUIDSchema,
});
export type UpdateBookingEventParams = z.infer<typeof UpdateBookingEventParams>;

export const UpdateBookingEventBody = BookingEventUpdateRequestSchema;
export type UpdateBookingEventBody = z.infer<typeof UpdateBookingEventBody>;

export const UpdateBookingEventResponse = BookingEventSchema;

export const DeleteBookingEventParams = z.object({
  event: UUIDSchema,
});
export type DeleteBookingEventParams = z.infer<typeof DeleteBookingEventParams>;

export const DeleteBookingEventResponse = z.void();

export const CreateScheduleExceptionParams = z.object({
  event: UUIDSchema,
});
export type CreateScheduleExceptionParams = z.infer<typeof CreateScheduleExceptionParams>;

export const CreateScheduleExceptionBody = ScheduleExceptionCreateRequestSchema;
export type CreateScheduleExceptionBody = z.infer<typeof CreateScheduleExceptionBody>;

export const CreateScheduleExceptionResponse = ScheduleExceptionSchema;

export const ListScheduleExceptionsParams = z.object({
  event: UUIDSchema,
});
export type ListScheduleExceptionsParams = z.infer<typeof ListScheduleExceptionsParams>;

export const ListScheduleExceptionsQuery = z.object({
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListScheduleExceptionsQuery = z.infer<typeof ListScheduleExceptionsQuery>;

export const ListScheduleExceptionsResponse = z.object({
  data: z.array(ScheduleExceptionSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetScheduleExceptionParams = z.object({
  event: UUIDSchema,
  exception: UUIDSchema,
});
export type GetScheduleExceptionParams = z.infer<typeof GetScheduleExceptionParams>;

export const GetScheduleExceptionResponse = ScheduleExceptionSchema;

export const DeleteScheduleExceptionParams = z.object({
  event: UUIDSchema,
  exception: UUIDSchema,
});
export type DeleteScheduleExceptionParams = z.infer<typeof DeleteScheduleExceptionParams>;

export const DeleteScheduleExceptionResponse = z.void();

export const ListEventSlotsParams = z.object({
  event: UUIDSchema,
});
export type ListEventSlotsParams = z.infer<typeof ListEventSlotsParams>;

export const ListEventSlotsQuery = z.object({
  startTime: z.string().datetime().transform((str) => new Date(str)).optional(),
  endTime: z.string().datetime().transform((str) => new Date(str)).optional(),
  status: SlotStatusSchema.optional(),
});
export type ListEventSlotsQuery = z.infer<typeof ListEventSlotsQuery>;

export const ListEventSlotsResponse = z.array(TimeSlotSchema);

export const GetTimeSlotParams = z.object({
  slotId: UUIDSchema,
});
export type GetTimeSlotParams = z.infer<typeof GetTimeSlotParams>;

export const GetTimeSlotQuery = z.object({
  expand: z.string().optional(),
});
export type GetTimeSlotQuery = z.infer<typeof GetTimeSlotQuery>;

export const GetTimeSlotResponse = TimeSlotSchema;

export const CreateChatRoomBody = CreateChatRoomRequestSchema;
export type CreateChatRoomBody = z.infer<typeof CreateChatRoomBody>;

export const CreateChatRoomResponse = ChatRoomSchema;

export const ListChatRoomsQuery = z.object({
  status: ChatRoomStatusSchema.optional(),
  context: UUIDSchema.optional(),
  withParticipant: UUIDSchema.optional(),
  hasActiveCall: z.coerce.boolean().optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListChatRoomsQuery = z.infer<typeof ListChatRoomsQuery>;

export const ListChatRoomsResponse = z.object({
  data: z.array(ChatRoomSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetChatRoomParams = z.object({
  room: UUIDSchema,
});
export type GetChatRoomParams = z.infer<typeof GetChatRoomParams>;

export const GetChatRoomResponse = ChatRoomSchema;

export const GetChatMessagesParams = z.object({
  room: UUIDSchema,
});
export type GetChatMessagesParams = z.infer<typeof GetChatMessagesParams>;

export const GetChatMessagesQuery = z.object({
  messageType: MessageTypeSchema.optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type GetChatMessagesQuery = z.infer<typeof GetChatMessagesQuery>;

export const GetChatMessagesResponse = z.object({
  data: z.array(ChatMessageSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const SendChatMessageParams = z.object({
  room: UUIDSchema,
});
export type SendChatMessageParams = z.infer<typeof SendChatMessageParams>;

export const SendChatMessageBody = z.union([SendTextMessageRequestSchema, StartVideoCallRequestSchema]);
export type SendChatMessageBody = z.infer<typeof SendChatMessageBody>;

export const SendChatMessageResponse = ChatMessageSchema;

export const EndVideoCallParams = z.object({
  room: UUIDSchema,
});
export type EndVideoCallParams = z.infer<typeof EndVideoCallParams>;

export const EndVideoCallResponse = VideoCallEndResponseSchema;

export const JoinVideoCallParams = z.object({
  room: UUIDSchema,
});
export type JoinVideoCallParams = z.infer<typeof JoinVideoCallParams>;

export const JoinVideoCallBody = JoinVideoCallRequestSchema;
export type JoinVideoCallBody = z.infer<typeof JoinVideoCallBody>;

export const JoinVideoCallResponse = VideoCallJoinResponseSchema;

export const LeaveVideoCallParams = z.object({
  room: UUIDSchema,
});
export type LeaveVideoCallParams = z.infer<typeof LeaveVideoCallParams>;

export const LeaveVideoCallResponse = LeaveVideoCallResponseSchema;

export const UpdateVideoCallParticipantParams = z.object({
  room: UUIDSchema,
});
export type UpdateVideoCallParticipantParams = z.infer<typeof UpdateVideoCallParticipantParams>;

export const UpdateVideoCallParticipantBody = UpdateParticipantRequestSchema;
export type UpdateVideoCallParticipantBody = z.infer<typeof UpdateVideoCallParticipantBody>;

export const UpdateVideoCallParticipantResponse = CallParticipantSchema;

export const GetIceServersResponse = IceServersResponseSchema;

export const ListEmailQueueItemsQuery = z.object({
  status: z.union([EmailQueueStatusSchema, z.array(EmailQueueStatusSchema), z.string().transform(val => val.split(",").map(s => s.trim())).pipe(z.array(EmailQueueStatusSchema))]).optional(),
  recipientEmail: EmailSchema.optional(),
  dateFrom: z.string().datetime().transform((str) => new Date(str)).optional(),
  dateTo: z.string().datetime().transform((str) => new Date(str)).optional(),
  priority: z.coerce.number().int().optional(),
  scheduledOnly: z.coerce.boolean().optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListEmailQueueItemsQuery = z.infer<typeof ListEmailQueueItemsQuery>;

export const ListEmailQueueItemsResponse = z.object({
  data: z.array(EmailQueueItemSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetEmailQueueItemParams = z.object({
  queue: UUIDSchema,
});
export type GetEmailQueueItemParams = z.infer<typeof GetEmailQueueItemParams>;

export const GetEmailQueueItemResponse = EmailQueueItemSchema;

export const CancelEmailQueueItemParams = z.object({
  queue: UUIDSchema,
});
export type CancelEmailQueueItemParams = z.infer<typeof CancelEmailQueueItemParams>;

export const CancelEmailQueueItemBody = CancelEmailRequestSchema;
export type CancelEmailQueueItemBody = z.infer<typeof CancelEmailQueueItemBody>;

export const CancelEmailQueueItemResponse = EmailQueueItemSchema;

export const RetryEmailQueueItemParams = z.object({
  queue: UUIDSchema,
});
export type RetryEmailQueueItemParams = z.infer<typeof RetryEmailQueueItemParams>;

export const RetryEmailQueueItemResponse = EmailQueueItemSchema;

export const ListEmailTemplatesQuery = z.object({
  status: TemplateStatusSchema.optional(),
  tags: z.string().transform(val => val.split(",").filter(Boolean)).optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListEmailTemplatesQuery = z.infer<typeof ListEmailTemplatesQuery>;

export const ListEmailTemplatesResponse = z.object({
  data: z.array(EmailTemplateSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const CreateEmailTemplateBody = CreateTemplateRequestSchema;
export type CreateEmailTemplateBody = z.infer<typeof CreateEmailTemplateBody>;

export const CreateEmailTemplateResponse = EmailTemplateSchema;

export const GetEmailTemplateParams = z.object({
  template: UUIDSchema,
});
export type GetEmailTemplateParams = z.infer<typeof GetEmailTemplateParams>;

export const GetEmailTemplateResponse = EmailTemplateSchema;

export const UpdateEmailTemplateParams = z.object({
  template: UUIDSchema,
});
export type UpdateEmailTemplateParams = z.infer<typeof UpdateEmailTemplateParams>;

export const UpdateEmailTemplateBody = UpdateTemplateRequestSchema;
export type UpdateEmailTemplateBody = z.infer<typeof UpdateEmailTemplateBody>;

export const UpdateEmailTemplateResponse = EmailTemplateSchema;

export const TestEmailTemplateParams = z.object({
  template: UUIDSchema,
});
export type TestEmailTemplateParams = z.infer<typeof TestEmailTemplateParams>;

export const TestEmailTemplateBody = TestTemplateRequestSchema;
export type TestEmailTemplateBody = z.infer<typeof TestEmailTemplateBody>;

export const TestEmailTemplateResponse = TestTemplateResultSchema;

export const ListNotificationsQuery = z.object({
  type: NotificationTypeSchema.optional(),
  channel: NotificationChannelSchema.optional(),
  status: NotificationStatusSchema.optional(),
  startDate: z.string().datetime().transform((str) => new Date(str)).optional(),
  endDate: z.string().datetime().transform((str) => new Date(str)).optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListNotificationsQuery = z.infer<typeof ListNotificationsQuery>;

export const ListNotificationsResponse = z.object({
  data: z.array(NotificationSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const MarkAllNotificationsAsReadQuery = z.object({
  type: NotificationTypeSchema.optional(),
});
export type MarkAllNotificationsAsReadQuery = z.infer<typeof MarkAllNotificationsAsReadQuery>;

export const MarkAllNotificationsAsReadResponse = z.object({
  markedCount: z.number().int()
});

export const GetNotificationParams = z.object({
  notif: UUIDSchema,
});
export type GetNotificationParams = z.infer<typeof GetNotificationParams>;

export const GetNotificationResponse = NotificationSchema;

export const MarkNotificationAsReadParams = z.object({
  notif: UUIDSchema,
});
export type MarkNotificationAsReadParams = z.infer<typeof MarkNotificationAsReadParams>;

export const MarkNotificationAsReadResponse = NotificationSchema;

export const CreatePersonBody = PersonCreateRequestSchema;
export type CreatePersonBody = z.infer<typeof CreatePersonBody>;

export const CreatePersonResponse = PersonSchema;

export const ListPersonsQuery = z.object({
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListPersonsQuery = z.infer<typeof ListPersonsQuery>;

export const ListPersonsResponse = z.object({
  data: z.array(PersonSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetPersonParams = z.object({
  person: z.union([UUIDSchema, z.enum(["me"])]),
});
export type GetPersonParams = z.infer<typeof GetPersonParams>;

export const GetPersonResponse = PersonSchema;

export const UpdatePersonParams = z.object({
  person: UUIDSchema,
});
export type UpdatePersonParams = z.infer<typeof UpdatePersonParams>;

export const UpdatePersonBody = PersonUpdateRequestSchema;
export type UpdatePersonBody = z.infer<typeof UpdatePersonBody>;

export const UpdatePersonResponse = PersonSchema;

export const CreateReviewBody = CreateReviewRequestSchema;
export type CreateReviewBody = z.infer<typeof CreateReviewBody>;

export const CreateReviewResponse = ReviewSchema;

export const ListReviewsQuery = z.object({
  context: UUIDSchema.optional(),
  reviewer: UUIDSchema.optional(),
  reviewType: z.string().optional(),
  reviewedEntity: UUIDSchema.optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListReviewsQuery = z.infer<typeof ListReviewsQuery>;

export const ListReviewsResponse = z.object({
  data: z.array(ReviewSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const GetReviewParams = z.object({
  review: UUIDSchema,
});
export type GetReviewParams = z.infer<typeof GetReviewParams>;

export const GetReviewResponse = ReviewSchema;

export const DeleteReviewParams = z.object({
  review: UUIDSchema,
});
export type DeleteReviewParams = z.infer<typeof DeleteReviewParams>;

export const DeleteReviewResponse = z.void();

export const ListFilesQuery = z.object({
  status: FileStatusSchema.optional(),
  owner: UUIDSchema.optional(),
  offset: z.coerce.number().int().gte(0).optional(),
  limit: z.coerce.number().int().gte(1).lte(100).optional(),
  page: z.coerce.number().int().gte(1).optional(),
  pageSize: z.coerce.number().int().gte(1).lte(100).optional(),
  q: z.string().max(500).optional(),
  sort: z.string().optional(),
});
export type ListFilesQuery = z.infer<typeof ListFilesQuery>;

export const ListFilesResponse = z.object({
  data: z.array(StoredFileSchema),
  pagination: z.object({
  offset: z.number().int(),
  limit: z.number().int(),
  count: z.number().int(),
  totalCount: z.number().int(),
  totalPages: z.number().int(),
  currentPage: z.number().int(),
  hasNextPage: z.boolean(),
  hasPreviousPage: z.boolean()
})
});

export const UploadFileBody = FileUploadRequestSchema;
export type UploadFileBody = z.infer<typeof UploadFileBody>;

export const UploadFileResponse = FileUploadResponseSchema;

export const GetFileParams = z.object({
  file: UUIDSchema,
});
export type GetFileParams = z.infer<typeof GetFileParams>;

export const GetFileResponse = StoredFileSchema;

export const DeleteFileParams = z.object({
  file: UUIDSchema,
});
export type DeleteFileParams = z.infer<typeof DeleteFileParams>;

export const DeleteFileResponse = z.void();

export const CompleteFileUploadParams = z.object({
  file: UUIDSchema,
});
export type CompleteFileUploadParams = z.infer<typeof CompleteFileUploadParams>;

export const CompleteFileUploadResponse = StoredFileSchema;

export const GetFileDownloadParams = z.object({
  file: UUIDSchema,
});
export type GetFileDownloadParams = z.infer<typeof GetFileDownloadParams>;

export const GetFileDownloadResponse = FileDownloadResponseSchema;
