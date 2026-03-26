import { describe, test, expect } from 'bun:test';
import { detectDialect } from './database';

describe('detectDialect', () => {
  test('detects postgresql from postgres:// URL', () => {
    expect(detectDialect('postgres://user:pass@localhost:5432/db')).toBe('postgresql');
  });

  test('detects postgresql from postgresql:// URL', () => {
    expect(detectDialect('postgresql://user:pass@localhost:5432/db')).toBe('postgresql');
  });

  test('detects sqlite from sqlite:// URL', () => {
    expect(detectDialect('sqlite:///path/to/db.sqlite')).toBe('sqlite');
  });

  test('detects sqlite from :memory:', () => {
    expect(detectDialect(':memory:')).toBe('sqlite');
  });

  test('detects sqlite from .db extension', () => {
    expect(detectDialect('/path/to/data.db')).toBe('sqlite');
  });

  test('detects sqlite from .sqlite extension', () => {
    expect(detectDialect('/path/to/data.sqlite')).toBe('sqlite');
  });

  test('detects sqlite from .sqlite3 extension', () => {
    expect(detectDialect('/path/to/data.sqlite3')).toBe('sqlite');
  });

  test('defaults to postgresql for unknown URLs', () => {
    expect(detectDialect('mysql://localhost/db')).toBe('postgresql');
  });

  test('defaults to postgresql for empty string', () => {
    expect(detectDialect('')).toBe('postgresql');
  });
});
