/**
 * Email Template Registration Utilities
 * 
 * Provides utilities for modules to register their email templates
 * in a decentralized, modular way.
 */

import type { DatabaseInstance } from '@/core/database';
import type { Logger } from '@/types/logger';
import type { NewEmailTemplate, TemplateVariable } from '@/handlers/email/repos/email.schema';
import { EmailTemplateRepository } from '@/handlers/email/repos/template.repo';

/**
 * Template metadata for module-based templates
 */
export interface TemplateMetadata {
  name: string;
  description: string;
  subject: string;
  tags: string[];
  variables: TemplateVariable[];
  fromName?: string;
  fromEmail?: string;
  replyToEmail?: string;
  replyToName?: string;
}

/**
 * Template definition with content
 */
export interface TemplateDefinition {
  metadata: TemplateMetadata;
  content: {
    html: string;
    text?: string;
  };
}

/**
 * Register email templates for a module
 * 
 * @param db - Database instance
 * @param templates - Array of template definitions to register
 * @param logger - Optional logger for debugging
 */
export async function registerTemplates(
  db: DatabaseInstance,
  templates: TemplateDefinition[],
  logger?: Logger
): Promise<void> {
  const templateRepo = new EmailTemplateRepository(db, logger);
  
  for (const { metadata, content } of templates) {
    try {
      // Check if template with same tags already exists
      const existing = await templateRepo.findMany(
        { tags: metadata.tags },
        { pagination: { limit: 1, offset: 0 } }
      );
      
      if (existing.length > 0) {
        logger?.debug(
          { tags: metadata.tags, name: metadata.name }, 
          'Template already exists, skipping'
        );
        continue;
      }
      
      // Create template definition
      const templateDef: NewEmailTemplate = {
        name: metadata.name,
        description: metadata.description,
        subject: metadata.subject,
        bodyHtml: content.html,
        bodyText: content.text,
        tags: metadata.tags,
        variables: metadata.variables,
        fromName: metadata.fromName,
        fromEmail: metadata.fromEmail,
        replyToEmail: metadata.replyToEmail,
        replyToName: metadata.replyToName,
        status: 'active'
      };
      
      // Create new template
      const created = await templateRepo.createTemplate(templateDef);
      
      logger?.debug(
        { id: created.id, name: created.name, tags: created.tags },
        'Email template registered'
      );
      
    } catch (error) {
      logger?.error(
        { error, name: metadata.name, tags: metadata.tags },
        'Failed to register email template'
      );
    }
  }
}
