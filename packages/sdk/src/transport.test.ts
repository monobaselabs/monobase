import { describe, test, expect, beforeEach } from 'bun:test';
import {
  HttpTransport,
  isTauriEnvironment,
  isEmbeddedMode,
  setTransportMode,
  setHttpBaseUrl,
  getTransport,
  getTransportInfo,
} from './transport';

describe('Transport', () => {
  beforeEach(() => {
    // Reset to auto mode before each test
    setTransportMode('auto');
  });

  describe('isTauriEnvironment', () => {
    test('returns false when not in Tauri', () => {
      expect(isTauriEnvironment()).toBe(false);
    });
  });

  describe('isEmbeddedMode', () => {
    test('returns false when not in embedded mode', () => {
      expect(isEmbeddedMode()).toBe(false);
    });
  });

  describe('HttpTransport', () => {
    test('constructs with base URL', () => {
      const transport = new HttpTransport('http://localhost:3000');
      expect(transport).toBeDefined();
    });
  });

  describe('setTransportMode', () => {
    test('sets mode to http', () => {
      setTransportMode('http');
      const info = getTransportInfo();
      expect(info.mode).toBe('http');
      expect(info.active).toBe('http');
    });

    test('sets mode to auto', () => {
      setTransportMode('auto');
      const info = getTransportInfo();
      expect(info.mode).toBe('auto');
      // In test environment (not Tauri), auto defaults to http
      expect(info.active).toBe('http');
    });
  });

  describe('setHttpBaseUrl', () => {
    test('updates base URL', () => {
      setHttpBaseUrl('http://example.com:8080');
      const info = getTransportInfo();
      expect(info.baseUrl).toBe('http://example.com:8080');
    });
  });

  describe('getTransport', () => {
    test('returns HttpTransport in non-Tauri environment', () => {
      setTransportMode('http');
      const transport = getTransport();
      expect(transport).toBeInstanceOf(HttpTransport);
    });

    test('returns same instance on subsequent calls', () => {
      setTransportMode('http');
      const transport1 = getTransport();
      const transport2 = getTransport();
      expect(transport1).toBe(transport2);
    });
  });

  describe('getTransportInfo', () => {
    test('returns correct info structure', () => {
      setTransportMode('http');
      setHttpBaseUrl('http://localhost:7213');

      const info = getTransportInfo();

      expect(info).toEqual({
        mode: 'http',
        active: 'http',
        baseUrl: 'http://localhost:7213',
        isTauri: false,
        isEmbedded: false,
      });
    });
  });
});
