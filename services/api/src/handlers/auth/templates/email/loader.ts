/**
 * Auth Module Email Template Loader
 * 
 * Registers authentication-related email templates:
 * - Email verification
 * - Password reset
 * - Two-factor authentication
 * - Welcome email
 */

import type { DatabaseInstance } from '@/core/database';
import type { Logger } from '@/types/logger';
import { registerTemplates, type TemplateDefinition } from '@/utils/email';
import { EmailTemplateTags } from '@/handlers/email/repos/email.schema';

// Static template imports - bundler will include these in the bundle
import emailVerifyHtml from './email-verify.html.hbs' with { type: 'text' };
import emailVerifyText from './email-verify.text.hbs' with { type: 'text' };
import passwordResetHtml from './password-reset.html.hbs' with { type: 'text' };
import passwordResetText from './password-reset.text.hbs' with { type: 'text' };
import twoFactorHtml from './2fa.html.hbs' with { type: 'text' };
import twoFactorText from './2fa.text.hbs' with { type: 'text' };
import welcomeHtml from './welcome.html.hbs' with { type: 'text' };
import welcomeText from './welcome.text.hbs' with { type: 'text' };

/**
 * Auth module template definitions
 */
const AUTH_TEMPLATES: TemplateDefinition[] = [
  {
    metadata: {
      name: 'Email Verification',
      description: 'Email verification template for new user registration',
      subject: 'Verify your email address',
      tags: [EmailTemplateTags.AUTH_EMAIL_VERIFY],
      variables: [
        {
          id: 'name',
          type: 'string' as const,
          label: 'Recipient Name',
          required: true
        },
        {
          id: 'email',
          type: 'email' as const,
          label: 'Email Address',
          required: true
        },
        {
          id: 'verificationLink',
          type: 'url' as const,
          label: 'Verification Link',
          required: true
        }
      ]
    },
    content: {
      html: emailVerifyHtml,
      text: emailVerifyText
    }
  },

  {
    metadata: {
      name: 'Password Reset',
      description: 'Password reset template for forgot password flow',
      subject: 'Reset your password',
      tags: [EmailTemplateTags.AUTH_PASSWORD_RESET],
      variables: [
        {
          id: 'name',
          type: 'string' as const,
          label: 'Recipient Name',
          required: true
        },
        {
          id: 'email',
          type: 'email' as const,
          label: 'Email Address',
          required: true
        },
        {
          id: 'resetLink',
          type: 'url' as const,
          label: 'Password Reset Link',
          required: true
        },
        {
          id: 'expirationTime',
          type: 'number' as const,
          label: 'Link Expiration Time (minutes)',
          required: true,
          defaultValue: 15
        }
      ]
    },
    content: {
      html: passwordResetHtml,
      text: passwordResetText
    }
  },

  {
    metadata: {
      name: 'Two-Factor Authentication',
      description: '2FA verification code template',
      subject: 'Your verification code',
      tags: [EmailTemplateTags.AUTH_2FA],
      variables: [
        {
          id: 'name',
          type: 'string' as const,
          label: 'Recipient Name',
          required: true
        },
        {
          id: 'email',
          type: 'email' as const,
          label: 'Email Address',
          required: true
        },
        {
          id: 'code',
          type: 'string' as const,
          label: 'Verification Code',
          required: true,
          minLength: 4,
          maxLength: 8
        },
        {
          id: 'expirationTime',
          type: 'number' as const,
          label: 'Code Expiration Time (minutes)',
          required: true,
          defaultValue: 5
        }
      ]
    },
    content: {
      html: twoFactorHtml,
      text: twoFactorText
    }
  },

  {
    metadata: {
      name: 'Welcome Email',
      description: 'Welcome email for new users after successful registration',
      subject: 'Welcome to Monobase!',
      tags: [EmailTemplateTags.AUTH_WELCOME],
      variables: [
        {
          id: 'name',
          type: 'string' as const,
          label: 'Recipient Name',
          required: true
        },
        {
          id: 'email',
          type: 'email' as const,
          label: 'Email Address',
          required: true
        },
        {
          id: 'dashboardLink',
          type: 'url' as const,
          label: 'Dashboard Link',
          required: true
        }
      ]
    },
    content: {
      html: welcomeHtml,
      text: welcomeText
    }
  }
];

/**
 * Register auth module email templates
 * 
 * @param db - Database instance
 * @param logger - Optional logger for debugging
 */
export async function registerAuthTemplates(
  db: DatabaseInstance,
  logger?: Logger
): Promise<void> {
  logger?.debug('Registering auth module email templates');
  await registerTemplates(db, AUTH_TEMPLATES, logger);
  logger?.debug('Auth module email templates registered');
}
