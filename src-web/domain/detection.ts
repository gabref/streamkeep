import type { CaptureRequestPayload } from '@/api/capture';

export type QualityOption = {
  id: string;
  label: string;
};

export type DetectedStream = {
  id: string;
  masterUrl: string;
  pageUrl: string;
  referer: string | null;
  userAgent: string | null;
  cookies: string | null;
  detectedAt: string;
  titleSuggestion: string;
  qualities: QualityOption[];
};

export type ConfirmedDownloadIntent = {
  stream: DetectedStream;
  fileName: string;
  qualityId: string;
  confirmedAt: string;
};

const DEFAULT_QUALITY: QualityOption = {
  id: 'best-available',
  label: 'Best available',
};

const INVALID_FILENAME_CHARS = /[<>:"/\\|?*]/g;
const RESERVED_WINDOWS_NAMES = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/i;
const MAX_STEM_LENGTH = 120;

export function createDetectedStream(payload: CaptureRequestPayload): DetectedStream {
  const masterUrl = payload.masterUrl ?? payload.url;
  const detectedAt = payload.detectedAt || new Date().toISOString();
  const titleSuggestion = inferTitleSuggestion(payload);

  return {
    id: `stream-${hashValue(`${masterUrl}:${detectedAt}`)}`,
    masterUrl,
    pageUrl: payload.pageUrl ?? '',
    referer: payload.referer,
    userAgent: payload.userAgent,
    cookies: payload.cookies,
    detectedAt,
    titleSuggestion,
    qualities: [DEFAULT_QUALITY],
  };
}

export function inferTitleSuggestion(payload: CaptureRequestPayload): string {
  const title = firstPresent(
    payload.documentTitle,
    payload.pageTitle,
    payload.openGraphTitle,
    payload.headingTitle,
    payload.titleSuggestion,
    titleFromUrl(payload.pageUrl),
    titleFromUrl(payload.masterUrl ?? payload.url)
  );

  if (title) {
    return sanitizeFileStem(title, fallbackTimestampTitle(payload.detectedAt));
  }

  return fallbackTimestampTitle(payload.detectedAt);
}

export function sanitizeFileStem(value: string, fallback = 'Streamkeep capture'): string {
  const withoutExtension = value.replace(/\.mp4$/i, '');
  let safe = replaceControlCharacters(withoutExtension)
    .replace(INVALID_FILENAME_CHARS, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/[. ]+$/g, '');

  if (!safe || RESERVED_WINDOWS_NAMES.test(safe)) {
    safe = fallback;
  }

  if (safe.length > MAX_STEM_LENGTH) {
    safe = safe.slice(0, MAX_STEM_LENGTH).trim().replace(/[. ]+$/g, '');
  }

  return safe || fallback;
}

export function buildMp4FileName(value: string): string {
  return `${sanitizeFileStem(value)}.mp4`;
}

export function getInitialFileStem(stream: DetectedStream): string {
  return sanitizeFileStem(stream.titleSuggestion, fallbackTimestampTitle(stream.detectedAt));
}

function firstPresent(...values: Array<string | null | undefined>): string | null {
  for (const value of values) {
    const normalized = value?.trim();
    if (normalized) {
      return normalized;
    }
  }
  return null;
}

function fallbackTimestampTitle(value: string): string {
  const date = Number.isNaN(Date.parse(value)) ? new Date() : new Date(value);
  const stamp = date.toISOString().slice(0, 16).replace('T', ' ');
  return `Streamkeep capture ${stamp}`;
}

function titleFromUrl(value: string | null | undefined): string | null {
  if (!value) {
    return null;
  }

  try {
    const url = new URL(value);
    const segments = url.pathname
      .split('/')
      .map((segment) => decodeURIComponent(segment))
      .filter(Boolean)
      .filter((segment) => !/^(master|index|prog_index)\.m3u8$/i.test(segment))
      .filter((segment) => !/\.m3u8$/i.test(segment));
    const candidate = segments.at(-1) ?? url.hostname.replace(/\./g, ' ');
    return candidate.replace(/[-_]+/g, ' ');
  } catch {
    return null;
  }
}

function replaceControlCharacters(value: string): string {
  return Array.from(value, (character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint < 32 ? ' ' : character;
  }).join('');
}

function hashValue(value: string): string {
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = (hash * 31 + value.charCodeAt(index)) | 0;
  }
  return Math.abs(hash).toString(36);
}
