fn content_initialization_script(
    _global_zoom: f64,
    ad_block_script: &str,
    fingerprint: bool,
    fingerprint_seed: &str,
    x_login_compat: bool,
    strict: bool,
    site_permissions: &config::SitePermissionMap,
    default_permissions: &config::SitePermissions,
) -> String {
    let ad_prefix = if ad_block_script.is_empty() {
        String::new()
    } else {
        format!("{}\n", ad_block_script)
    };
    let privacy_prefix = privacy_initialization_script(
        fingerprint,
        fingerprint_seed,
        x_login_compat,
        strict,
        site_permissions,
        default_permissions,
    );
    let script = r#"
(() => {
  try {
    if (window.top !== window) return;
    if (!/^https?:$/.test(location.protocol)) return;
    const h = location.hostname || '';
    if (!h) return;
    const ex = '=;expires=Thu, 01 Jan 1970 00:00:00 GMT;path=/';
    const del = (d) => { try { document.cookie = 'googtrans' + ex + (d ? (';domain=' + d) : ''); } catch (_) {} };
    del('');
    const parts = h.split('.');
    for (let i = 0; i < parts.length - 1; i++) {
      const d = parts.slice(i).join('.');
      del(d);
      del('.' + d);
    }
  } catch (_) {}
})();
(() => {
  let isTop = false;
  try { isTop = window.top === window; } catch (_) {}
  if (window.__neuraContentBridgeInstalled) return;
  window.__neuraContentBridgeInstalled = true;
  const post = (payload) => {
    try {
      if (window.ipc && typeof window.ipc.postMessage === 'function') {
        window.ipc.postMessage(JSON.stringify(payload));
      }
    } catch (_) {}
  };
  if (!isTop) return;
  (() => {
    const Native = window.Notification;
    if (!Native) return;
    let seq = 0;
    const live = {};
    function VN(title, opts) {
      opts = opts || {};
      const id = 'wn' + (++seq) + '_' + Date.now();
      this.title = String(title == null ? '' : title);
      this.body = opts.body || '';
      this.icon = opts.icon || '';
      this.tag = opts.tag || '';
      this.data = opts.data;
      this.dir = opts.dir || 'auto';
      this.lang = opts.lang || '';
      this.onclick = null; this.onclose = null; this.onerror = null; this.onshow = null;
      const L = {click: [], close: [], show: [], error: []};
      this.addEventListener = (t, f) => { if (L[t] && typeof f === 'function') L[t].push(f); };
      this.removeEventListener = (t, f) => { const a = L[t]; if (a) { const i = a.indexOf(f); if (i >= 0) a.splice(i, 1); } };
      this.dispatchEvent = () => true;
      this._fire = (t) => {
        const e = {type: t, target: this};
        try { const h = this['on' + t]; if (typeof h === 'function') h.call(this, e); } catch (_) {}
        (L[t] || []).forEach(f => { try { f.call(this, e); } catch (_) {} });
      };
      this.close = () => { post({cmd: 'web_notification_close', id}); delete live[id]; };
      live[id] = this;
      let icon = '';
      try { if (this.icon) icon = new URL(this.icon, location.href).href; } catch (_) {}
      post({cmd: 'web_notification', id, title: this.title, body: this.body, icon, origin: location.origin});
      setTimeout(() => this._fire('show'), 0);
    }
    Object.defineProperty(VN, 'permission', {get: () => Native.permission, configurable: true});
    VN.requestPermission = function() { try { return Native.requestPermission.apply(Native, arguments); } catch (_) { return Promise.resolve(Native.permission); } };
    try { VN.maxActions = Native.maxActions; } catch (_) {}
    window.__neuraNotifClick = (id) => { const n = live[id]; if (n) { n._fire('click'); try { window.focus(); } catch (_) {} } };
    window.__neuraNotifClose = (id) => { const n = live[id]; if (n) { n._fire('close'); delete live[id]; } };
    try { window.Notification = VN; } catch (_) {}
  })();
  const findApi = (() => {
    let q = '';
    let ranges = [];
    let spans = [];
    let idx = -1;
    const sid = '__ventus_find_style';
    const hn = 'ventus-find-match';
    const an = 'ventus-find-active';
    const addStyle = () => {
      if (document.getElementById(sid)) return;
      const el = document.createElement('style');
      el.id = sid;
      el.textContent = '::highlight(ventus-find-match){background:rgba(255,218,68,.72);color:inherit}::highlight(ventus-find-active){background:#ff9f1c;color:#111}.__ventus-find-match{background:rgba(255,218,68,.72);color:inherit;border-radius:2px}.__ventus-find-active{background:#ff9f1c!important;color:#111!important}';
      (document.head || document.documentElement).appendChild(el);
    };
    const useRanges = () => !!(window.CSS && CSS.highlights && typeof Highlight !== 'undefined' && typeof Range !== 'undefined');
    const clearRanges = () => {
      try {
        if (window.CSS && CSS.highlights) {
          CSS.highlights.delete(hn);
          CSS.highlights.delete(an);
        }
      } catch (_) {}
      ranges = [];
    };
    const clearSpans = () => {
      for (const span of spans) {
        const p = span.parentNode;
        if (!p) continue;
        p.replaceChild(document.createTextNode(span.textContent || ''), span);
        try { p.normalize(); } catch (_) {}
      }
      spans = [];
    };
    const clear = () => {
      clearRanges();
      clearSpans();
      idx = -1;
      try {
        const sel = window.getSelection && window.getSelection();
        if (sel) sel.removeAllRanges();
      } catch (_) {}
    };
    const skip = (n) => {
      if (!n || !n.nodeValue || !n.nodeValue.trim()) return true;
      const p = n.parentElement;
      if (!p) return true;
      return !!p.closest('script,style,noscript,textarea,input,select,option,.__ventus-find-match');
    };
    const nodes = () => {
      const root = document.body;
      if (!root || !window.NodeFilter) return [];
      const out = [];
      const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
        acceptNode(n) {
          return skip(n) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT;
        }
      });
      let n = w.nextNode();
      while (n) {
        out.push(n);
        n = w.nextNode();
      }
      return out;
    };
    const matchRanges = (needle) => {
      const out = [];
      const low = needle.toLowerCase();
      for (const n of nodes()) {
        const text = n.nodeValue || '';
        const hay = text.toLowerCase();
        let at = hay.indexOf(low);
        while (at >= 0) {
          const r = document.createRange();
          r.setStart(n, at);
          r.setEnd(n, at + needle.length);
          out.push(r);
          at = hay.indexOf(low, at + needle.length);
        }
      }
      return out;
    };
    const total = () => ranges.length || spans.length;
    const next = (same, forward) => {
      const n = total();
      if (!n) return -1;
      if (!same || idx < 0) return forward ? 0 : n - 1;
      return (idx + (forward ? 1 : -1) + n) % n;
    };
    const res = (needle) => ({query: needle, total: total(), index: idx >= 0 ? idx + 1 : 0});
    const rangeRect = (r) => {
      const rect = r.getBoundingClientRect();
      if (rect && (rect.width || rect.height)) return rect;
      const rs = r.getClientRects();
      return rs && rs.length ? rs[0] : null;
    };
    const showRange = () => {
      if (idx < 0 || !ranges[idx]) return;
      try {
        const h = new Highlight();
        h.add(ranges[idx]);
        CSS.highlights.set(an, h);
      } catch (_) {}
      try {
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(ranges[idx]);
      } catch (_) {}
      const rect = rangeRect(ranges[idx]);
      if (rect) window.scrollBy({top: rect.top - window.innerHeight * 0.35, left: rect.left - window.innerWidth * 0.35, behavior: 'smooth'});
    };
    const runRanges = (needle, forward) => {
      ranges = matchRanges(needle);
      try {
        const h = new Highlight();
        ranges.forEach(r => h.add(r));
        CSS.highlights.set(hn, h);
      } catch (_) {}
      idx = next(false, forward);
      showRange();
      return res(needle);
    };
    const makeSpans = (needle) => {
      const low = needle.toLowerCase();
      for (const n of nodes()) {
        const text = n.nodeValue || '';
        const hay = text.toLowerCase();
        let at = hay.indexOf(low);
        if (at < 0) continue;
        const frag = document.createDocumentFragment();
        let last = 0;
        while (at >= 0) {
          if (at > last) frag.appendChild(document.createTextNode(text.slice(last, at)));
          const span = document.createElement('span');
          span.className = '__ventus-find-match';
          span.textContent = text.slice(at, at + needle.length);
          spans.push(span);
          frag.appendChild(span);
          last = at + needle.length;
          at = hay.indexOf(low, last);
        }
        if (last < text.length) frag.appendChild(document.createTextNode(text.slice(last)));
        n.parentNode.replaceChild(frag, n);
      }
    };
    const showSpan = () => {
      spans.forEach(s => s.classList.remove('__ventus-find-active'));
      if (idx < 0 || !spans[idx]) return;
      const s = spans[idx];
      s.classList.add('__ventus-find-active');
      try { s.scrollIntoView({block: 'center', inline: 'nearest', behavior: 'smooth'}); } catch (_) {}
      try {
        const r = document.createRange();
        r.selectNodeContents(s);
        const sel = window.getSelection();
        sel.removeAllRanges();
        sel.addRange(r);
      } catch (_) {}
    };
    const runSpans = (needle, forward) => {
      makeSpans(needle);
      idx = next(false, forward);
      showSpan();
      return res(needle);
    };
    const run = (value, forward) => {
      const needle = String(value || '');
      if (!needle) {
        clear();
        q = '';
        return res('');
      }
      const same = needle === q && total() > 0;
      if (same) {
        idx = next(true, forward !== false);
        if (ranges.length) showRange(); else showSpan();
        return res(needle);
      }
      clear();
      q = needle;
      addStyle();
      return useRanges() ? runRanges(needle, forward !== false) : runSpans(needle, forward !== false);
    };
    return {run, clear};
  })();
  window.__neuraFind = findApi;
  const isErrorDoc = () => {
    let h = '';
    try { h = location.href || ''; } catch (_) {}
    return h.indexOf('chrome-error:') === 0 || h.indexOf('chrome://') === 0 || h.indexOf('edge://') === 0;
  };
  const sendProgress = (progress) => {
    if (isTop && !isErrorDoc()) post({cmd:'content_progress', progress, url: location.href});
  };
  const faviconHref = () => {
    const root = document.head || document.documentElement;
    if (!root) return '';
    const icons = Array.from(root.querySelectorAll('link[rel]'));
    for (const token of ['icon', 'apple-touch-icon', 'apple-touch-icon-precomposed', 'mask-icon']) {
      const icon = icons.find(link => (link.getAttribute('rel') || '').toLowerCase().split(/\s+/).includes(token) && link.href);
      if (icon) return icon.href;
    }
    try { return new URL('/favicon.ico', location.href).href; } catch (_) { return ''; }
  };
  let lastHref = location.href;
  let lastMeta = '';
  let metaTimer = 0;
  const sendMetadata = (replace = false) => {
    if (!isTop || isErrorDoc()) return;
    const favicon = faviconHref();
    const title = document.title || location.href;
    const key = location.href + '\n' + title + '\n' + favicon + '\n' + replace;
    if (key === lastMeta) return;
    lastMeta = key;
    post({
      cmd:'content_metadata',
      url: location.href,
      title,
      favicon,
      replace
    });
  };
  const queueMetadata = (replace = false, wait = 50) => {
    clearTimeout(metaTimer);
    metaTimer = setTimeout(() => sendMetadata(replace), wait);
  };
  const sendLocationChange = (replace = false) => {
    if (!isTop || isErrorDoc()) return;
    const href = location.href;
    if (href === lastHref) {
      setTimeout(() => {
        queueMetadata(replace);
        sendNavState();
      }, 50);
      return;
    }
    lastHref = href;
    post({cmd:'content_load_start', url: href});
    sendProgress(0.22);
    let done = false;
    let obs = null;
    let settleTimer = 0;
    const finish = () => {
      if (done) return;
      done = true;
      clearTimeout(settleTimer);
      if (obs) obs.disconnect();
      queueMetadata(replace);
      sendNavState();
      sendProgress(0.96);
    };
    const settle = () => {
      clearTimeout(settleTimer);
      settleTimer = setTimeout(finish, 600);
    };
    try {
      if (document.body) {
        obs = new MutationObserver(settle);
        obs.observe(document.body, {childList:true, subtree:true});
      }
    } catch (_) {}
    setTimeout(() => {
      queueMetadata(replace);
      sendNavState();
      sendProgress(0.72);
    }, 350);
    setTimeout(finish, 8000);
  };
  const sendNavState = () => {
    if (!isTop) return;
    let canBack = false;
    let canFwd = false;
    try {
      const nav = window.navigation;
      if (nav && typeof nav.canGoBack === 'boolean') {
        canBack = nav.canGoBack;
        canFwd = nav.canGoForward;
      } else if (nav && nav.currentEntry) {
        const idx = nav.currentEntry.index;
        const len = nav.entries ? nav.entries().length : 0;
        canBack = idx > 0;
        canFwd = len > 0 && idx < len - 1;
      } else {
        canBack = history.length > 1;
      }
    } catch(_) {}
    try { post({cmd:'content_nav_state', can_back: canBack, can_forward: canFwd}); } catch(_) {}
  };

  sendProgress(0.12);
  document.addEventListener('readystatechange', () => {
    if (document.readyState === 'interactive') {
      sendProgress(0.65);
      queueMetadata();
    } else if (document.readyState === 'complete') {
      queueMetadata();
      sendNavState();
      sendProgress(0.92);
    }
  });
  window.addEventListener('DOMContentLoaded', () => {
    sendProgress(0.75);
    queueMetadata();
    sendNavState();
  });
  window.addEventListener('load', () => {
    queueMetadata();
    sendNavState();
    sendProgress(0.96);
  });
  try {
    const watchFavicons = () => {
      const head = document.head;
      if (!head) {
        setTimeout(watchFavicons, 120);
        return;
      }
      const favObs = new MutationObserver(records => {
        for (const r of records) {
          if (r.target && r.target.tagName === 'LINK') {
            queueMetadata(true, 80);
            return;
          }
          for (const n of r.addedNodes || []) {
            if (n.tagName === 'LINK') {
              queueMetadata(true, 80);
              return;
            }
          }
        }
      });
      favObs.observe(head, {subtree:true, childList:true, attributes:true, attributeFilter:['href','rel']});
    };
    watchFavicons();
  } catch (_) {}
  setInterval(() => {
    if (location.href !== lastHref) sendLocationChange(false);
  }, 1000);
  const pushState = history.pushState;
  history.pushState = function() {
    const result = pushState.apply(this, arguments);
    sendLocationChange(false);
    return result;
  };
  const replaceState = history.replaceState;
  history.replaceState = function() {
    const result = replaceState.apply(this, arguments);
    sendLocationChange(true);
    return result;
  };
  window.addEventListener('popstate', () => { sendLocationChange(true); });
  setTimeout(sendMetadata, 1200);


  // Drop a link dragged from outside (another browser, the desktop) → open a new tab.
  // Bubble phase + defaultPrevented check so a page's own drop zone wins; editable targets
  // keep native text-insert behavior; file drags are left to the page so uploads still work.
  const dragHasLink = (dt) => {
    if (!dt) return false;
    const t = dt.types || [];
    const has = (x) => Array.prototype.indexOf.call(t, x) !== -1;
    if (has('Files')) return false;
    return has('text/uri-list') || has('text/plain');
  };
  const dropTargetEditable = (el) => !!(el && el.closest && el.closest(
    'input,textarea,select,[contenteditable=""],[contenteditable="true"]'
  ));
  const extractDropUrl = (dt) => {
    if (!dt) return '';
    try {
      let u = (dt.getData('text/uri-list') || '').split('\n').find((l) => l && l[0] !== '#');
      if (!u) u = dt.getData('text/plain') || '';
      u = (u || '').trim();
      if (/^https?:\/\//i.test(u)) return u;
      if (/^[a-z0-9.-]+\.[a-z]{2,}([\/?#]|$)/i.test(u)) return u;
    } catch (_) {}
    return '';
  };
  let __edgeTimer = 0;
  let __neuraInternalDrag = false;
  const __clearEdgeTimer = () => { if (__edgeTimer) { clearTimeout(__edgeTimer); __edgeTimer = 0; } };
  window.addEventListener('dragstart', function() { __neuraInternalDrag = true; }, true);
  window.addEventListener('dragover', function(e) {
    if (e.defaultPrevented || dropTargetEditable(e.target)) { __clearEdgeTimer(); return; }
    if (__neuraInternalDrag) { __clearEdgeTimer(); return; }
    if (!dragHasLink(e.dataTransfer)) { __clearEdgeTimer(); return; }
    e.preventDefault();
    try { e.dataTransfer.dropEffect = 'copy'; } catch (_) {}
    // Dwell at the left window edge for 2s during a drag → open the auto-hide sidebar so the
    // user can drop the link onto it. Rust ignores this unless the sidebar is auto-hide + closed.
    if (e.clientX <= 24) {
      if (!__edgeTimer) {
        __edgeTimer = setTimeout(function() { __edgeTimer = 0; post({cmd:'drag_edge_peek'}); }, 2000);
      }
    } else {
      __clearEdgeTimer();
    }
  }, false);
  window.addEventListener('drop', function(e) {
    __clearEdgeTimer();
    if (e.defaultPrevented || dropTargetEditable(e.target)) return;
    if (__neuraInternalDrag) return;
    const url = extractDropUrl(e.dataTransfer);
    if (!url) return;
    e.preventDefault();
    post({cmd:'open_in_new_tab', url});
  }, false);
  window.addEventListener('dragend', function() {
    __neuraInternalDrag = false;
    __clearEdgeTimer();
    post({cmd:'sidebar_auto_close'});
  }, false);

  const fsChange = function() {
    post({cmd: 'content_fullscreen_change', active: !!document.fullscreenElement || !!document.webkitFullscreenElement});
  };
  const fsNames = ['requestFullscreen', 'webkitRequestFullscreen', 'webkitRequestFullScreen', 'msRequestFullscreen'];
  const fsName = fsNames.find(name => typeof Element.prototype[name] === 'function');
  const reqFs = fsName ? Element.prototype[fsName] : null;
  if (reqFs && !Element.prototype.__neuraFs) {
    Element.prototype.__neuraFs = true;
    const reqWrap = function() {
      post({cmd: 'content_fullscreen_change', active: true});
      const p = reqFs.apply(this, arguments);
      return p;
    };
    fsNames.forEach(name => {
      if (typeof Element.prototype[name] === 'function') Element.prototype[name] = reqWrap;
    });
  }
  document.addEventListener('keydown', function(e) {
    if (e.key === 'F11') {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'toggle_fullscreen'});
      return;
    }
    if (e.key === 'Escape' && window.__neuraContentFullscreen) {
      e.preventDefault();
      e.stopPropagation();
      if (document.fullscreenElement && document.exitFullscreen) {
        try { document.exitFullscreen().catch(function() {}); } catch(_) {}
      }
      post({cmd:'content_fullscreen_change', active:false});
      return;
    }
    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl) return;
    if (e.key === 'Tab') {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'switch_tab_offset', delta:e.shiftKey ? -1 : 1});
      return;
    }
    const key = e.key.toLowerCase();
    if (key === 't' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'begin_spotlight'});
    } else if (key === 't' && e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'reopen_tab'});
    } else if (key === 'h' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'open_history_panel'});
    } else if (key === 'j' && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      post({cmd:'open_downloads_panel'});
    }
  }, true);
  window.addEventListener('wheel', function(e) {
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();
    e.stopImmediatePropagation();
    const delta = e.deltaY < 0 ? 0.1 : -0.1;
    post({cmd:'zoom_delta', delta});
  }, {capture:true, passive:false});

  // Signal Rust when cursor enters the content area so the auto-hide sidebar can close.
  // Chrome's SetWindowRgn clip means WM_MOUSELEAVE never fires when cursor moves from
  // sidebar (inside clip) to content area (outside clip but inside window rectangle).
  // This IPC is the only reliable signal source for that transition.
  // Also tracks cursor proximity to the window edge so resize handles work.
  const RESIZE_ZONE = 6; // px from edge to show resize cursor
  let __resizeEdge = null;
  let __sbThrottle = false;
  document.addEventListener('mousemove', function(e) {
    // Sidebar auto-close throttle
    if (!__sbThrottle) {
      __sbThrottle = true;
      try { window.ipc.postMessage('{"cmd":"sidebar_auto_close"}'); } catch(_) {}
      setTimeout(function() { __sbThrottle = false; }, 300);
    }
    // Window-edge resize cursor detection
    var W = document.documentElement.clientWidth;
    var H = document.documentElement.clientHeight;
    var x = e.clientX, y = e.clientY;
    var onR = x >= W - RESIZE_ZONE;
    var onB = y >= H - RESIZE_ZONE;
    // __vLeftEdge is the content WebView's left offset in window coords, injected by Rust
    // via evaluate_script on every layout change + page load. When a sidebar is visible
    // content_x > 0, so the content's left edge is NOT the window's left edge — adding
    // the offset makes onL false for all practical cursor positions, suppressing the
    // spurious ew-resize cursor that otherwise appears at the sidebar's right border.
    var __wle = (typeof window.__vLeftEdge === 'number') ? window.__vLeftEdge : 0;
    var onL = (x + __wle) <= RESIZE_ZONE;
    var edge = null;
    var cur  = '';
    if (onR && onB) { edge = 'bottomright'; cur = 'nwse-resize'; }
    else if (onL && onB) { edge = 'bottomleft'; cur = 'nesw-resize'; }
    else if (onR)  { edge = 'right';  cur = 'ew-resize'; }
    else if (onB)  { edge = 'bottom'; cur = 's-resize'; }
    else if (onL)  { edge = 'left';   cur = 'ew-resize'; }
    if (edge !== __resizeEdge) {
      __resizeEdge = edge;
      document.documentElement.style.cursor = cur;
    }
  }, {passive: true, capture: true});

  document.addEventListener('mousedown', function(e) {
    if (__resizeEdge && e.button === 0) {
      try { post({cmd: 'begin_resize', edge: __resizeEdge}); } catch(_) {}
      // Don't preventDefault — let the page also handle it normally
    }
    // A press inside the live page dismisses chrome popovers that are clipped to their
    // own rect (e.g. the download panel), giving them click-outside-to-close behaviour
    // without the chrome having to cover — and block — the whole page.
    try { post({cmd: 'content_pointer_down'}); } catch(_) {}
  }, {capture: true});

  const __neuraPostContextMenu = function(e) {
    const target = e.target;
    const el = target && target.nodeType === 1 ? target : (target && target.parentElement ? target.parentElement : null);
    const linkEl = el && el.closest ? el.closest('a[href]') : null;
    const linkUrl = linkEl ? (linkEl.href || '') : '';
    let imageSrc = '';
    const tag = el && el.tagName;
    if (tag === 'IMG') {
      imageSrc = el.src || el.currentSrc || '';
    } else if (tag === 'VIDEO' || tag === 'AUDIO') {
      imageSrc = el.src || el.currentSrc || '';
    }
    const sel = window.getSelection ? window.getSelection().toString().trim() : '';
    post({
      cmd: 'context_menu',
      x: e.clientX,
      y: e.clientY,
      link_url: linkUrl,
      image_src: imageSrc,
      selected_text: sel.length > 300 ? sel.slice(0, 300) : sel,
      page_url: location.href,
      can_back: history.length > 1
    });
  };
  document.addEventListener('contextmenu', function(e) {
    setTimeout(function() {
      if (e.defaultPrevented) return;
      __neuraPostContextMenu(e);
    }, 0);
  }, true);

  // Relay native fullscreen changes (e.g. YouTube player fullscreen button)
  // so Rust can resize the content WebView to fill the entire window.
  document.addEventListener('fullscreenchange', fsChange);
  document.addEventListener('webkitfullscreenchange', fsChange);

  // Audio/video playback detection — reports tab_audio_state to Rust via IPC so the
  // sidebar can show an animated speaker indicator and allow mute from the tab list.
  if (isTop) {
    let __audioPlaying = false;
    let __mediaActive = false;
    const __checkAudio = function() {
      const all = Array.from(document.querySelectorAll('audio,video'));
      const audible = all.some(function(m) { return !m.paused && !m.muted && !m.ended && m.readyState > 2; });
      const watching = all.some(function(m) {
        if (m.tagName !== 'VIDEO' || m.paused || m.ended || m.readyState < 3) return false;
        const r = m.getBoundingClientRect();
        return (r.width * r.height) >= 30000;
      });
      const active = audible || watching;
      if (audible !== __audioPlaying || active !== __mediaActive) {
        __audioPlaying = audible;
        __mediaActive = active;
        post({cmd:'tab_audio_state', playing: audible, active: active});
      }
    };
    // 'playing'/'waiting'/'loadeddata' matter for livestreams: a stream that rebuffers fires
    // 'waiting' then 'playing' (not 'play') when it resumes, so without these the keep-alive
    // state could stay stale across a stall.
    ['play','playing','pause','ended','waiting','volumechange','ratechange','loadeddata','emptied','seeked']
      .forEach(function(ev){ document.addEventListener(ev, __checkAudio, true); });
    setInterval(__checkAudio, 3000);
    window.__muteTab = function(muted) {
      document.querySelectorAll('audio,video').forEach(function(m) { m.muted = muted; });
    };
  }
})();
(() => {
  try { if (window.top !== window) return; } catch (_) { return; }
  if (window.__ventusPwd) return;
  const post = (o) => { try { window.ipc && window.ipc.postMessage(JSON.stringify(o)); } catch (_) {} };
  const vis = (el) => {
    if (!el) return false;
    const s = getComputedStyle(el);
    if (s.display === 'none' || s.visibility === 'hidden') return false;
    const r = el.getBoundingClientRect();
    return r.width > 4 && r.height > 4;
  };
  const pwFields = () => Array.prototype.slice.call(document.querySelectorAll('input[type=password]')).filter(vis);
  const userFor = (pw) => {
    const scope = pw.form || document;
    const inputs = Array.prototype.slice.call(scope.querySelectorAll('input'));
    const pi = inputs.indexOf(pw);
    for (let i = pi - 1; i >= 0; i--) {
      const el = inputs[i];
      const t = (el.type || 'text').toLowerCase();
      if ((t === 'text' || t === 'email' || t === 'tel') && vis(el)) return el;
    }
    const g = scope.querySelector('input[autocomplete="username"], input[type="email"], input[name*="user" i], input[name*="email" i], input[id*="user" i], input[id*="email" i]');
    return (g && vis(g)) ? g : null;
  };
  const setVal = (el, val) => {
    if (!el) return;
    try {
      const proto = el.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
      Object.getOwnPropertyDescriptor(proto, 'value').set.call(el, val);
    } catch (_) { el.value = val; }
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
  };
  window.__ventusPwd = {
    fill: (username, password) => {
      const pws = pwFields();
      if (!pws.length) return;
      const pw = pws[0];
      const u = userFor(pw);
      if (u && username) setVal(u, username);
      if (password) setVal(pw, password);
    }
  };
  let asked = '';
  const ask = () => {
    if (!pwFields().length) return;
    if (asked === location.origin) return;
    asked = location.origin;
    post({ cmd: 'pwd_fill_request', origin: location.origin });
  };
  const capture = () => {
    const pws = pwFields();
    if (!pws.length) return;
    const pw = pws.filter((p) => p.value)[0] || pws[0];
    if (!pw.value) return;
    const u = userFor(pw);
    post({ cmd: 'pwd_capture', origin: location.origin, username: u ? u.value : '', password: pw.value });
  };
  document.addEventListener('submit', capture, true);
  document.addEventListener('click', (e) => {
    const t = e.target;
    const b = t && t.closest && t.closest('button, input[type=submit], input[type=button], [role=button]');
    if (b) setTimeout(capture, 60);
  }, true);
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && e.target && e.target.tagName === 'INPUT') setTimeout(capture, 60);
  }, true);
  const start = () => ask();
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
  let n = 0;
  const iv = setInterval(() => { n++; ask(); if (n > 25) clearInterval(iv); }, 600);
})();
"#;
    format!("{ad_prefix}{privacy_prefix}{script}")
}

