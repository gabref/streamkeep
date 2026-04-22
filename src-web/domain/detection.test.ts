import { describe, expect, it } from 'vitest';
import type { CaptureRequestPayload } from '@/api/capture';
import {
  buildMp4FileName,
  createDetectedStream,
  inferTitleSuggestion,
  sanitizeFileStem,
} from '@/domain/detection';

function payload(overrides: Partial<CaptureRequestPayload> = {}): CaptureRequestPayload {
  return {
    url: 'https://media.example/show/master.m3u8',
    requestUrl: 'https://media.example/show/master.m3u8',
    masterUrl: 'https://media.example/show/master.m3u8',
    pageUrl: 'https://video.example/watch/the-show',
    referer: null,
    userAgent: null,
    cookies: null,
    pageTitle: null,
    documentTitle: null,
    openGraphTitle: null,
    headingTitle: null,
    titleSuggestion: null,
    detectedAt: '2026-04-22T13:15:00.000Z',
    source: 'webview',
    requestType: 'master',
    confidence: 'strong',
    ...overrides,
  };
}

describe('detection title inference', () => {
  it('prefers document title over other detected metadata', () => {
    expect(
      inferTitleSuggestion(
        payload({
          documentTitle: 'Episode 4: Field Notes',
          openGraphTitle: 'Open Graph Episode',
          headingTitle: 'Heading Episode',
        })
      )
    ).toBe('Episode 4 Field Notes');
  });

  it('falls back to a URL-derived title before timestamp fallback', () => {
    expect(inferTitleSuggestion(payload({ pageUrl: 'https://example.test/library/summer-recap' }))).toBe(
      'summer recap'
    );
  });

  it('creates a detected stream with an editable MP4-ready title', () => {
    const stream = createDetectedStream(payload({ documentTitle: 'Clip / Final?' }));

    expect(stream.masterUrl).toBe('https://media.example/show/master.m3u8');
    expect(stream.titleSuggestion).toBe('Clip Final');
    expect(stream.qualities).toEqual([{ id: 'best-available', label: 'Best available' }]);
  });
});

describe('filename sanitization', () => {
  it('removes invalid filename characters while preserving safe unicode', () => {
    expect(sanitizeFileStem('Résumé: lesson / 1?')).toBe('Résumé lesson 1');
  });

  it('ensures a single mp4 extension', () => {
    expect(buildMp4FileName('Training.mp4')).toBe('Training.mp4');
  });
});
