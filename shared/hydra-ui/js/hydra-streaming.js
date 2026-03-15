/**
 * HYDRA STREAMING ENGINE — Shared JavaScript
 * Used by: Desktop (Tauri webview) + VS Code (extension webview)
 * Implements Pattern 0 (human-paced output) across all surfaces.
 */

class HydraStreamer {
  constructor(containerEl) {
    this.container = containerEl;
    this.buffer = '';
    this.revealed = 0;
    this.active = false;
    this.speed = 1.0;
    this.tier = 'text'; // text, tool, table, summary, error, briefing
    this.frameId = null;
    this.lastReveal = 0;
    this.onComplete = null;
  }

  /** Start streaming new content */
  start(tier = 'text') {
    this.buffer = '';
    this.revealed = 0;
    this.active = true;
    this.speed = 1.0;
    this.tier = tier;
    this.lastReveal = performance.now();
    this._tick();
  }

  /** Append content from SSE chunk */
  append(text) {
    this.buffer += text;
  }

  /** Stop streaming, reveal everything */
  stop() {
    this.active = false;
    if (this.frameId) cancelAnimationFrame(this.frameId);
    this.revealed = this.buffer.length;
    this._render();
  }

  /** Accelerate (user scrolled or typed) */
  accelerate(factor) {
    this.speed = Math.min(this.speed * factor, 10);
  }

  /** Internal tick — called via requestAnimationFrame */
  _tick() {
    if (!this.active) return;

    const now = performance.now();
    const interval = this._interval();
    if (now - this.lastReveal < interval) {
      this.frameId = requestAnimationFrame(() => this._tick());
      return;
    }

    this.lastReveal = now;
    const remaining = this.buffer.length - this.revealed;
    if (remaining <= 0) {
      if (this.onComplete) this.onComplete();
      return;
    }

    let chars = Math.max(1, Math.floor(this._baseChars() * this.speed));
    chars = Math.min(chars, remaining);

    // Check for natural pause points
    const segment = this.buffer.substring(this.revealed, this.revealed + chars);
    if (segment.includes('. ') || segment.includes('.\n')) {
      // Sentence pause: delay next reveal by 80ms
      this.lastReveal += 80;
    }
    if (segment.includes('\n\n')) {
      // Paragraph pause: 120ms
      this.lastReveal += 120;
    }
    if (segment.includes('```')) {
      // Code block pause: 200ms
      this.lastReveal += 200;
    }

    this.revealed += chars;
    this._render();

    this.frameId = requestAnimationFrame(() => this._tick());
  }

  _interval() {
    const intervals = {
      text: 16,      // ~60fps
      tool: 200,
      table: 200,
      summary: 100,
      error: 100,
      briefing: 300,
    };
    return intervals[this.tier] || 16;
  }

  _baseChars() {
    const bases = { text: 3, tool: 5, table: 10, summary: 5, error: 2, briefing: 3 };
    return bases[this.tier] || 3;
  }

  _render() {
    const visible = this.buffer.substring(0, this.revealed);
    this.container.innerHTML = HydraMarkdown.render(visible);
    // Add cursor if still streaming
    if (this.active && this.revealed < this.buffer.length) {
      this.container.innerHTML += '<span class="stream-cursor"></span>';
    }
    // Auto-scroll
    this.container.scrollTop = this.container.scrollHeight;
  }
}

/**
 * HYDRA MARKDOWN RENDERER
 * Renders markdown to HTML with Hydra styling classes.
 */
