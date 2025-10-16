/**
 * Email Service Interface
 * Provides a thin abstraction layer over email functionality for module integration
 * This service is injected into the app context for use by other modules
 */

import type { DatabaseInstance } from '@/core/database';
import type { Logger } from '@/types/logger';
import type { Config } from '@/core/config';
import { EmailTemplateRepository } from '@/handlers/email/repos/template.repo';
import { EmailQueueRepository } from '@/handlers/email/repos/queue.repo';
import {
  EmailTemplateTags,
  type QueueEmailRequest,
  type EmailQueueItem,
  type EmailTemplate,
  type SendEmailRequest,
  type EmailSendResult,
  type TemplatePreviewResult
} from '@/handlers/email/repos/email.schema';
import nodemailer from 'nodemailer';
import type { Transporter } from 'nodemailer';
import postmark from 'postmark';
import * as OneSignal from '@onesignal/node-onesignal';

/**
 * Email configuration
 */
export interface EmailConfig {
  provider: 'smtp' | 'postmark' | 'onesignal';
  from: {
    name: string;
    email: string;
  };

  // SMTP configuration
  smtp?: {
    host: string;
    port: number;
    secure: boolean;
    auth: {
      user: string;
      pass: string;
    };
  };

  // Postmark configuration
  postmark?: {
    apiKey: string;
    messageStream?: string;
  };

  // OneSignal configuration
  onesignal?: {
    appId: string;
    apiKey: string;
  };
}

/**
 * Email provider interface
 */
interface EmailProvider {
  send(request: SendEmailRequest): Promise<EmailSendResult>;
}

/**
 * SMTP provider implementation
 */
class SMTPProvider implements EmailProvider {
  private transporter: Transporter | null = null;
  private config: EmailConfig['smtp'];

  constructor(config: EmailConfig['smtp']) {
    this.config = config;
  }
  
  /**
   * Lazy initialization of SMTP transporter - only creates instance when first needed
   */
  private ensureInitialized(): Transporter {
    if (this.transporter) {
      return this.transporter;
    }
    
    if (!this.config) {
      throw new Error(
        'SMTP configuration is required for email operations. Please set SMTP_HOST, SMTP_USER, and SMTP_PASS environment variables.'
      );
    }
    
    this.transporter = nodemailer.createTransport({
      host: this.config.host,
      port: this.config.port,
      secure: this.config.secure,
      auth: {
        user: this.config.auth.user,
        pass: this.config.auth.pass
      }
    });
    
    return this.transporter;
  }
  