fn privacy_initialization_script(
    fingerprint: bool,
    fingerprint_seed: &str,
    x_login_compat: bool,
    strict: bool,
    site_permissions: &config::SitePermissionMap,
    default_permissions: &config::SitePermissions,
) -> String {
    let fingerprint_script = if fingerprint {
        let seed_json =
            serde_json::to_string(fingerprint_seed).unwrap_or_else(|_| "\"\"".to_string());
        r#"
(() => {
  if (window.__neuraPrivacyFp) return;
  window.__neuraPrivacyFp = true;
  const fpCompat = __X_LOGIN_COMPAT__;
  const fpHost = String(location.hostname || '').toLowerCase();
  const fpAuthHost = fpHost === 'x.com' || fpHost.endsWith('.x.com') || fpHost === 'twitter.com' || fpHost.endsWith('.twitter.com');
  const fpPath = String(location.pathname || '/').toLowerCase();
  const fpAuthPath = fpPath === '/' || fpPath === '/login' || fpPath.startsWith('/i/flow/') || fpPath.startsWith('/account/') || fpPath.startsWith('/oauth/') || fpPath.startsWith('/i/oauth');
  if (fpCompat && fpAuthHost && fpAuthPath && (location.protocol === 'https:' || location.protocol === 'http:')) return;
  const fpProfileSeed = __FINGERPRINT_SEED__;
  const fpHash = value => {
    let h = 2166136261 >>> 0;
    const s = String(value || '');
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 16777619) >>> 0;
    }
    return h >>> 0;
  };
  const fpSeed = fpHash(fpProfileSeed + '|' + location.origin);
  const fpDelta = i => {
    let h = (fpSeed ^ Math.imul(i + 0x9e3779b9, 2654435761)) >>> 0;
    h ^= h >>> 13;
    h = Math.imul(h, 0x85ebca6b) >>> 0;
    h ^= h >>> 16;
    return (h % 3) - 1;
  };
  const noise = data => {
    if (!data || !data.length) return;
    const step = Math.max(4, Math.floor(data.length / 64));
    for (let i = 0; i < data.length; i += step) {
      const n = fpDelta(i);
      data[i] = (data[i] + n + 256) & 255;
      if (i + 1 < data.length) data[i + 1] = (data[i + 1] - n + 256) & 255;
    }
  };
  const noiseImage = img => {
    try { if (img && img.data) noise(img.data); } catch (_) {}
    return img;
  };
  const cloneCanvas = canvas => {
    const c = document.createElement('canvas');
    c.width = canvas.width;
    c.height = canvas.height;
    const ctx = c.getContext('2d', {willReadFrequently:true});
    if (!ctx) return null;
    ctx.drawImage(canvas, 0, 0);
    try {
      const img = ctx.getImageData(0, 0, c.width, c.height);
      noiseImage(img);
      ctx.putImageData(img, 0, 0);
    } catch (_) {
      return null;
    }
    return c;
  };
  const patch = (obj, name, fn) => {
    try {
      const orig = obj && obj[name];
      if (typeof orig !== 'function' || orig.__neuraPatched) return;
      const wrapped = fn(orig);
      wrapped.__neuraPatched = true;
      Object.defineProperty(obj, name, {value: wrapped, configurable:true, writable:true});
    } catch (_) {}
  };
  patch(window.CanvasRenderingContext2D && window.CanvasRenderingContext2D.prototype, 'getImageData', orig => function() {
    return noiseImage(orig.apply(this, arguments));
  });
  patch(window.HTMLCanvasElement && window.HTMLCanvasElement.prototype, 'toDataURL', orig => function() {
    const c = cloneCanvas(this);
    return orig.apply(c || this, arguments);
  });
  patch(window.HTMLCanvasElement && window.HTMLCanvasElement.prototype, 'toBlob', orig => function() {
    const c = cloneCanvas(this);
    return orig.apply(c || this, arguments);
  });
  const patchGl = proto => {
    if (!proto) return;
    patch(proto, 'readPixels', orig => function() {
      const result = orig.apply(this, arguments);
      const pixels = arguments[6];
      if (pixels && typeof pixels.length === 'number') noise(pixels);
      return result;
    });
  };
  patchGl(window.WebGLRenderingContext && WebGLRenderingContext.prototype);
  patchGl(window.WebGL2RenderingContext && WebGL2RenderingContext.prototype);
})();
"#
        .replace("__FINGERPRINT_SEED__", &seed_json)
        .replace(
            "__X_LOGIN_COMPAT__",
            if x_login_compat { "true" } else { "false" },
        )
    } else {
        String::new()
    };
    let site_permissions_json =
        serde_json::to_string(site_permissions).unwrap_or_else(|_| "{}".to_string());
    let default_permissions_json =
        serde_json::to_string(default_permissions).unwrap_or_else(|_| "{}".to_string());
    let has_default = default_permissions_json != "{}";
    let strict_script = if strict || !site_permissions.is_empty() || has_default {
        r#"
(() => {
  if (window.__neuraPrivacyPerms) return;
  window.__neuraPrivacyPerms = true;
  const sitePermissions = __SITE_PERMISSIONS__;
  const defaultPermissions = __DEFAULT_PERMISSIONS__;
  const strictDefault = __STRICT__;
  const rules = (() => {
    try { return sitePermissions[location.origin] || {}; } catch (_) { return {}; }
  })();
  const decisive = v => (v === 'allow' || v === 'block') ? v : null;
  const askByDefault = key => key === 'microphone' || key === 'camera' || key === 'notifications';
  const action = key => decisive(rules[key]) || decisive(defaultPermissions[key]) || ((strictDefault && !askByDefault(key)) ? 'block' : 'ask');
  const isBlocked = key => action(key) === 'block';
  const nativeMask = (fn, name) => {
    try { if (name) Object.defineProperty(fn, 'name', {value: name, configurable: true}); } catch (_) {}
    try {
      const s = 'function ' + (name || fn.name || '') + '() { [native code] }';
      Object.defineProperty(fn, 'toString', {value: () => s, configurable: true, writable: true});
    } catch (_) {}
    return fn;
  };
  const blk = name => nativeMask(function () { return Promise.reject(new DOMException('Blocked by Ventus strict permissions', 'NotAllowedError')); }, name);
  try {
    if (navigator.geolocation && isBlocked('geolocation')) {
      navigator.geolocation.getCurrentPosition = nativeMask(function(_, err) {
        if (typeof err === 'function') setTimeout(() => err({code:1, message:'Blocked by Ventus strict permissions'}), 0);
      }, 'getCurrentPosition');
      navigator.geolocation.watchPosition = nativeMask(function(_, err) {
        if (typeof err === 'function') setTimeout(() => err({code:1, message:'Blocked by Ventus strict permissions'}), 0);
        return 0;
      }, 'watchPosition');
      navigator.geolocation.clearWatch = nativeMask(function() {}, 'clearWatch');
    }
  } catch (_) {}
  try {
    if (navigator.clipboard && isBlocked('clipboard')) {
      navigator.clipboard.read = blk('read');
      navigator.clipboard.readText = blk('readText');
    }
  } catch (_) {}
  try {
    if (window.queryLocalFonts && isBlocked('local_fonts')) window.queryLocalFonts = blk('queryLocalFonts');
  } catch (_) {}
  try {
    if (navigator.requestMIDIAccess && isBlocked('midi')) navigator.requestMIDIAccess = blk('requestMIDIAccess');
  } catch (_) {}
  try {
    if (window.getScreenDetails && isBlocked('window_management')) window.getScreenDetails = blk('getScreenDetails');
  } catch (_) {}
  try {
    if (window.showOpenFilePicker && isBlocked('file_system')) window.showOpenFilePicker = blk('showOpenFilePicker');
    if (window.showSaveFilePicker && isBlocked('file_system')) window.showSaveFilePicker = blk('showSaveFilePicker');
    if (window.showDirectoryPicker && isBlocked('file_system')) window.showDirectoryPicker = blk('showDirectoryPicker');
  } catch (_) {}
  try {
    if (window.Notification && Notification.requestPermission && isBlocked('notifications')) {
      Notification.requestPermission = nativeMask(function(cb) {
        if (typeof cb === 'function') setTimeout(() => cb('denied'), 0);
        return Promise.resolve('denied');
      }, 'requestPermission');
    }
  } catch (_) {}
})();
"#
        .replace("__SITE_PERMISSIONS__", &site_permissions_json)
        .replace("__DEFAULT_PERMISSIONS__", &default_permissions_json)
        .replace("__STRICT__", if strict { "true" } else { "false" })
    } else {
        String::new()
    };
    let ua_data_script = r#"