const HydraMarkdown = {
  render(text) {
    let html = '';
    let inCode = false;
    let codeLang = '';
    let codeContent = '';

    for (const line of text.split('\n')) {
      const trimmed = line.trim();

      // Code blocks
      if (trimmed.startsWith('```')) {
        if (inCode) {
          html += this._codeBlock(codeLang, codeContent);
          inCode = false;
          codeContent = '';
          codeLang = '';
          continue;
        }
        inCode = true;
        codeLang = trimmed.substring(3);
        continue;
      }
      if (inCode) {
        codeContent += line + '\n';
        continue;
      }

      // Headers
      if (trimmed.startsWith('### ')) {
        html += `<div class="md-h3">${this._inline(trimmed.substring(4))}</div>`;
      } else if (trimmed.startsWith('## ')) {
        html += `<div class="md-h2">${this._inline(trimmed.substring(3))}</div>`;
      } else if (trimmed.startsWith('# ')) {
        html += `<div class="md-h1">${this._inline(trimmed.substring(2))}</div>`;
      }
      // Blockquote
      else if (trimmed.startsWith('> ')) {
        html += `<div class="md-blockquote">${this._inline(trimmed.substring(2))}</div>`;
      }
      // Horizontal rule
      else if (trimmed === '---' || trimmed === '***' || trimmed === '___') {
        html += '<hr class="md-hr">';
      }
      // Bullet list
      else if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
        html += `<div class="md-list-item">${this._inline(trimmed.substring(2))}</div>`;
      }
      // Numbered list
      else if (/^\d+\.\s/.test(trimmed)) {
        const rest = trimmed.replace(/^\d+\.\s/, '');
        const num = trimmed.match(/^(\d+)/)[1];
        html += `<div class="md-num-item"><span style="color:var(--hydra-cyan)">${num}.</span> ${this._inline(rest)}</div>`;
      }
      // Empty line
      else if (trimmed === '') {
        html += '<br>';
      }
      // Regular text
      else {
        html += `<div>${this._inline(line)}</div>`;
      }
    }

    // Close unclosed code block
    if (inCode) {
      html += this._codeBlock(codeLang, codeContent);
    }

    return html;
  },

  _codeBlock(lang, content) {
    const lines = content.split('\n').filter((_, i, arr) => i < arr.length - 1 || _.trim());
    const numbered = lines.map((l, i) =>
      `<div><span class="line-number">${i + 1}</span>${this._escapeHtml(l)}</div>`
    ).join('');
    const langBadge = lang ? `<span class="lang-badge">${lang}</span>` : '';
    return `<div class="code-block">${langBadge}<button class="code-copy-btn" onclick="copyCode(this)">Copy</button>${numbered}</div>`;
  },

  _inline(text) {
    return text
      .replace(/\*\*(.+?)\*\*/g, '<span class="md-bold">$1</span>')
      .replace(/\*(.+?)\*/g, '<span class="md-italic">$1</span>')
      .replace(/`(.+?)`/g, '<span class="md-inline-code">$1</span>')
      .replace(/~~(.+?)~~/g, '<del>$1</del>')
      .replace(/\[(.+?)\]\((.+?)\)/g, '<a class="md-link" href="$2">$1</a>');
  },

  _escapeHtml(text) {
    return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  },
};

/**
 * HYDRA TABLE RENDERER
 * Sortable, filterable tables.
 */
const HydraTable = {
  render(headers, rows, options = {}) {
    const id = 'table-' + Math.random().toString(36).substr(2, 6);
    let html = '';

    if (options.filterable) {
      html += `<input class="table-filter" placeholder="Filter..." oninput="filterTable('${id}', this.value)">`;
    }
    if (options.exportable) {
      html += `<button class="table-export-btn" onclick="exportTable('${id}')">Export CSV</button>`;
    }

    html += `<table class="hydra-table" id="${id}"><thead><tr>`;
    headers.forEach((h, i) => {
      html += `<th onclick="sortTable('${id}', ${i})">${h} <span class="sort-icon">⇅</span></th>`;
    });
    html += '</tr></thead><tbody>';
    rows.forEach(row => {
      html += '<tr class="hydra-row-highlight">';
      row.forEach(cell => {
        let cls = '';
        if (typeof cell === 'string') {
          if (cell.includes('100%') || cell.includes('connected') || cell === '✅') cls = 'cell-green';
          else if (cell.includes('warning') || cell.includes('partial')) cls = 'cell-yellow';
          else if (cell.includes('error') || cell.includes('failed') || cell === '❌') cls = 'cell-red';
        }
        html += `<td class="${cls}">${cell}</td>`;
      });
      html += '</tr>';
    });
    html += '</tbody></table>';
    return html;
  }
};

/**
 * HYDRA DIFF RENDERER
 * Full-color diff with side-by-side option.
 */
