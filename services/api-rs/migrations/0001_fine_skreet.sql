CREATE TYPE "public"."consultation_status" AS ENUM('draft', 'finalized', 'amended');--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "consultation_note" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"patient_id" uuid NOT NULL,
	"provider_id" uuid NOT NULL,
	"context" varchar(255),
	"chief_complaint" text,
	"assessment" text,
	"plan" text,
	"vitals" jsonb,
	"symptoms" jsonb,
	"prescriptions" jsonb,
	"follow_up" jsonb,
	"external_documentation" jsonb,
	"status" "consultation_status" DEFAULT 'draft' NOT NULL,
	"finalized_at" timestamp,
	"finalized_by" uuid,
	CONSTRAINT "consultation_notes_context_unique" UNIQUE("context"),
	CONSTRAINT "consultation_notes_chief_complaint_length_check" CHECK ("consultation_note"."chief_complaint" IS NULL OR (LENGTH("consultation_note"."chief_complaint") >= 1 AND LENGTH("consultation_note"."chief_complaint") <= 500)),
	CONSTRAINT "consultation_notes_assessment_length_check" CHECK ("consultation_note"."assessment" IS NULL OR (LENGTH("consultation_note"."assessment") >= 1 AND LENGTH("consultation_note"."assessment") <= 2000)),
	CONSTRAINT "consultation_notes_plan_length_check" CHECK ("consultation_note"."plan" IS NULL OR (LENGTH("consultation_note"."plan") >= 1 AND LENGTH("consultation_note"."plan") <= 2000)),
	CONSTRAINT "consultation_notes_finalized_at_constraint" CHECK (("consultation_note"."status" = 'finalized' AND "consultation_note"."finalized_at" IS NOT NULL AND "consultation_note"."finalized_by" IS NOT NULL) OR "consultation_note"."status" != 'finalized')
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "patient" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"person_id" uuid NOT NULL,
	"primary_provider" jsonb,
	"primary_pharmacy" jsonb
);
--> statement-breakpoint
CREATE TABLE IF NOT EXISTS "provider" (
	"id" uuid PRIMARY KEY DEFAULT gen_random_uuid() NOT NULL,
	"created_at" timestamp DEFAULT now() NOT NULL,
	"updated_at" timestamp DEFAULT now() NOT NULL,
	"version" integer DEFAULT 1 NOT NULL,
	"created_by" uuid,
	"updated_by" uuid,
	"person_id" uuid NOT NULL,
	"provider_type" varchar(50) NOT NULL,
	"years_of_experience" integer,
	"biography" text,
	"minor_ailments_specialties" jsonb,
	"minor_ailments_practice_locations" jsonb,
	CONSTRAINT "providers_person_id_unique" UNIQUE("person_id")
);
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "consultation_note" ADD CONSTRAINT "consultation_note_patient_id_patient_id_fk" FOREIGN KEY ("patient_id") REFERENCES "public"."patient"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "consultation_note" ADD CONSTRAINT "consultation_note_provider_id_provider_id_fk" FOREIGN KEY ("provider_id") REFERENCES "public"."provider"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "patient" ADD CONSTRAINT "patient_person_id_person_id_fk" FOREIGN KEY ("person_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
DO $$ BEGIN
 ALTER TABLE "provider" ADD CONSTRAINT "provider_person_id_person_id_fk" FOREIGN KEY ("person_id") REFERENCES "public"."person"("id") ON DELETE cascade ON UPDATE no action;
EXCEPTION
 WHEN duplicate_object THEN null;
END $$;
--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_patient_id_idx" ON "consultation_note" USING btree ("patient_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_provider_id_idx" ON "consultation_note" USING btree ("provider_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_status_idx" ON "consultation_note" USING btree ("status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_finalized_at_idx" ON "consultation_note" USING btree ("finalized_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_finalized_by_idx" ON "consultation_note" USING btree ("finalized_by");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_context_idx" ON "consultation_note" USING btree ("context");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_patient_status_idx" ON "consultation_note" USING btree ("patient_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_provider_status_idx" ON "consultation_note" USING btree ("provider_id","status");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_patient_created_at_idx" ON "consultation_note" USING btree ("patient_id","created_at");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_finalized_idx" ON "consultation_note" USING btree ("patient_id","finalized_at") WHERE "consultation_note"."status" = 'finalized';--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "consultation_notes_draft_idx" ON "consultation_note" USING btree ("provider_id","created_at") WHERE "consultation_note"."status" = 'draft';--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "patients_person_id_idx" ON "patient" USING btree ("person_id");--> statement-breakpoint
CREATE UNIQUE INDEX IF NOT EXISTS "patients_person_id_unique" ON "patient" USING btree ("person_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "providers_person_id_idx" ON "provider" USING btree ("person_id");--> statement-breakpoint
CREATE INDEX IF NOT EXISTS "providers_provider_type_idx" ON "provider" USING btree ("provider_type");