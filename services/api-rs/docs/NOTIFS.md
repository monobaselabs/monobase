# Notification System Documentation

## Overview

The Monobase Application Platform implements a comprehensive multi-channel notification system that ensures patients, providers, and administrators stay informed throughout all healthcare workflows. The system supports in-app, email, and SMS delivery with priority-based routing and non-blocking error handling.

## Architecture

### Core Components

- **NotificationService** (`@/core/notifs`): Central service for creating and managing notifications
- **Multi-channel delivery**: Supports in-app, email, and SMS channels
- **Priority system**: High, normal priority levels for different notification types
- **Non-blocking design**: Notification failures don't interrupt primary workflows
- **Comprehensive logging**: Full audit trail for all notification activities

### Integration Points

The notification system is integrated across all major healthcare workflows:
- Appointment booking lifecycle
- Payment processing workflows
- Provider confirmation processes
- Automated background jobs

## Notification Types

### 1. Appointment Workflow Notifications

#### 1.1 Appointment Confirmed (`appointment_confirmed`)
**Trigger**: Provider confirms a pending appointment  
**Recipients**: Patient  
**Channels**: in-app, email, SMS  
**Priority**: high  
**Location**: `confirmAppointment.ts:98-111`

**Data Payload**:
```typescript
{
  appointmentId: string;
  providerId: string;
  confirmedAt: string; // ISO timestamp
  scheduledAt: string; // ISO timestamp
}
```

**Message**: "Your appointment has been confirmed by the provider."

#### 1.2 Appointment Confirmation Sent (`appointment_confirmation_sent`)
**Trigger**: Provider confirms a pending appointment  
**Recipients**: Provider  
**Channels**: in-app  
**Priority**: normal  
**Location**: `confirmAppointment.ts:114-126`

**Data Payload**:
```typescript
{
  appointmentId: string;
  clientId: string;
  confirmedAt: string; // ISO timestamp
}
```

**Message**: "Appointment confirmation has been sent to the patient."

#### 1.3 Appointment Cancelled by You (`appointment_cancelled_by_you`)
**Trigger**: User cancels their own appointment  
**Recipients**: Cancelling user (patient or provider)  
**Channels**: in-app  
**Priority**: normal  
**Location**: `cancelAppointment.ts:123-137`

**Data Payload**:
```typescript
{
  appointmentId: string;
  reason: string;
  cancelledAt: string; // ISO timestamp
  scheduledAt: string; // ISO timestamp
  cancelledBy: 'client' | 'provider';
}
```

**Message**: "You have successfully cancelled your appointment. The [other party] has been notified."

#### 1.4 Appointment Cancelled by Other (`appointment_cancelled_by_other`)
**Trigger**: Other party cancels the appointment  
**Recipients**: Non-cancelling party  
**Channels**: in-app, email, SMS  
**Priority**: high  
**Location**: `cancelAppointment.ts:140-155`

**Data Payload**:
```typescript
{
  appointmentId: string;
  reason: string;
  cancelledAt: string; // ISO timestamp
  scheduledAt: string; // ISO timestamp
  cancelledBy: 'client' | 'provider';
  cancelledById: string;
}
```

**Message**: "Your appointment has been cancelled by the [canceller]. Reason: [reason]"

#### 1.5 Appointment Rejected (`appointment_rejected`)
**Trigger**: Provider rejects a pending appointment  
**Recipients**: Patient  
**Channels**: in-app, email, SMS  
**Priority**: high  
**Location**: `rejectAppointment.ts:121-136`

**Data Payload**:
```typescript
{
  appointmentId: string;
  providerId: string;
  rejectedAt: string; // ISO timestamp
  scheduledAt: string; // ISO timestamp
  reason: string;
  slotReleased: string; // slot ID
}
```

**Message**: "Your appointment request has been rejected by the provider. Reason: [reason]"

