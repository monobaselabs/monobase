import { Context } from 'hono';
import type { Config } from '@/core/config';

/**
 * getIceServers
 *
 * Path: GET /comms/ice-servers
 * OperationId: getIceServers
 *
 * Get TURN/STUN server configuration for WebRTC
 */
export async function getIceServers(ctx: Context) {
  // Get config from context
  const config = ctx.get('config') as Config;

  // Return ICE servers configuration
  return ctx.json({
    iceServers: config.webrtc.iceServers
  }, 200);
}
