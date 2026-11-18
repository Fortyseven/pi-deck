/*
 * Safe markdown renderer.
 *
 * The original monolith built the DOM node-by-node to avoid innerHTML. Here the
 * components use {@html}, so instead every content fragment is HTML-escaped
 * before being placed into the output string. Links keep their real href (for
 * copy/right-click) but are intercepted by a delegated click handler in the
 * row component, which routes them through `open_external` — the webview never
 * navigates. Only http(s) URLs become links: javascript:/data:/file: render as
 * plain text (XSS via href).
 */

const esc = (s) =>
  String(s == null ? '' : s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');

/* Inline: code, bold, italic, [text](url), bare http(s) urls. */
export function inlineMd(text) {
  const out = [];
  const re = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\*[^*\n]+\*)|(\[[^\]]+\]\([^)\s]+\))|(https?:\/\/[^\s)]+)/g;
  let last = 0, m;

  while ((m = re.exec(text)) !== null) {
    if (m.index > last) out.push(esc(text.slice(last, m.index)));
    const tok = m[0];

    if (tok.startsWith('`')) {
      out.push('<code>' + esc(tok.slice(1, -1)) + '</code>');
    } else if (tok.startsWith('**')) {
      out.push('<strong>' + inlineMd(tok.slice(2, -2)) + '</strong>');
    } else if (tok.startsWith('[')) {
      const parts = tok.match(/\[([^\]]+)\]\(([^)\s]+)\)/);
      const url = parts[2];
      if (!/^https?:\/\//i.test(url)) {
        out.push(esc(tok));
      } else {
        out.push('<a data-ext="' + esc(url) + '" rel="noopener noreferrer">' + esc(parts[1]) + '</a>');
      }
    } else if (tok.startsWith('http')) {
      out.push('<a data-ext="' + esc(tok) + '" rel="noopener noreferrer">' + esc(tok) + '</a>');
    } else if (tok.startsWith('*')) {
      out.push('<em>' + inlineMd(tok.slice(1, -1)) + '</em>');
    }
    last = m.index + tok.length;
  }
  if (last < text.length) out.push(esc(text.slice(last)));
  return out.join('');
}

function splitRow(line) {
  let s = line.trim();
  if (s.startsWith('|')) s = s.slice(1);
  if (s.endsWith('|')) s = s.slice(0, -1);
  return s.split('|').map((c) => c.trim());
}

function tableHtml(header, rows) {
  let t = '<div class="scroll-x"><table class="md-table"><thead><tr>';
  for (const h of header) t += '<th>' + inlineMd(h) + '</th>';
  t += '</tr></thead><tbody>';
  for (const r of rows) {
    t += '<tr>';
    for (let c = 0; c < header.length; c++) t += '<td>' + inlineMd(r[c] === undefined ? '' : r[c]) + '</td>';
    t += '</tr>';
  }
  t += '</tbody></table></div>';
  return t;
}

/*
 * A fenced code block with a copy button. The button copies the raw code read
 * from the DOM (pre/code text), so no payload needs to be embedded in HTML.
 */
function codeBoxHtml(lang, code) {
  let h = '<div class="md-code-box">';
  if (lang) h += '<div class="md-code-lang">' + esc(lang) + '</div>';
  h += '<button class="copybtn code-copy" type="button" title="Copy this block">copy</button>';
  h += '<div class="scroll-x"><pre><code>' + esc(code) + '</code></pre></div></div>';
  return h;
}

const isTableRow = (s) => /^\s*\|.*\|\s*$/.test(s);
const isTableSep = (s) => /^\s*\|[\s:|-]+\|\s*$/.test(s);
const isBlockStart = (s) =>
  /^\s*```/.test(s) || /^\s*#{1,6}\s/.test(s) || /^\s*>/.test(s) ||
  /^\s*[-*+]\s+/.test(s) || /^\s*\d+\.\s+/.test(s) || /^\s*(-{3,}|\*{3,})\s*$/.test(s) ||
  isTableRow(s);

/*
 * Block-level parse: code fences, tables, headings, rules, blockquotes,
 * lists, paragraphs (lines joined with <br>).
 * Mirrors the original parser's behavior, including the "line that looks like
 * a block but matched no case" guard that prevents an infinite loop.
 */
export function renderMarkdown(md) {
  const out = [];
  const lines = String(md == null ? '' : md).split('\n');
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (!line.trim()) { i++; continue; }

    // code fence
    if (/^\s*```/.test(line)) {
      const lang = line.replace(/^\s*```/, '').trim();
      const buf = [];
      i++;
      while (i < lines.length && !/^\s*```/.test(lines[i])) { buf.push(lines[i]); i++; }
      i++;
      out.push(codeBoxHtml(lang, buf.join('\n')));
      continue;
    }

    // table
    if (isTableRow(line) && i + 1 < lines.length && isTableSep(lines[i + 1])) {
      const header = splitRow(line);
      i += 2;
      const rows = [];
      while (i < lines.length && isTableRow(lines[i])) { rows.push(splitRow(lines[i])); i++; }
      out.push(tableHtml(header, rows));
      continue;
    }

    // heading
    const h = line.match(/^\s*(#{1,6})\s+(.*)$/);
    if (h) {
      out.push('<h' + h[1].length + '>' + inlineMd(h[2]) + '</h' + h[1].length + '>');
      i++;
      continue;
    }

    // rule
    if (/^\s*(-{3,}|\*{3,})\s*$/.test(line)) { out.push('<hr>'); i++; continue; }

    // blockquote
    if (/^\s*>/.test(line)) {
      const buf = [];
      while (i < lines.length && /^\s*>/.test(lines[i])) {
        buf.push(lines[i].replace(/^\s*>\s?/, ''));
        i++;
      }
      out.push('<blockquote>' + inlineMd(buf.join(' ')) + '</blockquote>');
      continue;
    }

    // list
    if (/^\s*[-*+]\s+/.test(line) || /^\s*\d+\.\s+/.test(line)) {
      const ordered = /^\s*\d+\.\s+/.test(line);
      const re = ordered ? /^\s*\d+\.\s+/ : /^\s*[-*+]\s+/;
      const tag = ordered ? 'ol' : 'ul';
      let items = '';
      while (i < lines.length && re.test(lines[i])) {
        items += '<li>' + inlineMd(lines[i].replace(re, '')) + '</li>';
        i++;
      }
      out.push('<' + tag + '>' + items + '</' + tag + '>');
      continue;
    }

    // paragraph
    const buf = [];
    while (i < lines.length && lines[i].trim() && !isBlockStart(lines[i])) {
      buf.push(lines[i]);
      i++;
    }

    // Line that "looks like a block" but matched no case above (e.g. `| a | b |`
    // without the separator row). Without this the loop would never advance.
    if (!buf.length) {
      out.push('<p>' + inlineMd(lines[i]) + '</p>');
      i++;
      continue;
    }

    out.push('<p>' + buf.map((t) => inlineMd(t)).join('<br>') + '</p>');
  }

  return out.join('');
}
