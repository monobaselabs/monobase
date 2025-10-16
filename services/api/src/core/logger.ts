/**
 * Pino logger configuration
 * Factory function that accepts config as parameter
 */

import pino from 'pino';
import type { Config } from '@/core/config';
import type { Logger } from '@/types/logger';

// Create logger instance with config
export function createLogger(config: Config): Logger {
  const options: any = {
    level: config.logging.level,
    serializers: {
      req: (req) => ({
        method: req.method,
        url: req.url,
        headers: req.headers,
      }),
      res: (res) => ({
        statusCode: res.statusCode,
      }),
      error: pino.stdSerializers.err,
    },
    base: {
      service: 'api',
    },
  };

  // Pretty printing configuration - dead code eliminated in production builds
  // The NODE_ENV check is replaced at compile time via --define flag
  if (process.env.NODE_ENV !== 'production' && config.logging.pretty) {
    options.transport = {
      target: 'pino-pretty',
      options: {
        colorize: true,
        ignore: 'pid,hostname',
        translateTime: 'HH:MM:ss',
      },
    };
  }

  const logger = pino(options);
  return logger;
}
