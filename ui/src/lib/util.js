/* Pure helpers ported from the old single-file front-end. No DOM, no state. */

/* pi sends ANSI codes in setStatus/notify (terminal colors); the log is plain text. */
export const stripAnsi = (s) => String(s == null ? '' : s).replace(/\x1b\[[0-9;?]*[A-Za-z]/g, '');

/* "2026-01-05T10-30-00Z_suffix" -> "05/01 10:30 · suffix" (14 chars max). */
export function prettyId(stem) {
  const m = String(stem).match(/^(\d{4})-(\d{2})-(\d{2})T(\d{2})-(\d{2})-\d{2}-\d{3}Z[_-]?(.*)$/);
  if (!m) return stem;
  const [, , mo, d, h, mi, rest] = m;
  return `${d}/${mo} ${h}:${mi}${rest ? ' · ' + rest.slice(0, 14) : ''}`;
}

/* Project name for tabs: `cwd` without its last segment (same rule as the sidebar). */
export function projectName(cwd, homeDir) {
  if (!cwd) return '';
  if (homeDir && cwd === homeDir) return '~';
  const name = String(cwd).replace(/\/+$/, '').split('/').pop();
  return name || cwd;
}

/* Context window label: 131072 -> "131k", 1000000 -> "1M". */
export function contextLabel(n) {
  if (!n) return '';
  if (n >= 1000000) return (Math.round(n / 100000) / 10) + 'M';
  return Math.round(n / 1000) + 'k';
}

/* Token counters: 1234 -> "1.2k" style, with B/M/k units. */
export function fmtTokens(n) {
  if (n == null) return '—';
  if (n >= 1e9) return (n / 1e9).toFixed(1).replace('.0', '') + 'B';
  if (n >= 1e6) return (n / 1e6).toFixed(1).replace('.0', '') + 'M';
  if (n >= 1e3) return Math.round(n / 1e3) + 'k';
  return String(n);
}

/* Cost in USD, pi-gpui style. */
export function fmtCost(c) {
  return typeof c === 'number' ? 'US$ ' + c.toFixed(2) : '—';
}

/* Relative time: "now", "5m", "3h", "2d". */
export function relTime(ms) {
  if (!ms) return '';
  const s = Math.round((Date.now() - ms) / 1000);
  if (s < 60) return 'now';
  if (s < 3600) return Math.round(s / 60) + 'm';
  if (s < 86400) return Math.round(s / 3600) + 'h';
  return Math.round(s / 86400) + 'd';
}

/* Elapsed duration: "45s", "5m03", "1h05". */
export function fmtElapsed(from, to) {
  if (!from) return '';
  const s = Math.max(0, Math.round(((to || Date.now()) - from) / 1000));
  if (s < 60) return s + 's';
  const m = Math.floor(s / 60);
  if (m < 60) return m + 'm' + String(s % 60).padStart(2, '0');
  return Math.floor(m / 60) + 'h' + String(m % 60).padStart(2, '0');
}

/* "a/b/c.dart" -> ["c.dart", "a/b"] (accepts the Windows separator too). */
export function splitPath(p) {
  const s = String(p || '');
  const i = Math.max(s.lastIndexOf('/'), s.lastIndexOf('\\'));
  return i < 0 ? [s, ''] : [s.slice(i + 1), s.slice(0, i)];
}

/* One-line summary of a tool call's arguments. */
export function summarize(name, args) {
  if (!args) return '';
  const a = args.args || args;
  if (name === 'bash') return '$ ' + (a.command || '');
  if (a.path || a.file_path) return String(a.path || a.file_path).replace(/^.*\/(?=[^/]+$)/, '');
  if (a.pattern) return '/' + a.pattern + '/';
  if (a.agent) return a.agent + (a.task ? ' · ' + a.task : '');
  if (a.query) return a.query;
  if (a.url) return a.url;
  const k = Object.keys(a)[0];
  return k ? k + '=' + String(a[k]).slice(0, 60) : '';
}

/* HH:MM (local) for tool card timestamps. */
export function nowTime(ms) {
  const d = ms ? new Date(ms) : new Date();
  return String(d.getHours()).padStart(2, '0') + ':' + String(d.getMinutes()).padStart(2, '0');
}

/* Truncate tool output to 20k chars, preserving a trailing note. */
export function outputText(result) {
  const MAX = 20000;
  const cap = (s) => (s.length > MAX ? s.slice(0, MAX) + '\n… (' + (s.length - MAX) + ' characters truncated)' : s);

  if (!result) return '';
  if (typeof result === 'string') return cap(result);
  const content = result.content || result;
  if (Array.isArray(content)) {
    return cap(content.map((b) => (b && b.type === 'text' ? b.text : '')).join('\n').trim());
  }
  if (typeof content === 'string') return cap(content);
  return cap(JSON.stringify(result));
}

/* Subsequence match with prefix/contiguity bonuses; -1 = no match. */
export function fuzzyScore(hay, q) {
  if (!q) return 0;
  let score = 0, hi = 0, streak = 0;
  for (let qi = 0; qi < q.length; qi++) {
    const at = hay.indexOf(q[qi], hi);
    if (at < 0) return -1;
    if (at === hi) { streak++; score += 10 + streak * 5; }
    else { streak = 0; score += at === 0 ? 8 : 1; }
    if (at > 0 && /[\s\/_:-]/.test(hay[at - 1])) score += 4;
    hi = at + 1;
  }
  if (hay.startsWith(q)) score += 20;
  return score - hay.length * 0.01;
}

/* Split a command line respecting single/double quotes (paths with spaces). */
export function splitArgs(text) {
  const out = [];
  let cur = '';
  let quote = null;
  for (const ch of String(text || '')) {
    if (quote) {
      if (ch === quote) quote = null;
      else cur += ch;
      continue;
    }
    if (ch === '"' || ch === "'") { quote = ch; continue; }
    if (/\s/.test(ch)) { if (cur) { out.push(cur); cur = ''; } continue; }
    cur += ch;
  }
  if (cur) out.push(cur);
  return out;
}


/* Normalize fork text: collapse whitespace, cap at 120 chars. */
export function normForkText(s) {
  return String(s || '').replace(/\s+/g, ' ').trim().slice(0, 120);
}

/*
 * Parse a raw session file (one JSON record per line).
 * Records: {type:'__deck_meta'} (pagination meta) and {type:'message', message}.
 */
export function parseSessionFile(raw) {
  const messages = [];
  let meta = null;
  for (const line of String(raw).split('\n')) {
    if (!line.trim()) continue;
    let rec = null;
    try { rec = JSON.parse(line); } catch { continue; }
    if (!rec) continue;
    if (rec.type === '__deck_meta') { meta = rec; continue; }
    if (rec.type === 'message' && rec.message) messages.push(rec.message);
  }
  return { messages, meta };
}

/*
 * The pi-subagents widget prints live state as
 *   PI_SUBAGENT_ASYNC_JSON:"<escaped JSON>"
 * inside a stderr line. The payload is a JSON-encoded string, so the outer
 * quotes are parsed first (honoring backslash escapes), then the inner JSON.
 */
export const PREFIX = 'PI_SUBAGENT_ASYNC_JSON:';
export function extractSnapshot(line) {
  const i = line.indexOf(PREFIX);
  if (i < 0) return null;
  const rest = line.slice(i + PREFIX.length);
  let end = -1, backslashes = 0;
  for (let k = 0; k < rest.length; k++) {
    const ch = rest[k];
    if (ch === '\\') { backslashes++; continue; }
    if (ch === '"' && backslashes % 2 === 0) { end = k; break; }
    backslashes = 0;
  }
  if (end < 0) return null;
  try { return JSON.parse(JSON.parse('"' + rest.slice(0, end) + '"')); }
  catch { return null; }
}

/*
 * Which packages are installed: `pi list` indents sources with 2 spaces and
 * resolved paths with 4. The "npm:" prefix is stripped.
 */
export function parseInstalledSources(text) {
  const set = new Set();
  for (const line of String(text).split('\n')) {
    const m = line.match(/^ {2}(?! )(\S.*)$/);
    if (!m) continue;
    let src = m[1].trim();
    if (src.startsWith('npm:')) src = src.slice(4);
    set.add(src);
  }
  return set;
}