(() => {
  if (window.__neuraUAData) return;
  window.__neuraUAData = true;
  try {
    const ua = navigator.userAgent;
    const m = ua.match(/Chrome\/(\d+)/);
    const major = m ? m[1] : '149';
    const brands = [
      {brand: 'Not)A;Brand', version: '24'},
      {brand: 'Chromium', version: major},
      {brand: 'Google Chrome', version: major},
      {brand: 'Ventus', version: major}
    ];
    const fullVersionList = [
      {brand: 'Not)A;Brand', version: '24.0.0.0'},
      {brand: 'Chromium', version: major + '.0.0.0'},
      {brand: 'Google Chrome', version: major + '.0.0.0'},
      {brand: 'Ventus', version: major + '.0.0.0'}
    ];
    const ghev = function getHighEntropyValues(hints) {
      const r = {};
      const h = hints || [];
      if (h.includes('brands')) r.brands = brands;
      if (h.includes('mobile')) r.mobile = false;
      if (h.includes('platform')) r.platform = 'Windows';
      if (h.includes('platformVersion')) r.platformVersion = '10.0.0';
      if (h.includes('architecture')) r.architecture = 'x86';
      if (h.includes('bitness')) r.bitness = '64';
      if (h.includes('model')) r.model = '';
      if (h.includes('uaFullVersion')) r.uaFullVersion = major + '.0.0.0';
      if (h.includes('fullVersionList')) r.fullVersionList = fullVersionList;
      if (h.includes('wow64')) r.wow64 = false;
      return Promise.resolve(r);
    };
    try {
      Object.defineProperty(ghev, 'toString', {value: function() { return 'function getHighEntropyValues() { [native code] }'; }, configurable: true});
    } catch (_) {}
    const uaData = {brands, mobile: false, platform: 'Windows', getHighEntropyValues: ghev, toJSON() { return {brands, mobile: false, platform: 'Windows'}; }};
    try {
      Object.defineProperty(navigator, 'userAgentData', {get: () => uaData, configurable: true});
    } catch (_) {}
  } catch (_) {}
})();
"#;
    format!("{ua_data_script}{fingerprint_script}{strict_script}")
}

#[cfg(test)]
mod content_menu_tests {
    use super::*;

    fn script() -> String {
        let sites = config::SitePermissionMap::new();
        let defaults = config::SitePermissions::default();
        content_initialization_script(1.0, "", false, "test-seed", false, false, &sites, &defaults)
    }

    #[test]
    fn content_context_menu_respects_page_prevent_default() {
        let script = script();
        assert!(script.contains("setTimeout(function() {"));
        assert!(script.contains("if (e.defaultPrevented) return;"));
        assert!(script.contains("__neuraPostContextMenu(e);"));
        assert!(!script.contains("__neuraSiteCtx"));
        assert!(!script.contains("e.stopImmediatePropagation();\n      __neuraPostContextMenu(e);"));
        assert!(!script.contains("e.preventDefault();\n    e.stopPropagation();\n    const target = e.target;"));
    }
}