#### 1.6 Appointment Rejection Sent (`appointment_rejection_sent`)
**Trigger**: Provider rejects a pending appointment  
**Recipients**: Provider  
**Channels**: in-app  
**Priority**: normal  
**Location**: `rejectAppointment.ts:139-153`

**Data Payload**:
```typescript
{
  appointmentId: string;
  clientId: string;
  rejectedAt: string; // ISO timestamp
  reason: string;
  slotReleased: string; // slot ID
}
```

**Message**: "Appointment rejection has been sent to the patient. The time slot is now available."

#### 1.7 Appointment Auto-Rejected (`appointment_auto_rejected`)
**Trigger**: Confirmation timer job - provider doesn't confirm within 15 minutes  
**Recipients**: Patient  
**Channels**: in-app, email, SMS  
**Priority**: high  
**Location**: `confirmationTimer.ts:180-194`

**Data Payload**:
```typescript
{
  appointmentId: string;
  providerId: string;
  scheduledAt: string; // ISO timestamp
  autoRejectedAt: string; // ISO timestamp
  reason: 'Provider did not confirm within 15 minutes';
}
```

**Message**: "Your appointment request has expired as the provider did not confirm within 15 minutes."

#### 1.8 Appointment Expired (`appointment_expired`)
**Trigger**: Confirmation timer job - provider doesn't confirm within 15 minutes  
**Recipients**: Provider  
**Channels**: in-app, email  
**Priority**: normal  
**Location**: `confirmationTimer.ts:197-211`

**Data Payload**:
```typescript
{
  appointmentId: string;
  clientId: string;
  scheduledAt: string; // ISO timestamp
  autoRejectedAt: string; // ISO timestamp
  missedDeadline: true;
}
```

**Message**: "An appointment request has expired due to no confirmation within the time limit."

### 2. Payment Workflow Notifications

#### 2.1 Payment Authorized (`payment_authorized`)
**Trigger**: Stripe webhook - payment_intent.succeeded event  
**Recipients**: Patient  
**Channels**: in-app, email  
**Priority**: normal  
**Location**: `stripeWebhook.ts:177-191`

**Data Payload**:
```typescript
{
  appointmentId: string;
  paymentIntentId: string;
  amount: number; // in cents
  currency: string;
  status: 'authorized';
}
```

**Message**: "Your payment has been authorized and is being held until the appointment is completed."

#### 2.2 Payment Captured (`payment_captured`)
**Trigger**: Stripe webhook - charge.succeeded event  
**Recipients**: Patient  
**Channels**: in-app, email  
**Priority**: normal  
**Location**: `stripeWebhook.ts:333-348`

**Data Payload**:
```typescript
{
  appointmentId: string;
  chargeId: string;
  amount: number; // in cents
  currency: string;
  status: 'captured';
  capturedAt: string; // ISO timestamp
}
```

**Message**: "Your payment has been successfully processed and captured."

#### 2.3 Payment Received (`payment_received`)
**Trigger**: Stripe webhook - charge.succeeded event  
**Recipients**: Provider  
**Channels**: in-app, email  
**Priority**: normal  
**Location**: `stripeWebhook.ts:351-366`

**Data Payload**:
```typescript
{
  appointmentId: string;
  chargeId: string;
  transferId: string;
  amount: number; // in cents
  currency: string;
  clientId: string;
}
```

**Message**: "Payment for your appointment has been processed and will be transferred to your account."

#### 2.4 Payment Failed (`payment_failed`)
**Trigger**: Stripe webhook - payment_intent.payment_failed event  
**Recipients**: Patient  
**Channels**: in-app, email, SMS  
**Priority**: high  
**Location**: `stripeWebhook.ts:239-253`

**Data Payload**:
```typescript
{
  appointmentId: string;
  paymentIntentId: string;
  failureReason: string;
  status: 'failed';
  failedAt: string; // ISO timestamp
}
```

**Message**: "Your payment could not be processed. Please update your payment method and try again."

#### 2.5 Charge Failed (`charge_failed`)
**Trigger**: Stripe webhook - charge.failed event  
**Recipients**: Patient  
**Channels**: in-app, email  
**Priority**: high  
**Location**: `stripeWebhook.ts:456-472`