  async send(request: SendEmailRequest): Promise<EmailSendResult> {
    try {
      const transporter = this.ensureInitialized();
      const result = await transporter.sendMail({
        from: request.from ? { 
          name: request.from.name || '', 
          address: request.from.email || '' 
        } : undefined,
        to: request.to,
        subject: request.subject,
        html: request.html,
        text: request.text,
        replyTo: request.replyTo?.email
      });
      
      return {
        success: true,
        messageId: result?.messageId || 'unknown',
        provider: 'smtp'
      };
    } catch (error) {
      return {
        success: false,
        provider: 'smtp',
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }
}

/**
 * Postmark provider implementation
 */
class PostmarkProvider implements EmailProvider {
  private client: postmark.ServerClient | null = null;
  private config: EmailConfig;

  constructor(config: EmailConfig) {
    this.config = config;
  }
  
  /**
   * Lazy initialization of Postmark client - only creates instance when first needed
   */
  private ensureInitialized(): postmark.ServerClient {
    if (this.client) {
      return this.client;
    }
    
    if (!this.config.postmark) {
      throw new Error(
        'Postmark configuration is required for email operations. Please set POSTMARK_API_KEY environment variable.'
      );
    }
    
    this.client = new postmark.ServerClient(this.config.postmark.apiKey);
    
    return this.client;
  }
  
  async send(request: SendEmailRequest): Promise<EmailSendResult> {
    try {
      const client = this.ensureInitialized();
      const fromEmail = request.from?.email || this.config.from.email;
      const fromName = request.from?.name || this.config.from.name;
      const messageStream = this.config.postmark?.messageStream || 'outbound';
      
      const result = await client.sendEmail({
        From: `${fromName} <${fromEmail}>`,
        To: request.to,
        Subject: request.subject,
        HtmlBody: request.html,
        TextBody: request.text,
        ReplyTo: request.replyTo?.email || undefined,
        MessageStream: messageStream
      });
      
      return {
        success: true,
        messageId: result.MessageID,
        provider: 'postmark'
      };
    } catch (error) {
      return {
        success: false,
        provider: 'postmark',
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }
}

/**
 * OneSignal provider implementation
 */
class OneSignalProvider implements EmailProvider {
  private client: OneSignal.DefaultApi | null = null;
  private config: EmailConfig;
  
  constructor(config: EmailConfig) {
    this.config = config;
  }
  
  /**
   * Lazy initialization of OneSignal client - only creates instance when first needed
   */
  private ensureInitialized(): OneSignal.DefaultApi {
    if (this.client) {
      return this.client;
    }
    
    if (!this.config.onesignal) {
      throw new Error(
        'OneSignal configuration is required for email operations. Please set ONESIGNAL_APP_ID and ONESIGNAL_API_KEY environment variables.'
      );
    }
    
    const configuration = OneSignal.createConfiguration({
      restApiKey: this.config.onesignal.apiKey,
    });
    
    this.client = new OneSignal.DefaultApi(configuration);
    
    return this.client;
  }
  
  async send(request: SendEmailRequest): Promise<EmailSendResult> {
    try {
      const client = this.ensureInitialized();
      const notification = new OneSignal.Notification();
      
      notification.app_id = this.config.onesignal!.appId;
      notification.include_email_tokens = [request.to];
      notification.email_subject = request.subject;
      notification.email_body = request.html;
      notification.include_unsubscribed = true; // Required for transactional emails
      
      // Set from email if provided
      if (request.from) {
        notification.email_from_name = request.from.name;
        notification.email_from_address = request.from.email;
      }
      
      const result = await client.createNotification(notification);
      
      return {
        success: true,
        messageId: result.id || 'unknown',
        provider: 'onesignal'
      };
    } catch (error) {
      return {
        success: false,
        provider: 'onesignal',
        error: error instanceof Error ? error.message : 'Unknown error'
      };
    }
  }
}


/**
 * Email service interface
 */
export interface EmailService {
  /**
   * Queue an email for sending (modern templateId-based)
   */
  queueEmail(request: QueueEmailRequest): Promise<EmailQueueItem>;
  
  /**
   * Send an email immediately (used by job processor)
   */
  sendEmail(request: SendEmailRequest): Promise<EmailSendResult>;
  
  /**
   * Preview a template with variables
   */
  previewTemplate(templateId: string, variables?: Record<string, any>): Promise<TemplatePreviewResult>;
  
  /**
   * Render a template (used by job processor)
   */
  renderTemplate(templateId: string, variables: Record<string, any>): Promise<TemplatePreviewResult>;
  
  /**
   * Process pending emails (called by job)
   */
  processPendingEmails(): Promise<void>;
  
}

/**
 * EmailService implementation
 */
class EmailServiceImpl implements EmailService {
  private templateRepo: EmailTemplateRepository;
  private queueRepo: EmailQueueRepository;
  private provider: EmailProvider | null = null;
  private config: Config['email'];
  private fullConfig: Config;
  private db: DatabaseInstance;
  private logger: Logger;
  
  constructor(
    db: DatabaseInstance,
    config: Config,
    logger: Logger
  ) {
    this.db = db;
    this.logger = logger;
    this.templateRepo = new EmailTemplateRepository(db, logger);
    this.queueRepo = new EmailQueueRepository(db, logger);
    this.config = config.email;
    this.fullConfig = config;
    
    // Bind methods to maintain context
    this.queueEmail = this.queueEmail.bind(this);
    this.sendEmail = this.sendEmail.bind(this);
    this.previewTemplate = this.previewTemplate.bind(this);
    this.renderTemplate = this.renderTemplate.bind(this);
    this.processPendingEmails = this.processPendingEmails.bind(this);
  }
  
  /**
   * Lazy initialization of email provider - only creates instance when first needed
   */
  private ensureProviderInitialized(): EmailProvider {
    if (this.provider) {
      return this.provider;
    }
    
    // Initialize provider based on configuration
    if (this.config.provider === 'postmark') {
      this.provider = new PostmarkProvider(this.config);
    } else if (this.config.provider === 'onesignal') {
      this.provider = new OneSignalProvider(this.config);
    } else {
      this.provider = new SMTPProvider(this.config.smtp);
    }
    
    return this.provider;
  }
  
  
  /**
   * Send an email immediately
   */
  async sendEmail(request: SendEmailRequest): Promise<EmailSendResult> {
    const provider = this.ensureProviderInitialized();
    
    // Add default from if not provided
    if (!request.from) {
      request.from = {
        name: this.config.from.name,
        email: this.config.from.email
      };
    }
    
    return provider.send(request);
  }
  
  /**
   * Queue an email for sending with template tags
   */
  async queueEmail(request: QueueEmailRequest): Promise<EmailQueueItem> {
    // Validate that template tags are provided
    if (!request.templateTags || request.templateTags.length === 0) {
      throw new Error('Template tags are required');
    }
    
    // Queue the email - template resolution and validation will happen during processing
    return this.queueRepo.queueEmail(request);
  }
  
  /**
   * Preview a template with variables
   */
  async previewTemplate(templateId: string, variables?: Record<string, any>): Promise<TemplatePreviewResult> {
    return this.templateRepo.previewTemplate(templateId, variables);
  }
  
  /**
   * Render a template with variables
   */
  async renderTemplate(templateId: string, variables: Record<string, any>): Promise<TemplatePreviewResult> {
    return this.templateRepo.renderTemplate(templateId, variables);
  }
  
  /**
   * Process pending emails from the queue
   */
  async processPendingEmails(): Promise<void> {
    const pendingEmails = await this.queueRepo.getPendingEmails(50);
    
    for (const email of pendingEmails) {
      await this.processEmail(email);
    }
  }
  
  /**
   * Process a single email
   */
  private async processEmail(email: EmailQueueItem): Promise<void> {
    try {
      // Mark as processing
      await this.queueRepo.markAsProcessing(email.id);
      
      // Resolve template by tags
      const template = await this.resolveTemplateByTags(email.templateTags || []);
      if (!template) {
        throw new Error(`No active template found for tags: ${email.templateTags?.join(', ') || 'none'}`);
      }
      
      // Render template with variables
      const rendered = await this.templateRepo.renderTemplate(
        template.id,
        email.variables
      );
      
      // Send email
      const result = await this.sendEmail({
        to: email.recipientEmail,
        subject: rendered.subject,
        html: rendered.bodyHtml,
        text: rendered.bodyText,
        from: template?.fromEmail ? {
          name: template.fromName || this.config.from.name,
          email: template.fromEmail
        } : undefined,
        replyTo: template?.replyToEmail ? {
          email: template.replyToEmail,
          name: template.replyToName || undefined
        } : undefined
      });
      
      if (result.success) {
        // Mark as sent
        await this.queueRepo.markAsSent(
          email.id,
          result.provider,
          result.messageId!
        );
      } else {
        // Mark as failed
        await this.queueRepo.markAsFailed(
          email.id,
          result.error || 'Unknown error',
          email.attempts
        );
      }
    } catch (error) {
      this.logger?.error({ error, emailId: email.id }, 'Failed to process email');
      // Mark as failed
      await this.queueRepo.markAsFailed(
        email.id,
        error instanceof Error ? error.message : 'Unknown error',
        email.attempts
      );
    }
  }
  
  
  /**
   * Resolve template by tags
   */
  private async resolveTemplateByTags(tags: string[]): Promise<EmailTemplate | null> {
    // Find template by checking if any of the tags match
    const templates = await this.templateRepo.findMany({
      status: 'active'
    });
    
    // Find template where its tags array contains any of the provided tags
    const template = templates.find(t =>
      t.tags && t.tags.some(tag => tags.includes(tag))
    );
    
    return template || null;
  }
}

/**
 * Create an email service instance
 * Factory function following the pattern of other services
 */
export function createEmailService(
  db: DatabaseInstance,
  config: Config,
  logger: Logger
): EmailService {
  return new EmailServiceImpl(db, config, logger);
}