const HydraDiff = {
  render(filePath, lines, options = {}) {
    let html = `<div class="diff-container">`;
    html += `<div class="tool-result"><div class="tool-header">`;
    html += `<span class="dot dot-success"></span>`;
    html += `<span class="tool-sister">Forge</span>`;
    html += `<span class="tool-connector"> ▸ </span>`;
    html += `<span class="tool-action">Edit(${filePath})</span>`;
    html += `</div></div>`;

    lines.forEach(l => {
      const cls = l.type === 'added' ? 'diff-line-added'
        : l.type === 'removed' ? 'diff-line-removed' : 'diff-line-context';
      const prefix = l.type === 'added' ? '+' : l.type === 'removed' ? '-' : ' ';
      html += `<div class="diff-line ${cls}">`;
      html += `<span class="line-num">${l.lineNum || ''}</span>`;
      html += `<span class="diff-prefix">${prefix}</span>`;
      html += `<span>${HydraMarkdown._escapeHtml(l.content)}</span>`;
      html += `</div>`;
    });

    if (options.applyable) {
      html += `<button class="diff-apply-btn" onclick="applyDiff('${filePath}')">Apply</button>`;
      html += `<button class="diff-open-btn" onclick="openFile('${filePath}')">Open file</button>`;
    }
    html += `</div>`;
    return html;
  }
};

/**
 * HYDRA BELIEF BOX RENDERER
 */
const HydraBeliefBox = {
  render(text, confidence, timesTested, provenance) {
    const level = confidence > 0.85 ? 'high' : confidence >= 0.5 ? 'mid' : 'low';
    let html = `<div class="belief-box confidence-${level}">`;
    html += `<div class="belief-header">`;
    html += `<span>Belief</span>`;
    html += `<span class="confidence-pill ${level}">${confidence.toFixed(2)}</span>`;
    html += `</div>`;
    html += `<div class="belief-text">"${text}"</div>`;
    if (provenance) {
      html += `<div class="belief-provenance">${provenance}</div>`;
    }
    html += `</div>`;
    return html;
  }
};

/**
 * HYDRA BRIEFING CARD RENDERER
 */
const HydraBriefing = {
  render(items) {
    let html = `<div class="briefing-card"><h3>Morning Briefing</h3>`;
    html += `<div style="color:var(--hydra-dim);margin-bottom:8px">While you were away:</div>`;
    items.forEach(item => {
      const cls = item.priority === 'urgent' ? 'urgent'
        : item.priority === 'important' ? 'important' : 'info';
      const icon = item.priority === 'urgent' ? '▲'
        : item.priority === 'important' ? '●' : '○';
      html += `<div class="briefing-item ${cls}">`;
      html += `<span class="priority-icon">${icon}</span>`;
      html += `<span>${item.text}</span>`;
      if (item.actions) {
        html += `<span class="briefing-actions">`;
        item.actions.forEach(a => {
          html += `<button onclick="${a.handler}">${a.label}</button>`;
        });
        html += `</span>`;
      }
      html += `</div>`;
    });
    html += `</div>`;
    return html;
  }
};

// Table utilities
function sortTable(tableId, colIdx) {
  const table = document.getElementById(tableId);
  const tbody = table.querySelector('tbody');
  const rows = Array.from(tbody.querySelectorAll('tr'));
  const asc = !table.dataset.sortAsc || table.dataset.sortCol != colIdx;
  table.dataset.sortAsc = asc;
  table.dataset.sortCol = colIdx;
  rows.sort((a, b) => {
    const aVal = a.cells[colIdx]?.textContent || '';
    const bVal = b.cells[colIdx]?.textContent || '';
    return asc ? aVal.localeCompare(bVal, undefined, {numeric: true})
               : bVal.localeCompare(aVal, undefined, {numeric: true});
  });
  rows.forEach(r => tbody.appendChild(r));
}

function filterTable(tableId, query) {
  const table = document.getElementById(tableId);
  const rows = table.querySelectorAll('tbody tr');
  const q = query.toLowerCase();
  rows.forEach(r => {
    r.style.display = r.textContent.toLowerCase().includes(q) ? '' : 'none';
  });
}

function exportTable(tableId) {
  const table = document.getElementById(tableId);
  let csv = '';
  for (const row of table.rows) {
    csv += Array.from(row.cells).map(c => `"${c.textContent}"`).join(',') + '\n';
  }
  navigator.clipboard.writeText(csv);
}

function copyCode(btn) {
  const block = btn.parentElement;
  const text = Array.from(block.querySelectorAll('div'))
    .map(d => d.textContent.replace(/^\d+/, '').trim()).join('\n');
  navigator.clipboard.writeText(text);
  btn.textContent = 'Copied!';
  setTimeout(() => btn.textContent = 'Copy', 1500);
}

// Export for module systems
if (typeof module !== 'undefined') {
  module.exports = { HydraStreamer, HydraMarkdown, HydraTable, HydraDiff, HydraBeliefBox, HydraBriefing };
}
