/**
 * Confirmation Timer Background Job
 * Auto-rejects pending bookings that haven't been confirmed within 15 minutes
 */

import type { JobContext } from '@/core/jobs';
import type { NotificationService } from '@/core/notifs';

// Extended job context that includes notification service
interface ExtendedJobContext extends JobContext {
  notificationService: NotificationService;
}
import { BookingRepository } from '../repos/booking.repo';
import { TimeSlotRepository } from '../repos/timeSlot.repo';
import { subMinutes, differenceInMinutes, addMinutes, differenceInMilliseconds } from 'date-fns';
import { eq, and, lte, isNull } from 'drizzle-orm';
import { bookings, timeSlots } from '../repos/booking.schema';

/**
 * Configuration for confirmation timer job
 */
export interface ConfirmationTimerConfig {
  confirmationWindowMinutes: number; // Time window for provider to confirm (default: 15)
  batchSize: number; // Number of bookings to process at once (default: 50)
  includeNotifications: boolean; // Whether to send notifications (default: true)
}

const DEFAULT_CONFIG: ConfirmationTimerConfig = {
  confirmationWindowMinutes: 15,
  batchSize: 50,
  includeNotifications: true
};

/**
 * Auto-rejection job for unconfirmed bookings
 * Runs every minute to check for expired pending bookings
 */
export async function confirmationTimerJob(context: ExtendedJobContext): Promise<void> {
  const { db, logger, jobId } = context;
  const config = { ...DEFAULT_CONFIG };
  
  logger.debug(`Starting confirmation timer job`, { jobId, config });
  
  const bookingRepo = new BookingRepository(db, logger);
  const timeSlotRepo = new TimeSlotRepository(db, logger);
  
  try {
    // Calculate the cutoff time for auto-rejection
    const cutoffTime = subMinutes(new Date(), config.confirmationWindowMinutes);
    
    logger.debug('Checking for expired pending bookings', {
      cutoffTime: cutoffTime.toISOString(),
      windowMinutes: config.confirmationWindowMinutes
    });
    
    // Find all pending bookings that have exceeded the confirmation window
    const expiredBookings = await db
      .select()
      .from(bookings)
      .where(
        and(
          eq(bookings.status, 'pending'),
          lte(bookings.bookedAt, cutoffTime),
          isNull(bookings.confirmationTimestamp)
        )
      )
      .limit(config.batchSize);
    
    if (expiredBookings.length === 0) {
      logger.debug('No expired pending bookings found');
      return;
    }
    
    logger.info(`Found ${expiredBookings.length} expired pending bookings`);
    
    const results = {
      rejected: 0,
      failed: 0,
      errors: [] as string[]
    };
    
    // Process each expired booking
    for (const booking of expiredBookings) {
      try {
        // Start a transaction to ensure consistency
        await db.transaction(async (tx) => {
          // Update booking status to rejected
          await tx
            .update(bookings)
            .set({
              status: 'rejected',
              cancellationReason: 'Auto-rejected: Provider did not confirm within 15 minutes',
              cancelledBy: 'system',
              cancelledAt: new Date(),
              updatedAt: new Date()
            })
            .where(
              and(
                eq(bookings.id, booking.id),
                eq(bookings.status, 'pending') // Double-check status hasn't changed
              )
            );
          
          // Release the associated time slot
          if (booking.slot) {
            await tx
              .update(timeSlots)
              .set({
                status: 'available',
                booking: null,
                updatedAt: new Date()
              })
              .where(eq(timeSlots.id, booking.slot));
          }
        });
        
        results.rejected++;
        
        logger.info('Auto-rejected booking', {
          bookingId: booking.id,
          clientId: booking.client,
          providerId: booking.provider,
          bookingTime: booking.bookedAt,
          minutesExpired: differenceInMinutes(new Date(), booking.bookedAt)
        });
        
        // Queue notification for client and provider
        if (config.includeNotifications) {
          await queueAutoRejectionNotifications(booking, context);
        }
        
      } catch (error) {
        results.failed++;
        const errorMsg = `Failed to auto-reject booking ${booking.id}: ${error}`;
        results.errors.push(errorMsg);
        
        logger.error('Failed to auto-reject booking', {
          bookingId: booking.id,
          error: error instanceof Error ? error.message : String(error)
        });
      }
    }
    
    // Log final results
    logger.info('Confirmation timer job completed', {
      jobId,
      totalProcessed: expiredBookings.length,
      rejected: results.rejected,
      failed: results.failed,
      successRate: `${(results.rejected / expiredBookings.length * 100).toFixed(2)}%`
    });
    
    if (results.failed > 0) {
      logger.warn('Some bookings failed to auto-reject', {
        failedCount: results.failed,
        errors: results.errors
      });
    }
    
  } catch (error) {
    logger.error('Confirmation timer job failed', {
      jobId,
      error: error instanceof Error ? error.message : String(error)
    });
    throw error;
  }
}