**Data Payload**:
```typescript
{
  appointmentId: string;
  chargeId: string;
  paymentIntentId: string;
  failureCode: string;
  failureMessage: string;
  status: 'failed';
  failedAt: string; // ISO timestamp
}
```

**Message**: "There was an issue processing your payment charge. Please contact support if this continues."

## Implementation Patterns

### 1. Service Injection

All handlers that create notifications follow this pattern:

```typescript
// Import notification service type
import type { NotificationService } from '@/core/notifs';

// Get service from context
const notificationService = ctx.get('notificationService') as NotificationService;
```

### 2. Notification Creation

Standard notification creation pattern:

```typescript
try {
  await notificationService.createNotification({
    recipientId: userId,
    type: 'notification_type',
    title: 'Human readable title',
    message: 'Detailed message for user',
    data: {
      // Relevant data payload
    },
    channels: ['in-app', 'email', 'sms'],
    priority: 'high' | 'normal'
  });

  logger?.info({ /* context */ }, 'Notification sent successfully');
} catch (error) {
  // Non-blocking error handling
  logger?.error({ error }, 'Failed to send notification');
}
```

### 3. Error Handling

All notification implementations use non-blocking error handling:
- Notification failures don't interrupt primary workflows
- Comprehensive error logging for troubleshooting
- Graceful degradation when notification service is unavailable

### 4. Audit Logging

Each notification integration includes detailed audit logging:
- Success and failure events
- Recipient information
- Context data (appointment IDs, payment IDs)
- Timestamps and user actions

## Background Jobs Integration

### Confirmation Timer Job

**Location**: `confirmationTimer.ts`  
**Schedule**: Every minute  
**Extended Context**: Includes `NotificationService` via job wrapper

The confirmation timer job required special integration:
1. Extended `JobContext` interface to include `NotificationService`
2. Modified `registerBookingJobs` to accept and inject notification service
3. Updated job registration in `app.ts` to pass notification service

```typescript
// Job wrapper that injects notification service
scheduler.registerInterval('booking.confirmationTimer', 60000, async (context) => {
  const extendedContext = {
    ...context,
    notificationService
  };
  await confirmationTimerJob(extendedContext as any);
});
```

## Configuration and Channels

### Channel Priority

- **High Priority Notifications**: in-app + email + SMS
  - Payment failures
  - Appointment rejections
  - Auto-rejections
  - Cancellations (to affected party)

- **Normal Priority Notifications**: in-app + email
  - Payment confirmations
  - Provider acknowledgments

- **Internal Notifications**: in-app only
  - Provider confirmations
  - Administrative notifications

### Message Personalization

All notifications include:
- Dynamic user names and roles
- Contextual information (appointment times, reasons)
- Clear action items when applicable
- Professional healthcare-appropriate language

## Troubleshooting

### Common Issues

1. **Notification Service Not Available**
   - Check service injection in handler
   - Verify `notificationService` is available in context
   - Review application startup logs

2. **Background Job Notifications Failing**
   - Ensure notification service is passed to job registration
   - Check extended job context implementation
   - Verify job wrapper includes notification service

3. **Missing Notifications**
   - Check error logs for notification creation failures
   - Verify recipient IDs are valid
   - Confirm notification channels are configured

### Debugging

Enable detailed logging for notification troubleshooting:

```typescript
logger?.debug({
  recipientId,
  notificationType,
  channels,
  priority,
  data
}, 'Creating notification');
```

## Future Enhancements

Planned improvements to the notification system:

1. **Template System**: Configurable notification templates
2. **User Preferences**: Per-user channel preferences
3. **Delivery Confirmation**: Read receipts and delivery status
4. **Batch Notifications**: Efficient bulk notification sending
5. **Real-time Delivery**: WebSocket integration for instant in-app notifications
6. **Advanced Scheduling**: Timezone-aware notification scheduling