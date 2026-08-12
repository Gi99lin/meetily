import { describe, expect, test } from 'bun:test';
import { DEFAULT_BETA_FEATURES } from '../../src/types/betaFeatures';

describe('speaker diarization defaults', () => {
  test('is enabled for installations without a saved override', () => {
    expect(DEFAULT_BETA_FEATURES.speakerDiarization).toBe(true);
  });
});