/**
 * Queue notifications for auto-rejected bookings
 */
async function queueAutoRejectionNotifications(
  booking: any,
  context: ExtendedJobContext
): Promise<void> {
  const { logger, notificationService } = context;
  
  try {
    // Client notification - booking was auto-rejected
    await notificationService.createNotification({
      recipientId: booking.client,
      type: 'booking_auto_rejected',
      title: 'Booking Request Expired',
      message: 'Your booking request has expired as the provider did not confirm within 15 minutes.',
      data: {
        bookingId: booking.id,
        providerId: booking.provider,
        scheduledAt: booking.scheduledAt?.toISOString(),
        autoRejectedAt: new Date().toISOString(),
        reason: 'Provider did not confirm within 15 minutes'
      },
      channels: ['in-app', 'email', 'sms'],
      priority: 'high'
    });
    
    // Provider notification - booking expired
    await notificationService.createNotification({
      recipientId: booking.provider,
      type: 'booking_expired',
      title: 'Booking Request Expired',
      message: 'A booking request has expired due to no confirmation within the time limit.',
      data: {
        bookingId: booking.id,
        clientId: booking.client,
        scheduledAt: booking.scheduledAt?.toISOString(),
        autoRejectedAt: new Date().toISOString(),
        missedDeadline: true
      },
      channels: ['in-app', 'email'],
      priority: 'normal'
    });

    logger.info('Auto-rejection notifications sent successfully', {
      bookingId: booking.id,
      clientId: booking.client,
      providerId: booking.provider
    });
    
  } catch (error) {
    logger.error('Failed to queue auto-rejection notifications', {
      bookingId: booking.id,
      error: error instanceof Error ? error.message : String(error)
    });
    // Don't throw - notification failure shouldn't fail the job
  }
}

/**
 * Check if booking is eligible for auto-rejection
 * Used for manual checks or UI display
 */
export function isEligibleForAutoRejection(
  booking: any,
  confirmationWindowMinutes: number = 15
): boolean {
  if (booking.status !== 'pending') {
    return false;
  }
  
  if (booking.confirmationTimestamp) {
    return false; // Already confirmed
  }
  
  const cutoffTime = subMinutes(new Date(), confirmationWindowMinutes);
  return booking.bookedAt <= cutoffTime;
}

/**
 * Get time remaining before auto-rejection
 * Returns null if not applicable
 */
export function getTimeUntilAutoRejection(
  booking: any,
  confirmationWindowMinutes: number = 15
): number | null {
  if (booking.status !== 'pending' || booking.confirmationTimestamp) {
    return null;
  }
  
  const expirationTime = addMinutes(new Date(booking.bookedAt), confirmationWindowMinutes);

  const now = new Date();
  const remainingMs = differenceInMilliseconds(expirationTime, now);

  return remainingMs > 0 ? Math.floor(remainingMs / 1000) : 0; // Return seconds
}