CREATE TYPE "public"."audit_action" AS ENUM('create', 'read', 'update', 'delete', 'login', 'logout');--> statement-breakpoint
CREATE TYPE "public"."audit_category" AS ENUM('hipaa', 'security', 'privacy', 'administrative', 'clinical', 'financial');--> statement-breakpoint
CREATE TYPE "public"."audit_event_type" AS ENUM('authentication', 'data-access', 'data-modification', 'system-config', 'security', 'compliance');--> statement-breakpoint
CREATE TYPE "public"."audit_outcome" AS ENUM('success', 'failure', 'partial', 'denied');--> statement-breakpoint
CREATE TYPE "public"."audit_retention_status" AS ENUM('active', 'archived', 'pending-purge');--> statement-breakpoint
CREATE TYPE "public"."capture_method" AS ENUM('automatic', 'manual');--> statement-breakpoint
CREATE TYPE "public"."invoice_status" AS ENUM('draft', 'open', 'paid', 'void', 'uncollectible');--> statement-breakpoint
CREATE TYPE "public"."payment_status" AS ENUM('pending', 'requires_capture', 'processing', 'succeeded', 'failed', 'canceled');--> statement-breakpoint
CREATE TYPE "public"."booking_event_status" AS ENUM('draft', 'active', 'paused', 'archived');--> statement-breakpoint
CREATE TYPE "public"."booking_status" AS ENUM('pending', 'confirmed', 'rejected', 'cancelled', 'completed', 'no_show_client', 'no_show_provider');--> statement-breakpoint
CREATE TYPE "public"."consultation_mode" AS ENUM('video', 'phone', 'in-person');--> statement-breakpoint
CREATE TYPE "public"."location_type" AS ENUM('video', 'phone', 'in-person');--> statement-breakpoint
CREATE TYPE "public"."recurrence_type" AS ENUM('daily', 'weekly', 'monthly', 'yearly');--> statement-breakpoint
CREATE TYPE "public"."slot_status" AS ENUM('available', 'booked', 'blocked');--> statement-breakpoint
CREATE TYPE "public"."chat_room_status" AS ENUM('active', 'archived');--> statement-breakpoint
CREATE TYPE "public"."message_type" AS ENUM('text', 'system', 'video_call');--> statement-breakpoint
CREATE TYPE "public"."participant_type" AS ENUM('patient', 'provider');--> statement-breakpoint
CREATE TYPE "public"."video_call_status" AS ENUM('starting', 'active', 'ended', 'cancelled');--> statement-breakpoint
CREATE TYPE "public"."email_provider" AS ENUM('smtp', 'postmark', 'onesignal');--> statement-breakpoint
CREATE TYPE "public"."email_queue_status" AS ENUM('pending', 'processing', 'sent', 'failed', 'cancelled');--> statement-breakpoint
CREATE TYPE "public"."template_status" AS ENUM('draft', 'active', 'archived');--> statement-breakpoint
CREATE TYPE "public"."variable_type" AS ENUM('string', 'number', 'boolean', 'date', 'datetime', 'url', 'email', 'array');--> statement-breakpoint
CREATE TYPE "public"."notification_channel" AS ENUM('email', 'push', 'in-app');--> statement-breakpoint
CREATE TYPE "public"."notification_status" AS ENUM('queued', 'sent', 'delivered', 'read', 'failed', 'expired');--> statement-breakpoint
CREATE TYPE "public"."notification_type" AS ENUM('billing', 'security', 'system', 'booking.created', 'booking.confirmed', 'booking.rejected', 'booking.cancelled', 'booking.no-show-client', 'booking.no-show-provider', 'comms.video-call-started', 'comms.video-call-joined', 'comms.video-call-left', 'comms.video-call-ended', 'comms.chat-message');--> statement-breakpoint
CREATE TYPE "public"."gender" AS ENUM('male', 'female', 'non-binary', 'other', 'prefer-not-to-say');--> statement-breakpoint
CREATE TYPE "public"."file_status" AS ENUM('uploading', 'processing', 'available', 'failed');--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "account" (
	"id" text PRIMARY KEY NOT NULL,
	"account_id" text NOT NULL,
	"provider_id" text NOT NULL,
	"user_id" text NOT NULL,
	"access_token" text,
	"refresh_token" text,
	"id_token" text,
	"access_token_expires_at" timestamp,
	"refresh_token_expires_at" timestamp,
	"scope" text,
	"password" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "apikey" (
	"id" text PRIMARY KEY NOT NULL,
	"name" text,
	"start" text,
	"prefix" text,
	"key" text NOT NULL,
	"user_id" text NOT NULL,
	"refill_interval" integer,
	"refill_amount" integer,
	"last_refill_at" timestamp,
	"enabled" boolean DEFAULT true,
	"rate_limit_enabled" boolean DEFAULT true,
	"rate_limit_time_window" integer DEFAULT 86400000,
	"rate_limit_max" integer DEFAULT 10,
	"request_count" integer DEFAULT 0,
	"remaining" integer,
	"last_request" timestamp,
	"expires_at" timestamp,
	"created_at" timestamp NOT NULL,
	"updated_at" timestamp NOT NULL,
	"permissions" text,
	"metadata" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "passkey" (
	"id" text PRIMARY KEY NOT NULL,
	"name" text,
	"public_key" text NOT NULL,
	"user_id" text NOT NULL,
	"credential_id" text NOT NULL,
	"counter" integer NOT NULL,
	"device_type" text NOT NULL,
	"backed_up" boolean NOT NULL,
	"transports" text,
	"created_at" timestamp,
	"aaguid" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "session" (
	"id" text PRIMARY KEY NOT NULL,
	"expires_at" timestamp NOT NULL,
	"token" text NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp NOT NULL,
	"ip_address" text,
	"user_agent" text,
	"user_id" text NOT NULL,
	"impersonated_by" text,
	CONSTRAINT "session_token_unique" UNIQUE("token")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "two_factor" (
	"id" text PRIMARY KEY NOT NULL,
	"secret" text NOT NULL,
	"backup_codes" text NOT NULL,
	"user_id" text NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "user" (
	"id" text PRIMARY KEY NOT NULL,
	"name" text NOT NULL,
	"email" text NOT NULL,
	"email_verified" boolean DEFAULT false NOT NULL,
	"image" text,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"role" text,
	"banned" boolean DEFAULT false,
	"ban_reason" text,
	"ban_expires" timestamp,
	"two_factor_enabled" boolean DEFAULT false,
	CONSTRAINT "user_email_unique" UNIQUE("email")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "verification" (
	"id" text PRIMARY KEY NOT NULL,
	"identifier" text NOT NULL,
	"value" text NOT NULL,
	"expires_at" timestamp NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "audit_log_entry" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"event_type" "audit_event_type" NOT NULL,
	"category" "audit_category" NOT NULL,
	"action" "audit_action" NOT NULL,
	"outcome" "audit_outcome" NOT NULL,
	"user" uuid,
	"user_type" varchar(20),
	"resource_type" varchar(100) NOT NULL,
	"resource" varchar(255) NOT NULL,
	"description" varchar(1000) NOT NULL,
	"details" jsonb,
	"ip_address" varchar(45),
	"user_agent" varchar(500),
	"session_id" varchar(255),
	"request_id" varchar(255),
	"integrity_hash" varchar(64),
	"retention_status" "audit_retention_status" DEFAULT 'active' NOT NULL,
	"archived_at" timestamp,
	"archived_by" text,
	"purge_after" timestamp
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "invoice_line_item" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"invoice" uuid NOT NULL,
	"description" varchar(500) NOT NULL,
	"quantity" integer DEFAULT 1 NOT NULL,
	"unit_price" integer NOT NULL,
	"amount" integer NOT NULL,
	"metadata" jsonb
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "invoice" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"invoice_number" varchar(50) NOT NULL,
	"customer" uuid NOT NULL,
	"merchant" uuid NOT NULL,
	"merchant_account" uuid,
	"context" varchar(255),
	"status" "invoice_status" DEFAULT 'draft' NOT NULL,
	"subtotal" integer NOT NULL,
	"tax" integer,
	"total" integer NOT NULL,
	"currency" varchar(3) DEFAULT 'USD' NOT NULL,
	"payment_capture_method" "capture_method" DEFAULT 'automatic' NOT NULL,
	"payment_due_at" timestamp,
	"payment_status" "payment_status",
	"paid_at" timestamp,
	"paid_by" uuid,
	"voided_at" timestamp,
	"voided_by" uuid,
	"void_threshold_minutes" integer,
	"authorized_at" timestamp,
	"authorized_by" uuid,
	"metadata" jsonb,
	CONSTRAINT "invoices_invoice_number_unique" UNIQUE("invoice_number"),
	CONSTRAINT "invoices_context_unique" UNIQUE("context")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "merchant_account" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"person" uuid NOT NULL,
	"active" boolean DEFAULT true NOT NULL,
	"metadata" jsonb NOT NULL,
	CONSTRAINT "merchant_accounts_person_unique" UNIQUE("person")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "booking_event" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"owner_id" uuid NOT NULL,
	"context_id" text,
	"title" text NOT NULL,
	"description" text,
	"keywords" jsonb DEFAULT '[]'::jsonb,
	"tags" jsonb DEFAULT '[]'::jsonb,
	"timezone" text DEFAULT 'America/New_York' NOT NULL,
	"location_types" jsonb DEFAULT '["video","phone","in-person"]'::jsonb NOT NULL,
	"max_booking_days" integer DEFAULT 30 NOT NULL,
	"min_booking_minutes" integer DEFAULT 1440 NOT NULL,
	"form_config" jsonb,
	"billing_config" jsonb,
	"status" "booking_event_status" DEFAULT 'active' NOT NULL,
	"effective_from" timestamp DEFAULT now() NOT NULL,
	"effective_to" timestamp,
	"daily_configs" jsonb NOT NULL,
	CONSTRAINT "booking_events_max_booking_days_check" CHECK ("booking_event"."max_booking_days" >= 0 AND "booking_event"."max_booking_days" <= 365),
	CONSTRAINT "booking_events_min_booking_minutes_check" CHECK ("booking_event"."min_booking_minutes" >= 0 AND "booking_event"."min_booking_minutes" <= 4320)
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "booking" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"client_id" uuid NOT NULL,
	"provider_id" uuid NOT NULL,
	"slot_id" uuid NOT NULL,
	"location_type" "location_type" NOT NULL,
	"reason" text,
	"status" "booking_status" DEFAULT 'pending' NOT NULL,
	"booked_at" timestamp DEFAULT now() NOT NULL,
	"confirmation_timestamp" timestamp,
	"scheduled_at" timestamp NOT NULL,
	"duration_minutes" integer NOT NULL,
	"cancellation_reason" text,
	"cancelled_by" text,
	"cancelled_at" timestamp,
	"no_show_marked_by" text,
	"no_show_marked_at" timestamp,
	"form_responses" jsonb,
	"invoice" uuid,
	CONSTRAINT "bookings_reason_check" CHECK (LENGTH("booking"."reason") <= 500),
	CONSTRAINT "bookings_duration_minutes_check" CHECK ("booking"."duration_minutes" >= 15 AND "booking"."duration_minutes" <= 480)
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "schedule_exception" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"event_id" uuid NOT NULL,
	"owner_id" uuid NOT NULL,
	"context_id" text,
	"timezone" text DEFAULT 'America/New_York' NOT NULL,
	"start_datetime" timestamp NOT NULL,
	"end_datetime" timestamp NOT NULL,
	"reason" text NOT NULL,
	"recurring" boolean DEFAULT false NOT NULL,
	"recurrence_pattern" jsonb,
	CONSTRAINT "schedule_exceptions_reason_check" CHECK (LENGTH("schedule_exception"."reason") <= 500),
	CONSTRAINT "schedule_exceptions_date_range_check" CHECK ("schedule_exception"."end_datetime" > "schedule_exception"."start_datetime")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "time_slot" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"owner_id" uuid NOT NULL,
	"event_id" uuid NOT NULL,
	"context_id" text,
	"start_time" timestamp NOT NULL,
	"end_time" timestamp NOT NULL,
	"location_types" jsonb NOT NULL,
	"status" "slot_status" DEFAULT 'available' NOT NULL,
	"billing_config" jsonb,
	"booking_id" uuid,
	CONSTRAINT "time_slots_event_time_unique" UNIQUE("event_id","start_time")
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "chat_message" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"chat_room_id" uuid NOT NULL,
	"sender_id" uuid NOT NULL,
	"timestamp" timestamp DEFAULT now() NOT NULL,
	"message_type" "message_type" NOT NULL,
	"message" text,
	"video_call_data" jsonb
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "chat_room" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"participants" jsonb NOT NULL,
	"admins" jsonb NOT NULL,
	"context_id" text,
	"status" "chat_room_status" DEFAULT 'active' NOT NULL,
	"last_message_at" timestamp,
	"message_count" integer DEFAULT 0 NOT NULL,
	"active_video_call_message_id" uuid
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "email_queue" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"template" uuid,
	"template_tags" jsonb,
	"recipient_email" varchar(255) NOT NULL,
	"recipient_name" varchar(255),
	"variables" jsonb NOT NULL,
	"metadata" jsonb,
	"status" "email_queue_status" DEFAULT 'pending' NOT NULL,
	"priority" integer DEFAULT 5 NOT NULL,
	"scheduled_at" timestamp,
	"attempts" integer DEFAULT 0 NOT NULL,
	"last_attempt_at" timestamp,
	"next_retry_at" timestamp,
	"last_error" text,
	"sent_at" timestamp,
	"provider" "email_provider",
	"provider_message_id" varchar(255),
	"cancelled_at" timestamp,
	"cancelled_by" uuid,
	"cancellation_reason" text
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "email_template" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"name" varchar(255) NOT NULL,
	"description" text,
	"subject" varchar(500) NOT NULL,
	"body_html" text NOT NULL,
	"body_text" text,
	"tags" jsonb,
	"variables" jsonb NOT NULL,
	"from_name" varchar(255),
	"from_email" varchar(255),
	"reply_to_email" varchar(255),
	"reply_to_name" varchar(255),
	"status" "template_status" DEFAULT 'draft' NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "notification" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"recipient_id" uuid NOT NULL,
	"type" "notification_type" NOT NULL,
	"channel" "notification_channel" NOT NULL,
	"title" varchar(200) NOT NULL,
	"message" varchar(1000) NOT NULL,
	"scheduled_at" timestamp,
	"related_entity_type" varchar(50),
	"related_entity" uuid,
	"status" "notification_status" DEFAULT 'queued' NOT NULL,
	"sent_at" timestamp,
	"read_at" timestamp,
	"consent_validated" boolean DEFAULT false NOT NULL
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "person" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"first_name" varchar(50) NOT NULL,
	"last_name" varchar(50),
	"middle_name" varchar(50),
	"date_of_birth" date,
	"gender" "gender",
	"primary_address" jsonb,
	"contact_info" jsonb,
	"avatar" jsonb,
	"languages_spoken" jsonb,
	"timezone" varchar(50)
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "review" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"context_id" uuid NOT NULL,
	"reviewer_id" uuid NOT NULL,
	"review_type" varchar(50) NOT NULL,
	"reviewed_entity_id" uuid,
	"nps_score" integer NOT NULL,
	"comment" text,
	CONSTRAINT "reviews_context_reviewer_type_unique" UNIQUE("context_id","reviewer_id","review_type"),
	CONSTRAINT "reviews_nps_score_check" CHECK ("review"."nps_score" >= 0 AND "review"."nps_score" <= 10),
	CONSTRAINT "reviews_comment_check" CHECK (LENGTH("review"."comment") <= 1000),
	CONSTRAINT "reviews_review_type_check" CHECK (LENGTH("review"."review_type") <= 50)
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "stored_file" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"filename" varchar(255) NOT NULL,
	"mime_type" varchar(100) NOT NULL,
	"size" bigint NOT NULL,
	"status" "file_status" DEFAULT 'uploading' NOT NULL,
	"owner" uuid NOT NULL,
	"uploaded_at" timestamp DEFAULT now()
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "account" ADD CONSTRAINT "account_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "apikey" ADD CONSTRAINT "apikey_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "passkey" ADD CONSTRAINT "passkey_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "session" ADD CONSTRAINT "session_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "two_factor" ADD CONSTRAINT "two_factor_user_id_user_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."user"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "audit_log_entry" ADD CONSTRAINT "audit_log_entry_archived_by_user_id_fk" FOREIGN KEY ("archived_by") REFERENCES "public"."user"("id") ON DELETE no action ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invoice_line_item" ADD CONSTRAINT "invoice_line_item_invoice_invoice_id_fk" FOREIGN KEY ("invoice") REFERENCES "public"."invoice"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invoice" ADD CONSTRAINT "invoice_customer_person_id_fk" FOREIGN KEY ("customer") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invoice" ADD CONSTRAINT "invoice_merchant_person_id_fk" FOREIGN KEY ("merchant") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "invoice" ADD CONSTRAINT "invoice_merchant_account_merchant_account_id_fk" FOREIGN KEY ("merchant_account") REFERENCES "public"."merchant_account"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "merchant_account" ADD CONSTRAINT "merchant_account_person_person_id_fk" FOREIGN KEY ("person") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "booking_event" ADD CONSTRAINT "booking_event_owner_id_person_id_fk" FOREIGN KEY ("owner_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "booking" ADD CONSTRAINT "booking_client_id_person_id_fk" FOREIGN KEY ("client_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "booking" ADD CONSTRAINT "booking_provider_id_person_id_fk" FOREIGN KEY ("provider_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "booking" ADD CONSTRAINT "booking_slot_id_time_slot_id_fk" FOREIGN KEY ("slot_id") REFERENCES "public"."time_slot"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "schedule_exception" ADD CONSTRAINT "schedule_exception_event_id_booking_event_id_fk" FOREIGN KEY ("event_id") REFERENCES "public"."booking_event"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "schedule_exception" ADD CONSTRAINT "schedule_exception_owner_id_person_id_fk" FOREIGN KEY ("owner_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "time_slot" ADD CONSTRAINT "time_slot_owner_id_person_id_fk" FOREIGN KEY ("owner_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "time_slot" ADD CONSTRAINT "time_slot_event_id_booking_event_id_fk" FOREIGN KEY ("event_id") REFERENCES "public"."booking_event"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "time_slot" ADD CONSTRAINT "time_slot_booking_id_booking_id_fk" FOREIGN KEY ("booking_id") REFERENCES "public"."booking"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "chat_message" ADD CONSTRAINT "chat_message_chat_room_id_chat_room_id_fk" FOREIGN KEY ("chat_room_id") REFERENCES "public"."chat_room"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "email_queue" ADD CONSTRAINT "email_queue_template_email_template_id_fk" FOREIGN KEY ("template") REFERENCES "public"."email_template"("id") ON DELETE set null ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "review" ADD CONSTRAINT "review_reviewer_id_person_id_fk" FOREIGN KEY ("reviewer_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "review" ADD CONSTRAINT "review_reviewed_entity_id_person_id_fk" FOREIGN KEY ("reviewed_entity_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_event_type_idx" ON "audit_log_entry" USING btree ("event_type");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_category_idx" ON "audit_log_entry" USING btree ("category");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_user_idx" ON "audit_log_entry" USING btree ("user");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_resource_idx" ON "audit_log_entry" USING btree ("resource_type","resource");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_created_at_idx" ON "audit_log_entry" USING btree ("created_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_retention_status_idx" ON "audit_log_entry" USING btree ("retention_status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_user_event_idx" ON "audit_log_entry" USING btree ("user","event_type");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_resource_type_event_idx" ON "audit_log_entry" USING btree ("resource_type","event_type");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "audit_date_range_idx" ON "audit_log_entry" USING btree ("created_at","retention_status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoice_line_items_invoice_idx" ON "invoice_line_item" USING btree ("invoice");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_customer_idx" ON "invoice" USING btree ("customer");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_merchant_idx" ON "invoice" USING btree ("merchant");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_merchant_account_idx" ON "invoice" USING btree ("merchant_account");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_status_idx" ON "invoice" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_payment_status_idx" ON "invoice" USING btree ("payment_status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_context_idx" ON "invoice" USING btree ("context");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_payment_due_at_idx" ON "invoice" USING btree ("payment_due_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_customer_status_idx" ON "invoice" USING btree ("customer","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "invoices_merchant_status_idx" ON "invoice" USING btree ("merchant","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "merchant_accounts_person_idx" ON "merchant_account" USING btree ("person");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "merchant_accounts_active_idx" ON "merchant_account" USING btree ("active");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_owner_id_idx" ON "booking_event" USING btree ("owner_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_context_id_idx" ON "booking_event" USING btree ("context_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_status_idx" ON "booking_event" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_active_idx" ON "booking_event" USING btree ("owner_id","status") WHERE "booking_event"."status" = 'active';--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_effective_dates_idx" ON "booking_event" USING btree ("effective_from","effective_to");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_search_idx" ON "booking_event" USING gin (to_tsvector('english', "title" || ' ' || COALESCE("description", '')));--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_keywords_idx" ON "booking_event" USING gin ("keywords");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "booking_events_tags_idx" ON "booking_event" USING gin ("tags");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_client_id_idx" ON "booking" USING btree ("client_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_provider_id_idx" ON "booking" USING btree ("provider_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_status_idx" ON "booking" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_scheduled_at_idx" ON "booking" USING btree ("scheduled_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_slot_id_idx" ON "booking" USING btree ("slot_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_client_status_idx" ON "booking" USING btree ("client_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_provider_status_idx" ON "booking" USING btree ("provider_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_provider_date_idx" ON "booking" USING btree ("provider_id","scheduled_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "bookings_pending_idx" ON "booking" USING btree ("status","booked_at") WHERE "booking"."status" = 'pending';--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "schedule_exceptions_event_id_idx" ON "schedule_exception" USING btree ("event_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "schedule_exceptions_owner_id_idx" ON "schedule_exception" USING btree ("owner_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "schedule_exceptions_context_id_idx" ON "schedule_exception" USING btree ("context_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "schedule_exceptions_date_range_idx" ON "schedule_exception" USING btree ("start_datetime","end_datetime");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "schedule_exceptions_owner_date_range_idx" ON "schedule_exception" USING btree ("owner_id","start_datetime","end_datetime");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "time_slots_owner_start_time_idx" ON "time_slot" USING btree ("owner_id","start_time");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "time_slots_status_idx" ON "time_slot" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "time_slots_bookable_idx" ON "time_slot" USING btree ("owner_id","start_time") WHERE "time_slot"."status" = 'available';--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "time_slots_event_id_idx" ON "time_slot" USING btree ("event_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "time_slots_context_id_idx" ON "time_slot" USING btree ("context_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "time_slots_booking_id_idx" ON "time_slot" USING btree ("booking_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_chat_room_idx" ON "chat_message" USING btree ("chat_room_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_sender_idx" ON "chat_message" USING btree ("sender_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_timestamp_idx" ON "chat_message" USING btree ("timestamp");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_type_idx" ON "chat_message" USING btree ("message_type");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_room_timestamp_idx" ON "chat_message" USING btree ("chat_room_id","timestamp");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_room_type_idx" ON "chat_message" USING btree ("chat_room_id","message_type");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_messages_sender_timestamp_idx" ON "chat_message" USING btree ("sender_id","timestamp");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_participants_idx" ON "chat_room" USING gin ("participants");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_admins_idx" ON "chat_room" USING gin ("admins");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_context_idx" ON "chat_room" USING btree ("context_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_status_idx" ON "chat_room" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_last_message_at_idx" ON "chat_room" USING btree ("last_message_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_active_video_call_idx" ON "chat_room" USING btree ("active_video_call_message_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "chat_rooms_status_last_message_idx" ON "chat_room" USING btree ("status","last_message_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_status_idx" ON "email_queue" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_priority_idx" ON "email_queue" USING btree ("priority");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_scheduled_idx" ON "email_queue" USING btree ("scheduled_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_recipient_idx" ON "email_queue" USING btree ("recipient_email");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_template_idx" ON "email_queue" USING btree ("template");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_template_tags_idx" ON "email_queue" USING gin ("template_tags");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_queue_processing_idx" ON "email_queue" USING btree ("status","priority","scheduled_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_template_status_idx" ON "email_template" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "email_template_tags_idx" ON "email_template" USING gin ("tags");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "notifications_recipient_status_idx" ON "notification" USING btree ("recipient_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "notifications_scheduled_status_idx" ON "notification" USING btree ("scheduled_at","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "notifications_type_channel_idx" ON "notification" USING btree ("type","channel");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "notifications_created_at_idx" ON "notification" USING btree ("created_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "persons_name_idx" ON "person" USING btree ("first_name","last_name");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reviews_context_idx" ON "review" USING btree ("context_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reviews_reviewer_idx" ON "review" USING btree ("reviewer_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reviews_review_type_idx" ON "review" USING btree ("review_type");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "reviews_reviewed_entity_idx" ON "review" USING btree ("reviewed_entity_id");