/**
 * User WebSocket Handler
 * Global personal connection for user-specific events
 *
 * Endpoint: /ws/user
 * Auth: Required (user can only connect to own channel)
 *
 * Receives:
 * - General notifications
 * - Booking updates (confirmed, rejected, rescheduled)
 * - New message alerts from other rooms
 * - System announcements
 */

import type { Context } from 'hono';
import type { WSContext } from 'hono/ws';
import type { WebSocketHandler } from '@/core/ws';
import type { User } from '@/types/auth';
import { authMiddleware } from '@/middleware/auth';

export const config: WebSocketHandler = {
  path: '/ws/user',
  description: 'Global user notifications and updates',
  middleware: [authMiddleware()],

  async onConnect(ctx: Context, ws: WSContext) {
    const user = ctx.get('user') as User;
    const wsService = ctx.get('ws');
    const logger = ctx.get('logger');

    // Track this user's connection
    wsService.trackUser(user.id, ws);

    // Send connection confirmation
    ws.send(JSON.stringify({
      event: 'connected',
      payload: {
        userId: user.id,
        timestamp: new Date().toISOString(),
      },
    }));

    logger.info({ userId: user.id }, 'User WebSocket connected');
  },

  async onMessage(ctx: Context, ws: WSContext, message: any) {
    const user = ctx.get('user') as User;
    const logger = ctx.get('logger');

    const { type, data } = message;

    switch (type) {
      case 'ping':
        // Heartbeat/keepalive
        ws.send(JSON.stringify({ event: 'pong', payload: { timestamp: new Date().toISOString() } }));
        break;

      case 'subscribe':
        // Future: Handle topic subscriptions
        logger.debug({ userId: user.id, data }, 'User subscription request');
        break;

      case 'unsubscribe':
        // Future: Handle topic unsubscriptions
        logger.debug({ userId: user.id, data }, 'User unsubscription request');
        break;

      default:
        logger.warn({ userId: user.id, type }, 'Unknown message type from user WebSocket');
    }
  },

  async onClose(ctx: Context, ws: WSContext) {
    const user = ctx.get('user') as User;
    const wsService = ctx.get('ws');
    const logger = ctx.get('logger');

    // Untrack user connection
    wsService.untrackUser(user.id);

    logger.info({ userId: user.id }, 'User WebSocket disconnected');
  },
};
