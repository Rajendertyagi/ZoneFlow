/**
 * Tests for the pure settings helpers in `settings-helpers.ts` (hex validation
 * for the zone-tint editor and the close-action decision for the top bar).
 */
import { describe, expect, test } from 'bun:test';
import { isValidHexColor, resolveCloseAction } from '../src/lib/state/settings-helpers';

describe('isValidHexColor', () => {
  test('accepts 6-digit hex in either case', () => {
    expect(isValidHexColor('#F5B544')).toBe(true);
    expect(isValidHexColor('#4dd0e1')).toBe(true);
    expect(isValidHexColor('#000000')).toBe(true);
    expect(isValidHexColor('#ffffff')).toBe(true);
  });

  test('accepts 3-digit shorthand', () => {
    expect(isValidHexColor('#abc')).toBe(true);
    expect(isValidHexColor('#0F8')).toBe(true);
  });

  test('rejects malformed values so the existing tint is kept', () => {
    expect(isValidHexColor('')).toBe(false);
    expect(isValidHexColor('F5B544')).toBe(false); // missing '#'
    expect(isValidHexColor('#12345')).toBe(false); // wrong length
    expect(isValidHexColor('#1234567')).toBe(false);
    expect(isValidHexColor('#GGGGGG')).toBe(false); // non-hex digits
    expect(isValidHexColor('red')).toBe(false);
    expect(isValidHexColor('#12 456')).toBe(false);
  });
});

describe('resolveCloseAction', () => {
  test('remembered choices bypass the dialog', () => {
    expect(resolveCloseAction('minimize')).toBe('minimize');
    expect(resolveCloseAction('exit')).toBe('exit');
  });

  test('ask, unset and unknown values keep the dialog', () => {
    expect(resolveCloseAction('ask')).toBe('dialog');
    expect(resolveCloseAction(null)).toBe('dialog');
    expect(resolveCloseAction(undefined)).toBe('dialog');
    expect(resolveCloseAction('')).toBe('dialog');
    expect(resolveCloseAction('something-else')).toBe('dialog');
  });
});
