pub fn chrome_html() -> String {
    let logo = crate::ui::assets::logo_data_url();
    let version = crate::version::APP_VERSION;
    let html = r##"<!DOCTYPE html>
<html lang="en" data-theme="dark">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Ventus</title>
<style>
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{
  --sidebar-w:240px;
  --toolbar-h:44px;
  --top-chrome-h:44px;
  --bookmarks-bar-h:30px;
  --ai-w:340px;
  --radius:8px;
  --radius-sm:5px;
  --radius-lg:12px;
  --transition:0.15s ease;
  --font:'Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',system-ui,sans-serif;

  /* Ventus dark theme — pure neutral dark grey */
  --bg:#0f0f0f;
  --bg-elevated:#1a1a1a;
  --bg-hover:#212121;
  --bg-active:#2a2a2a;
  --bg-card:#161616;
  --border:#2e2e2e;
  --border-subtle:#1e1e1e;
  --text:#e8e8e8;
  --text-muted:#888888;
  --text-dim:#484848;
  /* Blue-purple accent — matches the Ventus logo gradient */
  --accent:#6366f1;
  --accent-hover:#7c7ef8;
  --accent-dim:rgba(99,102,241,0.14);
  --accent-glow:rgba(99,102,241,0.32);
  --accent-gradient:linear-gradient(135deg,#3b82f6,#6366f1,#8b5cf6);
  /* AI panel */
  --ai-bg:#0f0f0f;
  --ai-panel:#141414;
  --ai-panel-strong:#1c1c1c;
  --ai-text:#ececec;
  --ai-muted:#888888;
  --ai-dim:#555555;
  --ai-line:rgba(99,102,241,0.12);
  --ai-soft:rgba(99,102,241,0.07);
  --ai-ring:rgba(99,102,241,0.36);
  --ai-ok:#6ee7b7;
  --ai-warn:#fbbf24;
  --ai-danger:#f87171;
  --ai-shadow:rgba(0,0,0,0.42);
  --danger:#ef4444;
  --danger-dim:rgba(239,68,68,0.14);
  --success:#34d399;
  --warning:#f59e0b;
  --shadow:0 4px 24px rgba(0,0,0,0.55);
  --shadow-sm:0 2px 10px rgba(0,0,0,0.4);
  /* New-tab */
  --nt-scrim:linear-gradient(rgba(18,18,18,0.10),rgba(18,18,18,0.10));
  --nt-vignette:linear-gradient(transparent,transparent);
  --nt-panel:rgba(14,14,14,0.72);
  --nt-panel-strong:rgba(14,14,14,0.88);
  --nt-glass:rgba(255,255,255,0.10);
  --nt-glass-strong:rgba(255,255,255,0.18);
  --nt-glass-border:rgba(255,255,255,0.16);
  --nt-white:#f0f0f0;
  --nt-soft:#cccccc;
  --nt-muted:#888888;
  --nt-news-white:#f0f0f0;
  --nt-news-soft:#cccccc;
  --nt-news-empty:linear-gradient(135deg,rgba(21,31,48,0.94),rgba(37,55,84,0.78));
  --nt-news-empty-overlay:radial-gradient(circle at top left,rgba(124,106,247,0.28),transparent 42%),linear-gradient(180deg,rgba(3,7,18,0.08),rgba(3,7,18,0.48));
  --nt-shadow:0 24px 80px rgba(0,0,0,0.5);
  --nt-hero-shadow:0 34px 120px rgba(0,0,0,0.42);
  --nt-clock-shadow:0 24px 90px rgba(0,0,0,0.54);
  --nt-search-bg:rgba(16,16,16,0.44);
  --nt-search-bg-focus:rgba(16,16,16,0.48);
  --nt-search-shadow:0 18px 60px rgba(0,0,0,0.34),inset 0 1px 0 rgba(255,255,255,0.12);
  --nt-search-focus-shadow:0 24px 70px rgba(0,0,0,0.42),0 0 0 4px rgba(255,255,255,0.08),inset 0 1px 0 rgba(255,255,255,0.16);
  --nt-focus-ring:rgba(255,255,255,0.34);
  --nt-top-bg:rgba(255,255,255,0.09);
  --nt-pill-shadow:0 14px 38px rgba(0,0,0,0.18),inset 0 1px 0 rgba(255,255,255,0.13);
  --nt-shortcut-bg:rgba(255,255,255,0.12);
  --nt-shortcut-bg-hover:rgba(255,255,255,0.20);
  --nt-shortcut-icon-bg:rgba(255,255,255,0.16);
  --nt-shortcut-shadow:0 16px 44px rgba(0,0,0,0.18),inset 0 1px 0 rgba(255,255,255,0.10);
  --nt-clock-muted:rgba(255,255,255,0.64);
  --scrollbar-w:4px;
  /* chrome shell */
  --chrome-bg:#141414;
  --chrome-border:rgba(255,255,255,0.06);
  /* modal / dialog surfaces */
  --modal-bg:#1a1a1a;
  --modal-bg-2:#222222;
  --modal-border:rgba(255,255,255,0.08);
  --overlay-bg:rgba(0,0,0,0.66);
  --overlay-bg-soft:rgba(0,0,0,0.48);
  --modal-shadow:0 32px 80px rgba(0,0,0,0.55),0 0 0 0.5px rgba(255,255,255,0.06);
  --popover-shadow:0 8px 32px rgba(0,0,0,0.5);
  --sidebar-shadow:2px 0 20px rgba(0,0,0,0.25);
  --sidebar-float-shadow:4px 0 28px rgba(0,0,0,0.3);
  --ws-glow-rgb:139,92,246;
  --ws-picker-rgb:139,92,246;
  --accent-text:#a5b4fc;
  --soft-btn-bg:rgba(255,255,255,0.06);
  --soft-btn-bg-hover:rgba(255,255,255,0.10);
  --sidebar-bottom-icon:rgba(255,255,255,0.72);
  --sidebar-bottom-icon-hover:rgba(255,255,255,0.94);
  --sidebar-bottom-icon-disabled:rgba(255,255,255,0.34);
  --load-ring:rgba(255,255,255,0.82);
  --load-ring-soft:rgba(255,255,255,0.30);
  --load-ring-faint:rgba(255,255,255,0.09);
}
[data-theme="light"]{
  /* Ventus light theme — soft paper-white with purple tint */
  --bg:#f7f8ff;
  --bg-elevated:#ffffff;
  --bg-hover:#eef0fd;
  --bg-active:#e5e8fa;
  --bg-card:#f3f4fe;
  --border:#d1d5f0;
  --border-subtle:#e5e8fa;
  --text:#1a1b2e;
  --text-muted:#5c6082;
  --text-dim:#9298ba;
  --accent:#5557e8;
  --accent-hover:#6366f1;
  --accent-dim:rgba(85,87,232,0.10);
  --accent-glow:rgba(85,87,232,0.22);
  --accent-gradient:linear-gradient(135deg,#2563eb,#5557e8,#7c3aed);
  --shadow:0 4px 20px rgba(30,40,100,0.10);
  --shadow-sm:0 2px 8px rgba(30,40,100,0.07);
  --ai-bg:#f0f2ff;
  --ai-panel:#e8ebfc;
  --ai-panel-strong:#dde2fa;
  --ai-text:#1a1b2e;
  --ai-muted:#5c6082;
  --ai-dim:#9298ba;
  --ai-line:rgba(85,87,232,0.14);
  --ai-soft:rgba(85,87,232,0.07);
  --ai-ring:rgba(85,87,232,0.30);
  --chrome-bg:#f2f3f7;
  --chrome-border:rgba(0,0,0,0.07);
  --modal-bg:#ffffff;
  --modal-bg-2:#f4f5fb;
  --modal-border:rgba(0,0,0,0.09);
  --overlay-bg:rgba(70,76,110,0.26);
  --overlay-bg-soft:rgba(70,76,110,0.18);
  --modal-shadow:0 24px 60px rgba(30,40,100,0.16),0 0 0 1px rgba(0,0,0,0.05);
  --popover-shadow:0 16px 42px rgba(30,40,100,0.14);
  --sidebar-shadow:2px 0 18px rgba(30,40,100,0.10);
  --sidebar-float-shadow:4px 0 28px rgba(30,40,100,0.14);
  --ws-glow-rgb:85,87,232;
  --ws-picker-rgb:85,87,232;
  --accent-text:var(--accent);
  --soft-btn-bg:rgba(0,0,0,0.04);
  --soft-btn-bg-hover:rgba(0,0,0,0.07);
  --sidebar-bottom-icon:rgba(26,27,46,0.66);
  --sidebar-bottom-icon-hover:rgba(26,27,46,0.9);
  --sidebar-bottom-icon-disabled:rgba(26,27,46,0.28);
  --load-ring:rgba(255,255,255,0.92);
  --load-ring-soft:rgba(255,255,255,0.44);
  --load-ring-faint:rgba(255,255,255,0.20);
  --nt-scrim:linear-gradient(rgba(18,18,18,0.10),rgba(18,18,18,0.10));
  --nt-vignette:linear-gradient(transparent,transparent);
  --nt-panel:rgba(255,255,255,0.72);
  --nt-panel-strong:rgba(255,255,255,0.92);
  --nt-glass:rgba(255,255,255,0.56);
  --nt-glass-strong:rgba(255,255,255,0.78);
  --nt-glass-border:rgba(85,87,232,0.18);
  --nt-white:#1a1b2e;
  --nt-soft:#5c6082;
  --nt-muted:#9298ba;
  --nt-shadow:0 24px 70px rgba(30,40,100,0.16);
  --nt-hero-shadow:0 30px 100px rgba(30,40,100,0.12);
  --nt-clock-shadow:0 18px 70px rgba(30,40,100,0.20);
  --nt-search-bg:rgba(255,255,255,0.42);
  --nt-search-bg-focus:rgba(255,255,255,0.48);
  --nt-search-shadow:0 18px 54px rgba(30,40,100,0.14),inset 0 1px 0 rgba(255,255,255,0.74);
  --nt-search-focus-shadow:0 24px 66px rgba(30,40,100,0.18),0 0 0 4px rgba(85,87,232,0.08),inset 0 1px 0 rgba(255,255,255,0.84);
  --nt-focus-ring:rgba(85,87,232,0.26);
  --nt-top-bg:rgba(255,255,255,0.62);
  --nt-pill-shadow:0 14px 36px rgba(30,40,100,0.10),inset 0 1px 0 rgba(255,255,255,0.70);
  --nt-shortcut-bg:rgba(255,255,255,0.58);
  --nt-shortcut-bg-hover:rgba(255,255,255,0.82);
  --nt-shortcut-icon-bg:rgba(255,255,255,0.70);
  --nt-shortcut-shadow:0 14px 36px rgba(30,40,100,0.10),inset 0 1px 0 rgba(255,255,255,0.62);
  --nt-clock-muted:rgba(26,27,46,0.62);
  --nt-news-empty:linear-gradient(135deg,rgba(255,255,255,0.92),rgba(238,240,253,0.86));
  --nt-news-empty-overlay:radial-gradient(circle at top left,rgba(85,87,232,0.13),transparent 42%),linear-gradient(180deg,rgba(255,255,255,0.02),rgba(255,255,255,0.28));
}

html,body{height:100%;overflow:hidden;user-select:none;-webkit-user-select:none;background:transparent}
body{font-family:var(--font);background:transparent;color:var(--text);font-size:13px;line-height:1.5}

/* scrollbar */
::-webkit-scrollbar{width:var(--scrollbar-w)}
::-webkit-scrollbar-track{background:transparent}
::-webkit-scrollbar-thumb{background:var(--border);border-radius:2px}
::-webkit-scrollbar-thumb:hover{background:var(--text-dim)}

/* layout — sidebar is a fixed overlay, not a grid column */
#app{
  display:grid;
  grid-template-columns:1fr;
  grid-template-rows:var(--top-chrome-h) 1fr;
  grid-template-areas:"toolbar" "content";
  height:100vh;
  width:100vw;
  background:transparent;
}
#app.ai-open{
  grid-template-columns:1fr var(--ai-w);
  grid-template-areas:"toolbar toolbar" "content ai";
}
#app.ai-open #top-chrome{grid-column:1/3}
#app.ai-open #ai-sidebar{display:flex;grid-area:ai}
#app.content-fullscreen #top-chrome,
#app.content-fullscreen #sidebar,
#app.content-fullscreen #ai-sidebar{display:none!important}

#top-chrome{
  grid-area:toolbar;
  display:flex;
  flex-direction:column;
  height:var(--top-chrome-h);
  background:var(--chrome-bg);
  z-index:100;
  min-width:0;
  overflow:visible;
  position:relative;
}

#toolbar{
  display:flex;
  align-items:center;
  gap:4px;
  padding:0 8px;
  background:var(--chrome-bg);
  border-bottom:1px solid var(--chrome-border);
  height:var(--toolbar-h);
  flex:0 0 var(--toolbar-h);
  z-index:101;
  min-width:0;
  overflow:hidden;
  position:relative;
}
#toolbar-nav,
#toolbar-actions{
  display:flex;
  align-items:center;
  gap:4px;
  flex-shrink:0;
  min-width:0;
  position:relative;
  z-index:2;
}
#toolbar-actions{justify-content:flex-end;align-self:stretch;margin-left:auto}
/* address bar floats absolutely centered across the full toolbar width */
#toolbar-url-area{
  position:absolute;
  left:0;right:0;top:0;bottom:0;
  display:flex;
  align-items:center;
  padding:0 8px;
  pointer-events:none;
  z-index:1;
}

#sidebar{
  position:fixed;left:0;top:var(--top-chrome-h);
  width:240px;height:calc(100vh - var(--top-chrome-h));
  display:flex;flex-direction:column;
  background:var(--chrome-bg);
  border-right:1px solid var(--chrome-border);
  overflow:hidden;z-index:150;
  transform:translateX(-240px);
  transition:transform 0.2s cubic-bezier(0.4,0,0.2,1),box-shadow 0.2s ease,width 0.2s ease;
  will-change:transform;
}
#sidebar::after{
  content:"";position:absolute;left:-110px;right:-110px;bottom:-190px;height:380px;
  background:radial-gradient(ellipse at bottom,rgba(var(--ws-glow-rgb),0.82),rgba(var(--ws-glow-rgb),0.36) 42%,rgba(var(--ws-glow-rgb),0.12) 64%,transparent 82%);
  opacity:1;filter:blur(34px);pointer-events:none;z-index:0;
  transition:background .2s ease,opacity .2s ease;
}
.sidebar-brand,.sb-viewport,.sb-bottom{position:relative;z-index:1}
/* Non-auto-hide: sidebar is always visible as an overlay */
#app:not(.sidebar-auto-hide) #sidebar{
  transform:translateX(0);
  box-shadow:var(--sidebar-shadow);
}
#app:not(.sidebar-auto-hide).sidebar-collapsed #sidebar{width:52px}

#content-area{
  grid-area:content;
  position:relative;
  overflow:hidden;
  background:transparent;
}

#ai-sidebar{
  display:none;
  flex-direction:column;
  background:
    radial-gradient(circle at 22% 0%,var(--ai-ring),transparent 30%),
    linear-gradient(180deg,var(--ai-bg),var(--ai-bg));
  border-left:1px solid var(--ai-line);
  width:var(--ai-w);
  z-index:90;
  color:var(--ai-text);
  min-width:0;
  overflow:hidden;
}

/* toolbar items */
.tb-btn{
  display:flex;align-items:center;justify-content:center;
  width:32px;height:32px;border-radius:var(--radius-sm);
  border:none;background:transparent;color:var(--text-muted);
  cursor:pointer;transition:background var(--transition),color var(--transition);
  flex-shrink:0;
}
.tb-btn:hover{background:var(--bg-hover);color:var(--text)}
.tb-btn:active{background:var(--bg-active)}
.tb-btn:disabled{opacity:0.3;cursor:default;pointer-events:none}
.tb-btn.active{color:var(--accent);background:var(--accent-dim)}

#address-bar{
  --address-fill:var(--bg);
  display:flex;
  align-items:center;
  gap:6px;
  background:var(--address-fill);
  border:1px solid var(--border);
  border-radius:999px;
  padding:0 10px;
  height:32px;
  transition:border-color 0.2s ease,box-shadow 0.2s ease,background 0.2s ease;
  cursor:text;
  overflow:hidden;
  pointer-events:auto;
  width:clamp(220px,calc(100% - 520px),560px);
  margin:0 auto;
  flex-shrink:0;
  position:relative;
  isolation:isolate;
}
#address-bar > *{position:relative;z-index:1}
#address-bar:focus-within{
  --address-fill:var(--bg-elevated);
  border-color:var(--accent);
  box-shadow:0 0 0 3px var(--accent-dim),0 0 12px var(--accent-glow);
  background:var(--address-fill);
}
@property --load-angle{
  syntax:"<angle>";
  inherits:false;
  initial-value:0deg;
}
#address-bar::before{
  content:"";
  position:absolute;
  inset:0;
  border-radius:inherit;
  pointer-events:none;
  opacity:0;
  border:1px solid transparent;
  background:
    linear-gradient(var(--address-fill),var(--address-fill)) padding-box,
    conic-gradient(from var(--load-angle),transparent 0deg,transparent 206deg,var(--load-ring-faint) 236deg,var(--load-ring-soft) 258deg,var(--load-ring) 282deg,var(--load-ring-soft) 306deg,var(--load-ring-faint) 330deg,transparent 360deg) border-box;
  transition:opacity 0.32s ease;
  z-index:0;
}
#address-bar.loading{border-color:transparent}
#address-bar.loading::before{
  opacity:1;
  animation:address-load-orbit 2.9s linear infinite;
}
#address-bar.loading.done{border-color:var(--border)}
#address-bar.loading.done:focus-within{border-color:var(--accent)}
#address-bar.loading.done::before{
  animation:none;
  opacity:0;
}
@keyframes address-load-orbit{
  to{--load-angle:360deg}
}
#address-bar .favicon{width:14px;height:14px;flex-shrink:0;border-radius:2px}
/* small icon buttons inside the address bar pill */
#address-bar .ab-icon-btn{
  width:22px;height:22px;border-radius:6px;
  display:flex;align-items:center;justify-content:center;
  border:none;background:transparent;color:var(--text-muted);
  cursor:pointer;transition:background var(--transition),color var(--transition);
  flex-shrink:0;
}
#address-bar .ab-icon-btn:hover{background:var(--bg-hover);color:var(--text)}
#address-bar .ab-icon-btn:active{background:var(--bg-active)}
#url-input{
  flex:1;border:none;background:transparent;color:var(--text);
  font-size:13px;outline:none;font-family:var(--font);
  min-width:0;
}
#url-input::placeholder{color:var(--text-dim)}
#url-input:focus::placeholder{color:var(--text-muted)}
#lock-icon{color:var(--success);flex-shrink:0}
#insecure-icon{color:var(--warning);flex-shrink:0}

.suggestions-panel{
  display:none;
  background:var(--bg-elevated);
  border:1px solid var(--border);
  border-radius:var(--radius);
  box-shadow:var(--shadow);
  overflow:hidden;
  z-index:260;
}
.suggestions-panel.open{display:block}
#url-suggestions{
  position:fixed;
  top:calc(var(--toolbar-h) + 6px);
  max-height:360px;
  overflow-y:auto;
  padding:6px;
}
.suggestion-section{
  padding:6px 8px 4px;
  color:var(--text-dim);
  font-size:10px;
  font-weight:700;
  text-transform:uppercase;
  letter-spacing:0.08em;
}
.suggestion-item{
  display:flex;
  align-items:center;
  gap:10px;
  min-height:36px;
  padding:7px 9px;
  border-radius:var(--radius-sm);
  cursor:pointer;
  color:var(--text);
  transition:background var(--transition),color var(--transition);
}
.suggestion-item:hover,.suggestion-item.highlighted{background:var(--bg-hover)}
.suggestion-item-icon{
  width:18px;
  height:18px;
  border-radius:5px;
  display:flex;
  align-items:center;
  justify-content:center;
  color:var(--text-muted);
  background:var(--bg);
  flex-shrink:0;
}
.suggestion-item-info{min-width:0;flex:1}
.suggestion-item-title{
  font-size:12px;
  font-weight:500;
  overflow:hidden;
  text-overflow:ellipsis;
  white-space:nowrap;
}
.suggestion-item-sub{
  font-size:11px;
  color:var(--text-muted);
  overflow:hidden;
  text-overflow:ellipsis;
  white-space:nowrap;
}
.suggestion-item-kbd{
  color:var(--text-dim);
  font-size:10px;
  flex-shrink:0;
}

.tb-sep{width:1px;height:20px;background:var(--border-subtle);flex-shrink:0;margin:0 2px}

/* ── More menu ─────────────────────────────────────────────────────────────── */
#btn-more{position:relative}
#more-btn-badge{
  position:absolute;top:4px;right:4px;
  width:7px;height:7px;border-radius:50%;background:var(--accent);
  display:none;pointer-events:none;border:1.5px solid var(--chrome-bg);
}
#btn-more.has-downloads #more-btn-badge{display:block}
#more-menu{
  position:fixed;top:0;right:0;
  z-index:290;
  min-width:230px;
  background:var(--bg-elevated);
  border:1px solid var(--border);
  border-radius:var(--radius-lg);
  padding:5px;
  box-shadow:var(--popover-shadow);
  display:none;
  flex-direction:column;
}
#more-menu.open{display:flex}
.more-item{
  display:flex;align-items:center;gap:10px;
  padding:7px 10px;border-radius:6px;
  background:none;border:none;cursor:pointer;
  color:var(--text);font-size:13px;text-align:left;
  width:100%;transition:background var(--transition);
  font-family:inherit;
}
.more-item:hover{background:var(--bg-hover)}
.more-item-icon{flex-shrink:0;color:var(--text-muted);display:flex;align-items:center}
.more-item-label{flex:1}
.more-item-kbd{
  font-size:10px;color:var(--text-dim);
  background:var(--bg-active);border:1px solid var(--border-subtle);
  border-radius:3px;padding:1px 5px;flex-shrink:0;
}
.more-sep{height:1px;background:var(--border-subtle);margin:3px 0}
.more-zoom-row{
  display:flex;align-items:center;justify-content:space-between;
  padding:7px 10px;border-radius:6px;
}
.more-zoom-row:hover{background:var(--bg-hover)}
.more-zoom-label{font-size:13px;color:var(--text-muted);flex:1}
.more-zoom-controls{display:flex;align-items:center;gap:4px}
.more-zoom-btn{
  width:26px;height:26px;border-radius:5px;border:1px solid var(--border);
  background:var(--bg-active);color:var(--text);font-size:15px;
  cursor:pointer;display:flex;align-items:center;justify-content:center;
  transition:background var(--transition);line-height:1;font-family:inherit;
}
.more-zoom-btn:hover{background:var(--bg-hover)}
#more-zoom-pct{
  font-size:12px;min-width:38px;text-align:center;
  color:var(--text);cursor:pointer;font-variant-numeric:tabular-nums;
  padding:3px 4px;border-radius:4px;transition:background var(--transition);
}
#more-zoom-pct:hover{background:var(--bg-active)}

/* ── Tab audio indicator ───────────────────────────────────────────────────── */
.tab-audio-btn{
  width:18px;height:18px;border-radius:4px;
  display:flex;align-items:center;justify-content:center;
  border:none;background:transparent;color:var(--accent);
  cursor:pointer;flex-shrink:0;opacity:0;transition:all var(--transition);
}
.tab-item:hover .tab-audio-btn,
.tab-item.audio-playing .tab-audio-btn,
.tab-item.tab-muted .tab-audio-btn{opacity:1}
.tab-item.tab-muted .tab-audio-btn{color:var(--text-dim)}
.tab-audio-btn:hover{background:var(--accent-dim)}
@keyframes speaker-pulse{
  0%,100%{transform:scale(1);opacity:1}
  50%{transform:scale(1.2);opacity:.75}
}
.tab-item.audio-playing .tab-audio-btn svg{animation:speaker-pulse 1.6s ease-in-out infinite}

@media (max-width: 900px) {
  #toolbar{gap:2px;padding:0 4px}
  #toolbar-nav,#toolbar-actions{gap:2px}
}
@media (max-width: 740px) {
  #btn-forward{display:none}
}
@media (max-width: 600px) {
  #btn-ai{display:none}
  .tb-btn{width:28px}
}
@media (max-width: 500px) {
  #btn-reload{display:none}
  #address-bar{padding:0 6px;gap:4px}
}
@media (max-width: 420px) {
  #btn-back{display:none}
  #toolbar-nav .tb-sep{display:none}
}

/* ── Window edge resize handles ───────────────────────────────────────────────
   Transparent strips anchored to the window edges inside the chrome clip region.
   Mousedown fires send('BeginResize',{edge}) → Rust → ReleaseCapture + WM_NCLBUTTONDOWN.
*/
.resize-handle{
  position:fixed;z-index:9999;
  background:transparent;
}
.resize-handle[data-edge="left"]  {left:0;top:0;width:5px;height:100%;cursor:ew-resize}
.resize-handle[data-edge="right"] {right:0;top:0;width:5px;height:100%;cursor:ew-resize}
.resize-handle[data-edge="bottom"]{bottom:0;left:0;width:100%;height:5px;cursor:s-resize}
.resize-handle[data-edge="topleft"]    {left:0;top:0;width:12px;height:12px;cursor:nwse-resize}
.resize-handle[data-edge="topright"]   {right:0;top:0;width:12px;height:12px;cursor:nesw-resize}
.resize-handle[data-edge="bottomleft"] {left:0;bottom:0;width:12px;height:12px;cursor:nesw-resize}
.resize-handle[data-edge="bottomright"]{right:0;bottom:0;width:12px;height:12px;cursor:nwse-resize}

/* ── Spotlight calculator card ────────────────────────────────────────────────
   Shown above search results when the query is a pure math expression.
*/
.tsp-calc-card{
  display:flex;flex-direction:column;align-items:flex-end;
  background:color-mix(in srgb,var(--accent) 8%,var(--bg-elevated));
  border:1px solid color-mix(in srgb,var(--accent) 30%,var(--border));
  border-radius:12px;padding:14px 18px 12px;margin:0 8px 6px;
}
.tsp-calc-expr{
  font-size:12px;color:var(--text-muted);
  font-family:var(--font-mono,monospace);word-break:break-all;margin-bottom:6px;
}
.tsp-calc-result{
  font-size:30px;font-weight:700;color:var(--text);letter-spacing:-0.5px;
  font-variant-numeric:tabular-nums;
}
.tsp-calc-copy-hint{
  font-size:10px;color:var(--text-dim);margin-top:4px;opacity:.7;
}
.tsp-conv-type{
  font-size:10px;font-weight:600;text-transform:uppercase;letter-spacing:.5px;
  color:var(--accent);opacity:.8;margin-bottom:4px;align-self:flex-start;
}
.tsp-conv-disclaimer{
  font-size:10px;color:var(--text-dim);margin-top:2px;opacity:.55;align-self:flex-start;
}
/* ── Region settings ──────────────────────────────────────────────────────── */
.region-preview-card{
  background:color-mix(in srgb,var(--accent) 6%,var(--bg-elevated));
  border:1px solid color-mix(in srgb,var(--accent) 20%,var(--border));
  border-radius:10px;padding:14px 16px;display:flex;flex-direction:column;gap:10px;
}
.region-preview-row{display:flex;align-items:center;justify-content:space-between;gap:12px}
.region-preview-lbl{font-size:12px;color:var(--text-muted)}
.region-badge{
  font-size:11px;font-weight:600;
  background:color-mix(in srgb,var(--accent) 15%,transparent);
  color:var(--accent);padding:2px 8px;border-radius:4px;
  text-transform:uppercase;letter-spacing:0.4px;white-space:nowrap;
}
.region-example{font-size:12px;color:var(--text);font-style:italic}
.region-filter{margin-bottom:6px}

.sidebar-brand{
  display:flex;align-items:center;justify-content:space-between;
  padding:11px 10px 11px 14px;
  border-bottom:1px solid var(--border-subtle);
  flex-shrink:0;
}
.sidebar-brand-left{
  display:flex;align-items:center;gap:9px;min-width:0;flex:1;overflow:hidden;
  cursor:pointer;border-radius:7px;padding:2px 4px;margin:-2px -4px;
  transition:background var(--transition);
}
.sidebar-brand-left:hover{background:var(--bg-hover)}
.sidebar-brand-logo{width:24px;height:24px;object-fit:contain;flex-shrink:0}
.sidebar-brand-info{display:flex;flex-direction:column;min-width:0;overflow:hidden}
.sidebar-brand-name{font-size:13px;font-weight:700;color:var(--text);letter-spacing:-0.3px;white-space:nowrap;overflow:hidden;line-height:1.2}
.sidebar-brand-add{
  width:26px;height:26px;border:none;border-radius:8px;
  background:transparent;color:var(--text-dim);cursor:pointer;
  display:flex;align-items:center;justify-content:center;
  transition:background var(--transition),color var(--transition);
  flex-shrink:0;
}
.sidebar-brand-add:hover{background:var(--bg-hover);color:var(--text)}
.sidebar-brand-add:active{background:var(--bg-active)}
#app.sidebar-collapsed .sidebar-brand{justify-content:center;padding:10px 0;gap:0}
#app.sidebar-collapsed .sidebar-brand-left{justify-content:center;gap:0}
#app.sidebar-collapsed .sidebar-brand-info{display:none}
#app.sidebar-collapsed .sidebar-brand-add{display:none}
#app.sidebar-collapsed .sidebar-brand-logo{width:28px;height:28px}

.newtab-logo{width:56px;height:56px;object-fit:contain}

/* ── Workspace dot hover popover ───────────────────────────── */
.sb-ws-popover{
  position:fixed;z-index:295;
  background:var(--bg-elevated);border:1px solid var(--border);
  border-radius:var(--radius-lg);
  box-shadow:0 8px 24px rgba(0,0,0,0.18),0 2px 8px rgba(0,0,0,0.12);
  padding:10px 12px;
  flex-direction:column;align-items:center;gap:5px;
  min-width:150px;max-width:190px;
  opacity:0;transform:translateY(4px) scale(0.97);
  transition:opacity 0.15s ease,transform 0.18s cubic-bezier(0.34,1.15,0.64,1);
  pointer-events:none;display:none;
}
.sb-ws-popover.visible{
  opacity:1;transform:translateY(0) scale(1);
  pointer-events:auto;
}
.sb-ws-pop-avatar{
  width:34px;height:34px;border-radius:10px;
  display:flex;align-items:center;justify-content:center;
  font-size:13px;font-weight:700;color:#fff;
  letter-spacing:0.02em;flex-shrink:0;margin-bottom:1px;
}
.sb-ws-pop-name{
  font-size:12px;font-weight:700;color:var(--text);
  text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;
  max-width:100%;
}
.sb-ws-pop-count{
  font-size:10px;color:var(--text-dim);text-align:center;margin-top:-2px;
}
.sb-ws-pop-note{
  display:none;
  font-size:10px;line-height:1.35;color:var(--accent-text);text-align:center;
  background:var(--accent-dim);border:1px solid var(--ai-line);
  border-radius:8px;padding:6px 8px;margin-top:3px;width:100%;
}
.sb-ws-pop-actions{display:flex;gap:5px;width:100%;margin-top:4px}
.sb-ws-pop-btn{
  flex:1;padding:5px 0;border:none;border-radius:6px;
  font-size:10px;font-weight:600;cursor:pointer;
  background:var(--bg-hover);color:var(--text);
  transition:all 0.14s ease;
}
.sb-ws-pop-btn:hover{background:var(--bg-active);color:var(--text)}
.sb-ws-pop-btn.danger:hover{background:rgba(239,68,68,0.12);color:#f87171}
.sb-ws-pop-btn:disabled{opacity:.35;cursor:not-allowed;pointer-events:none}

/* ── Pages viewport ─────────────────────────────────────────── */
.sb-viewport{flex:1;overflow:hidden;position:relative}
.sb-page{
  height:100%;overflow-y:auto;overflow-x:hidden;
  padding:4px 6px;display:flex;flex-direction:column;gap:1px;
  scrollbar-width:thin;scrollbar-color:var(--border) transparent;
  will-change:transform,opacity;
}
.sb-page::-webkit-scrollbar{width:3px}
.sb-page::-webkit-scrollbar-track{background:transparent}
.sb-page::-webkit-scrollbar-thumb{background:var(--border);border-radius:99px}

/* ── Sidebar bottom bar ─────────────────────────────────────── */
.sb-bottom{
  display:flex;align-items:center;justify-content:space-between;
  padding:0 8px;border-top:1px solid var(--border-subtle);
  flex-shrink:0;height:42px;gap:4px;
}
.sb-bottom-btn{
  width:28px;height:28px;border:none;border-radius:8px;
  background:transparent;color:var(--sidebar-bottom-icon);cursor:pointer;
  display:flex;align-items:center;justify-content:center;
  transition:all 0.15s ease;flex-shrink:0;
}
.sb-bottom-btn:hover{background:var(--bg-hover);color:var(--sidebar-bottom-icon-hover)}
.sb-ws-nav{
  flex:1 1 auto;min-width:0;overflow:hidden;
  display:flex;align-items:center;justify-content:center;gap:3px;
}
.sb-ws-nav-btn{
  width:18px;height:18px;border:none;border-radius:5px;padding:0;
  background:transparent;color:var(--sidebar-bottom-icon);cursor:pointer;
  display:flex;align-items:center;justify-content:center;flex-shrink:0;
  opacity:0;pointer-events:none;
  transition:opacity 0.2s ease,background 0.15s ease,color 0.15s ease;
}
.sb-ws-nav-btn.visible{opacity:1;pointer-events:auto}
.sb-ws-nav-btn:hover:not(:disabled){background:var(--bg-hover);color:var(--sidebar-bottom-icon-hover)}
.sb-ws-nav-btn:disabled{opacity:1;color:var(--sidebar-bottom-icon-disabled);cursor:not-allowed}
.sb-ws-dots{
  flex:1 1 auto;min-width:0;max-width:100%;
  display:flex;align-items:center;justify-content:center;gap:4px;
  overflow-x:auto;overflow-y:hidden;scrollbar-width:none;scroll-behavior:smooth;
  overscroll-behavior-x:contain;padding:0 2px;
  -webkit-mask-image:linear-gradient(90deg,transparent,#000 10px,#000 calc(100% - 10px),transparent);
  mask-image:linear-gradient(90deg,transparent,#000 10px,#000 calc(100% - 10px),transparent);
}
.sb-ws-dots.scrollable{justify-content:flex-start}
.sb-ws-dots::-webkit-scrollbar{display:none}
.ws-dot{
  width:26px;height:26px;border-radius:999px;cursor:pointer;flex-shrink:0;
  background:transparent;border:none;padding:0;position:relative;
  display:flex;align-items:center;justify-content:center;
  font-size:14px;line-height:1;
  transition:transform 0.2s cubic-bezier(0.34,1.15,0.64,1);
}
.ws-dot-icon{opacity:1;transform:scale(1);transition:opacity 0.16s ease,transform 0.2s cubic-bezier(0.34,1.15,0.64,1);filter:drop-shadow(0 1px 4px var(--ai-shadow))}
.ws-dot-mark{position:absolute;width:5px;height:5px;border-radius:999px;background:var(--text);opacity:0;transform:scale(.7);transition:opacity 0.16s ease,transform 0.2s cubic-bezier(0.34,1.15,0.64,1)}
.ws-dot.active .ws-dot-icon{opacity:1;transform:scale(1.05)}
.ws-dot.muted .ws-dot-icon{opacity:0;transform:scale(.68)}
.ws-dot.muted .ws-dot-mark{opacity:.78;transform:scale(1)}
.ws-dot.muted:hover .ws-dot-icon{opacity:1;transform:scale(1)}
.ws-dot.muted:hover .ws-dot-mark{opacity:0;transform:scale(.45)}
.ws-dot:hover{transform:scale(1.08)}

/* ── Emoji picker (workspace modal) ─────────────────────────── */
.ws-emoji-row{display:flex;align-items:flex-start;gap:12px;width:100%}
.ws-emoji-preview{
  width:52px;height:52px;border-radius:14px;flex-shrink:0;
  display:flex;align-items:center;justify-content:center;
  font-size:26px;line-height:1;
  background:var(--modal-bg-2);border:1.5px solid var(--modal-border);
}
.ws-emoji-grid{
  display:grid;grid-template-columns:repeat(8,1fr);
  gap:3px;flex:1;
}
.ws-emoji-opt{
  aspect-ratio:1;border:2px solid transparent;border-radius:7px;
  background:transparent;font-size:17px;cursor:pointer;
  display:flex;align-items:center;justify-content:center;
  transition:background 0.1s ease;padding:0;line-height:1;
}
.ws-emoji-opt:hover{background:var(--bg-hover)}
.ws-emoji-opt.selected{background:var(--accent-dim);border-color:var(--accent)}
.ws-color-row{display:flex;align-items:center;gap:12px;width:100%}
.ws-color-preview{
  width:52px;height:42px;border-radius:14px;flex-shrink:0;
  background:linear-gradient(135deg,rgba(var(--ws-picker-rgb),0.92),rgba(var(--ws-picker-rgb),0.34));
  border:1px solid var(--modal-border);box-shadow:0 14px 32px rgba(var(--ws-picker-rgb),0.24);
}
.ws-color-main{flex:1;display:flex;align-items:center;gap:8px;min-width:0}
.ws-color-swatches{display:flex;flex-wrap:wrap;gap:6px;flex:1}
.ws-color-opt{
  width:24px;height:24px;border-radius:999px;border:2px solid transparent;
  background:rgb(var(--swatch-rgb));cursor:pointer;padding:0;box-shadow:0 6px 16px rgba(var(--swatch-rgb),0.22);
  transition:transform var(--transition),border-color var(--transition),box-shadow var(--transition);
}
.ws-color-opt:hover{transform:translateY(-1px);box-shadow:0 8px 20px rgba(var(--swatch-rgb),0.30)}
.ws-color-opt.selected{border-color:var(--text);box-shadow:0 0 0 3px var(--accent-dim),0 8px 20px rgba(var(--swatch-rgb),0.30)}
#ws-color-input{
  width:34px;height:34px;border:none;background:transparent;cursor:pointer;
  padding:0;flex-shrink:0;
}

/* ── Preserve workspace modal avatar in dialogs ─────────────── */
.workspace-avatar{
  width:22px;height:22px;border-radius:6px;
  display:flex;align-items:center;justify-content:center;
  font-size:10px;font-weight:700;color:#fff;flex-shrink:0;
  text-transform:uppercase;letter-spacing:0.02em;
}
#toolbar-incognito-badge{display:none;align-items:center;gap:4px;font-size:10px;font-weight:700;color:#fff;background:linear-gradient(135deg,#3730a3,#6366f1);border-radius:5px;padding:2px 7px 2px 5px;flex-shrink:0;cursor:default;white-space:nowrap;letter-spacing:0.03em;box-shadow:0 1px 4px rgba(99,102,241,0.4)}
#toolbar-incognito-badge.visible{display:flex}
#toolbar-incognito-badge svg{flex-shrink:0}

#workspace-modal{
  display:none;position:fixed;inset:0;z-index:285;
  align-items:center;justify-content:center;
  background:var(--overlay-bg);
  -webkit-backdrop-filter:blur(20px) saturate(150%);
  backdrop-filter:blur(20px) saturate(150%);
}
#workspace-modal.open{display:flex}
.workspace-dialog{
  width:min(440px,calc(100vw - 32px));
  background:var(--modal-bg);border:1px solid var(--modal-border);
  border-radius:20px;
  box-shadow:var(--modal-shadow);
  padding:28px;display:flex;flex-direction:column;gap:20px;
  animation:ventus-scale-in 0.2s cubic-bezier(0.16,1,0.3,1);
}
[data-theme="light"] .workspace-dialog{box-shadow:0 24px 60px rgba(30,40,100,0.16),0 0 0 1px rgba(0,0,0,0.05)}
.workspace-dialog-head{display:flex;align-items:center;justify-content:space-between;gap:12px}
.workspace-dialog-title{font-size:17px;font-weight:700;color:var(--text);letter-spacing:-0.2px}
.workspace-dialog-close{
  width:30px;height:30px;border:none;border-radius:8px;
  background:transparent;color:var(--text-muted);cursor:pointer;
  display:flex;align-items:center;justify-content:center;
  transition:background var(--transition),color var(--transition);
}
.workspace-dialog-close:hover{background:var(--soft-btn-bg-hover);color:var(--text)}
.workspace-form{display:flex;flex-direction:column;gap:16px}
.workspace-field{display:flex;flex-direction:column;gap:7px}
.workspace-field label{font-size:11px;font-weight:600;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.08em}
.workspace-field input{
  width:100%;height:42px;border:1px solid var(--modal-border);border-radius:10px;
  background:var(--modal-bg-2);color:var(--text);font-family:var(--font);
  font-size:14px;outline:none;padding:0 14px;
  transition:border-color var(--transition),box-shadow var(--transition);
}
.workspace-field input:focus{border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-dim)}
.workspace-field input::placeholder{color:var(--text-dim)}
.workspace-error{min-height:14px;color:var(--danger);font-size:11px;font-weight:600}
.workspace-actions{display:flex;justify-content:flex-end;gap:8px}
.workspace-btn{
  height:38px;padding:0 18px;border-radius:10px;
  border:1px solid var(--modal-border);background:transparent;
  color:var(--text-muted);font-family:var(--font);font-size:13px;font-weight:500;
  cursor:pointer;display:inline-flex;align-items:center;gap:6px;
  transition:all var(--transition);
}
.workspace-btn:hover{background:var(--soft-btn-bg);color:var(--text)}
.workspace-btn-primary{
  border:none;background:var(--accent-gradient);color:#fff;font-weight:600;
  letter-spacing:0.01em;box-shadow:0 2px 12px var(--accent-glow);
}
.workspace-btn-primary:hover{opacity:0.88;transform:translateY(-1px)}
.workspace-field input[type="checkbox"]{
  appearance:none;-webkit-appearance:none;
  width:18px;height:18px;min-width:18px;border:1.5px solid var(--modal-border);
  border-radius:5px;background:var(--modal-bg-2);cursor:pointer;
  position:relative;transition:all var(--transition);margin:0;
}
.workspace-field input[type="checkbox"]:checked{background:var(--accent);border-color:var(--accent)}
.workspace-field input[type="checkbox"]:checked::after{
  content:'';display:block;position:absolute;
  left:5px;top:1px;width:5px;height:9px;
  border:2px solid #fff;border-top:none;border-left:none;
  transform:rotate(45deg);
}

#workspace-delete-modal{
  display:none;position:fixed;inset:0;z-index:286;
  align-items:center;justify-content:center;
  background:var(--overlay-bg);
  -webkit-backdrop-filter:blur(20px) saturate(150%);
  backdrop-filter:blur(20px) saturate(150%);
}
#workspace-delete-modal.open{display:flex}
.delete-dialog{
  width:min(390px,calc(100vw - 36px));
  background:var(--modal-bg);border:1px solid var(--modal-border);
  border-radius:20px;
  box-shadow:var(--modal-shadow);
  padding:28px;
  display:flex;flex-direction:column;align-items:center;text-align:center;
  gap:16px;
  animation:ventus-scale-in 0.2s cubic-bezier(0.16,1,0.3,1);
}
[data-theme="light"] .delete-dialog{box-shadow:0 24px 60px rgba(30,40,100,0.16),0 0 0 1px rgba(0,0,0,0.05)}
.delete-icon{
  width:44px;height:44px;border-radius:14px;
  display:flex;align-items:center;justify-content:center;
  color:#ff6961;background:rgba(255,105,97,0.14);
}
.delete-title{font-size:18px;font-weight:750;color:var(--text);letter-spacing:-0.2px}
.delete-copy{font-size:12px;color:var(--text-muted);line-height:1.5;max-width:310px}
.delete-copy strong{color:var(--text);font-weight:700}
.delete-actions{
  width:100%;display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-top:4px;
}
.delete-btn{
  height:36px;border-radius:10px;border:1px solid var(--border);
  background:var(--soft-btn-bg);color:var(--text);
  font-family:var(--font);font-size:13px;font-weight:700;cursor:pointer;
}
.delete-btn:hover{background:var(--soft-btn-bg-hover)}
.delete-btn-danger{border-color:rgba(255,105,97,0.3);background:#ff453a;color:#fff}
.delete-btn-danger:hover{background:#ff5c52}

/* legacy alias — sb-page now handles this */
.sidebar-tabs{flex:1;overflow:hidden}
.tab-item{
  display:flex;align-items:center;gap:9px;
  padding:7px 8px;border-radius:8px;
  cursor:pointer;transition:background var(--transition);
  position:relative;min-height:36px;
}
.tab-item:hover{background:var(--bg-hover)}
.tab-item.active{background:var(--accent-dim)}
.tab-item.active .tab-title{color:var(--accent-text)}
.tab-item.pinned::before{
  content:'';position:absolute;top:7px;right:7px;
  width:4px;height:4px;border-radius:50%;background:var(--accent);
}
.tab-favicon{width:14px;height:14px;border-radius:3px;flex-shrink:0;object-fit:contain}
.tab-favicon.loading{opacity:0.4}
.tab-info{flex:1;min-width:0}
.tab-title{
  font-size:12px;font-weight:500;color:var(--text);
  overflow:hidden;text-overflow:ellipsis;white-space:nowrap;
  line-height:1.3;
}
.tab-url{
  font-size:10.5px;color:var(--text-dim);
  overflow:hidden;text-overflow:ellipsis;white-space:nowrap;
  margin-top:1px;
}
.tab-close{
  width:18px;height:18px;border-radius:4px;
  display:flex;align-items:center;justify-content:center;
  border:none;background:transparent;color:var(--text-dim);
  cursor:pointer;flex-shrink:0;opacity:0;transition:all var(--transition);
}
.tab-item:hover .tab-close{opacity:1}
.tab-close:hover{background:rgba(239,68,68,0.15);color:#f87171}
.tab-item.loading{animation:tab-chip-pulse 1.8s ease-in-out infinite}
.tab-item.active.loading{animation:tab-chip-pulse-active 1.8s ease-in-out infinite}

#ai-header{
  display:flex;align-items:center;justify-content:space-between;gap:12px;
  padding:18px 16px 10px;flex-shrink:0;
}
.ai-top-left,.ai-top-right{display:flex;align-items:center;gap:8px;min-width:0}
.ai-icon-btn{
  width:30px;height:30px;border-radius:10px;border:1px solid transparent;
  background:transparent;color:var(--ai-muted);cursor:pointer;
  display:flex;align-items:center;justify-content:center;
  transition:background var(--transition),color var(--transition),border-color var(--transition),transform var(--transition);
}
.ai-icon-btn:hover{background:var(--ai-soft);color:var(--ai-text);border-color:var(--ai-line)}
.ai-icon-btn:active{transform:scale(.96)}
/* Custom provider dropdown (replaces native select) */
.ai-provider-dd{position:relative}
.ai-provider-dd-btn{
  height:30px;border-radius:10px;border:1px solid var(--ai-line);
  background:var(--ai-soft);color:var(--ai-text);
  padding:0 10px 0 12px;font-size:12px;font-weight:650;font-family:var(--font);
  cursor:pointer;display:flex;align-items:center;gap:7px;white-space:nowrap;
  transition:background var(--transition),border-color var(--transition),color var(--transition);
}
.ai-provider-dd-btn:hover,.ai-provider-dd.open .ai-provider-dd-btn{
  background:var(--ai-panel);border-color:var(--accent);color:var(--ai-text);
}
.ai-provider-dd-chevron{
  transition:transform .18s ease;flex-shrink:0;opacity:.7;
}
.ai-provider-dd.open .ai-provider-dd-chevron{transform:rotate(180deg)}
.ai-provider-dd-menu{
  /* position:fixed so overflow:hidden on #ai-sidebar doesn't clip the panel */
  position:fixed;top:0;right:0;z-index:8000;
  min-width:150px;background:var(--bg-elevated);border:1px solid var(--border);
  border-radius:12px;padding:5px;
  box-shadow:var(--popover-shadow);
  opacity:0;transform:translateY(-6px) scale(.97);pointer-events:none;
  transition:opacity .15s ease,transform .15s ease;
}
.ai-provider-dd.open .ai-provider-dd-menu{
  opacity:1;transform:translateY(0) scale(1);pointer-events:all;
}
.ai-provider-dd-item{
  padding:8px 12px;border-radius:8px;cursor:pointer;
  font-size:13px;font-weight:550;color:var(--text);
  transition:background var(--transition);display:flex;align-items:center;gap:8px;
}
.ai-provider-dd-item:hover{background:var(--bg)}
.ai-provider-dd-item.active{
  color:var(--accent);
  background:color-mix(in srgb,var(--accent) 10%,var(--bg));
}
.ai-provider-dd-item-dot{
  width:6px;height:6px;border-radius:50%;background:var(--border);flex-shrink:0;
}
.ai-provider-dd-item.active .ai-provider-dd-item-dot{background:var(--accent)}
.ai-hero{
  padding:26px 24px 0;flex-shrink:0;
}
.ai-hero h3{
  max-width:330px;color:var(--ai-text);font-size:32px;line-height:1.18;
  letter-spacing:-0.04em;font-weight:780;
}
.ai-hero-sub{
  margin-top:14px;color:var(--ai-muted);font-size:12px;line-height:1.5;
}
#ai-sidebar.ai-chatting .ai-hero{display:none}
#ai-sidebar.ai-chatting #ai-quick-actions{display:none}
#ai-key-status{
  display:inline-flex;align-items:center;gap:7px;
  margin-top:12px;padding:7px 10px;border-radius:999px;
  background:var(--ai-soft);border:1px solid var(--ai-line);
  color:var(--ai-muted);font-size:11px;font-weight:650;font-family:var(--font);
  cursor:pointer;
}
#ai-key-dot{
  width:7px;height:7px;border-radius:50%;background:var(--ai-warn);
  box-shadow:0 0 0 3px color-mix(in srgb,var(--ai-warn) 18%,transparent);
}
#ai-key-dot.ok{
  background:var(--ai-ok);
  box-shadow:0 0 0 3px color-mix(in srgb,var(--ai-ok) 18%,transparent);
}
#ai-messages{
  flex:1;overflow-y:auto;padding:18px 16px 12px;
  display:flex;flex-direction:column;gap:10px;min-height:0;
}
.ai-empty{flex:1;min-height:180px}
.ai-msg{
  padding:11px 13px;border-radius:18px;font-size:13px;line-height:1.55;max-width:88%;
  word-wrap:break-word;
}
.ai-msg.user{
  background:linear-gradient(135deg,#3b82f6,#6366f1,#8b5cf6);
  color:#fff;align-self:flex-end;
  box-shadow:0 8px 24px rgba(99,102,241,0.32);
}
.ai-msg.assistant{
  background:var(--ai-panel);border:1px solid var(--ai-line);
  color:var(--ai-text);align-self:flex-start;
}
.ai-msg.system{
  background:var(--ai-soft);border:1px solid var(--ai-line);
  color:var(--ai-muted);font-size:12px;text-align:center;border-radius:999px;
  padding:7px 12px;align-self:center;max-width:90%;
}
.ai-thinking{
  display:flex;gap:5px;align-items:center;align-self:flex-start;
  padding:12px 14px;border-radius:18px;background:var(--ai-panel);border:1px solid var(--ai-line);
}
.ai-tool-call{
  display:flex;gap:8px;align-items:center;align-self:flex-start;
  padding:8px 12px;border-radius:12px;
  background:color-mix(in srgb,var(--accent) 8%,var(--ai-soft));
  border:1px solid color-mix(in srgb,var(--accent) 22%,var(--ai-line));
  font-size:12px;color:var(--ai-muted);max-width:90%;
}
.ai-tool-icon{font-size:11px;opacity:0.7;flex-shrink:0;animation:spin 1.8s linear infinite}
.ai-tool-label{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
@keyframes spin{0%{transform:rotate(0deg)}100%{transform:rotate(360deg)}}
.ai-dot{width:6px;height:6px;border-radius:50%;background:var(--ai-muted);animation:bounce 1.4s ease-in-out infinite}
.ai-dot:nth-child(2){animation-delay:0.2s}
.ai-dot:nth-child(3){animation-delay:0.4s}
#ai-quick-actions{
  padding:0 16px 10px;display:flex;flex-direction:column;gap:8px;flex-shrink:0;
}
#ai-page-chip,.ai-qa-btn{
  width:max-content;max-width:100%;height:46px;border-radius:16px;
  border:1px solid var(--ai-line);background:var(--ai-soft);color:var(--ai-muted);
  display:flex;align-items:center;gap:10px;padding:0 14px;
  font-family:var(--font);font-size:13px;font-weight:650;cursor:pointer;
  transition:background var(--transition),border-color var(--transition),color var(--transition),transform var(--transition);
}
#ai-page-chip{
  width:100%;background:var(--ai-panel);color:var(--ai-text);justify-content:space-between;
}
#ai-page-chip:hover,.ai-qa-btn:hover{
  background:var(--ai-panel-strong);border-color:var(--ai-ring);color:var(--ai-text);
  transform:translateY(-1px);
}
/* Chevron rotates 90° when quick-actions are collapsed */
#ai-page-chip .ai-chip-chevron{transition:transform .22s ease}
#ai-quick-actions.qa-collapsed #ai-page-chip .ai-chip-chevron{transform:rotate(-90deg)}
#ai-quick-actions.qa-collapsed .ai-qa-btn{display:none}
.ai-page-left{display:flex;align-items:center;gap:10px;min-width:0}
.ai-page-icon{
  width:24px;height:24px;border-radius:50%;background:var(--bg);
  display:flex;align-items:center;justify-content:center;color:var(--ai-muted);font-size:10px;flex-shrink:0;
}
#ai-page-title{overflow:hidden;text-overflow:ellipsis;white-space:nowrap;min-width:0}
#ai-input-area{
  padding:0 14px 14px;flex-shrink:0;
  display:flex;flex-direction:column;gap:8px;
}
.ai-composer{
  background:var(--ai-panel);border:1px solid var(--ai-line);
  border-radius:34px;padding:16px 14px 12px;
  box-shadow:0 18px 42px var(--ai-shadow),inset 0 0 0 1px color-mix(in srgb,var(--ai-text) 4%,transparent);
  transition:border-color var(--transition),box-shadow var(--transition);
}
.ai-composer:focus-within{
  border-color:var(--accent);
  box-shadow:0 18px 46px var(--ai-shadow),0 0 0 3px var(--ai-ring),inset 0 0 0 1px color-mix(in srgb,var(--ai-text) 6%,transparent);
}
#ai-input{
  width:100%;resize:none;background:transparent;border:none;
  color:var(--ai-text);padding:4px 10px 12px;font-size:15px;
  font-family:var(--font);outline:none;line-height:1.5;
  min-height:58px;max-height:150px;
}
#ai-input::placeholder{color:var(--ai-muted)}
.ai-composer-actions{display:flex;align-items:center;justify-content:space-between;gap:10px}
.ai-composer-left,.ai-composer-right{display:flex;align-items:center;gap:8px;min-width:0}
.ai-pill-btn{
  height:38px;border-radius:999px;border:1px solid var(--ai-line);
  background:transparent;color:var(--ai-muted);font-family:var(--font);
  font-size:13px;font-weight:650;cursor:pointer;padding:0 14px;
  display:flex;align-items:center;gap:8px;transition:background var(--transition),color var(--transition),border-color var(--transition);
}
.ai-pill-btn:hover{background:var(--ai-soft);color:var(--ai-text);border-color:var(--ai-ring)}
/* ── Model picker modal ───────────────────────────────────────────────────── */
#model-modal{
  /* Constrain to AI sidebar column so the panel never overflows the Chrome clip region */
  position:fixed;top:0;right:0;width:var(--ai-w);height:100%;
  z-index:9000;display:flex;align-items:flex-end;justify-content:flex-end;
  background:transparent;pointer-events:none;
  transition:background .18s ease;
}
#model-modal.open{pointer-events:all;background:var(--overlay-bg-soft)}
#model-modal-panel{
  /* width adapts to sidebar width so it never clips outside the Chrome hit-test region */
  max-width:360px;width:calc(var(--ai-w) - 24px);
  max-height:540px;background:var(--bg-elevated);
  border:1px solid var(--border);border-radius:18px 18px 0 18px;
  box-shadow:var(--modal-shadow);
  display:flex;flex-direction:column;overflow:hidden;
  margin:0 12px 76px 0;
  opacity:0;transform:translateY(12px) scale(.97);
  transition:opacity .18s ease,transform .18s ease;pointer-events:none;
}
#model-modal.open #model-modal-panel{
  opacity:1;transform:translateY(0) scale(1);pointer-events:all;
}
.mm-header{
  display:flex;align-items:center;justify-content:space-between;
  padding:14px 16px 10px;border-bottom:1px solid var(--border);flex-shrink:0;
}
.mm-title{font-size:13px;font-weight:700;color:var(--text)}
.mm-close{
  width:26px;height:26px;border-radius:50%;border:none;background:var(--bg);
  color:var(--text-muted);cursor:pointer;display:flex;align-items:center;justify-content:center;
  font-size:16px;line-height:1;transition:background var(--transition);
}
.mm-close:hover{background:var(--border);color:var(--text)}
.mm-providers{
  display:flex;gap:6px;padding:10px 14px 8px;flex-shrink:0;flex-wrap:wrap;
}
.mm-tab{
  height:28px;padding:0 12px;border-radius:999px;border:1px solid var(--border);
  background:transparent;color:var(--text-muted);font-family:var(--font);
  font-size:12px;font-weight:650;cursor:pointer;transition:all var(--transition);white-space:nowrap;
}
.mm-tab:hover{background:var(--bg);color:var(--text)}
.mm-tab.active{background:var(--accent);border-color:var(--accent);color:#fff}
.mm-models{flex:1;overflow-y:auto;padding:6px 10px 8px;display:flex;flex-direction:column;gap:3px;min-height:0}
.mm-model{
  display:flex;align-items:center;gap:10px;padding:9px 10px;border-radius:10px;
  cursor:pointer;transition:background var(--transition);border:1px solid transparent;
}
.mm-model:hover{background:var(--bg)}
.mm-model.selected{background:color-mix(in srgb,var(--accent) 10%,var(--bg));border-color:color-mix(in srgb,var(--accent) 30%,transparent)}
.mm-model-dot{
  width:8px;height:8px;border-radius:50%;background:var(--border);flex-shrink:0;
  transition:background var(--transition);
}
.mm-model.selected .mm-model-dot{background:var(--accent)}
.mm-model-info{flex:1;min-width:0}
.mm-model-name{font-size:13px;font-weight:650;color:var(--text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.mm-model-meta{display:flex;gap:5px;margin-top:2px;flex-wrap:wrap}
.mm-tag{
  font-size:10px;padding:1px 6px;border-radius:999px;
  background:var(--bg);border:1px solid var(--border);color:var(--text-muted);white-space:nowrap;
}
.mm-tag.tools{border-color:color-mix(in srgb,var(--accent) 40%,transparent);color:var(--accent)}
.mm-tag.fast{border-color:color-mix(in srgb,#22c55e 40%,transparent);color:#22c55e}
.mm-tag.flagship{border-color:color-mix(in srgb,#f59e0b 40%,transparent);color:#f59e0b}
.mm-loading{padding:20px;text-align:center;color:var(--text-muted);font-size:13px}
.mm-custom{display:flex;gap:8px;padding:8px 10px 10px;border-top:1px solid var(--border);flex-shrink:0}
.mm-custom-input{
  flex:1;height:32px;border-radius:8px;border:1px solid var(--border);
  background:var(--bg);color:var(--text);font-family:var(--font);font-size:12px;padding:0 10px;
  outline:none;transition:border-color var(--transition);
}
.mm-custom-input:focus{border-color:var(--accent)}
.mm-custom-btn{
  height:32px;padding:0 12px;border-radius:8px;border:none;
  background:var(--accent);color:#fff;font-family:var(--font);font-size:12px;font-weight:650;
  cursor:pointer;white-space:nowrap;transition:opacity var(--transition);
}
.mm-custom-btn:hover{opacity:.85}
.ai-circle-btn{
  width:38px;height:38px;border-radius:50%;border:1px solid var(--ai-line);
  background:transparent;color:var(--ai-muted);display:flex;align-items:center;justify-content:center;
  cursor:pointer;transition:background var(--transition),color var(--transition),border-color var(--transition),transform var(--transition);
}
.ai-circle-btn:hover{background:var(--ai-soft);color:var(--ai-text);border-color:var(--ai-ring)}
.ai-circle-btn:active{transform:scale(.96)}
#ai-send-btn{
  width:42px;height:42px;border-radius:50%;
  background:linear-gradient(135deg,#3b82f6,#6366f1,#8b5cf6);
  color:#fff;border:none;
  display:flex;align-items:center;justify-content:center;
  cursor:pointer;transition:filter var(--transition),transform var(--transition),opacity var(--transition);
}
#ai-send-btn:hover{filter:brightness(1.07)}
#ai-send-btn:active{transform:scale(.96)}
#ai-send-btn:disabled{opacity:0.45;cursor:default;transform:none}
#ai-clear-btn{display:none}

/* settings overlay — sits in front of the live content WebView */
#settings-overlay{
  position:fixed;inset:0;
  background:var(--overlay-bg);
  -webkit-backdrop-filter:blur(16px) saturate(140%);
  backdrop-filter:blur(16px) saturate(140%);
  z-index:200;display:none;align-items:center;justify-content:center;
}
#settings-overlay.open{display:flex}
#settings-panel{
  background:var(--modal-bg);border:1px solid var(--modal-border);
  border-radius:20px;width:min(840px,calc(100vw - 32px));max-height:88vh;
  display:flex;
  box-shadow:var(--modal-shadow);
  overflow:hidden;
  animation:ventus-scale-in 0.22s cubic-bezier(0.16,1,0.3,1);
}
[data-theme="light"] #settings-panel{box-shadow:0 24px 70px rgba(30,40,100,0.18),0 0 0 1px rgba(0,0,0,0.06)}
.settings-nav{
  width:192px;background:var(--modal-bg-2);
  border-right:1px solid var(--modal-border);
  padding:16px 0;display:flex;flex-direction:column;gap:1px;
  flex-shrink:0;overflow-y:auto;
}
.settings-nav-item{
  padding:8px 18px;font-size:12.5px;cursor:pointer;
  color:var(--text-muted);border-left:2px solid transparent;
  transition:all 0.12s ease;
}
.settings-nav-item:hover{background:var(--soft-btn-bg);color:var(--text)}
.settings-nav-item.active{color:var(--accent);border-left-color:var(--accent);background:rgba(99,102,241,0.08);font-weight:500}
[data-theme="light"] .settings-nav-item.active{background:rgba(85,87,232,0.07)}
.settings-nav-group{
  padding:16px 18px 5px;font-size:10px;font-weight:700;
  color:var(--text-dim);text-transform:uppercase;letter-spacing:0.1em;
}
.settings-content{
  flex:1;overflow-y:auto;padding:28px 32px;
}
.settings-section{display:none}
.settings-section.active{display:block;animation:ventus-fade-up 0.18s ease}
.settings-section h2{font-size:18px;font-weight:700;margin-bottom:4px;color:var(--text);letter-spacing:-0.3px}
.settings-section .subtitle{color:var(--text-muted);font-size:12px;margin-bottom:24px;line-height:1.5}
.settings-group{margin-bottom:24px}
.settings-group label{
  display:block;font-size:12px;font-weight:600;
  color:var(--text);margin-bottom:7px;
}
.settings-group .hint{font-size:11px;color:var(--text-muted);margin-top:4px;line-height:1.5}
.settings-input{
  width:100%;background:var(--modal-bg-2);border:1px solid var(--modal-border);
  color:var(--text);border-radius:10px;
  padding:9px 12px;font-size:13px;outline:none;font-family:var(--font);
  transition:border-color var(--transition),box-shadow var(--transition);
}
.settings-input:focus{border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-dim)}
.settings-input::placeholder{color:var(--text-dim)}
.settings-path-row{display:flex;gap:8px;align-items:center}
.settings-path-row .settings-input{flex:1;min-width:0}
.settings-btn{
  border:1px solid var(--modal-border);background:var(--modal-bg-2);
  color:var(--text);border-radius:10px;
  padding:9px 16px;font-size:12px;cursor:pointer;font-family:var(--font);
  transition:all var(--transition);white-space:nowrap;font-weight:500;
}
.settings-btn:hover{border-color:var(--accent);background:var(--accent-dim)}
.settings-btn-sm{
  border:1px solid var(--modal-border);background:var(--modal-bg-2);
  color:var(--text);border-radius:8px;
  padding:6px 12px;font-size:11px;cursor:pointer;font-family:var(--font);
  transition:all var(--transition);white-space:nowrap;font-weight:500;
}
.settings-btn-sm:hover{border-color:var(--accent);background:var(--accent-dim)}
.settings-select{
  background:var(--modal-bg-2);border:1px solid var(--modal-border);
  color:var(--text);border-radius:10px;
  padding:9px 12px;font-size:12px;cursor:pointer;font-family:var(--font);
  appearance:none;-webkit-appearance:none;width:100%;outline:none;
  transition:border-color var(--transition);
}
.settings-select:focus{border-color:var(--accent)}
.settings-toggle{
  display:flex;align-items:center;justify-content:space-between;
  padding:13px 0;border-bottom:1px solid var(--modal-border);
}
.settings-toggle:last-child{border-bottom:none}
.settings-toggle-info{flex:1;padding-right:16px}
.settings-toggle-info .toggle-title{font-size:13px;font-weight:500;color:var(--text)}
.settings-toggle-info .toggle-desc{font-size:11px;color:var(--text-muted);margin-top:3px;line-height:1.4}
.toggle-switch{
  position:relative;width:40px;height:22px;
  background:rgba(255,255,255,0.14);border-radius:11px;cursor:pointer;
  transition:background 0.2s ease;flex-shrink:0;
}
[data-theme="light"] .toggle-switch{background:rgba(0,0,0,0.14)}
.toggle-switch.on{background:var(--accent)}
.toggle-switch::after{
  content:'';position:absolute;top:3px;left:3px;
  width:16px;height:16px;border-radius:50%;background:#fff;
  transition:transform 0.2s cubic-bezier(0.16,1,0.3,1);
  box-shadow:0 1px 4px rgba(0,0,0,0.3);
}
.toggle-switch.on::after{transform:translateX(18px)}

.settings-close{
  position:absolute;top:18px;right:18px;
  width:30px;height:30px;border-radius:8px;
  display:flex;align-items:center;justify-content:center;
  border:none;background:var(--soft-btn-bg);color:var(--text-muted);cursor:pointer;
  transition:all var(--transition);
}
.settings-close:hover{color:var(--text);background:var(--soft-btn-bg-hover)}

/* theme cards */
.theme-cards{display:flex;gap:12px;margin-top:4px}
.theme-card{
  flex:1;padding:14px;border-radius:12px;border:2px solid var(--modal-border);
  cursor:pointer;transition:all var(--transition);text-align:center;
  background:var(--modal-bg-2);
}
.theme-card:hover{border-color:var(--text-muted)}
.theme-card.selected{border-color:var(--accent);background:rgba(99,102,241,0.1)}
[data-theme="light"] .theme-card.selected{background:rgba(85,87,232,0.08)}
.theme-card .theme-preview{
  width:100%;height:44px;border-radius:8px;margin-bottom:10px;
}
.theme-card .theme-name{font-size:12px;font-weight:500;color:var(--text)}

/* onboarding ─────────────────────────────────────────────────────────────── */
#onboarding-overlay{
  position:fixed;inset:0;background:rgba(0,0,0,0.72);
  -webkit-backdrop-filter:blur(20px) saturate(160%);
  backdrop-filter:blur(20px) saturate(160%);
  z-index:300;display:none;align-items:center;justify-content:center;
}
#onboarding-overlay.open{display:flex}
#onboarding-modal{
  background:var(--modal-bg);border:1px solid var(--modal-border);
  border-radius:24px;width:min(580px,calc(100vw - 32px));
  box-shadow:0 40px 120px rgba(0,0,0,0.55),0 0 0 1px rgba(255,255,255,0.05);
  position:relative;overflow:hidden;
  animation:ventus-scale-in 0.28s cubic-bezier(0.16,1,0.3,1);
}
[data-theme="light"] #onboarding-modal{
  box-shadow:0 40px 100px rgba(20,30,80,0.18),0 0 0 1px rgba(0,0,0,0.07);
}
.ob-bar-track{height:2px;background:var(--modal-border);width:100%}
.ob-bar-fill{height:2px;background:var(--accent-gradient);transition:width 0.4s cubic-bezier(0.4,0,0.2,1)}
.ob-step-counter{
  font-size:11px;font-weight:600;color:var(--text-dim);letter-spacing:0.07em;
  text-transform:uppercase;text-align:right;padding:14px 44px 0;min-height:32px;
}
.ob-inner{padding:28px 44px 40px}
.ob-mark{
  width:48px;height:48px;border-radius:14px;
  display:flex;align-items:center;justify-content:center;
  flex-shrink:0;margin-bottom:18px;
}
.ob-mark-accent{background:var(--accent-gradient);box-shadow:0 8px 24px var(--accent-glow)}
.ob-step{display:none}
.ob-step.active{display:block}
@keyframes ob-slide-r{from{opacity:0;transform:translateX(28px)} to{opacity:1;transform:translateX(0)}}
@keyframes ob-slide-l{from{opacity:0;transform:translateX(-28px)} to{opacity:1;transform:translateX(0)}}
.ob-step.slide-r{animation:ob-slide-r 0.22s cubic-bezier(0.4,0,0.2,1)}
.ob-step.slide-l{animation:ob-slide-l 0.22s cubic-bezier(0.4,0,0.2,1)}
.ob-title{font-size:26px;font-weight:700;color:var(--text);letter-spacing:-0.5px;line-height:1.2;margin-bottom:8px}
.ob-sub{font-size:13px;color:var(--text-muted);line-height:1.65;margin-bottom:22px}
.ob-section-lbl{font-size:11px;font-weight:700;text-transform:uppercase;letter-spacing:0.07em;color:var(--text-muted);margin-bottom:8px;margin-top:18px}
.ob-section-lbl:first-child{margin-top:0}
.ob-actions{
  display:flex;justify-content:space-between;align-items:center;
  margin-top:26px;padding-top:22px;border-top:1px solid var(--modal-border);
}
.ob-btn-primary{
  padding:11px 26px;border-radius:12px;
  background:var(--accent-gradient);color:#fff;border:none;
  font-size:13.5px;font-weight:600;cursor:pointer;letter-spacing:0.01em;
  transition:opacity 0.15s,transform 0.15s;
  box-shadow:0 4px 18px var(--accent-glow);font-family:var(--font);
  display:flex;align-items:center;gap:8px;
}
.ob-btn-primary:hover{opacity:0.88;transform:translateY(-1px)}
.ob-btn-secondary{
  background:none;border:none;color:var(--text-muted);
  font-size:12.5px;cursor:pointer;transition:color 0.15s;
  font-family:var(--font);padding:8px 4px;
}
.ob-btn-secondary:hover{color:var(--text)}
.ob-theme-row{display:grid;grid-template-columns:repeat(3,1fr);gap:10px;margin-bottom:4px}
.ob-sidebar-chips{display:flex;gap:8px;flex-wrap:wrap}
.ob-sidebar-chip{
  padding:8px 16px;border-radius:20px;border:1.5px solid var(--modal-border);
  background:var(--modal-bg-2);color:var(--text-muted);cursor:pointer;
  font-size:12px;font-weight:500;transition:all 0.15s;font-family:var(--font);
}
.ob-sidebar-chip:hover{border-color:var(--text-dim);color:var(--text)}
.ob-sidebar-chip.selected{border-color:var(--accent);color:var(--accent);background:var(--accent-dim)}
.ob-detect-banner{
  display:flex;align-items:center;gap:12px;
  background:color-mix(in srgb,var(--accent) 10%,var(--modal-bg-2));
  border:1px solid color-mix(in srgb,var(--accent) 25%,var(--border));
  border-radius:12px;padding:12px 16px;margin-bottom:10px;
}
.ob-detect-flag{font-size:26px;line-height:1;flex-shrink:0}
.ob-detect-body{flex:1;min-width:0}
.ob-detect-label{font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:0.08em;color:var(--accent);margin-bottom:2px}
.ob-detect-name{font-size:13.5px;font-weight:600;color:var(--text)}
.ob-engine-grid{display:grid;grid-template-columns:1fr 1fr;gap:8px;margin-bottom:4px}
.ob-engine-btn{
  padding:12px 14px;border-radius:12px;border:1.5px solid var(--modal-border);
  background:var(--modal-bg-2);color:var(--text-muted);cursor:pointer;
  transition:all 0.15s;text-align:left;font-size:12.5px;
  display:flex;align-items:center;gap:9px;font-family:var(--font);font-weight:500;
}
.ob-engine-btn:hover{border-color:var(--text-dim);color:var(--text)}
.ob-engine-btn.selected{border-color:var(--accent);color:var(--accent);background:var(--accent-dim)}
.ob-api-row{display:flex;flex-direction:column;gap:5px;margin-bottom:11px}
.ob-api-row label{font-size:10.5px;font-weight:700;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.08em}
.ob-api-input{
  background:var(--modal-bg-2);border:1px solid var(--modal-border);
  color:var(--text);border-radius:10px;padding:10px 12px;font-size:12px;
  outline:none;width:100%;box-sizing:border-box;
  transition:border-color 0.15s,box-shadow 0.15s;font-family:monospace;
}
.ob-api-input:focus{border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-dim)}
.ob-api-input::placeholder{color:var(--text-dim)}
.ob-done-ring{
  width:76px;height:76px;border-radius:50%;
  background:linear-gradient(135deg,#34d399,#059669);
  display:flex;align-items:center;justify-content:center;
  margin:8px auto 22px;box-shadow:0 12px 32px rgba(52,211,153,0.35);
  animation:ob-pop 0.4s cubic-bezier(0.34,1.56,0.64,1) 0.1s both;
}
@keyframes ob-pop{from{transform:scale(0.6);opacity:0} to{transform:scale(1);opacity:1}}

/* toast */
#toast-container{
  position:fixed;bottom:16px;right:16px;z-index:500;
  display:flex;flex-direction:column;gap:6px;
}
.toast{
  padding:10px 14px;border-radius:var(--radius);
  font-size:12px;color:#fff;box-shadow:var(--shadow);
  animation:slide-up 0.2s ease;max-width:300px;
}
.toast.success{background:#2d7a4f}
.toast.error{background:#7a2d2d}
.toast.info{background:#2d4a7a}
[data-theme="light"] .toast.success{background:#137a4b}
[data-theme="light"] .toast.error{background:#b42318}
[data-theme="light"] .toast.info{background:#315ab8}

#newtab-placeholder{
  position:absolute;top:0;right:0;bottom:0;
  left:calc(var(--sidebar-w) + var(--frame-side-w,5px));
  display:flex;flex-direction:column;align-items:stretch;justify-content:flex-start;
  color:var(--nt-white);
  background:var(--nt-vignette),var(--nt-scrim),var(--nt-bg-image,linear-gradient(transparent,transparent)),linear-gradient(135deg,var(--bg),var(--bg-elevated));
  background-position:center;
  background-repeat:no-repeat;
  background-size:auto,auto,cover,auto;
  padding:32px clamp(18px,4.4vw,56px) 36px;
  text-align:left;
  overflow-x:hidden;
  overflow-y:auto;
  isolation:isolate;
}
#newtab-bg{
  display:none;
}
.newtab-shell{position:relative;z-index:1;width:min(1280px,100%);max-width:100%;min-width:0;margin:0 auto;display:flex;flex-direction:column;gap:24px;min-height:100%}
.newtab-top{display:flex;align-items:center;justify-content:space-between;gap:16px;min-height:36px}
.newtab-brand{display:flex;align-items:center;gap:10px;color:var(--nt-soft);font-size:12px;font-weight:700}
.newtab-logo{width:28px;height:28px;object-fit:contain;filter:drop-shadow(0 8px 18px rgba(0,0,0,0.28))}
.newtab-top-actions{display:flex;align-items:center;gap:8px}
.newtab-settings-btn{width:36px;height:36px;border-radius:999px;border:1px solid var(--nt-glass-border);background:var(--nt-top-bg);color:var(--nt-soft);display:flex;align-items:center;justify-content:center;cursor:pointer;box-shadow:var(--nt-pill-shadow);backdrop-filter:saturate(180%) blur(18px);transition:transform var(--transition),border-color var(--transition),background var(--transition),color var(--transition)}
.newtab-settings-btn:hover{border-color:var(--nt-focus-ring);background:var(--nt-glass-strong);color:var(--nt-white);transform:translateY(-1px)}
.newtab-date{font-size:12px;font-weight:700;color:var(--nt-soft);padding:8px 12px;border:1px solid var(--nt-glass-border);border-radius:999px;background:var(--nt-top-bg);box-shadow:var(--nt-pill-shadow);backdrop-filter:saturate(180%) blur(18px)}
.newtab-hero{display:flex;flex-direction:column;align-items:center;gap:14px;padding-top:clamp(18px,8vh,98px);filter:drop-shadow(var(--nt-hero-shadow))}
.newtab-search-wrap{
  width:min(650px,100%);
  position:relative;
}
#newtab-placeholder .newtab-search{
  width:100%;height:52px;display:flex;align-items:center;gap:10px;
  background:var(--nt-search-bg);border:1px solid var(--nt-glass-border);
  border-radius:18px;padding:0 16px;
  box-shadow:var(--nt-search-shadow);
  backdrop-filter:saturate(180%) blur(64px);
  transition:border-color var(--transition),box-shadow var(--transition),background var(--transition);
}
#newtab-placeholder .newtab-search:focus-within{border-color:var(--nt-focus-ring);background:var(--nt-search-bg-focus);box-shadow:var(--nt-search-focus-shadow)}
#newtab-input{
  flex:1;border:none;background:transparent;color:var(--nt-white);
  font-size:16px;font-weight:500;outline:none;font-family:var(--font);
  min-width:0;
}
#newtab-input::placeholder{color:var(--nt-muted)}
#newtab-search-logo{width:22px;height:22px;object-fit:contain;flex-shrink:0;filter:drop-shadow(0 6px 18px rgba(0,0,0,0.25));opacity:.92}
#newtab-suggestions{
  position:absolute;
  left:0;
  right:0;
  top:calc(100% + 10px);
  max-height:320px;
  overflow-y:auto;
  padding:6px;
  text-align:left;
  background:var(--nt-panel-strong);
  border-color:var(--nt-glass-border);
  backdrop-filter:saturate(180%) blur(24px);
}
.newtab-greeting{font-size:14px;line-height:1.3;font-weight:500;color:var(--nt-clock-muted);text-align:center;letter-spacing:0;max-width:calc(100% - 48px);text-shadow:0 10px 34px rgba(0,0,0,0.30);overflow-wrap:anywhere}
.newtab-sub{display:none}
.newtab-shortcuts{display:grid;grid-template-columns:repeat(8,minmax(56px,1fr));gap:12px;width:min(650px,100%);margin:2px auto 0}
.newtab-shortcut{
  display:flex;flex-direction:column;align-items:center;gap:7px;
  padding:8px 6px 9px;border-radius:18px;background:var(--nt-shortcut-bg);
  border:1px solid var(--nt-glass-border);cursor:pointer;min-width:0;
  color:var(--nt-white);font-family:var(--font);
  transition:transform var(--transition),background var(--transition),border-color var(--transition);text-decoration:none;
  box-shadow:var(--nt-shortcut-shadow);
  backdrop-filter:saturate(180%) blur(22px);
}
.newtab-shortcut:hover{border-color:var(--nt-focus-ring);background:var(--nt-shortcut-bg-hover);transform:translateY(-2px)}
.newtab-shortcut-icon{
  width:38px;height:38px;border-radius:13px;
  background:var(--nt-shortcut-icon-bg);display:flex;align-items:center;justify-content:center;
  color:var(--nt-white);overflow:hidden;
}
.newtab-shortcut-icon img{width:21px;height:21px;object-fit:contain;display:block}
.newtab-shortcut-label{font-size:10px;font-weight:700;color:var(--nt-soft);text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;width:100%}
.newtab-feed{display:flex;flex-direction:column;gap:14px;margin-top:auto;padding-top:clamp(12px,4vh,42px);min-width:0}
.newtab-feed-head{display:flex;align-items:center;justify-content:space-between;gap:12px}
.newtab-feed-title{font-size:15px;font-weight:800;color:var(--nt-white)}
.newtab-feed-actions{display:flex;align-items:center;gap:8px}
.newtab-feed-btn{border:1px solid var(--nt-glass-border);background:var(--nt-glass);color:var(--nt-soft);height:32px;padding:0 12px;border-radius:999px;font-family:var(--font);font-size:11px;font-weight:800;cursor:pointer}
.newtab-feed-main{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));grid-auto-rows:168px;gap:18px;align-items:stretch;min-width:0}
.news-card{position:relative;overflow:hidden;border:1px solid var(--nt-glass-border);border-radius:24px;background:var(--nt-panel);box-shadow:var(--nt-shadow);backdrop-filter:blur(24px);cursor:pointer;transition:transform var(--transition),border-color var(--transition),background var(--transition),box-shadow var(--transition);font-family:var(--font);text-align:left;color:inherit;padding:0}
.news-card:hover{transform:translateY(-4px);border-color:var(--nt-glass-strong);background:var(--nt-panel-strong);box-shadow:var(--nt-shadow)}
.news-card img{width:100%;height:100%;object-fit:cover;display:block}
.news-card.no-image{background:var(--nt-news-empty)}
.news-card::after{content:"";position:absolute;inset:0;background:linear-gradient(180deg,rgba(3,7,18,0.02) 20%,rgba(3,7,18,0.78))}
.news-card.no-image::after{background:var(--nt-news-empty-overlay)}
.news-card-body{position:absolute;left:0;right:0;bottom:0;z-index:1;padding:18px;color:var(--nt-news-white)}
.news-card.no-image .news-card-body{color:var(--nt-white)}
.news-card-meta{display:flex;align-items:center;gap:8px;color:var(--nt-news-soft);font-size:11px;font-weight:800;margin-bottom:8px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.news-card.no-image .news-card-meta{color:var(--nt-soft)}
.news-card-title{font-size:18px;line-height:1.16;font-weight:850;letter-spacing:0;display:-webkit-box;-webkit-line-clamp:3;-webkit-box-orient:vertical;overflow:hidden}
.news-card-summary{font-size:13px;color:var(--nt-news-soft);line-height:1.45;margin-top:8px;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.news-card.no-image .news-card-summary{color:var(--nt-soft)}
.news-card.featured{grid-column:span 2;grid-row:span 2}
.news-card.featured .news-card-title{font-size:clamp(24px,2.35vw,34px);line-height:1.1;-webkit-line-clamp:3}
.news-card.wide{grid-column:span 2}
.news-card.tall{grid-row:span 2}
.newtab-empty-feed{grid-column:1/-1;padding:24px;color:var(--nt-soft);font-weight:700;border:1px solid var(--nt-glass-border);border-radius:24px;background:var(--nt-panel)}
#newtab-placeholder.nt-hide-background{background:linear-gradient(135deg,var(--bg),var(--bg-elevated))}
#newtab-placeholder.nt-hide-search .newtab-search-wrap{display:none}
#newtab-placeholder.nt-hide-shortcuts .newtab-shortcuts{display:none}
#newtab-placeholder.nt-feed-headlines .newtab-feed-main{display:flex;flex-direction:column;gap:10px}
#newtab-placeholder.nt-feed-headlines .news-card{min-height:76px;border-radius:16px}
#newtab-placeholder.nt-feed-headlines .news-card img,#newtab-placeholder.nt-feed-headlines .news-card::after{display:none}
#newtab-placeholder.nt-feed-headlines .news-card-body{position:static;padding:14px}
#newtab-placeholder.nt-feed-headlines .news-card-title{font-size:14px;line-height:1.25;-webkit-line-clamp:2}
#newtab-placeholder.nt-feed-headlines .news-card-summary{display:none}
#newtab-placeholder.nt-feed-headlines .news-card.featured,#newtab-placeholder.nt-feed-headlines .news-card.wide,#newtab-placeholder.nt-feed-headlines .news-card.tall{grid-column:auto;grid-row:auto}
#newtab-placeholder.nt-feed-compact .newtab-feed-main{grid-template-columns:repeat(3,minmax(0,1fr));grid-auto-rows:122px}
#newtab-placeholder.nt-feed-compact .news-card{border-radius:18px}
#newtab-placeholder.nt-feed-compact .news-card-title{font-size:14px;line-height:1.22;-webkit-line-clamp:2}
#newtab-placeholder.nt-feed-compact .news-card-summary{display:none}
@media(max-width:1220px){.newtab-feed-main{grid-template-columns:repeat(3,minmax(0,1fr))}.newtab-shortcuts{grid-template-columns:repeat(4,minmax(72px,1fr))}}
@media(max-width:820px){.newtab-feed-main{grid-template-columns:repeat(2,minmax(0,1fr));grid-auto-rows:170px}.news-card.featured,.news-card.wide{grid-column:span 2}.newtab-shortcuts{grid-template-columns:repeat(4,minmax(72px,1fr))}}
@media(max-width:720px){#newtab-placeholder{padding:18px 14px 26px}.newtab-top{align-items:flex-start}.newtab-date{display:none}.newtab-hero{padding-top:clamp(16px,6vh,44px)}.newtab-greeting{font-size:14px}.newtab-shortcuts{grid-template-columns:repeat(3,minmax(64px,1fr))}.newtab-feed-main{grid-template-columns:1fr;grid-auto-rows:250px}.news-card.featured,.news-card.wide{grid-column:span 1}.news-card.tall,.news-card.featured{grid-row:span 1}.newtab-feed{gap:14px}}

/* ── Newtab clock ─────────────────────────────────────────── */
#newtab-clock{
  font-family:'Segoe UI Variable Display','SF Pro Display',-apple-system,BlinkMacSystemFont,var(--font);
  font-size:clamp(72px,8vw,128px);font-weight:200;letter-spacing:-0.07em;line-height:.9;
  color:var(--nt-white);text-shadow:var(--nt-clock-shadow);
  font-variant-numeric:tabular-nums;display:block;text-align:center;
  font-feature-settings:'tnum' 1;
  user-select:none;
}
#newtab-placeholder.nt-clock-sf #newtab-clock{font-family:'Segoe UI Variable Display','SF Pro Display',-apple-system,BlinkMacSystemFont,var(--font);font-weight:200;letter-spacing:-0.07em}
#newtab-placeholder.nt-clock-rounded #newtab-clock{font-family:'SF Pro Rounded','Aptos Rounded','Segoe UI Variable Display',var(--font);font-weight:300;letter-spacing:-0.055em}
#newtab-placeholder.nt-clock-mono #newtab-clock{font-family:'SF Mono','Cascadia Code','Consolas',monospace;font-weight:300;letter-spacing:-0.08em;font-feature-settings:'tnum' 1,'zero' 1}
#newtab-placeholder.nt-clock-serif #newtab-clock{font-family:'New York','Iowan Old Style','Georgia',serif;font-weight:400;letter-spacing:-0.055em}
#newtab-placeholder.nt-theme-minimal .newtab-shell{justify-content:center;align-items:center;gap:0}
#newtab-placeholder.nt-theme-minimal .newtab-top{display:none}
#newtab-placeholder.nt-theme-minimal .newtab-feed{display:none!important}
#newtab-placeholder.nt-theme-minimal .newtab-sub{display:none}
#newtab-placeholder.nt-theme-minimal .newtab-hero{gap:20px;padding:0;width:min(620px,100%)}
#newtab-placeholder.nt-theme-minimal .newtab-greeting{font-size:14px;color:var(--text-muted);text-shadow:none;font-weight:500}
#newtab-placeholder.nt-theme-minimal #newtab-clock{font-size:78px;color:var(--text);text-shadow:none}
#newtab-placeholder.nt-theme-minimal .newtab-search-wrap{width:100%}
#newtab-placeholder.nt-theme-minimal .newtab-search{background:var(--bg-elevated)!important;border-color:var(--border)!important;backdrop-filter:none!important;box-shadow:0 2px 12px rgba(0,0,0,0.08)!important}
#newtab-placeholder.nt-theme-minimal .newtab-search:focus-within{border-color:var(--accent)!important;box-shadow:0 0 0 3px var(--accent-dim)!important}
#newtab-placeholder.nt-theme-minimal #newtab-input{color:var(--text)}
#newtab-placeholder.nt-theme-minimal #newtab-input::placeholder{color:var(--text-muted)}
#newtab-placeholder.nt-theme-minimal .newtab-shortcuts{width:100%}
#newtab-placeholder.nt-theme-minimal .newtab-shortcut{background:var(--bg-hover)!important;border-color:var(--border)!important;backdrop-filter:none!important}
#newtab-placeholder.nt-theme-minimal .newtab-shortcut:hover{background:var(--bg-active)!important;transform:translateY(-1px)}
#newtab-placeholder.nt-theme-minimal .newtab-shortcut-label{color:var(--text-muted)}
/* ── Newtab theme: FOCUS (wallpaper, centered, huge clock) ── */
#newtab-placeholder.nt-theme-focus .newtab-shell{justify-content:center;gap:18px}
#newtab-placeholder.nt-theme-focus .newtab-top{display:none}
#newtab-placeholder.nt-theme-focus .newtab-feed{display:none!important}
#newtab-placeholder.nt-theme-focus .newtab-sub{display:none}
#newtab-placeholder.nt-theme-focus .newtab-hero{gap:16px;padding:0}
#newtab-placeholder.nt-theme-focus #newtab-clock{font-size:clamp(84px,9vw,136px)}
#newtab-placeholder.nt-theme-focus .newtab-greeting{font-size:15px;font-weight:400;opacity:0.72;text-shadow:none}
#newtab-placeholder.nt-theme-focus .newtab-search-wrap{width:min(580px,100%)}
#newtab-placeholder.nt-theme-focus .newtab-shortcuts{width:min(580px,100%)}
/* ── Newtab theme: HORIZON (wallpaper, top bar, no feed) ──── */
#newtab-placeholder.nt-theme-horizon .newtab-feed{display:none!important}
/* ── Newtab theme: INFORMATIVE is the default (no overrides) */

/* ── Wallpaper settings panel ─────────────────────────────── */
.nt-theme-grid{display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin-bottom:4px}
.nt-theme-card{display:flex;flex-direction:column;align-items:center;gap:7px;border:2px solid var(--border);border-radius:12px;padding:8px 6px 10px;background:var(--bg-elevated);cursor:pointer;transition:border-color var(--transition),background var(--transition);width:100%}
.nt-theme-card:hover{border-color:var(--accent-hover);background:var(--bg-hover)}
.nt-theme-card.selected{border-color:var(--accent);background:var(--accent-dim)}
.nt-theme-card-preview{width:100%;aspect-ratio:16/10;border-radius:7px;overflow:hidden}
.nt-theme-card-name{font-size:11px;font-weight:700;color:var(--text-muted)}
.nt-theme-card.selected .nt-theme-card-name{color:var(--accent)}
.nt-theme-preview{width:100%;min-height:44px;display:flex;flex-direction:column;justify-content:center;border-radius:7px;overflow:hidden;padding:6px 0;color:var(--text-muted)}
.nt-theme-card.selected .nt-theme-preview{color:var(--accent)}
.nt-theme-label{font-size:11px;font-weight:600;color:var(--text-muted)}
.nt-theme-card.selected .nt-theme-label{color:var(--accent)}
.nt-clock-grid{display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin-bottom:4px}
.nt-clock-card{display:flex;flex-direction:column;align-items:center;gap:7px;border:2px solid var(--border);border-radius:12px;padding:9px 6px 10px;background:var(--bg-elevated);cursor:pointer;transition:border-color var(--transition),background var(--transition);width:100%;font-family:var(--font);color:var(--text-muted)}
.nt-clock-card:hover{border-color:var(--accent-hover);background:var(--bg-hover)}
.nt-clock-card.selected{border-color:var(--accent);background:var(--accent-dim);color:var(--accent)}
.nt-clock-preview{font-size:20px;line-height:1;font-weight:300;letter-spacing:-0.06em;font-variant-numeric:tabular-nums}
.nt-clock-card[data-clock="sf"] .nt-clock-preview{font-family:'Segoe UI Variable Display','SF Pro Display',-apple-system,BlinkMacSystemFont,var(--font);font-weight:200}
.nt-clock-card[data-clock="rounded"] .nt-clock-preview{font-family:'SF Pro Rounded','Aptos Rounded','Segoe UI Variable Display',var(--font);font-weight:300;letter-spacing:-0.04em}
.nt-clock-card[data-clock="mono"] .nt-clock-preview{font-family:'SF Mono','Cascadia Code','Consolas',monospace;font-size:18px;letter-spacing:-0.06em}
.nt-clock-card[data-clock="serif"] .nt-clock-preview{font-family:'New York','Iowan Old Style','Georgia',serif;font-weight:400;letter-spacing:-0.04em}
.nt-clock-label{font-size:11px;font-weight:600;color:inherit}
.nt-wp-sources{display:flex;flex-wrap:wrap;gap:6px;margin-bottom:10px}
.nt-wp-src-btn{height:30px;padding:0 13px;border-radius:999px;border:1px solid var(--border);background:var(--bg-elevated);color:var(--text-muted);font-size:12px;font-weight:600;cursor:pointer;font-family:var(--font);transition:border-color var(--transition),color var(--transition),background var(--transition)}
.nt-wp-src-btn:hover{border-color:var(--accent);color:var(--text)}
.nt-wp-src-btn.active{border-color:var(--accent);background:var(--accent-dim);color:var(--accent)}
.nt-wp-extra{display:flex;align-items:center;gap:10px;margin-top:4px}
.nt-wp-upload-preview{width:80px;height:50px;border-radius:8px;object-fit:cover;border:1px solid var(--border);display:none;cursor:pointer}

#update-modal{
  position:fixed;inset:0;z-index:2100;display:none;align-items:center;justify-content:center;
  pointer-events:none;
}
#update-modal.open{display:flex}
.update-modal-panel{
  width:min(468px,calc(100vw - 32px));max-height:78vh;
  background:color-mix(in srgb,var(--modal-bg) 92%,transparent);
  border:1px solid var(--modal-border);
  border-radius:24px;
  box-shadow:var(--modal-shadow);
  -webkit-backdrop-filter:blur(24px) saturate(150%);
  backdrop-filter:blur(24px) saturate(150%);
  overflow:hidden;pointer-events:auto;
  animation:ventus-scale-in 0.24s cubic-bezier(0.16,1,0.3,1);
}
.update-modal-head{display:flex;align-items:flex-start;gap:14px;padding:22px 22px 16px}
.update-modal-icon{
  width:42px;height:42px;border-radius:14px;display:flex;align-items:center;justify-content:center;
  background:var(--accent-dim);color:var(--accent);flex-shrink:0;
}
.update-modal-copy{flex:1;min-width:0}
.update-modal-title{font-size:18px;font-weight:700;letter-spacing:-0.3px;color:var(--text)}
.update-modal-sub{font-size:12px;color:var(--text-muted);line-height:1.5;margin-top:4px}
.update-modal-close{
  width:30px;height:30px;border-radius:999px;border:none;background:var(--soft-btn-bg);
  color:var(--text-muted);display:flex;align-items:center;justify-content:center;cursor:pointer;
  transition:background var(--transition),color var(--transition);flex-shrink:0;
}
.update-modal-close:hover{background:var(--soft-btn-bg-hover);color:var(--text)}
.update-modal-body{padding:0 22px 20px}
.update-modal-notes{
  display:none;max-height:220px;overflow-y:auto;padding:13px 14px;border-radius:16px;
  background:var(--modal-bg-2);border:1px solid var(--modal-border);
  color:var(--text-muted);font-size:12px;line-height:1.6;white-space:pre-wrap;
}
.update-modal-notes.visible{display:block}
.update-modal-progress{display:none;margin-top:4px}
.update-modal-progress.visible{display:block}
.update-modal-track{height:5px;background:var(--border);border-radius:999px;overflow:hidden}
.update-modal-bar{height:100%;width:0%;background:var(--accent);border-radius:999px;transition:width 0.25s ease}
.update-modal-progress-label{font-size:11px;color:var(--text-muted);margin-top:8px}
.update-modal-actions{
  display:flex;justify-content:flex-end;gap:8px;padding:14px 22px 22px;border-top:1px solid var(--modal-border);
}
.update-spin{animation:spin 1s linear infinite}

/* tab search modal */
#tab-search-modal{
  position:fixed;top:80px;left:50%;transform:translateX(-50%);
  background:var(--modal-bg);border:1px solid var(--modal-border);
  border-radius:14px;width:480px;
  box-shadow:var(--modal-shadow);
  z-index:250;display:none;overflow:hidden;
}
#tab-search-modal.open{display:block}
#tab-search-input{
  width:100%;background:transparent;border:none;border-bottom:1px solid var(--border-subtle);
  color:var(--text);padding:14px 16px;font-size:14px;outline:none;
}
#tab-search-results{max-height:320px;overflow-y:auto;padding:6px}
.tab-search-item{
  display:flex;align-items:center;gap:10px;
  padding:8px 10px;border-radius:var(--radius-sm);cursor:pointer;
  transition:background var(--transition);
}
.tab-search-item:hover{background:var(--bg-hover)}
.tab-search-item.highlighted{background:var(--accent-dim)}
.tab-search-item .ts-title{font-size:12px;font-weight:500;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.tab-search-item .ts-url{font-size:11px;color:var(--text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}

/* download badge lives on #btn-more — see #more-btn-badge CSS above */

/* ── Download panel ─────────────────────────────────────────────────────────── */
#download-panel{
  position:fixed;display:none;flex-direction:column;
  width:348px;max-height:500px;overflow:hidden;
  background:var(--modal-bg);
  border:1px solid var(--modal-border);
  border-radius:20px;
  box-shadow:var(--modal-shadow);
  z-index:350;
}
#download-panel.open{display:flex}
.dl-panel-head{
  display:flex;align-items:center;gap:10px;
  padding:16px 18px 14px;
  border-bottom:1px solid var(--modal-border);
  flex-shrink:0;
}
.dl-panel-head-icon{
  width:30px;height:30px;border-radius:9px;flex-shrink:0;
  background:var(--accent-dim);
  display:flex;align-items:center;justify-content:center;
  color:var(--accent);
}
.dl-panel-head-title{font-size:14px;font-weight:700;color:var(--text);letter-spacing:-0.3px;flex:1}
.dl-panel-head-right{display:flex;align-items:center;gap:2px}
.dl-clear-btn{
  background:none;border:none;color:var(--text-muted);cursor:pointer;
  font-size:11px;font-family:var(--font);font-weight:500;
  padding:5px 9px;border-radius:8px;
  transition:all var(--transition);
}
.dl-clear-btn:hover{color:var(--text);background:var(--bg-hover)}
#dl-panel-list{overflow-y:auto;flex:1;min-height:0;padding:4px 0}
.dl-panel-empty{
  display:flex;flex-direction:column;align-items:center;justify-content:center;
  padding:40px 20px;gap:10px;
}
.dl-panel-empty-icon{
  width:48px;height:48px;border-radius:14px;
  background:var(--bg-hover);
  display:flex;align-items:center;justify-content:center;
  color:var(--text-dim);margin-bottom:4px;
}
.dl-panel-empty-label{font-size:13px;color:var(--text-muted);font-weight:600}
.dl-panel-empty-sub{font-size:11px;color:var(--text-dim);font-weight:400;text-align:center;max-width:200px;line-height:1.5}
.dl-panel-item{
  display:flex;align-items:center;gap:12px;
  padding:10px 18px;
  border-bottom:1px solid var(--border-subtle);
  transition:background var(--transition);
}
.dl-panel-item:last-child{border-bottom:none}
.dl-panel-item:hover{background:var(--bg-hover)}
.dl-pi-icon-wrap{
  width:36px;height:36px;border-radius:10px;
  display:flex;align-items:center;justify-content:center;
  flex-shrink:0;font-size:11px;font-weight:700;letter-spacing:0.03em;
}
/* file type color tints */
.dl-pi-icon-wrap.ft-img{background:rgba(52,211,153,0.12);color:#34d399}
.dl-pi-icon-wrap.ft-vid{background:rgba(139,92,246,0.12);color:#a78bfa}
.dl-pi-icon-wrap.ft-aud{background:rgba(236,72,153,0.12);color:#f472b6}
.dl-pi-icon-wrap.ft-doc{background:rgba(59,130,246,0.12);color:#60a5fa}
.dl-pi-icon-wrap.ft-arc{background:rgba(245,158,11,0.12);color:#fbbf24}
.dl-pi-icon-wrap.ft-exe{background:rgba(239,68,68,0.12);color:#f87171}
.dl-pi-icon-wrap.ft-code{background:rgba(20,184,166,0.12);color:#2dd4bf}
.dl-pi-icon-wrap.ft-active{background:var(--accent-dim);color:var(--accent)}
.dl-pi-icon-wrap.ft-default{background:var(--bg-hover);color:var(--text-muted)}
.dl-pi-body{flex:1;min-width:0}
.dl-pi-name{font-size:12px;font-weight:600;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;line-height:1.35}
.dl-pi-row{display:flex;align-items:center;gap:7px;margin-top:3px}
.dl-pi-status-badge{
  display:inline-flex;align-items:center;
  padding:1px 7px;border-radius:999px;
  font-size:10px;font-weight:600;letter-spacing:0.02em;
}
.dl-pi-status-badge.done{background:rgba(52,211,153,0.12);color:#34d399}
.dl-pi-status-badge.failed{background:rgba(239,68,68,0.12);color:#f87171}
.dl-pi-status-badge.active{background:var(--accent-dim);color:var(--accent)}
.dl-pi-meta{font-size:10.5px;color:var(--text-dim);line-height:1.3}
.dl-pi-bar{
  height:3px;background:var(--bg-active);
  border-radius:2px;margin-top:6px;overflow:hidden;
}
.dl-pi-bar-fill{
  height:100%;background:var(--accent-gradient);border-radius:2px;
  transition:width 0.3s ease;
  background-size:200% 100%;
  animation:dl-bar-shimmer 1.8s linear infinite;
}
@keyframes dl-bar-shimmer{
  0%{background-position:100% 0}
  100%{background-position:-100% 0}
}
.dl-pi-actions{display:flex;gap:4px;flex-shrink:0}
.dl-pi-btn{
  background:var(--bg-hover);border:none;
  color:var(--text-muted);cursor:pointer;border-radius:7px;
  height:26px;padding:0 9px;font-size:11px;font-family:var(--font);font-weight:500;
  transition:all var(--transition);white-space:nowrap;
  display:flex;align-items:center;
}
.dl-pi-btn:hover{background:var(--bg-active);color:var(--text)}
.dl-pi-btn.danger:hover{background:rgba(239,68,68,0.12);color:#f87171}
.dl-panel-foot{
  padding:12px 18px;
  border-top:1px solid var(--modal-border);
  flex-shrink:0;text-align:center;
}
.dl-panel-foot a{
  display:inline-flex;align-items:center;gap:5px;
  font-size:12px;font-weight:500;color:var(--text-muted);
  text-decoration:none;cursor:pointer;
  transition:color var(--transition);
}
.dl-panel-foot a:hover{color:var(--accent)}

/* ── Adblock modal ──────────────────────────────────────────────────────────── */
#adblock-backdrop{
  display:none;position:fixed;inset:0;z-index:9998;background:transparent;
}
#adblock-backdrop.open{display:block}
#adblock-modal{
  display:none;position:fixed;z-index:9999;top:50px;right:auto;left:auto;width:284px;
  background:var(--modal-bg);
  border:1px solid var(--modal-border);
  border-radius:20px;
  box-shadow:var(--modal-shadow);
  padding:0;overflow:hidden;
}
#adblock-modal.open{display:block}
.abm-header{
  display:flex;align-items:center;gap:10px;
  padding:16px 16px 14px;
  border-bottom:1px solid var(--modal-border);
}
.abm-header-icon{
  width:32px;height:32px;border-radius:10px;flex-shrink:0;
  background:var(--accent-dim);
  display:flex;align-items:center;justify-content:center;
}
.abm-header-icon svg{color:var(--accent)}
.abm-title{font-size:14px;font-weight:700;color:var(--text);letter-spacing:-0.3px;flex:1}
.abm-body{padding:16px;display:flex;flex-direction:column;gap:14px}
.abm-status-row{
  display:inline-flex;align-items:center;gap:8px;
  padding:6px 12px 6px 10px;border-radius:999px;align-self:flex-start;
  background:rgba(52,211,153,0.1);
  transition:background var(--transition);
}
.abm-status-row.off{background:rgba(107,114,128,0.1)}
.abm-status-row.warn{background:rgba(245,158,11,0.1)}
.abm-status-dot{
  width:7px;height:7px;border-radius:50%;
  background:#22c55e;flex-shrink:0;
}
.abm-status-dot.off{background:#6b7280}
.abm-status-dot.warn{background:#f59e0b}
.abm-status-label{
  font-size:11px;font-weight:600;color:#22c55e;letter-spacing:0.01em;
}
.abm-status-label.muted{color:var(--text-dim)}
.abm-status-label.warn{color:#f59e0b}
.abm-action-btn,
.abm-settings-btn{
  display:flex;align-items:center;justify-content:space-between;
  width:100%;padding:10px 12px;border-radius:10px;
  background:var(--bg-hover);border:1px solid var(--border-subtle);
  color:var(--text-muted);font-size:12px;font-family:var(--font);font-weight:500;
  cursor:pointer;text-align:left;
  transition:all var(--transition);
}
.abm-action-btn{
  background:rgba(52,211,153,0.1);border-color:rgba(52,211,153,0.16);color:#34d399;
}
.abm-action-btn.warn{
  background:rgba(245,158,11,0.1);border-color:rgba(245,158,11,0.18);color:#f59e0b;
}
.abm-action-btn.off{
  background:var(--bg-hover);border-color:var(--border-subtle);color:var(--text-dim);cursor:not-allowed;opacity:.65;
}
.abm-action-btn:not(.off):hover,
.abm-settings-btn:hover{
  background:var(--bg-active);border-color:var(--border);color:var(--text);
}
.abm-btn-right,.abm-settings-btn-right{display:flex;align-items:center;gap:4px;color:var(--accent)}
.abm-action-btn .abm-btn-right{color:currentColor}
.dl-spin{
  width:12px;height:12px;border:2px solid var(--border);
  border-top-color:var(--accent);border-radius:50%;
  animation:spin 0.7s linear infinite;display:inline-block;
}
@keyframes spin{to{transform:rotate(360deg)}}
@keyframes utSlideIn{from{transform:translateY(calc(100% + 36px));opacity:0}to{transform:translateY(0);opacity:1}}
@keyframes utSlideOut{from{transform:translateY(0);opacity:1}to{transform:translateY(calc(100% + 36px));opacity:0}}
@keyframes tspIn{from{opacity:0}to{opacity:1}}
@keyframes tspCardIn{from{transform:scale(0.96) translateY(-10px);opacity:0}to{transform:scale(1) translateY(0);opacity:1}}

#tab-spotlight-overlay{
  position:fixed;inset:0;z-index:800;
  display:none;flex-direction:column;
  align-items:center;justify-content:flex-start;
  padding-top:clamp(60px,12vh,140px);
  background:var(--overlay-bg-soft);
  backdrop-filter:blur(12px);-webkit-backdrop-filter:blur(12px);
  cursor:default;
}
#tab-spotlight-overlay.open{
  display:flex;
  animation:tspIn 0.18s ease;
}
#tab-spotlight{
  width:min(640px,calc(100vw - 64px));
  background:var(--modal-bg);
  border:1px solid var(--modal-border);
  border-radius:20px;
  box-shadow:var(--modal-shadow);
  overflow:hidden;
  animation:tspCardIn 0.28s cubic-bezier(0.16,1,0.3,1);
  transform-origin:top center;
}
[data-theme="light"] #tab-spotlight{background:var(--modal-bg)}
#tsp-input-wrap{
  display:flex;align-items:center;gap:14px;
  padding:18px 22px;
  border-bottom:1px solid var(--border);
}
.tsp-ico{flex-shrink:0;color:var(--text-muted);}
#tsp-input{
  flex:1;background:transparent;border:none;outline:none;
  font-size:20px;font-family:var(--font);color:var(--text);
  caret-color:var(--accent);min-width:0;
}
#tsp-input::placeholder{color:var(--text-muted);opacity:0.6;}
#tsp-results{
  max-height:380px;overflow-y:auto;padding-bottom:6px;
  scrollbar-width:thin;scrollbar-color:var(--border) transparent;
}
.tsp-section-lbl{
  font-size:10.5px;font-weight:600;text-transform:uppercase;
  letter-spacing:0.07em;color:var(--text-muted);
  padding:12px 22px 4px;
}
.tsp-row{
  display:flex;align-items:center;gap:12px;
  padding:9px 22px;cursor:pointer;
  transition:background 0.1s;
}
.tsp-row:hover,.tsp-row.tsp-active{
  background:var(--bg-hover);
}
.tsp-row-icon{
  width:30px;height:30px;border-radius:8px;
  background:var(--bg);border:1px solid var(--border);
  display:flex;align-items:center;justify-content:center;
  flex-shrink:0;overflow:hidden;
  color:var(--text-muted);
}
.tsp-row-body{flex:1;min-width:0;}
.tsp-row-title{font-size:13.5px;font-weight:500;color:var(--text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}
.tsp-row-sub{font-size:11px;color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;margin-top:1px;}
/* Spotlight AI hint badge */
#tsp-ai-hint{
  display:none;
  flex-shrink:0;
  font-size:10.5px;font-weight:600;
  color:var(--accent);
  background:color-mix(in srgb, var(--accent) 14%, transparent);
  border:1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-radius:6px;padding:3px 8px;white-space:nowrap;
  pointer-events:none;
}
#tsp-ai-hint.visible{display:block;}
/* Spotlight AI answer panel */
#tsp-ai-panel{
  display:none;
  flex-direction:column;
}
#tsp-ai-panel.visible{display:flex;}
.tsp-ai-header{
  display:flex;align-items:center;gap:10px;
  padding:10px 22px 8px;
  border-bottom:1px solid var(--border);
}
.tsp-ai-back{
  background:none;border:none;cursor:pointer;
  color:var(--text-muted);padding:4px;border-radius:6px;
  display:flex;align-items:center;justify-content:center;
  transition:background 0.1s,color 0.1s;
}
.tsp-ai-back:hover{background:var(--bg-hover);color:var(--text);}
.tsp-ai-title{
  font-size:12px;font-weight:600;color:var(--text-muted);
  text-transform:uppercase;letter-spacing:0.07em;
}
.tsp-ai-content{
  padding:14px 22px 18px;
  font-size:14px;line-height:1.7;color:var(--text);
  max-height:340px;overflow-y:auto;
  scrollbar-width:thin;scrollbar-color:var(--border) transparent;
}
.tsp-ai-content strong{font-weight:600;}
.tsp-ai-content em{font-style:italic;}
.tsp-ai-content code{
  font-family:monospace;font-size:12.5px;
  background:var(--bg);border:1px solid var(--border);
  border-radius:4px;padding:1px 5px;
}
.tsp-ai-content ul{margin:6px 0 6px 18px;padding:0;}
.tsp-ai-content li{margin:3px 0;}
.tsp-ai-dots{display:flex;align-items:center;gap:5px;padding:4px 0;}
.tsp-ai-dots span{
  width:6px;height:6px;border-radius:50%;
  background:var(--accent);opacity:0.6;
  animation:tspdot 1.2s infinite;
}
.tsp-ai-dots span:nth-child(2){animation-delay:0.2s;}
.tsp-ai-dots span:nth-child(3){animation-delay:0.4s;}
@keyframes tspdot{0%,80%,100%{transform:scale(0.6);opacity:0.4;}40%{transform:scale(1);opacity:1;}}

#update-toast{
  position:fixed;bottom:24px;right:24px;z-index:2000;
  display:flex;align-items:center;gap:12px;
  width:340px;padding:14px 16px 14px 16px;
  background:var(--bg-elevated);
  border:1px solid rgba(255,255,255,0.10);
  border-radius:16px;
  box-shadow:0 12px 40px rgba(0,0,0,0.5),0 0 0 0.5px rgba(255,255,255,0.05);
  backdrop-filter:blur(20px);-webkit-backdrop-filter:blur(20px);
  transform:translateY(calc(100% + 36px));opacity:0;
  pointer-events:none;transition:none;
  user-select:none;-webkit-user-select:none;
}
#update-toast.visible{
  animation:utSlideIn 0.44s cubic-bezier(0.16,1,0.3,1) forwards;
  pointer-events:auto;
}
#update-toast.hiding{
  animation:utSlideOut 0.3s cubic-bezier(0.4,0,1,1) forwards;
  pointer-events:none;
}
[data-theme="light"] #update-toast{
  background:rgba(255,255,255,0.92);
  border:1px solid rgba(0,0,0,0.08);
  box-shadow:0 12px 40px rgba(0,0,0,0.18),0 0 0 0.5px rgba(0,0,0,0.04);
}
.ut-icon{
  flex-shrink:0;width:36px;height:36px;border-radius:10px;
  background:var(--accent-dim);color:var(--accent);
  display:flex;align-items:center;justify-content:center;
}
.ut-body{flex:1;min-width:0}
.ut-title{font-size:13px;font-weight:600;color:var(--text);line-height:1.3}
.ut-version{font-size:11px;color:var(--text-muted);margin-top:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.ut-buttons{display:flex;gap:6px;flex-shrink:0;align-items:center}
.ut-btn-later{
  padding:5px 12px;font-size:12px;font-weight:500;
  background:transparent;border:1px solid var(--border);
  border-radius:8px;color:var(--text-muted);cursor:pointer;
  transition:background var(--transition),color var(--transition);
  font-family:var(--font);
}
.ut-btn-later:hover{background:var(--bg-hover);color:var(--text)}
.ut-btn-update{
  padding:5px 14px;font-size:12px;font-weight:600;
  background:var(--accent);border:none;
  border-radius:8px;color:#fff;cursor:pointer;
  transition:background var(--transition),transform 0.1s ease,box-shadow 0.1s ease;
  font-family:var(--font);
  box-shadow:0 2px 10px var(--accent-glow);
}
.ut-btn-update:hover{background:var(--accent-hover);transform:translateY(-1px);box-shadow:0 4px 14px var(--accent-glow)}
.ut-btn-update:active{transform:translateY(0)}


/* context menu */
#context-menu{
  position:fixed;display:none;flex-direction:column;
  min-width:190px;max-width:280px;
  background:var(--bg-elevated);border:1px solid var(--border);
  border-radius:var(--radius);box-shadow:var(--popover-shadow);
  z-index:500;padding:4px 0;
  user-select:none;-webkit-user-select:none;
}
#context-menu.open{display:flex}
.ctx-item{
  padding:7px 14px;font-size:12px;color:var(--text);
  cursor:pointer;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;
  transition:background var(--transition);
}
.ctx-item:hover{background:var(--bg-hover)}
.ctx-item.ctx-disabled{color:var(--text-muted);pointer-events:none;opacity:0.5}
.ctx-sep{height:1px;background:var(--border-subtle);margin:3px 0;flex-shrink:0}

/* bookmarks bar */
#bookmarks-bar{
  height:var(--bookmarks-bar-h);
  flex:0 0 var(--bookmarks-bar-h);
  background:var(--bg-elevated);border-bottom:1px solid var(--border-subtle);
  display:none;align-items:center;gap:2px;padding:0 8px;
  z-index:100;overflow:hidden;
}
#app.show-bookmarks-bar #bookmarks-bar{display:flex}
.bm-bar-item{
  display:flex;align-items:center;gap:5px;
  padding:3px 8px;border-radius:var(--radius-sm);
  font-size:11px;color:var(--text-muted);cursor:pointer;
  white-space:nowrap;overflow:hidden;max-width:160px;
  transition:background var(--transition),color var(--transition);
  flex-shrink:0;
}
.bm-bar-item:hover{background:var(--bg-hover);color:var(--text)}
.bm-bar-icon,.bm-bar-fallback{width:14px;height:14px;border-radius:3px;flex-shrink:0}
.bm-bar-icon{object-fit:contain}
.bm-bar-fallback{display:flex;align-items:center;justify-content:center;background:var(--accent-dim);color:var(--accent);font-size:8px;font-weight:800}
.bm-bar-fallback.hidden{display:none}
.bm-bar-text{overflow:hidden;text-overflow:ellipsis}
.bm-bar-empty{font-size:11px;color:var(--text-muted);padding:0 8px}

/* zoom toast */
#zoom-toast{
  position:fixed;bottom:40px;left:50%;transform:translateX(-50%);
  background:var(--bg-elevated);border:1px solid var(--border);
  border-radius:var(--radius);padding:6px 14px;font-size:12px;color:var(--text);
  box-shadow:var(--shadow);z-index:500;opacity:0;pointer-events:none;
  transition:opacity 0.15s;
}
#zoom-toast.visible{opacity:1}

/* context menu */
#ctx-menu{
  position:fixed;background:var(--bg-elevated);border:1px solid var(--border);
  border-radius:var(--radius);box-shadow:var(--shadow);z-index:400;
  min-width:160px;padding:4px;display:none;
}
.ctx-item{
  padding:6px 12px;border-radius:var(--radius-sm);font-size:12px;
  color:var(--text);cursor:pointer;transition:background var(--transition);
  display:flex;align-items:center;gap:8px;
}
.ctx-item:hover{background:var(--bg-hover)}
.ctx-item.danger{color:var(--danger)}
.ctx-item.danger:hover{background:var(--danger-dim)}
.ctx-sep{height:1px;background:var(--border-subtle);margin:3px 0}

.bm-item{display:flex;align-items:center;gap:10px;padding:9px 12px;border-radius:var(--radius-sm);background:var(--bg);border:1px solid var(--border-subtle);cursor:pointer;transition:background var(--transition)}
.bm-item:hover{background:var(--bg-hover)}
.bm-item-info{flex:1;min-width:0}
.bm-item-title{font-size:12px;font-weight:500;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.bm-item-url{font-size:11px;color:var(--text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.bm-item-del{width:24px;height:24px;border:none;background:transparent;color:var(--text-dim);cursor:pointer;border-radius:3px;display:flex;align-items:center;justify-content:center;transition:all var(--transition);flex-shrink:0;opacity:0}
.bm-item:hover .bm-item-del{opacity:1}
.bm-item-del:hover{background:var(--danger-dim);color:var(--danger)}

.hist-item{display:flex;align-items:center;gap:10px;padding:8px 12px;border-radius:var(--radius-sm);transition:background var(--transition);cursor:pointer}
.hist-item:hover{background:var(--bg-hover)}
.hist-item-info{flex:1;min-width:0}
.hist-item-title{font-size:12px;font-weight:500;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.hist-item-url{font-size:11px;color:var(--text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.hist-item-time{font-size:11px;color:var(--text-dim);flex-shrink:0;white-space:nowrap}
.hist-item-del{width:24px;height:24px;border:none;background:transparent;color:var(--text-dim);cursor:pointer;border-radius:3px;display:flex;align-items:center;justify-content:center;transition:all var(--transition);flex-shrink:0;opacity:0}
.hist-item:hover .hist-item-del{opacity:1}
.hist-item-del:hover{background:var(--danger-dim);color:var(--danger)}

.dl-item{display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:var(--radius-sm);background:var(--bg);border:1px solid var(--border-subtle);cursor:pointer;transition:background var(--transition)}
.dl-item:hover{background:var(--bg-hover)}
.dl-item-icon{width:32px;height:32px;border-radius:var(--radius-sm);background:var(--accent-dim);display:flex;align-items:center;justify-content:center;color:var(--accent);flex-shrink:0}
.dl-item-info{flex:1;min-width:0}
.dl-item-name{font-size:12px;font-weight:500;color:var(--text);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.dl-item-meta{font-size:11px;color:var(--text-muted);margin-top:1px}
.dl-item-progress{margin-top:5px;height:3px;background:var(--border);border-radius:2px;overflow:hidden}
.dl-item-progress-bar{height:100%;background:var(--accent);transition:width 0.3s ease}
.dl-item-actions{display:flex;gap:4px;flex-shrink:0}
.dl-action-btn{width:26px;height:26px;border:none;background:var(--bg-hover);color:var(--text-muted);cursor:pointer;border-radius:3px;display:flex;align-items:center;justify-content:center;transition:all var(--transition)}
.dl-action-btn:hover{background:var(--bg-active);color:var(--text)}

/* ── About section ─────────────────────────────────────────────────────────── */
.about-identity-card{
  display:flex;align-items:center;gap:18px;
  padding:20px 20px 18px;
  background:var(--modal-bg-2);border:1px solid var(--modal-border);
  border-radius:16px;margin-top:10px;
  position:relative;overflow:hidden;
}
.about-identity-card::before{
  content:'';position:absolute;right:-40px;top:-60px;
  width:200px;height:200px;
  background:radial-gradient(circle,rgba(99,102,241,0.13),transparent 65%);
  pointer-events:none;
}
.about-identity-logo{
  width:64px;height:64px;
  flex-shrink:0;position:relative;z-index:1;
  display:flex;align-items:center;justify-content:center;
}
.about-identity-logo img{width:64px;height:64px;object-fit:contain;border-radius:16px}
.about-identity-body{min-width:0;flex:1;position:relative;z-index:1}
.about-identity-name{
  display:flex;align-items:baseline;gap:9px;flex-wrap:wrap;
  font-size:22px;font-weight:700;color:var(--text);letter-spacing:-0.5px;line-height:1.1;
}
.about-identity-ver{
  font-size:12px;font-weight:500;color:var(--text-muted);letter-spacing:0;
}
.about-identity-tagline{font-size:12px;color:var(--text-muted);margin-top:4px;line-height:1.4}
.about-identity-badges{display:flex;flex-wrap:wrap;gap:6px;margin-top:13px}
.aib{
  display:inline-flex;align-items:center;gap:5px;
  padding:3px 10px;border-radius:6px;
  background:var(--soft-btn-bg);
  color:var(--text-muted);font-size:11px;font-weight:500;
}
.aib-live{background:rgba(34,197,94,0.1);color:#34d399}
.aib-live-dot{width:5px;height:5px;border-radius:50%;background:#34d399;flex-shrink:0}

.about-rows{
  margin-top:12px;
  border:1px solid var(--modal-border);
  border-radius:12px;
  overflow:hidden;
}
.about-row{
  display:flex;align-items:center;justify-content:space-between;
  padding:11px 16px;
  border-bottom:1px solid var(--modal-border);
  font-size:12px;
}
.about-row:last-child{border-bottom:none}
.about-row-label{color:var(--text-muted)}
.about-row-val{color:var(--text);font-weight:500}
.about-row-link{color:var(--accent);text-decoration:none;font-weight:500}
.about-row-link:hover{text-decoration:underline}

.about-update-card{
  display:flex;align-items:center;justify-content:space-between;gap:12px;
  padding:14px 16px;
  background:var(--modal-bg-2);border:1px solid var(--modal-border);
  border-radius:12px;margin-top:10px;
}
.about-update-left{display:flex;align-items:center;gap:12px}
.about-update-icon{
  width:36px;height:36px;border-radius:10px;
  background:rgba(99,102,241,0.12);
  display:flex;align-items:center;justify-content:center;
  color:var(--accent);flex-shrink:0;
}
.about-update-title{font-size:13px;font-weight:600;color:var(--text);letter-spacing:-0.1px}
.about-update-sub{font-size:11px;color:var(--text-muted);margin-top:2px}

.about-actions{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}
.about-act-btn{
  display:inline-flex;align-items:center;gap:7px;
  padding:8px 16px;border-radius:8px;
  border:1px solid var(--modal-border);
  background:var(--soft-btn-bg);
  color:var(--text);font-size:12px;font-family:var(--font);font-weight:500;
  cursor:pointer;transition:all var(--transition);
}
.about-act-btn:hover{background:var(--soft-btn-bg-hover);border-color:var(--border)}
@media(max-width:760px){.about-identity-card{flex-direction:column;align-items:flex-start}}

/* custom window controls (frameless window) */
#win-controls{
  display:flex;align-items:stretch;gap:0;flex-shrink:0;
  align-self:stretch;margin-left:4px;
}
.win-btn{
  width:46px;height:100%;border:none;background:transparent;
  color:var(--text-dim);cursor:default;padding:0;
  display:flex;align-items:center;justify-content:center;
  transition:background var(--transition),color var(--transition);
  border-radius:0;flex-shrink:0;-webkit-user-select:none;
  -webkit-app-region:no-drag;
}
.win-btn:hover{background:var(--bg-hover);color:var(--text)}
.win-btn:active{background:var(--bg-active)}
.win-btn-close:hover{background:#c42b1c;color:#fff}
.win-btn-close:active{background:#b02316;color:#fff}

/* animations */
@keyframes spin{from{transform:rotate(0deg)}to{transform:rotate(360deg)}}
@keyframes bounce{0%,80%,100%{transform:scale(0.8);opacity:0.5}40%{transform:scale(1);opacity:1}}
@keyframes slide-up{from{transform:translateY(10px);opacity:0}to{transform:translateY(0);opacity:1}}
@keyframes tab-chip-pulse{0%,100%{background:transparent}50%{background:var(--bg-hover)}}
@keyframes tab-chip-pulse-active{0%,100%{background:var(--accent-dim)}50%{background:var(--accent-dim2,color-mix(in srgb,var(--accent) 22%,transparent))}}
@keyframes fade-in{from{opacity:0}to{opacity:1}}
/* Ventus-specific animations */
@keyframes ventus-fade-up{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:translateY(0)}}
@keyframes ventus-glow-pulse{0%,100%{box-shadow:0 0 0 0 rgba(99,102,241,0)}50%{box-shadow:0 0 0 4px rgba(99,102,241,0.18)}}
@keyframes ventus-slide-in-right{from{transform:translateX(8px);opacity:0}to{transform:translateX(0);opacity:1}}
@keyframes ventus-scale-in{from{transform:scale(0.96);opacity:0}to{transform:scale(1);opacity:1}}

/* collapsed sidebar */
#app.sidebar-collapsed .sb-bottom{display:none}
#app.sidebar-collapsed .tab-info,
#app.sidebar-collapsed .tab-close,
#app.sidebar-collapsed .tab-audio-btn{display:none}
#app.sidebar-collapsed .tab-item{justify-content:center;padding:7px;gap:0}
#app.sidebar-collapsed .tab-favicon{display:block}
#app.sidebar-collapsed #sidebar-toggle-btn{transform:rotate(180deg)}

/* auto-hide: base #sidebar already has position:fixed + translateX(-240px) */
#app.sidebar-auto-hide.sidebar-floating-open #sidebar{
  transform:translateX(0);
  box-shadow:var(--sidebar-float-shadow);
}
/* Content frame: thin strips on left, right, and bottom matching toolbar background.
   Left strip doubles as the auto-hide sidebar trigger. */
#sidebar-float-trigger{
  display:block;position:fixed;
  top:var(--top-chrome-h);left:var(--sidebar-w);
  width:var(--frame-side-w,5px);
  height:calc(100vh - var(--top-chrome-h) - var(--frame-bottom-h,5px));
  z-index:155;cursor:default;
  background:var(--bg);
  overflow:visible;
}
#frame-right{
  display:block;position:fixed;
  top:var(--top-chrome-h);right:var(--ai-w);
  width:var(--frame-side-w,5px);
  height:calc(100vh - var(--top-chrome-h) - var(--frame-bottom-h,5px));
  z-index:155;
  background:var(--bg);
}
#frame-bottom{
  display:block;position:fixed;
  bottom:0;left:0;right:0;
  height:var(--frame-bottom-h,5px);
  z-index:155;
  background:var(--bg);
}
#app.content-fullscreen #sidebar-float-trigger,
#app.content-fullscreen #frame-right,
#app.content-fullscreen #frame-bottom{display:none!important}
/* while sidebar is floating open the left strip passes events through */
#app.sidebar-auto-hide.sidebar-floating-open #sidebar-float-trigger{pointer-events:none}

/* pill indicator inside left frame — only shown in auto-hide mode */
#sidebar-pill{
  position:absolute;
  left:0;top:50%;
  transform:translateY(-50%);
  width:4px;height:52px;
  background:rgba(255,255,255,0.22);
  border-radius:0 5px 5px 0;
  transition:width 0.22s cubic-bezier(.4,0,.2,1),
             height 0.22s cubic-bezier(.4,0,.2,1),
             opacity 0.22s ease,
             background 0.22s ease;
  pointer-events:none;
}
#app:not(.sidebar-auto-hide) #sidebar-pill{display:none}
#sidebar-float-trigger:hover #sidebar-pill{
  width:22px;height:64px;
  background:rgba(255,255,255,0.55);
}
[data-theme="light"] #sidebar-pill{
  background:rgba(0,0,0,0.18);
}
[data-theme="light"] #sidebar-float-trigger:hover #sidebar-pill{
  background:rgba(0,0,0,0.40);
}
/* hide pill while sidebar is showing */
#app.sidebar-auto-hide.sidebar-floating-open #sidebar-pill{opacity:0;pointer-events:none}
/* transparent backdrop — catches clicks outside to close, no visual dim */
#sidebar-float-backdrop{
  display:none;position:fixed;
  top:var(--top-chrome-h);left:0;right:0;bottom:0;
  z-index:148;background:transparent;
}
#app.sidebar-auto-hide.sidebar-floating-open #sidebar-float-backdrop{display:block}
/* pinned: sidebar is solid — remove float shadow, hide backdrop, add border */
#app.sidebar-auto-hide.sidebar-pinned #sidebar{
  box-shadow:none;
  border-right:1px solid var(--border);
}
#app.sidebar-auto-hide.sidebar-pinned #sidebar-float-backdrop{display:none!important}

/* compact tab style */
#app.compact-tabs .tab-item{padding:4px 8px;min-height:26px}
#app.compact-tabs .tab-favicon{width:12px;height:12px}
#app.compact-tabs .tab-title{font-size:11px}
#app.compact-tabs .tab-url{display:none}

/* hide-tab-url */
#app.hide-tab-url .tab-url{display:none}

/* content overlay (loading) */
#content-loading{
  position:absolute;inset:0;
  display:flex;align-items:center;justify-content:center;
  background:var(--bg);z-index:10;pointer-events:none;opacity:0;
  transition:opacity 0.2s;
}
#content-loading.visible{opacity:1}

svg{display:block;flex-shrink:0}
</style>
</head>
<body>
<div id="app" class="sidebar-auto-hide">

<div id="top-chrome">
<div id="toolbar" onmousedown="handleToolbarDrag(event)" ondblclick="handleToolbarDblClick(event)">
  <div id="toolbar-nav">
  <button class="tb-btn" id="sidebar-toggle-btn" title="Toggle sidebar" onclick="toggleSidebar()">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M9 3v18"/></svg>
  </button>
  <div class="tb-sep"></div>
  <button class="tb-btn" id="btn-back" onclick="nav('Back')" disabled title="Back (Alt+Left)">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5m7-7-7 7 7 7"/></svg>
  </button>
  <button class="tb-btn" id="btn-forward" onclick="nav('Forward')" disabled title="Forward (Alt+Right)">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14m-7-7 7 7-7 7"/></svg>
  </button>
  <button class="tb-btn" id="btn-reload" onclick="nav('Reload')" title="Reload (F5)">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
  </button>
  </div>

  <div id="toolbar-url-area">
  <div id="address-bar" onclick="focusUrl()">
    <span id="toolbar-incognito-badge" onclick="event.stopPropagation()" title="Incognito workspace - history not saved">
      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><line x1="2" y1="2" x2="22" y2="22"/></svg>
      Incognito
    </span>
    <span id="lock-icon" style="display:none">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
    </span>
    <span id="insecure-icon" style="display:none">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
    </span>
    <img id="active-favicon" class="favicon" style="display:none" src="" alt="">
    <span id="active-loading-icon" class="favicon" style="display:none">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a10 10 0 0 1 0 20"/></svg>
    </span>
    <input id="url-input" type="text" placeholder="Search or enter a URL"
      onkeydown="handleUrlKey(event)"
      oninput="handleUrlInput(this.value)"
      onfocus="handleUrlFocus()"
      onblur="handleUrlBlur()">
    <button class="ab-icon-btn" id="btn-bookmark" onclick="event.stopPropagation();toggleBookmark()" title="Bookmark (Ctrl+D)">
      <svg id="bm-icon-empty" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/></svg>
      <svg id="bm-icon-filled" width="13" height="13" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:none;color:var(--accent)"><path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/></svg>
    </button>
    <button class="ab-icon-btn" id="btn-adblock" onclick="event.stopPropagation();openAdBlockModal()" title="Ad blocker">
      <svg id="adblock-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
      <svg id="adblock-icon-off" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="display:none"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="2" y1="2" x2="22" y2="22"/></svg>
    </button>
  </div>
  </div>

  <div id="toolbar-actions">
  <button class="tb-btn" id="btn-ai" onclick="toggleAi()" title="AI sidebar (Ctrl+Shift+A)">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.937A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.582a.5.5 0 0 1 0 .962L15.5 14.063A2 2 0 0 0 14.063 15.5l-1.582 6.135a.5.5 0 0 1-.963 0z"/><path d="M20 3v4m2-2h-4"/><path d="M4 17v2m1-1H3"/></svg>
  </button>
  <button class="tb-btn" id="btn-more" onclick="toggleMoreMenu(event)" title="More options" aria-haspopup="true">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="5" r="1.2" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="1.2" fill="currentColor" stroke="none"/><circle cx="12" cy="19" r="1.2" fill="currentColor" stroke="none"/></svg>
    <span id="more-btn-badge"></span>
  </button>
  <div class="tb-sep"></div>
  <div id="win-controls" onmousedown="event.stopPropagation()">
    <button class="win-btn" onclick="send('WindowMinimize')" title="Minimize">
      <svg width="11" height="11" viewBox="0 0 11 11" fill="currentColor"><path d="M11 4.399V5.5H0V4.399h11z"/></svg>
    </button>
    <button class="win-btn" id="win-btn-max" onclick="send('WindowMaximize')" title="Maximize / Restore">
      <svg width="11" height="11" viewBox="0 0 11 11" fill="none" stroke="currentColor" stroke-width="1"><rect x="0.5" y="0.5" width="10" height="10"/></svg>
    </button>
    <button class="win-btn win-btn-close" onclick="send('WindowClose')" title="Close">
      <svg width="11" height="11" viewBox="0 0 11 11" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"><line x1="0.5" y1="0.5" x2="10.5" y2="10.5"/><line x1="10.5" y1="0.5" x2="0.5" y2="10.5"/></svg>
    </button>
  </div>
  </div>
</div>
<div id="bookmarks-bar"></div>
</div>
<div id="url-suggestions" class="suggestions-panel"></div>

<!-- DOWNLOAD PANEL -->
<div id="download-panel">
  <div class="dl-panel-head">
    <div class="dl-panel-head-icon">
      <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
    </div>
    <span class="dl-panel-head-title">Downloads</span>
    <div class="dl-panel-head-right">
      <button id="dl-clear-btn" class="dl-clear-btn" onclick="send('ClearDownloads')" title="Clear all">Clear all</button>
    </div>
  </div>
  <div id="dl-panel-list"></div>
  <div class="dl-panel-foot">
    <a onclick="closeDownloadPanel();openSettings('downloads')">
      <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/><path d="M4.93 4.93a10 10 0 0 0 0 14.14"/></svg>
      Manage downloads
    </a>
  </div>
</div>

<!-- MORE MENU DROPDOWN -->
<div id="more-menu" role="menu" aria-label="Browser menu">
  <button class="more-item" onclick="closeMoreMenu();send('NewTab')" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14m-7-7h14"/></svg></span>
    <span class="more-item-label">New Tab</span>
    <span class="more-item-kbd">Ctrl+T</span>
  </button>
  <button class="more-item" onclick="closeMoreMenu();send('OpenInNewWindow',{url:'neura://newtab'})" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><polyline points="9 3 9 9 3 9"/></svg></span>
    <span class="more-item-label">New Window</span>
  </button>
  <button class="more-item" onclick="closeMoreMenu();send('NewWorkspace',{name:'Incognito',is_incognito:true,icon:'🔐',accent_color:'#6b7280'})" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><line x1="2" y1="2" x2="22" y2="22"/></svg></span>
    <span class="more-item-label">New Incognito Tab</span>
  </button>
  <div class="more-sep"></div>
  <button class="more-item" onclick="closeMoreMenu();send('OpenHistoryPanel')" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg></span>
    <span class="more-item-label">History</span>
    <span class="more-item-kbd">Ctrl+H</span>
  </button>
  <button class="more-item" onclick="closeMoreMenu();openSettings('bookmarks')" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/><line x1="9" y1="10" x2="15" y2="10"/></svg></span>
    <span class="more-item-label">Bookmarks</span>
  </button>
  <button class="more-item" onclick="closeMoreMenu();event.stopPropagation();toggleBookmark()" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z"/></svg></span>
    <span class="more-item-label">Bookmark this page</span>
    <span class="more-item-kbd">Ctrl+D</span>
  </button>
  <button class="more-item" onclick="closeMoreMenu();setTimeout(function(){toggleDownloadPanel(null)},60)" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg></span>
    <span class="more-item-label">Downloads</span>
    <span class="more-item-kbd">Ctrl+J</span>
  </button>
  <div class="more-sep"></div>
  <div class="more-zoom-row">
    <span class="more-zoom-label">Zoom</span>
    <div class="more-zoom-controls">
      <button class="more-zoom-btn" onclick="zoomOut();updateMoreMenuZoom()" title="Zoom out">−</button>
      <span id="more-zoom-pct" onclick="zoomReset();updateMoreMenuZoom()" title="Reset zoom">100%</span>
      <button class="more-zoom-btn" onclick="zoomIn();updateMoreMenuZoom()" title="Zoom in">+</button>
    </div>
  </div>
  <div class="more-sep"></div>
  <button class="more-item" onclick="closeMoreMenu();openSettings('general')" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg></span>
    <span class="more-item-label">Settings</span>
  </button>
  <button class="more-item" onclick="closeMoreMenu();checkForUpdate()" role="menuitem">
    <span class="more-item-icon"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg></span>
    <span class="more-item-label">Check for Updates</span>
  </button>
</div>

<!-- ADBLOCK INFO MODAL -->
<div id="adblock-backdrop" onclick="closeAdBlockModal()"></div>
<div id="adblock-modal">
  <div class="abm-header">
    <div class="abm-header-icon">
      <svg id="abm-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
    </div>
    <span class="abm-title">Ad Blocker</span>
  </div>
  <div class="abm-body">
    <div id="abm-status" class="abm-status-row">
      <div id="abm-dot" class="abm-status-dot"></div>
      <span id="abm-status-text" class="abm-status-label">Active on this page</span>
    </div>
    <button class="abm-action-btn" id="abm-site-toggle" onclick="send('AdBlockToggleSite')">
      <span id="abm-site-toggle-text">Pause for this site</span>
      <span class="abm-btn-right">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M10 9v6m4-6v6M5 5h14v14H5z"/></svg>
      </span>
    </button>
    <button class="abm-settings-btn" onclick="closeAdBlockModal();openSettings('privacy')">
      <span>Manage settings</span>
      <span class="abm-settings-btn-right">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14m-7-7 7 7-7 7"/></svg>
      </span>
    </button>
  </div>
</div>

<!-- ZOOM TOAST -->
<div id="zoom-toast"></div>

<!-- CONTEXT MENU -->
<div id="context-menu"></div>

<!-- SIDEBAR -->
<div id="sidebar">

  <!-- Brand: logo + title + new tab — unchanged -->
  <div class="sidebar-brand">
    <div class="sidebar-brand-left" onclick="send('NewTab')" title="New tab">
      <img class="sidebar-brand-logo" src="__LOGO_URL__" alt="">
      <div class="sidebar-brand-info">
        <span class="sidebar-brand-name">Ventus</span>
      </div>
    </div>
    <button class="sidebar-brand-add" onclick="openNewTabSpotlight()" title="New tab (Ctrl+T)">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14m-7-7h14"/></svg>
    </button>
  </div>

  <!-- Tab pages viewport — clips animated slide -->
  <div class="sb-viewport">
    <div class="sb-page" id="sb-page">
      <!-- populated by renderTabs() -->
    </div>
  </div>

  <!-- Bottom bar: history | workspace dots | new tab -->
  <div class="sb-bottom">
    <button class="sb-bottom-btn" onclick="send('OpenHistoryPanel')" title="History">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
    </button>
    <div class="sb-ws-nav">
      <button class="sb-ws-nav-btn" id="sb-ws-prev" onclick="prevWorkspace()" title="Previous workspace">
        <svg width="6" height="10" viewBox="0 0 6 10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 1 L1 5 L5 9"/></svg>
      </button>
      <div class="sb-ws-dots" id="sb-ws-dots" onwheel="scrollWsDots(event)">
        <!-- populated by renderWorkspaces() -->
      </div>
      <button class="sb-ws-nav-btn" id="sb-ws-next" onclick="nextWorkspace()" title="Next workspace">
        <svg width="6" height="10" viewBox="0 0 6 10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 1 L5 5 L1 9"/></svg>
      </button>
    </div>
    <button class="sb-bottom-btn" onclick="openWorkspaceModal()" title="New workspace">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/></svg>
    </button>
  </div>

</div>

<div id="sidebar-float-trigger"><div id="sidebar-pill"></div></div>
<div id="frame-right"></div>
<div id="frame-bottom"></div>
<div id="sidebar-float-backdrop" onclick="hideFloatingSidebar(true)"></div>

<!-- Workspace dot hover popover -->
<div id="sb-ws-popover" class="sb-ws-popover" onmouseenter="keepWsPop()" onmouseleave="hideWsPop()">
  <div class="sb-ws-pop-avatar" id="sb-ws-pop-avatar"></div>
  <div class="sb-ws-pop-name" id="sb-ws-pop-name"></div>
  <div class="sb-ws-pop-count" id="sb-ws-pop-count"></div>
  <div class="sb-ws-pop-note" id="sb-ws-pop-note"></div>
  <div class="sb-ws-pop-actions">
    <button class="sb-ws-pop-btn" onclick="showWsPopRename()">Rename</button>
    <button class="sb-ws-pop-btn danger" id="sb-ws-pop-delete" onclick="showWsPopDelete()">Delete</button>
  </div>
</div>

<div id="workspace-modal" onclick="handleWorkspaceModalClick(event)">
  <div class="workspace-dialog" role="dialog" aria-modal="true" aria-labelledby="workspace-modal-title">
    <div class="workspace-dialog-head">
      <div class="workspace-dialog-title" id="workspace-modal-title">New workspace</div>
      <button class="workspace-dialog-close" onclick="closeWorkspaceModal()" title="Close">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>
    </div>
    <form class="workspace-form" onsubmit="submitWorkspaceModal(event)">
      <div class="workspace-field" id="ws-icon-field">
        <label>Icon</label>
        <div class="ws-emoji-row">
          <div class="ws-emoji-preview" id="ws-emoji-preview">📁</div>
          <div class="ws-emoji-grid">
            <button type="button" class="ws-emoji-opt" data-emoji="🌐" onclick="selectWsEmoji(this,'🌐')">🌐</button><button type="button" class="ws-emoji-opt" data-emoji="💼" onclick="selectWsEmoji(this,'💼')">💼</button><button type="button" class="ws-emoji-opt" data-emoji="🏠" onclick="selectWsEmoji(this,'🏠')">🏠</button><button type="button" class="ws-emoji-opt" data-emoji="🔬" onclick="selectWsEmoji(this,'🔬')">🔬</button><button type="button" class="ws-emoji-opt" data-emoji="💬" onclick="selectWsEmoji(this,'💬')">💬</button><button type="button" class="ws-emoji-opt" data-emoji="🛍️" onclick="selectWsEmoji(this,'🛍️')">🛍️</button><button type="button" class="ws-emoji-opt" data-emoji="📰" onclick="selectWsEmoji(this,'📰')">📰</button><button type="button" class="ws-emoji-opt" data-emoji="💻" onclick="selectWsEmoji(this,'💻')">💻</button><button type="button" class="ws-emoji-opt" data-emoji="🎵" onclick="selectWsEmoji(this,'🎵')">🎵</button><button type="button" class="ws-emoji-opt" data-emoji="🎬" onclick="selectWsEmoji(this,'🎬')">🎬</button><button type="button" class="ws-emoji-opt selected" data-emoji="📁" onclick="selectWsEmoji(this,'📁')">📁</button><button type="button" class="ws-emoji-opt" data-emoji="🎨" onclick="selectWsEmoji(this,'🎨')">🎨</button><button type="button" class="ws-emoji-opt" data-emoji="🚀" onclick="selectWsEmoji(this,'🚀')">🚀</button><button type="button" class="ws-emoji-opt" data-emoji="❤️" onclick="selectWsEmoji(this,'❤️')">❤️</button><button type="button" class="ws-emoji-opt" data-emoji="⭐" onclick="selectWsEmoji(this,'⭐')">⭐</button><button type="button" class="ws-emoji-opt" data-emoji="🔥" onclick="selectWsEmoji(this,'🔥')">🔥</button><button type="button" class="ws-emoji-opt" data-emoji="💡" onclick="selectWsEmoji(this,'💡')">💡</button><button type="button" class="ws-emoji-opt" data-emoji="🏆" onclick="selectWsEmoji(this,'🏆')">🏆</button><button type="button" class="ws-emoji-opt" data-emoji="🎯" onclick="selectWsEmoji(this,'🎯')">🎯</button><button type="button" class="ws-emoji-opt" data-emoji="📚" onclick="selectWsEmoji(this,'📚')">📚</button><button type="button" class="ws-emoji-opt" data-emoji="🌟" onclick="selectWsEmoji(this,'🌟')">🌟</button><button type="button" class="ws-emoji-opt" data-emoji="🍕" onclick="selectWsEmoji(this,'🍕')">🍕</button><button type="button" class="ws-emoji-opt" data-emoji="🎮" onclick="selectWsEmoji(this,'🎮')">🎮</button><button type="button" class="ws-emoji-opt" data-emoji="🌈" onclick="selectWsEmoji(this,'🌈')">🌈</button><button type="button" class="ws-emoji-opt" data-emoji="🦋" onclick="selectWsEmoji(this,'🦋')">🦋</button><button type="button" class="ws-emoji-opt" data-emoji="🌸" onclick="selectWsEmoji(this,'🌸')">🌸</button><button type="button" class="ws-emoji-opt" data-emoji="🎸" onclick="selectWsEmoji(this,'🎸')">🎸</button><button type="button" class="ws-emoji-opt" data-emoji="🌙" onclick="selectWsEmoji(this,'🌙')">🌙</button><button type="button" class="ws-emoji-opt" data-emoji="☀️" onclick="selectWsEmoji(this,'☀️')">☀️</button><button type="button" class="ws-emoji-opt" data-emoji="🌊" onclick="selectWsEmoji(this,'🌊')">🌊</button><button type="button" class="ws-emoji-opt" data-emoji="🧩" onclick="selectWsEmoji(this,'🧩')">🧩</button><button type="button" class="ws-emoji-opt" data-emoji="🔐" onclick="selectWsEmoji(this,'🔐')">🔐</button><button type="button" class="ws-emoji-opt" data-emoji="📊" onclick="selectWsEmoji(this,'📊')">📊</button><button type="button" class="ws-emoji-opt" data-emoji="🎭" onclick="selectWsEmoji(this,'🎭')">🎭</button><button type="button" class="ws-emoji-opt" data-emoji="🏖️" onclick="selectWsEmoji(this,'🏖️')">🏖️</button><button type="button" class="ws-emoji-opt" data-emoji="🚗" onclick="selectWsEmoji(this,'🚗')">🚗</button><button type="button" class="ws-emoji-opt" data-emoji="✈️" onclick="selectWsEmoji(this,'✈️')">✈️</button><button type="button" class="ws-emoji-opt" data-emoji="🎓" onclick="selectWsEmoji(this,'🎓')">🎓</button><button type="button" class="ws-emoji-opt" data-emoji="💎" onclick="selectWsEmoji(this,'💎')">💎</button><button type="button" class="ws-emoji-opt" data-emoji="🎪" onclick="selectWsEmoji(this,'🎪')">🎪</button>
          </div>
        </div>
      </div>
      <div class="workspace-field">
        <label>Color theme</label>
        <div class="ws-color-row">
          <div class="ws-color-preview" id="ws-color-preview"></div>
          <div class="ws-color-main">
            <div class="ws-color-swatches" id="ws-color-swatches">
              <button type="button" class="ws-color-opt" data-color="#8b5cf6" style="--swatch-rgb:139,92,246" onclick="selectWsColor('#8b5cf6',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#3b82f6" style="--swatch-rgb:59,130,246" onclick="selectWsColor('#3b82f6',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#06b6d4" style="--swatch-rgb:6,182,212" onclick="selectWsColor('#06b6d4',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#22c55e" style="--swatch-rgb:34,197,94" onclick="selectWsColor('#22c55e',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#eab308" style="--swatch-rgb:234,179,8" onclick="selectWsColor('#eab308',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#f97316" style="--swatch-rgb:249,115,22" onclick="selectWsColor('#f97316',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#ef4444" style="--swatch-rgb:239,68,68" onclick="selectWsColor('#ef4444',true)"></button>
              <button type="button" class="ws-color-opt" data-color="#ec4899" style="--swatch-rgb:236,72,153" onclick="selectWsColor('#ec4899',true)"></button>
            </div>
            <input type="color" id="ws-color-input" value="#8b5cf6" oninput="selectWsColor(this.value,true)">
          </div>
        </div>
      </div>
      <div class="workspace-field">
        <label for="workspace-name-input">Name</label>
        <input id="workspace-name-input" type="text" maxlength="48" autocomplete="off" placeholder="Design, research, client work">
      </div>
      <div class="workspace-error" id="workspace-name-error"></div>
      <div class="workspace-field" id="workspace-incognito-row" style="flex-direction:row;align-items:center;gap:8px">
        <input type="checkbox" id="workspace-incognito-check" style="width:14px;height:14px;cursor:pointer;flex-shrink:0">
        <label for="workspace-incognito-check" style="cursor:pointer;margin:0;font-size:12px">Incognito (no history)</label>
      </div>
      <div class="workspace-actions">
        <button class="workspace-btn" type="button" onclick="closeWorkspaceModal()">Cancel</button>
        <button class="workspace-btn workspace-btn-primary" type="submit">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14m-7-7h14"/></svg>
          <span id="workspace-submit-label">Create</span>
        </button>
      </div>
    </form>
  </div>
</div>

<div id="workspace-delete-modal" onclick="handleWorkspaceDeleteModalClick(event)">
  <div class="delete-dialog" role="dialog" aria-modal="true" aria-labelledby="workspace-delete-title">
    <div class="delete-icon">
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/><path d="M10 11v6"/><path d="M14 11v6"/></svg>
    </div>
    <div class="delete-title" id="workspace-delete-title">Delete workspace?</div>
    <div class="delete-copy">This will close every tab in <strong id="workspace-delete-name">this workspace</strong>. You cannot undo this.</div>
    <div class="delete-actions">
      <button class="delete-btn" id="workspace-delete-cancel" onclick="closeWorkspaceDeleteModal()">Cancel</button>
      <button class="delete-btn delete-btn-danger" id="workspace-delete-confirm" onclick="confirmWorkspaceDelete()">Delete</button>
    </div>
  </div>
</div>

<!-- CONTENT AREA -->
<div id="content-area">
  <div id="newtab-placeholder">
    <div id="newtab-bg"></div>
    <div class="newtab-shell">
      <div class="newtab-top">
        <div class="newtab-brand">
          <img class="newtab-logo" src="__LOGO_URL__" alt="">
          <span>Ventus</span>
        </div>
        <div class="newtab-top-actions">
          <div class="newtab-date" id="newtab-date">Today</div>
          <button class="newtab-settings-btn" onclick="openSettings('newtab')" title="New tab settings">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="M12 15.5A3.5 3.5 0 1 0 12 8a3.5 3.5 0 0 0 0 7.5Z"/><path d="M19.4 15a1.8 1.8 0 0 0 .36 1.98l.06.06a2.15 2.15 0 1 1-3.04 3.04l-.06-.06a1.8 1.8 0 0 0-1.98-.36 1.8 1.8 0 0 0-1.09 1.65V21.5a2.15 2.15 0 1 1-4.3 0v-.09A1.8 1.8 0 0 0 8.26 19.8a1.8 1.8 0 0 0-1.98.36l-.06.06a2.15 2.15 0 1 1-3.04-3.04l.06-.06A1.8 1.8 0 0 0 3.6 15a1.8 1.8 0 0 0-1.65-1.09H1.86a2.15 2.15 0 1 1 0-4.3h.09A1.8 1.8 0 0 0 3.6 8.52a1.8 1.8 0 0 0-.36-1.98l-.06-.06a2.15 2.15 0 1 1 3.04-3.04l.06.06a1.8 1.8 0 0 0 1.98.36 1.8 1.8 0 0 0 1.09-1.65V2.12a2.15 2.15 0 1 1 4.3 0v.09a1.8 1.8 0 0 0 1.09 1.65 1.8 1.8 0 0 0 1.98-.36l.06-.06a2.15 2.15 0 1 1 3.04 3.04l-.06.06a1.8 1.8 0 0 0-.36 1.98 1.8 1.8 0 0 0 1.65 1.09h.09a2.15 2.15 0 1 1 0 4.3h-.09A1.8 1.8 0 0 0 19.4 15Z"/></svg>
          </button>
        </div>
      </div>
      <div class="newtab-hero">
        <div id="newtab-clock" aria-live="off">12:00</div>
        <div class="newtab-greeting" id="newtab-greeting">Good afternoon</div>
        <div class="newtab-sub" id="newtab-sub">Search, jump back in, or catch up on Neura Feed.</div>
        <div class="newtab-search-wrap">
          <div class="newtab-search">
            <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="var(--nt-muted)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
            <input id="newtab-input" placeholder="Search the web"
              onkeydown="handleNewtabKey(event)"
              oninput="handleNewtabInput(this.value)"
              onfocus="handleNewtabFocus()"
              onblur="handleNewtabBlur()">
            <img id="newtab-search-logo" src="__LOGO_URL__" alt="">
          </div>
          <div id="newtab-suggestions" class="suggestions-panel"></div>
        </div>
        <div class="newtab-shortcuts" id="newtab-shortcuts"></div>
      </div>
      <div class="newtab-feed">
        <div class="newtab-feed-head">
          <div class="newtab-feed-title">Latest from Neura Feed</div>
          <div class="newtab-feed-actions">
            <button class="newtab-feed-btn" data-more-feed>See more</button>
            <button class="newtab-feed-btn" data-refresh-feed>Refresh</button>
          </div>
        </div>
        <div class="newtab-feed-main" id="newtab-feed-main"></div>
      </div>
    </div>
  </div>
  <div id="content-loading">
    <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="animation:spin 1s linear infinite"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
  </div>
</div>

<!-- AI SIDEBAR -->
<div id="ai-sidebar">
  <div id="ai-header">
    <div class="ai-top-left">
      <button class="ai-icon-btn" onclick="aiClear()" title="New chat">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14"/><path d="M5 12h14"/></svg>
      </button>
      <button class="ai-icon-btn" onclick="openSettings('ai')" title="AI settings">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>
      </button>
    </div>
    <div class="ai-top-right">
      <div class="ai-provider-dd" id="ai-provider-dd">
        <button class="ai-provider-dd-btn" onclick="toggleProviderDd(event)">
          <span id="ai-provider-dd-label">Anthropic</span>
          <svg class="ai-provider-dd-chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
        </button>
        <div class="ai-provider-dd-menu" id="ai-provider-dd-menu">
          <div class="ai-provider-dd-item" data-provider="anthropic" onclick="selectProviderDd('anthropic')"><div class="ai-provider-dd-item-dot"></div>Anthropic</div>
          <div class="ai-provider-dd-item" data-provider="openai" onclick="selectProviderDd('openai')"><div class="ai-provider-dd-item-dot"></div>OpenAI</div>
          <div class="ai-provider-dd-item" data-provider="gemini" onclick="selectProviderDd('gemini')"><div class="ai-provider-dd-item-dot"></div>Gemini</div>
          <div class="ai-provider-dd-item" data-provider="openrouter" onclick="selectProviderDd('openrouter')"><div class="ai-provider-dd-item-dot"></div>OpenRouter</div>
          <div class="ai-provider-dd-item" data-provider="ollama" onclick="selectProviderDd('ollama')"><div class="ai-provider-dd-item-dot"></div>Ollama</div>
        </div>
      </div>
      <button class="ai-icon-btn" onclick="toggleAi()" title="Close AI">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>
    </div>
  </div>
  <div class="ai-hero">
    <h3>What can I help with?</h3>
    <div class="ai-hero-sub">Ask about the current page, summarize it, or use Ventus as your assistant.</div>
    <button id="ai-key-status" onclick="openSettings('ai')" title="Open AI provider settings">
      <span id="ai-key-dot"></span>
      <span id="ai-key-text">Add an API key locally</span>
    </button>
  </div>
  <div id="ai-messages">
    <div class="ai-empty"></div>
  </div>
  <div id="ai-quick-actions">
    <button id="ai-page-chip" onclick="togglePageChipCollapse()">
      <span class="ai-page-left">
        <span class="ai-page-icon">AI</span>
        <span id="ai-page-title">Current page</span>
      </span>
      <svg class="ai-chip-chevron" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
    </button>
    <button class="ai-qa-btn" onclick="aiQuickAction('summarize')">Create a summary of this page</button>
    <button class="ai-qa-btn" onclick="aiQuickAction('explain')">Expand on this topic</button>
    <button class="ai-qa-btn" onclick="aiQuickAction('key_points')">Pull out the key points</button>
  </div>
  <div id="ai-input-area">
    <div class="ai-composer">
      <textarea id="ai-input" placeholder="Ask Ventus anything about this page" rows="2" onkeydown="handleAiKey(event)"></textarea>
      <div class="ai-composer-actions">
        <div class="ai-composer-left">
          <button class="ai-circle-btn" onclick="openSettings('ai')" title="Add or manage API keys">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14"/><path d="M5 12h14"/></svg>
          </button>
          <button class="ai-pill-btn" onclick="openModelModal(event)" title="Change model">
            <span id="ai-model-pill">Smart</span>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>
          </button>
        </div>
        <div class="ai-composer-right">
          <button id="ai-clear-btn" onclick="aiClear()">Clear chat</button>
          <button id="ai-send-btn" onclick="sendAiMessage()" title="Send">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="m22 2-7 20-4-9-9-4 20-7z"/><path d="M22 2 11 13"/></svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<!-- MODEL PICKER MODAL -->
<div id="model-modal" onclick="handleModelModalBg(event)">
  <div id="model-modal-panel">
    <div class="mm-header">
      <span class="mm-title">Select Model</span>
      <button class="mm-close" onclick="closeModelModal()">&#x2715;</button>
    </div>
    <div class="mm-providers" id="mm-providers"></div>
    <div class="mm-models" id="mm-models"></div>
    <div class="mm-custom">
      <input class="mm-custom-input" id="mm-custom-input" placeholder="Custom model ID..." onkeydown="if(event.key==='Enter')applyCustomModel()">
      <button class="mm-custom-btn" onclick="applyCustomModel()">Use</button>
    </div>
  </div>
</div>

<!-- SETTINGS OVERLAY -->
<div id="settings-overlay" onclick="handleSettingsOverlayClick(event)">
  <div id="settings-panel">
    <nav class="settings-nav">
      <div class="settings-nav-group">General</div>
      <div class="settings-nav-item active" data-section="general" onclick="switchSettings('general')">General</div>
      <div class="settings-nav-item" data-section="appearance" onclick="switchSettings('appearance')">Appearance</div>
      <div class="settings-nav-item" data-section="search" onclick="switchSettings('search')">Search</div>
      <div class="settings-nav-group">Browser</div>
      <div class="settings-nav-item" data-section="newtab" onclick="switchSettings('newtab')">New tab</div>
      <div class="settings-nav-item" data-section="tabs" onclick="switchSettings('tabs')">Tabs</div>
      <div class="settings-nav-item" data-section="bookmarks" onclick="switchSettings('bookmarks')">Bookmarks</div>
      <div class="settings-nav-item" data-section="history" onclick="switchSettings('history')">History</div>
      <div class="settings-nav-item" data-section="downloads" onclick="switchSettings('downloads')">Downloads</div>
      <div class="settings-nav-group">AI</div>
      <div class="settings-nav-item" data-section="ai" onclick="switchSettings('ai')">AI Providers</div>
      <div class="settings-nav-group">System</div>
      <div class="settings-nav-item" data-section="privacy" onclick="switchSettings('privacy')">Privacy</div>
      <div class="settings-nav-item" data-section="keyboard" onclick="switchSettings('keyboard')">Keyboard</div>
      <div class="settings-nav-item" data-section="about" onclick="switchSettings('about')">About</div>
    </nav>
    <div class="settings-content" style="position:relative">
      <button class="settings-close" onclick="closeSettings()">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>

      <div class="settings-section active" id="section-general">
        <h2>General</h2>
        <p class="subtitle">Basic browser settings</p>
        <div class="settings-group">
          <label>Startup behavior</label>
          <select class="settings-select" id="set-startup" onchange="saveSetting('startup_behavior',this.value)">
            <option value="new_tab">Open a new tab</option>
            <option value="last_session">Restore last session</option>
            <option value="home_page">Open home page</option>
          </select>
          <div class="hint" id="current-startup"></div>
        </div>
        <div class="settings-group">
          <label>Home page URL</label>
          <input class="settings-input" id="set-homepage" placeholder="google.com or https://..." type="text" inputmode="url" onblur="saveSetting('homepage',this.value)" onkeydown="if(event.key==='Enter'){saveSetting('homepage',this.value);this.blur();}">
          <div class="hint" id="current-homepage"></div>
        </div>
        <div class="settings-group">
          <label>Download location</label>
          <div class="settings-path-row">
            <input class="settings-input" id="set-download-path" placeholder="Default downloads folder" onblur="saveSetting('download_path',this.value)" onkeydown="if(event.key==='Enter'){saveSetting('download_path',this.value);this.blur();}">
            <button class="settings-btn" type="button" onclick="browseDownloadFolder()">Browse</button>
          </div>
          <div class="hint" id="current-download-path"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Ask where to save files</div>
            <div class="toggle-desc" id="current-ask-download">Get prompted each time you download something</div>
          </div>
          <div class="toggle-switch" id="toggle-ask-download" onclick="toggleSetting('ask_download')"></div>
        </div>
        <div class="settings-group">
          <label>Country / Region</label>
          <select class="settings-select" id="set-region" onchange="onRegionChange(this.value)">
            <option value="">Not set</option>
          </select>
          <div class="hint">Adjusts unit and currency suggestions in Spotlight</div>
        </div>
      </div>

      <div class="settings-section" id="section-appearance">
        <h2>Appearance</h2>
        <p class="subtitle">Make it look the way you like</p>
        <div class="settings-group">
          <label>Theme</label>
          <div class="theme-cards">
            <div class="theme-card selected" id="theme-dark" onclick="setTheme('dark')">
              <div class="theme-preview" style="background:linear-gradient(135deg,#0f0f10,#1a1a1c)"></div>
              <div class="theme-name">Dark</div>
            </div>
            <div class="theme-card" id="theme-light" onclick="setTheme('light')">
              <div class="theme-preview" style="background:linear-gradient(135deg,#f5f5f7,#ffffff)"></div>
              <div class="theme-name">Light</div>
            </div>
            <div class="theme-card" id="theme-system" onclick="setTheme('system')">
              <div class="theme-preview" style="background:linear-gradient(135deg,#0f0f10 50%,#f5f5f7 50%)"></div>
              <div class="theme-name">System</div>
            </div>
          </div>
        </div>
        <div class="settings-group">
          <label>Sidebar mode</label>
          <select class="settings-select" id="set-sidebar-mode" onchange="saveSetting('sidebar_mode',this.value)">
            <option value="expanded">Expanded</option>
            <option value="compact">Compact</option>
            <option value="auto_hide">Auto-hide (float on hover)</option>
          </select>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Show bookmarks bar</div>
            <div class="toggle-desc">Display a bar below the toolbar with your most-used bookmarks</div>
          </div>
          <div class="toggle-switch" id="toggle-show-bookmarks-bar" onclick="toggleSetting('show_bookmarks_bar')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Show tab URL in sidebar</div>
            <div class="toggle-desc">Display the URL below tab titles in the vertical list</div>
          </div>
          <div class="toggle-switch" id="toggle-show-url" onclick="toggleSetting('show_tab_url')"></div>
        </div>
        <div class="settings-group">
          <label style="display:flex;align-items:center;justify-content:space-between">
            <span>Page zoom</span>
            <span id="zoom-level-display" style="font-weight:650;color:var(--accent);font-size:13px">100%</span>
          </label>
          <input type="range" id="global-zoom-slider" min="50" max="150" step="10" value="100"
            style="width:100%;accent-color:var(--accent);cursor:pointer;margin-top:6px"
            oninput="onZoomSliderInput(this.value)"
            onchange="applyGlobalZoom(this.value)">
          <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text-dim);margin-top:3px">
            <span>50%</span><span>100%</span><span>150%</span>
          </div>
        </div>
      </div>

      <div class="settings-section" id="section-search">
        <h2>Search</h2>
        <p class="subtitle">Pick your default search engine and shortcuts</p>
        <div class="settings-group">
          <label>Default search engine</label>
          <select class="settings-select" id="set-search-engine">
            <!-- populated by JS -->
          </select>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Show search suggestions</div>
            <div class="toggle-desc">Fetch suggestions as you type in the address bar</div>
          </div>
          <div class="toggle-switch" id="toggle-suggestions" onclick="toggleSetting('search_suggestions')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Trending searches</div>
            <div class="toggle-desc">Show popular searches in the dropdown</div>
          </div>
          <div class="toggle-switch" id="toggle-trending" onclick="toggleSetting('trending')"></div>
        </div>
      </div>

      <div class="settings-section" id="section-newtab">
        <h2>New tab</h2>
        <p class="subtitle">Choose what shows up when you open a new tab</p>

        <!-- Theme preset -->
        <div class="settings-group">
          <label>Layout theme</label>
          <div class="nt-theme-grid" id="nt-theme-grid">
            <button class="nt-theme-card" data-theme="minimal" onclick="setNewtabTheme('minimal')">
              <div class="nt-theme-preview">
                <div style="width:60%;height:3px;background:currentColor;border-radius:2px;margin:0 auto 5px"></div>
                <div style="width:36%;height:2px;background:currentColor;opacity:.4;border-radius:2px;margin:0 auto"></div>
              </div>
              <div class="nt-theme-label">Minimal</div>
            </button>
            <button class="nt-theme-card" data-theme="focus" onclick="setNewtabTheme('focus')">
              <div class="nt-theme-preview">
                <div style="width:28%;height:14px;background:currentColor;opacity:.25;border-radius:3px;margin:0 auto 4px"></div>
                <div style="width:60%;height:3px;background:currentColor;border-radius:2px;margin:0 auto"></div>
              </div>
              <div class="nt-theme-label">Focus</div>
            </button>
            <button class="nt-theme-card" data-theme="horizon" onclick="setNewtabTheme('horizon')">
              <div class="nt-theme-preview">
                <div style="width:50%;height:2px;background:currentColor;opacity:.5;border-radius:2px;margin:0 auto 5px"></div>
                <div style="width:70%;height:3px;background:currentColor;border-radius:2px;margin:0 auto 5px"></div>
                <div style="display:flex;gap:3px;justify-content:center"><div style="width:20%;height:10px;background:currentColor;opacity:.2;border-radius:2px"></div><div style="width:20%;height:10px;background:currentColor;opacity:.2;border-radius:2px"></div><div style="width:20%;height:10px;background:currentColor;opacity:.2;border-radius:2px"></div></div>
              </div>
              <div class="nt-theme-label">Horizon</div>
            </button>
            <button class="nt-theme-card" data-theme="informative" onclick="setNewtabTheme('informative')">
              <div class="nt-theme-preview">
                <div style="width:70%;height:3px;background:currentColor;border-radius:2px;margin:0 auto 4px"></div>
                <div style="display:flex;gap:2px;justify-content:center;margin-bottom:4px"><div style="width:22%;height:8px;background:currentColor;opacity:.3;border-radius:2px"></div><div style="width:22%;height:8px;background:currentColor;opacity:.3;border-radius:2px"></div><div style="width:22%;height:8px;background:currentColor;opacity:.3;border-radius:2px"></div></div>
                <div style="width:80%;height:2px;background:currentColor;opacity:.3;border-radius:2px;margin:0 auto 2px"></div>
                <div style="width:80%;height:2px;background:currentColor;opacity:.2;border-radius:2px;margin:0 auto"></div>
              </div>
              <div class="nt-theme-label">Informative</div>
            </button>
          </div>
        </div>

        <div class="settings-group">
          <label>Clock style</label>
          <div class="nt-clock-grid" id="nt-clock-grid">
            <button class="nt-clock-card" data-clock="sf" onclick="setClockStyle('sf')">
              <div class="nt-clock-preview">10:30</div>
              <div class="nt-clock-label">SF</div>
            </button>
            <button class="nt-clock-card" data-clock="rounded" onclick="setClockStyle('rounded')">
              <div class="nt-clock-preview">10:30</div>
              <div class="nt-clock-label">Rounded</div>
            </button>
            <button class="nt-clock-card" data-clock="mono" onclick="setClockStyle('mono')">
              <div class="nt-clock-preview">10:30</div>
              <div class="nt-clock-label">Mono</div>
            </button>
            <button class="nt-clock-card" data-clock="serif" onclick="setClockStyle('serif')">
              <div class="nt-clock-preview">10:30</div>
              <div class="nt-clock-label">Serif</div>
            </button>
          </div>
        </div>

        <!-- Wallpaper -->
        <div class="settings-group">
          <label>Wallpaper</label>
          <div class="nt-wp-sources" id="nt-wp-sources">
            <button class="nt-wp-src-btn" data-src="daily" onclick="setWallpaperSource('daily')">Daily photo</button>
            <button class="nt-wp-src-btn" data-src="nature" onclick="setWallpaperSource('nature')">Nature</button>
            <button class="nt-wp-src-btn" data-src="url" onclick="setWallpaperSource('url')">Custom URL</button>
            <button class="nt-wp-src-btn" data-src="upload" onclick="setWallpaperSource('upload')">Upload</button>
            <button class="nt-wp-src-btn" data-src="color" onclick="setWallpaperSource('color')">Color</button>
            <button class="nt-wp-src-btn" data-src="none" onclick="setWallpaperSource('none')">None</button>
          </div>
          <!-- URL input -->
          <div id="nt-wp-url-row" style="display:none;margin-top:8px">
            <input class="settings-input" id="nt-wp-url-input" type="url" placeholder="https://example.com/image.jpg"
              oninput="saveSetting('new_tab_wallpaper_url',this.value);initWallpaper(window.__ntState||{})"
              style="width:100%;box-sizing:border-box">
          </div>
          <!-- Upload -->
          <div id="nt-wp-upload-row" style="display:none;margin-top:8px;align-items:center;gap:10px">
            <button class="settings-btn-sm" id="nt-wp-upload-choose" onclick="ntPickWallpaperFile()">Choose file</button>
            <button class="settings-btn-sm" id="nt-wp-upload-use" onclick="useUploadedWallpaper(true)" style="display:none">Use photo</button>
            <img id="nt-wp-upload-preview" class="nt-wp-upload-preview" onclick="useUploadedWallpaper(true)" alt="">
          </div>
          <!-- Color -->
          <div id="nt-wp-color-row" style="display:none;margin-top:8px;align-items:center;gap:10px">
            <input type="color" id="nt-wp-color-input" value="#141414"
              oninput="saveSetting('new_tab_wallpaper_color',this.value);initWallpaper(window.__ntState||{})"
              style="width:40px;height:32px;border:none;background:none;cursor:pointer;padding:0;border-radius:6px">
            <span id="nt-wp-color-label" style="font-size:12px;color:var(--text-muted)">#141414</span>
          </div>
        </div>

        <!-- Toggles -->
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Search box</div>
            <div class="toggle-desc">Show the main search field in the center of the page</div>
          </div>
          <div class="toggle-switch" id="toggle-new-tab-show-search" onclick="toggleSetting('new_tab_show_search')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Quick links</div>
            <div class="toggle-desc">Show shortcut tiles for frequent sites and search engines</div>
          </div>
          <div class="toggle-switch" id="toggle-new-tab-show-quick-links" onclick="toggleSetting('new_tab_show_quick_links')"></div>
        </div>
      </div>

      <div class="settings-section" id="section-tabs">
        <h2>Tabs</h2>
        <p class="subtitle">How tabs behave</p>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Restore closed tabs</div>
            <div class="toggle-desc">Keep a history of recently closed tabs</div>
          </div>
          <div class="toggle-switch on" id="toggle-restore-tabs" onclick="toggleSetting('restore_tabs')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Compact tab style</div>
            <div class="toggle-desc">Show smaller tabs in the sidebar</div>
          </div>
          <div class="toggle-switch" id="toggle-compact-tabs" onclick="toggleSetting('compact_tabs')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Auto-pin essential tabs</div>
            <div class="toggle-desc">Automatically pin tabs marked as essential</div>
          </div>
          <div class="toggle-switch" id="toggle-auto-pin" onclick="toggleSetting('auto_pin')"></div>
        </div>
      </div>

      <div class="settings-section" id="section-bookmarks">
        <h2>Bookmarks</h2>
        <p class="subtitle">Your saved pages</p>
        <div id="bookmarks-list" style="display:flex;flex-direction:column;gap:4px">
          <div style="color:var(--text-muted);font-size:12px;text-align:center;padding:24px 0">No bookmarks yet. Hit the bookmark icon in the address bar to save a page.</div>
        </div>
      </div>

      <div class="settings-section" id="section-history">
        <h2>History</h2>
        <p class="subtitle">Pages you have visited</p>
        <div style="display:flex;gap:8px;align-items:center;margin-bottom:12px">
          <input class="settings-input" id="history-search" placeholder="Search history..." oninput="searchHistory(this.value)" style="flex:1">
          <button class="ob-btn-primary" style="background:var(--danger);flex-shrink:0" onclick="clearHistory()">Clear all</button>
        </div>
        <div id="history-list" style="display:flex;flex-direction:column;gap:2px">
          <div style="color:var(--text-muted);font-size:12px;text-align:center;padding:24px 0">No history yet.</div>
        </div>
      </div>

      <div class="settings-section" id="section-downloads">
        <h2>Downloads</h2>
        <p class="subtitle">Files you have downloaded</p>
        <div id="downloads-list" style="display:flex;flex-direction:column;gap:6px">
          <div style="color:var(--text-muted);font-size:12px;text-align:center;padding:24px 0">Nothing downloaded yet.</div>
        </div>
      </div>

      <div class="settings-section" id="section-ai">
        <h2>AI Providers</h2>
        <p class="subtitle">Connect your AI accounts to use the sidebar assistant</p>
        <div class="settings-group">
          <label>Anthropic (Claude)</label>
          <input class="settings-input" id="set-anthropic-key" type="password" placeholder="sk-ant-...">
          <div class="hint">Get a key at console.anthropic.com</div>
        </div>
        <div class="settings-group">
          <label>OpenAI</label>
          <input class="settings-input" id="set-openai-key" type="password" placeholder="sk-...">
          <div class="hint">Get a key at platform.openai.com</div>
        </div>
        <div class="settings-group">
          <label>Gemini</label>
          <input class="settings-input" id="set-gemini-key" type="password" placeholder="AIza...">
          <div class="hint">Get a key at aistudio.google.com</div>
        </div>
        <div class="settings-group">
          <label>OpenRouter</label>
          <input class="settings-input" id="set-openrouter-key" type="password" placeholder="sk-or-...">
          <div class="hint">Access hundreds of models at openrouter.ai</div>
        </div>
        <div class="settings-group">
          <label>Ollama (local)</label>
          <input class="settings-input" id="set-ollama-url" placeholder="http://localhost:11434">
          <div class="hint">Run open-source models locally with Ollama</div>
        </div>
        <button class="ob-btn-primary" onclick="saveAiSettings()">Save API Keys</button>
      </div>

      <div class="settings-section" id="section-privacy">
        <h2>Privacy</h2>
        <p class="subtitle">Control what data gets collected and stored</p>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Ad &amp; Tracker Blocker</div>
            <div class="toggle-desc">Block ads, trackers, and malicious scripts.</div>
          </div>
          <div class="toggle-switch on" id="toggle-ad-blocker-enabled" onclick="toggleSetting('ad_blocker_enabled')"></div>
        </div>
        <div id="adblock-exceptions-section" style="margin:0 0 12px;padding:10px 12px;background:var(--bg);border-radius:var(--radius-sm);border:1px solid var(--border-subtle)">
          <div style="font-size:11px;font-weight:600;color:var(--text-dim);letter-spacing:.04em;margin-bottom:6px">SITE EXCEPTIONS</div>
          <div id="adblock-exceptions-list" style="display:flex;flex-direction:column;gap:4px">
            <div style="font-size:12px;color:var(--text-muted);font-style:italic">No exceptions, blocker is active on all sites</div>
          </div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Secure DNS</div>
            <div class="toggle-desc">Use Cloudflare DNS-over-HTTPS for browser DNS lookups</div>
          </div>
          <div class="toggle-switch" id="toggle-secure-dns-enabled" onclick="toggleSetting('secure_dns_enabled')"></div>
        </div>
        <div id="secure-dns-options" style="margin:0 0 12px;padding:10px 12px;background:var(--bg);border-radius:var(--radius-sm);border:1px solid var(--border-subtle)">
          <div class="settings-group" style="margin-bottom:12px">
            <label for="set-secure-dns-provider">Provider</label>
            <select class="settings-select" id="set-secure-dns-provider" onchange="saveSetting('secure_dns_provider',this.value)">
              <option value="cloudflare">Cloudflare 1.1.1.1</option>
              <option value="cloudflare_malware">Cloudflare malware blocking</option>
              <option value="cloudflare_family">Cloudflare family filtering</option>
              <option value="custom">Custom HTTPS endpoint</option>
            </select>
          </div>
          <div class="settings-group" style="margin-bottom:12px">
            <label for="set-secure-dns-mode">Mode</label>
            <select class="settings-select" id="set-secure-dns-mode" onchange="saveSetting('secure_dns_mode',this.value)">
              <option value="secure">Strict, no local DNS fallback</option>
              <option value="automatic">Automatic fallback if DoH fails</option>
            </select>
          </div>
          <div class="settings-group" id="secure-dns-custom-row" style="margin-bottom:12px">
            <label for="set-secure-dns-template">Custom endpoint</label>
            <input class="settings-input" id="set-secure-dns-template" type="url" inputmode="url" placeholder="https://1.1.1.1/dns-query" onblur="saveSetting('secure_dns_template',this.value)" onkeydown="if(event.key==='Enter'){saveSetting('secure_dns_template',this.value);this.blur();}">
          </div>
          <div style="font-size:11px;color:var(--text-muted);line-height:1.5">Ventus restarts after DNS changes so the browser process uses it.</div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Save browsing history</div>
            <div class="toggle-desc">Track pages you visit locally on this device</div>
          </div>
          <div class="toggle-switch on" id="toggle-history" onclick="toggleSetting('save_history')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Send crash reports</div>
            <div class="toggle-desc">Help improve Ventus by sharing anonymous crash data</div>
          </div>
          <div class="toggle-switch" id="toggle-crash" onclick="toggleSetting('crash_reports')"></div>
        </div>
        <div class="settings-toggle">
          <div class="settings-toggle-info">
            <div class="toggle-title">Block third-party cookies</div>
            <div class="toggle-desc">Limit tracking from websites you did not visit directly</div>
          </div>
          <div class="toggle-switch on" id="toggle-cookies" onclick="toggleSetting('block_third_party')"></div>
        </div>
      </div>

      <div class="settings-section" id="section-keyboard">
        <h2>Keyboard Shortcuts</h2>
        <p class="subtitle">Speed up your workflow</p>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:4px">
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">New tab</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+T</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Close tab</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+W</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Focus address bar</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+L</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Search tabs</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+K</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">AI sidebar</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+Shift+A</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Settings</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+,</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Reload</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">F5 / Ctrl+R</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Toggle sidebar</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+B</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Fullscreen</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">F11 / Esc</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">History</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+H</div>
          </div>
          <div style="padding:8px;border-radius:var(--radius-sm);background:var(--bg)">
            <div style="font-size:11px;color:var(--text-muted)">Downloads</div>
            <div style="font-size:12px;font-weight:600;color:var(--text)">Ctrl+J</div>
          </div>
        </div>
      </div>

      <div class="settings-section" id="section-about">
        <h2>About Ventus</h2>
        <p class="subtitle">A focused desktop browser with AI built in.</p>

        <!-- App identity -->
        <div class="about-identity-card">
          <div class="about-identity-logo"><img src="__LOGO_URL__" alt=""></div>
          <div class="about-identity-body">
            <div class="about-identity-name">
              Ventus
              <span class="about-identity-ver">v__APP_VERSION__</span>
            </div>
            <div class="about-identity-tagline">Fast search · Focused workspaces · AI-native</div>
            <div class="about-identity-badges">
              <span class="aib aib-live"><span class="aib-live-dot"></span>Stable</span>
              <span class="aib">Windows</span>
              <span class="aib">AI-native</span>
              <span class="aib">Local-first</span>
            </div>
          </div>
        </div>

        <!-- Spec rows -->
        <div class="about-rows">
          <div class="about-row">
            <span class="about-row-label">Version</span>
            <span class="about-row-val">__APP_VERSION__</span>
          </div>
          <div class="about-row">
            <span class="about-row-label">Platform</span>
            <span class="about-row-val">Windows (x64)</span>
          </div>
          <div class="about-row">
            <span class="about-row-label">Data storage</span>
            <span class="about-row-val">Local · %APPDATA%\ventus</span>
          </div>
          <div class="about-row">
            <span class="about-row-label">Source</span>
            <a class="about-row-link" onclick="send('Navigate',{url:'https://github.com/neura-spheres/Ventus'})">github.com/neura-spheres/Ventus</a>
          </div>
        </div>

        <!-- Software updates -->
        <div class="about-update-card">
          <div class="about-update-left">
            <div class="about-update-icon">
              <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v6m0 0 3-3m-3 3L9 5"/><path d="M3 12a9 9 0 1 0 18 0 9 9 0 0 0-18 0"/></svg>
            </div>
            <div>
              <div class="about-update-title">Software updates</div>
              <div class="about-update-sub">Check for the latest Ventus build</div>
            </div>
          </div>
          <button class="ob-btn-primary" id="btn-check-update" onclick="checkForUpdate()" style="padding:7px 14px;font-size:12px;flex-shrink:0">Check for updates</button>
        </div>
        <div id="update-status-area"></div>

        <!-- Actions -->
        <div class="about-actions">
          <button class="about-act-btn" onclick="send('ExportSettings')">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
            Export settings
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<!-- ONBOARDING -->
<div id="onboarding-overlay">
  <div id="onboarding-modal">
    <div class="ob-bar-track"><div class="ob-bar-fill" id="ob-bar-fill" style="width:0%"></div></div>
    <div id="ob-step-counter" class="ob-step-counter"></div>
    <div class="ob-inner">

      <div class="ob-step active" id="ob-step-0">
        <div class="ob-mark ob-mark-accent">
          <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/><path d="M11 8v6m-3-3h6"/></svg>
        </div>
        <h2 class="ob-title">Welcome to Ventus</h2>
        <p class="ob-sub">A fast, AI-powered browser built for people who like getting things done. Takes about 2 minutes to set up.</p>
        <div class="ob-actions">
          <button class="ob-btn-secondary" onclick="finishOnboarding()">Skip</button>
          <button class="ob-btn-primary" onclick="obNext()">Get started <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg></button>
        </div>
      </div>

      <div class="ob-step" id="ob-step-1">
        <h2 class="ob-title">Look &amp; Feel</h2>
        <p class="ob-sub">Set your theme and sidebar style. Everything can be changed later.</p>
        <div class="ob-section-lbl">Theme</div>
        <div class="ob-theme-row">
          <div class="theme-card selected" id="ob-theme-dark" onclick="obSelectTheme('dark')">
            <div class="theme-preview" style="background:linear-gradient(135deg,#0f0f10,#1a1a1c)"></div>
            <div class="theme-name">Dark</div>
          </div>
          <div class="theme-card" id="ob-theme-light" onclick="obSelectTheme('light')">
            <div class="theme-preview" style="background:linear-gradient(135deg,#f5f5f7,#ffffff)"></div>
            <div class="theme-name">Light</div>
          </div>
          <div class="theme-card" id="ob-theme-system" onclick="obSelectTheme('system')">
            <div class="theme-preview" style="background:linear-gradient(135deg,#0f0f10 50%,#f5f5f7 50%)"></div>
            <div class="theme-name">System</div>
          </div>
        </div>
        <div class="ob-section-lbl">Sidebar</div>
        <div class="ob-sidebar-chips">
          <button class="ob-sidebar-chip selected" id="ob-sb-auto_hide" onclick="obSelectSidebar('auto_hide')">Auto-hide</button>
          <button class="ob-sidebar-chip" id="ob-sb-expanded" onclick="obSelectSidebar('expanded')">Always visible</button>
          <button class="ob-sidebar-chip" id="ob-sb-compact" onclick="obSelectSidebar('compact')">Compact</button>
        </div>
        <div class="ob-actions">
          <button class="ob-btn-secondary" onclick="obPrev()">Back</button>
          <button class="ob-btn-primary" onclick="obNext()">Next <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg></button>
        </div>
      </div>

      <div class="ob-step" id="ob-step-2">
        <h2 class="ob-title">Your Region</h2>
        <p class="ob-sub">Adjusts unit and currency suggestions in Spotlight to your local standards.</p>
        <div id="ob-detect-banner" class="ob-detect-banner" style="display:none">
          <div class="ob-detect-flag" id="ob-detect-flag"></div>
          <div class="ob-detect-body">
            <div class="ob-detect-label">Auto-detected</div>
            <div class="ob-detect-name" id="ob-detect-name"></div>
          </div>
          <button class="ob-btn-secondary" style="margin:0;padding:4px 10px;font-size:11px" onclick="obClearDetect()">Change</button>
        </div>
        <select id="ob-region-select" class="settings-select" style="width:100%;margin-top:6px" onchange="obSelectRegion(this.value)">
          <option value="">Not set</option>
        </select>
        <div class="ob-actions">
          <button class="ob-btn-secondary" onclick="obPrev()">Back</button>
          <button class="ob-btn-primary" onclick="obNext()">Next <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg></button>
        </div>
      </div>

      <div class="ob-step" id="ob-step-3">
        <h2 class="ob-title">Search engine</h2>
        <p class="ob-sub">Your default. You can still switch mid-search using shortcuts like @ddg or @b.</p>
        <div class="ob-engine-grid">
          <button class="ob-engine-btn selected" id="ob-engine-google" onclick="obSelectEngine('google',this)">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
            Google
          </button>
          <button class="ob-engine-btn" id="ob-engine-duckduckgo" onclick="obSelectEngine('duckduckgo',this)">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a10 10 0 0 1 0 20"/></svg>
            DuckDuckGo
          </button>
          <button class="ob-engine-btn" id="ob-engine-brave" onclick="obSelectEngine('brave',this)">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2L2 7l10 5 10-5-10-5z"/><path d="M2 17l10 5 10-5"/><path d="M2 12l10 5 10-5"/></svg>
            Brave Search
          </button>
          <button class="ob-engine-btn" id="ob-engine-perplexity" onclick="obSelectEngine('perplexity',this)">
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2a10 10 0 1 0 10 10"/><path d="M12 8v4l3 3"/><circle cx="19" cy="5" r="3"/></svg>
            Perplexity
          </button>
        </div>
        <div class="ob-actions">
          <button class="ob-btn-secondary" onclick="obPrev()">Back</button>
          <button class="ob-btn-primary" onclick="obNext()">Next <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg></button>
        </div>
      </div>

      <div class="ob-step" id="ob-step-4">
        <h2 class="ob-title">Connect an AI</h2>
        <p class="ob-sub">Paste an API key to unlock the AI sidebar. Optional, you can do this later in Settings.</p>
        <div class="ob-api-row">
          <label>Anthropic (Claude)</label>
          <input class="ob-api-input" id="ob-anthropic-key" type="password" placeholder="sk-ant-...">
        </div>
        <div class="ob-api-row">
          <label>OpenAI</label>
          <input class="ob-api-input" id="ob-openai-key" type="password" placeholder="sk-...">
        </div>
        <div class="ob-api-row">
          <label>Gemini</label>
          <input class="ob-api-input" id="ob-gemini-key" type="password" placeholder="AIza...">
        </div>
        <div class="ob-api-row">
          <label>OpenRouter</label>
          <input class="ob-api-input" id="ob-openrouter-key" type="password" placeholder="sk-or-...">
        </div>
        <div class="ob-actions">
          <button class="ob-btn-secondary" onclick="obPrev()">Back</button>
          <button class="ob-btn-primary" onclick="obSaveAndFinish()">Finish setup <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg></button>
        </div>
      </div>

      <div class="ob-step" id="ob-step-5">
        <div class="ob-done-ring">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
        </div>
        <h2 class="ob-title" style="text-align:center">You're all set</h2>
        <p class="ob-sub" style="text-align:center">Ventus is ready. Use Ctrl+T for new tabs, Ctrl+Shift+A for AI, and Ctrl+K to search your tabs.</p>
        <div class="ob-actions" style="justify-content:center;border-top:none;padding-top:0">
          <button class="ob-btn-primary" onclick="finishOnboarding()">Start browsing <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg></button>
        </div>
      </div>

    </div>
  </div>
</div>

<div id="update-modal">
  <div class="update-modal-panel" id="update-modal-panel" role="dialog" aria-modal="true" aria-labelledby="update-modal-title">
    <div class="update-modal-head">
      <div class="update-modal-icon" id="update-modal-icon">
        <svg id="update-modal-icon-svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
      </div>
      <div class="update-modal-copy">
        <div class="update-modal-title" id="update-modal-title">Checking for updates</div>
        <div class="update-modal-sub" id="update-modal-sub">This should only take a moment.</div>
      </div>
      <button class="update-modal-close" onclick="closeUpdateModal(false)" title="Close">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>
    </div>
    <div class="update-modal-body">
      <div class="update-modal-notes" id="update-modal-notes"></div>
      <div class="update-modal-progress" id="update-modal-progress">
        <div class="update-modal-track"><div class="update-modal-bar" id="update-modal-bar"></div></div>
        <div class="update-modal-progress-label" id="update-modal-progress-label"></div>
      </div>
    </div>
    <div class="update-modal-actions" id="update-modal-actions"></div>
  </div>
</div>

<!-- UPDATE TOAST -->
<div id="update-toast">
  <div class="ut-icon">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v6m0 0 3-3m-3 3-3-3"/><path d="M3 12a9 9 0 1 0 18 0 9 9 0 0 0-18 0"/><path d="M12 12v6"/></svg>
  </div>
  <div class="ut-body">
    <div class="ut-title">Update available</div>
    <div class="ut-version" id="ut-version-text"></div>
  </div>
  <div class="ut-buttons">
    <button class="ut-btn-later" onclick="dismissUpdateToast()">Later</button>
    <button class="ut-btn-update" onclick="installUpdate();dismissUpdateToast()">Update</button>
  </div>
</div>

<!-- Window edge resize handles — transparent hit zones, trigger native Win32 resize.
     LEFT edge is intentionally omitted: it conflicts with #sidebar-float-trigger.
     Left/bottom/right are handled by the content WebView initialization script instead.
     Chrome handles only the right edge (when AI panel is open) and top corners
     (which are always inside the toolbar clip region). -->
<div class="resize-handle" data-edge="right"       onmousedown="send('BeginResize',{edge:'right'})"></div>
<div class="resize-handle" data-edge="bottom"      onmousedown="send('BeginResize',{edge:'bottom'})"></div>
<div class="resize-handle" data-edge="topleft"     onmousedown="send('BeginResize',{edge:'topleft'})"></div>
<div class="resize-handle" data-edge="topright"    onmousedown="send('BeginResize',{edge:'topright'})"></div>
<div class="resize-handle" data-edge="bottomleft"  onmousedown="send('BeginResize',{edge:'bottomleft'})"></div>
<div class="resize-handle" data-edge="bottomright" onmousedown="send('BeginResize',{edge:'bottomright'})"></div>

<!-- TAB SPOTLIGHT -->
<div id="tab-spotlight-overlay" onclick="if(event.target===this)closeSpotlight()">
  <div id="tab-spotlight">
    <div id="tsp-input-wrap">
      <svg class="tsp-ico" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
      <input id="tsp-input" type="text" placeholder="Search or go to a website..." autocomplete="off" spellcheck="false" oninput="renderTspSuggestions(this.value)" onkeydown="tspKeydown(event)">
      <div id="tsp-ai-hint">Tab → AI</div>
    </div>
    <div id="tsp-results"></div>
    <div id="tsp-ai-panel">
      <div class="tsp-ai-header">
        <button class="tsp-ai-back" onclick="tspExitAiMode()" title="Back to search">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5"/><path d="m12 19-7-7 7-7"/></svg>
        </button>
        <span class="tsp-ai-title">AI Answer</span>
      </div>
      <div class="tsp-ai-content" id="tsp-ai-content"></div>
    </div>
  </div>
</div>

<!-- TAB SEARCH MODAL -->
<div id="tab-search-modal">
  <input id="tab-search-input" placeholder="Search tabs..." oninput="filterTabs(this.value)" onkeydown="handleTabSearchKey(event)">
  <div id="tab-search-results"></div>
</div>

<!-- CONTEXT MENU -->
<div id="ctx-menu" id="ctx-menu"></div>

<!-- TOAST CONTAINER -->
<div id="toast-container"></div>

<script>
// ============================================================
// STATE
// ============================================================
let state = {
  tabs: [],
  workspaces: [],
  workspace_tab_counts: {},
  active_tab_id: null,
  active_workspace_id: null,
  sidebar_collapsed: false,
  ai_open: false,
  bookmarks: [],
  history: [],
  downloads: [],
  search_engines: [],
  settings: {},
  ai_key_status: {},
  ai_provider: 'anthropic',
  ai_model: '',
  is_bookmarked: false,
};
let obStep = 0;
const OB_STEPS = 6;
let obTheme = 'dark';
let obEngine = 'google';
let obSidebarMode = 'auto_hide';
let obRegion = '';
let obDirection = 1;
let aiStreaming = false;
let currentStreamEl = null;
let loadProgress = 0;
let loadProgressTimer = null;
let activeSuggestionTarget = null;
let activeSuggestionIndex = -1;
let editingWorkspaceId = null;
let deletingWorkspaceId = null;
let selectedWsEmoji = '📁';
let selectedWsColor = '#8b5cf6';
let wsColorManual = false;
let activeSuggestions = [];
let suggestionHideTimer = null;
let neuraFeed = [];
let neuraFeedLoading = false;
let neuraFeedLoaded = false;
let neuraFeedError = '';
let newtabBgSeed = '';
let newtabBgCss = '';
let newtabWallpaperData = '';
let settingDrafts = {};
const trendingSearches = ['AI news', 'technology', 'Indonesia news', 'startup funding', 'web design'];

// ============================================================
// IPC  (auto-converts PascalCase cmd to snake_case for Rust)
// ============================================================
function send(cmd, data={}) {
  const sc = cmd.replace(/([A-Z])/g, (m, c, i) => (i > 0 ? '_' : '') + c.toLowerCase());
  window.ipc.postMessage(JSON.stringify({cmd: sc, ...data}));
}

function nav(action) { send(action); }

// ============================================================
// RUST -> JS INTERFACE
// ============================================================
let _liveRates = null;   // {USD:1, EUR:0.92, IDR:15700, ...} keyed UPPERCASE
let _liveRatesTs = 0;    // Date.now() when last fetched
const _RATES_TTL = 3600_000; // 1 hour

window.__neura = {
  setState(s) {
    state = {...state, ...s};
    render();
  },
  setNewtabWallpaperData(d) {
    newtabWallpaperData = d || '';
    if (newtabSettings().wallpaper_source === 'upload') applyNewtabSettings();
    syncNewtabSettingsUI();
  },
  setLayout(sidebarW, toolbarH, aiW, frameSideW, frameBottomH) {
    const root = document.documentElement;
    const mainH = Math.min(toolbarH, 44);
    root.style.setProperty('--sidebar-w', sidebarW + 'px');
    root.style.setProperty('--toolbar-h', mainH + 'px');
    root.style.setProperty('--top-chrome-h', toolbarH + 'px');
    root.style.setProperty('--ai-w', aiW + 'px');
    if (frameSideW != null) root.style.setProperty('--frame-side-w', frameSideW + 'px');
    if (frameBottomH != null) root.style.setProperty('--frame-bottom-h', frameBottomH + 'px');
  },
  appendAiChunk(text, done) {
    if (done && !text && !currentStreamEl) {
      finishAiBusy();
      return;
    }
    if (!currentStreamEl) {
      const msgs = document.getElementById('ai-messages');
      currentStreamEl = document.createElement('div');
      currentStreamEl.className = 'ai-msg assistant';
      currentStreamEl._rawText = '';
      // remove thinking dots if present
      const thinking = msgs.querySelector('.ai-thinking');
      if (thinking) thinking.remove();
      msgs.appendChild(currentStreamEl);
    }
    currentStreamEl._rawText += text;
    currentStreamEl.innerHTML = renderAiMarkdown(currentStreamEl._rawText);
    scrollAiToBottom();
    if (done) {
      currentStreamEl = null;
      finishAiBusy();
    }
  },
  appendAiToolCall(label) {
    const msgs = document.getElementById('ai-messages');
    // Remove old thinking dots so tool call appears in their place
    const thinking = msgs.querySelector('.ai-thinking');
    if (thinking) thinking.remove();
    const el = document.createElement('div');
    el.className = 'ai-tool-call';
    el.innerHTML = '<span class="ai-tool-icon">⚙</span><span class="ai-tool-label"></span>';
    el.querySelector('.ai-tool-label').textContent = label;
    msgs.appendChild(el);
    // Re-add thinking dots after the tool call bubble so user knows AI is still working
    showAiThinking();
    scrollAiToBottom();
  },
  showError(msg) { toast(msg, 'error'); finishAiBusy(); },
  showSuccess(msg) { toast(msg, 'success'); },
  setBookmarked(v) {
    state.is_bookmarked = v;
    renderBookmarkIcon();
  },
  setNeuraFeed(articles) {
    neuraFeed = Array.isArray(articles) ? articles : [];
    neuraFeedLoading = false;
    neuraFeedLoaded = true;
    neuraFeedError = '';
    renderNeuraFeed();
  },
  setNeuraFeedError(message) {
    neuraFeedLoading = false;
    neuraFeedLoaded = true;
    neuraFeedError = message || 'Neura Feed is not loading right now.';
    renderNeuraFeed();
  },
  setCurrencyRates(rates) {
    _liveRates = rates;
    _liveRatesTs = Date.now();
    const tspInput = document.getElementById('tsp-input');
    if (tspInput && tspInput.value.trim()) renderTspSuggestions(tspInput.value);
  },
  showOnboarding() { openOnboarding(); },
  updateNavState(canBack, canForward, loading) {
    document.getElementById('btn-back').disabled = !canBack;
    document.getElementById('btn-forward').disabled = !canForward;
    const reloadBtn = document.getElementById('btn-reload');
    if (loading) {
      if (!loadProgressTimer && loadProgress <= 0) startLoadProgress();
      reloadBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>';
      reloadBtn.onclick = () => nav('Stop');
      reloadBtn.title = 'Stop loading';
    } else {
      if (loadProgressTimer || loadProgress > 0 && loadProgress < 1) finishLoadProgress();
      reloadBtn.innerHTML = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>';
      reloadBtn.onclick = () => nav('Reload');
      reloadBtn.title = 'Reload (F5)';
    }
  },
  startLoadProgress() { startLoadProgress(); },
  setLoadProgress(progress) { setLoadProgress(progress); },
  finishLoadProgress() { finishLoadProgress(); },
  setUrl(url, title) {
    const input = document.getElementById('url-input');
    if (document.activeElement !== input) {
      input.value = formatDisplayUrl(url);
    }
    updateLockIcon(url);
    document.title = title || 'Ventus';
    checkNewtabPlaceholder(url);
  },
  setDownloadActive(active) {
    const btn = document.getElementById('btn-more');
    if (btn) btn.classList.toggle('has-downloads', !!active);
  },
  setHistory(items) {
    state.history = items || [];
    renderHistory();
    renderSuggestionPanels();
  },
  clearTransientUi() {
    hideSuggestions();
    const updateModal = document.getElementById('update-modal');
    if (updateModal && updateModal.classList.contains('open')) closeUpdateModal(false);
    ['settings-overlay','tab-search-modal','workspace-modal','workspace-delete-modal','context-menu','adblock-modal','adblock-backdrop','download-panel','model-modal','tab-spotlight-overlay','update-modal'].forEach(id => {
      const el = document.getElementById(id);
      if (el) el.classList.remove('open');
    });
    const ctx = document.getElementById('ctx-menu');
    if (ctx) ctx.style.display = 'none';
    const toast = document.getElementById('update-toast');
    if (toast) toast.classList.remove('visible','hiding');
    document.removeEventListener('click', _adblockOutside, true);
    if (spotlightOpen) {
      spotlightOpen = false;
      tspExitAiMode();
    }
  },
  closeSidebar() {
    // Called by Rust when content WebView detects cursor in the content area.
    // Respects sidebarPinned — only closes hover-triggered (unpinned) sidebar.
    scheduledHide();
  },
  openSidebar() {
    showFloatingSidebar(false);
  },
  setContentFullscreen(active) {
    const app = document.getElementById('app');
    if (active) {
      app.classList.add('content-fullscreen');
    } else {
      app.classList.remove('content-fullscreen');
    }
  },
  setUpdateState({ status, version, notes, error, received, total }) {
    const area = document.getElementById('update-status-area');
    const btn = document.getElementById('btn-check-update');
    if (status === 'checking') {
      btn && (btn.disabled = true);
      if (area) area.innerHTML = '<div style="display:flex;align-items:center;gap:8px;color:var(--text-muted);font-size:12px"><svg style="animation:spin 1s linear infinite" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>Checking for updates...</div>';
      if (__manualUpdateCheck || isUpdateModalOpen()) showUpdateModal({status});
    } else if (status === 'up_to_date') {
      btn && (btn.disabled = false);
      if (area) area.innerHTML = '<div style="display:flex;align-items:center;gap:8px;color:var(--success);font-size:12px"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>You\'re up to date.</div>';
      if (__manualUpdateCheck || isUpdateModalOpen()) showUpdateModal({status});
      __manualUpdateCheck = false;
    } else if (status === 'available') {
      btn && (btn.disabled = false);
      if (area) area.innerHTML = `<div style="display:flex;flex-direction:column;gap:10px">
        <div style="display:flex;align-items:center;gap:8px">
          <span style="padding:2px 8px;border-radius:10px;background:var(--accent-dim);color:var(--accent);font-size:11px;font-weight:600">v${escHtml(version)} available</span>
        </div>
        ${notes ? `<div style="font-size:11px;color:var(--text-muted);max-height:80px;overflow-y:auto;white-space:pre-wrap;line-height:1.5">${escHtml(notes.slice(0,400))}${notes.length>400?'...':''}</div>` : ''}
        <button class="ob-btn-primary" onclick="installUpdate()" style="align-self:flex-start;padding:6px 16px;font-size:12px">Download &amp; install</button>
      </div>`;
      if (__manualUpdateCheck || isUpdateModalOpen()) {
        showUpdateModal({status, version, notes});
      } else {
        showUpdateToast(version, notes);
      }
      __manualUpdateCheck = false;
    } else if (status === 'downloading') {
      btn && (btn.disabled = true);
      const pct = total > 0 ? Math.round((received / total) * 100) : null;
      const label = pct !== null ? `Downloading... ${pct}%` : 'Downloading...';
      const bar = pct !== null
        ? `<div style="margin-top:8px;height:4px;background:var(--border);border-radius:2px;overflow:hidden"><div style="height:100%;background:var(--accent);width:${pct}%;transition:width 0.3s ease"></div></div>`
        : '';
      if (area) area.innerHTML = `<div style="font-size:12px;color:var(--text-muted)">${label}${bar}</div>`;
      if (isUpdateModalOpen()) showUpdateModal({status, received, total});
    } else if (status === 'installing') {
      btn && (btn.disabled = true);
      if (area) area.innerHTML = '<div style="font-size:12px;color:var(--text-muted)">Installing update and restarting...</div>';
      if (isUpdateModalOpen()) showUpdateModal({status});
    } else if (status === 'error') {
      btn && (btn.disabled = false);
      if (area) area.innerHTML = `<div style="font-size:12px;color:var(--danger)">${escHtml(error || 'Update check failed.')}</div>`;
      if (__manualUpdateCheck || isUpdateModalOpen()) showUpdateModal({status, error});
      __manualUpdateCheck = false;
    }
  },
  showContextMenu(data) { showBrowserContextMenu(data); },
  spotlightAiChunk(text, done) {
    if (!tspAiMode) return;
    const content = document.getElementById('tsp-ai-content');
    if (!content) return;
    if (done) {
      tspAiStreaming = false;
      // Remove thinking dots if still present
      const dots = content.querySelector('.tsp-ai-dots');
      if (dots) dots.remove();
      return;
    }
    // Remove dots on first actual content
    const dots = content.querySelector('.tsp-ai-dots');
    if (dots) dots.remove();
    _tspAiRawText += text;
    content.innerHTML = renderAiMarkdown(_tspAiRawText);
    content.scrollTop = content.scrollHeight;
  },
  spotlightAiError(msg) {
    if (!tspAiMode) return;
    tspAiStreaming = false;
    const content = document.getElementById('tsp-ai-content');
    if (content) {
      content.innerHTML = '<span style="color:var(--danger)">' + escHtml(msg) + '</span>';
    }
  },
};

// ============================================================
// RENDER
// ============================================================
function render() {
  renderWorkspaces();
  renderTabs();
  renderAddressBar();
  renderIncognitoBadge();
  renderSearchSettings();
  renderBookmarks();
  renderDownloads();
  applyNewtabSettings();
  if (document.getElementById('download-panel') && document.getElementById('download-panel').classList.contains('open')) {
    renderDownloadPanel();
  }
  renderNewtabShortcuts();
  if (document.getElementById('newtab-feed-main')) renderNeuraFeed();
  applyAiSidebar();
  if (state.ai_open) renderAiSidebar();
  applyTheme();
  applySidebarMode();
  if (document.getElementById('settings-overlay').classList.contains('open')) {
    populateSettingsPanel();
  }
  window.__neura.updateNavState(!!state.can_go_back, !!state.can_go_fwd, !!state.is_loading);
  renderAdBlockBtn();
  _syncAdBlockModal();
  // renderBookmarksBar is called inside applySidebarMode after class toggle
}

function renderAdBlockBtn() {
  const btn = document.getElementById('btn-adblock');
  const iconOn = document.getElementById('adblock-icon');
  const iconOff = document.getElementById('adblock-icon-off');
  if (!btn || !iconOn || !iconOff) return;
  const active = !!state.ad_blocker_active;
  const excepted = !!state.ad_blocker_site_excepted;
  if (!active) {
    btn.style.color = 'var(--text-dim)';
    btn.title = 'Ad blocker - click for info';
    iconOn.style.display = 'none';
    iconOff.style.display = '';
  } else if (excepted) {
    btn.style.color = '#f59e0b';
    btn.title = 'Ad blocker paused for this site - click for info';
    iconOn.style.display = '';
    iconOff.style.display = 'none';
  } else {
    btn.style.color = '#22c55e';
    btn.title = 'Ad blocker active - click for info';
    iconOn.style.display = '';
    iconOff.style.display = 'none';
  }
  // Keep modal in sync if it's open
  _syncAdBlockModal();
}

function openAdBlockModal() {
  const modal = document.getElementById('adblock-modal');
  const backdrop = document.getElementById('adblock-backdrop');
  if (!modal) return;
  if (modal.classList.contains('open')) { closeAdBlockModal(); return; }
  if (backdrop) backdrop.classList.add('open');
  modal.classList.add('open');
  _syncAdBlockModal();
  // Position below the address bar, left-aligned to the adblock button
  const btn = document.getElementById('btn-adblock');
  if (btn) {
    const r = btn.getBoundingClientRect();
    const modalW = 284;
    let left = r.left;
    if (left + modalW > window.innerWidth - 8) left = window.innerWidth - modalW - 8;
    if (left < 8) left = 8;
    modal.style.left = left + 'px';
    modal.style.top = (r.bottom + 6) + 'px';
    modal.style.right = 'auto';
  }
  requestAnimationFrame(() => {
    send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
  });
  setTimeout(() => document.addEventListener('click', _adblockOutside, {once: true, capture: true}), 0);
}

function closeAdBlockModal() {
  const modal = document.getElementById('adblock-modal');
  const backdrop = document.getElementById('adblock-backdrop');
  if (backdrop) backdrop.classList.remove('open');
  if (modal) modal.classList.remove('open');
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
  document.removeEventListener('click', _adblockOutside, true);
}

function _adblockOutside(e) {
  const modal = document.getElementById('adblock-modal');
  const btn = document.getElementById('btn-adblock');
  if (modal && !modal.contains(e.target) && e.target !== btn && !btn.contains(e.target)) {
    closeAdBlockModal();
  }
}

function _syncAdBlockModal() {
  const modal = document.getElementById('adblock-modal');
  if (!modal || !modal.classList.contains('open')) return;
  const active = !!state.ad_blocker_active;
  const excepted = !!state.ad_blocker_site_excepted;
  const dot = document.getElementById('abm-dot');
  const statusText = document.getElementById('abm-status-text');
  const statusRow = document.getElementById('abm-status');
  const toggle = document.getElementById('abm-site-toggle');
  const toggleText = document.getElementById('abm-site-toggle-text');
  if (!active) {
    if (statusRow) statusRow.className = 'abm-status-row off';
    if (dot) dot.className = 'abm-status-dot off';
    if (statusText) { statusText.className = 'abm-status-label muted'; statusText.textContent = 'Ad blocker is disabled globally'; }
    if (toggle) { toggle.className = 'abm-action-btn off'; toggle.disabled = true; }
    if (toggleText) toggleText.textContent = 'Pause for this site';
  } else if (excepted) {
    if (statusRow) statusRow.className = 'abm-status-row warn';
    if (dot) dot.className = 'abm-status-dot warn';
    if (statusText) { statusText.className = 'abm-status-label warn'; statusText.textContent = 'Paused for this site'; }
    if (toggle) { toggle.className = 'abm-action-btn warn'; toggle.disabled = false; }
    if (toggleText) toggleText.textContent = 'Resume for this site';
  } else {
    if (statusRow) statusRow.className = 'abm-status-row';
    if (dot) dot.className = 'abm-status-dot';
    if (statusText) { statusText.className = 'abm-status-label'; statusText.textContent = 'Active on this page'; }
    if (toggle) { toggle.className = 'abm-action-btn'; toggle.disabled = false; }
    if (toggleText) toggleText.textContent = 'Pause for this site';
  }
}

function renderAdBlockExceptions(exceptions) {
  const list = document.getElementById('adblock-exceptions-list');
  if (!list) return;
  if (!exceptions || exceptions.length === 0) {
    list.innerHTML = '<div style="font-size:12px;color:var(--text-muted);font-style:italic">No exceptions, blocker is active on all sites</div>';
    return;
  }
  list.innerHTML = exceptions.map(host => `
    <div style="display:flex;align-items:center;justify-content:space-between;padding:3px 0">
      <span style="font-size:12px;color:var(--text)">${host}</span>
      <button onclick="removeAdBlockException('${host}')" style="background:none;border:none;color:var(--text-muted);cursor:pointer;padding:2px 4px;font-size:11px;border-radius:3px" title="Remove exception">✕</button>
    </div>`).join('');
}

function removeAdBlockException(host) {
  // Tell Rust to navigate to that site so it can toggle the exception off.
  // Simpler: we directly call send with a synthetic toggle by navigating to the host.
  // The toggle is host-level, so visiting it via ContentNavigate and then toggling works.
  // For settings-panel removal we use a dedicated path: save ad_blocker_exceptions minus this host.
  const priv = (state.settings || {}).privacy || {};
  const exceptions = (priv.ad_blocker_exceptions || []).filter(e => e !== host);
  send('SaveSettings', { key: 'ad_blocker_exceptions', value: exceptions });
}

function renderIncognitoBadge() {
  const badge = document.getElementById('toolbar-incognito-badge');
  if (!badge) return;
  const ws = (state.workspaces || []).find(w => w.id === state.active_workspace_id);
  badge.classList.toggle('visible', !!(ws && ws.is_incognito));
}

function renderWorkspaces() {
  const wsList = state.workspaces || [];
  const activeId = state.active_workspace_id;
  const activeIdx = wsList.findIndex(w => w.id === activeId);
  const multi = wsList.length > 1;
  setSidebarGlow(wsList.find(w => w.id === activeId));

  // Workspace page dots — always shown, with hover popover
  const dotsEl = document.getElementById('sb-ws-dots');
  if (dotsEl) {
    dotsEl.classList.toggle('scrollable', wsList.length > 4);
    dotsEl.innerHTML = wsList.map(ws => {
      const active = ws.id === activeId ? 'active' : '';
      const muted = multi && ws.id !== activeId ? 'muted' : '';
      const icon = escHtml(ws.icon || ws.name.substring(0,2).toUpperCase());
      return `<div class="ws-dot ${active} ${muted}" onclick="switchWorkspace('${ws.id}')" onmouseenter="showWsPop('${ws.id}',this)" onmouseleave="hideWsPop()" title="${escAttr(ws.name)}"><span class="ws-dot-mark"></span><span class="ws-dot-icon">${icon}</span></div>`;
    }).join('');
    requestAnimationFrame(syncWsDots);
  }

  // Prev / next nav triangles
  const prevBtn = document.getElementById('sb-ws-prev');
  const nextBtn = document.getElementById('sb-ws-next');
  if (prevBtn) {
    prevBtn.classList.toggle('visible', multi);
    prevBtn.disabled = activeIdx <= 0;
  }
  if (nextBtn) {
    nextBtn.classList.toggle('visible', multi);
    nextBtn.disabled = activeIdx >= wsList.length - 1;
  }
}

function setSidebarGlow(ws) {
  const color = cleanHex(ws && ws.accent_color) || '#8b5cf6';
  const rgb = hexRgb(color);
  const sidebar = document.getElementById('sidebar');
  if (!sidebar || !rgb) return;
  sidebar.style.setProperty('--ws-glow-rgb', rgb.join(','));
}

function syncWsDots() {
  const el = document.getElementById('sb-ws-dots');
  if (!el) return;
  const overflow = el.scrollWidth > el.clientWidth + 1;
  el.classList.toggle('scrollable', overflow);
  if (!overflow) { el.scrollLeft = 0; return; }
  const active = el.querySelector('.ws-dot.active');
  if (!active) return;
  const elRect = el.getBoundingClientRect();
  const aRect = active.getBoundingClientRect();
  el.scrollLeft += (aRect.left + aRect.width / 2) - (elRect.left + elRect.width / 2);
}

function scrollWsDots(e) {
  const el = e.currentTarget;
  if (!el || el.scrollWidth <= el.clientWidth) return;
  e.preventDefault();
  el.scrollLeft += e.deltaY || e.deltaX;
}

function prevWorkspace() {
  const wsList = state.workspaces || [];
  const idx = wsList.findIndex(w => w.id === state.active_workspace_id);
  if (idx > 0) switchWorkspace(wsList[idx - 1].id);
}

function nextWorkspace() {
  const wsList = state.workspaces || [];
  const idx = wsList.findIndex(w => w.id === state.active_workspace_id);
  if (idx < wsList.length - 1) switchWorkspace(wsList[idx + 1].id);
}

let __wsPopTimer = null;

function showWsPop(wsId, el) {
  clearTimeout(__wsPopTimer);
  const ws = (state.workspaces || []).find(w => w.id === wsId);
  if (!ws) return;
  const pop = document.getElementById('sb-ws-popover');
  if (!pop) return;
  const tabs = (state.tabs || []).filter(t => t.workspace_id === wsId);
  const counts = state.workspace_tab_counts || {};
  const tabCount = Number.isFinite(counts[wsId]) ? counts[wsId] : tabs.length;
  const color = ws.accent_color || '#7c6af7';
  const avatarEl = document.getElementById('sb-ws-pop-avatar');
  avatarEl.textContent = ws.icon || ws.name.substring(0, 2).toUpperCase();
  avatarEl.style.background = ws.icon ? 'var(--bg-hover)' : color;
  avatarEl.style.border = ws.icon ? '1.5px solid var(--border)' : 'none';
  document.getElementById('sb-ws-pop-name').textContent = ws.name;
  document.getElementById('sb-ws-pop-count').textContent = tabCount + ' tab' + (tabCount !== 1 ? 's' : '');
  const note = document.getElementById('sb-ws-pop-note');
  if (note) {
    note.textContent = ws.is_incognito ? 'Incognito workspace. History is not saved here.' : '';
    note.style.display = ws.is_incognito ? 'block' : 'none';
  }
  const delBtn = document.getElementById('sb-ws-pop-delete');
  if (delBtn) delBtn.disabled = (state.workspaces || []).length <= 1;
  pop.dataset.wsId = wsId;
  pop.style.display = 'flex';
  requestAnimationFrame(() => {
    const pr = pop.getBoundingClientRect();
    const er = el.getBoundingClientRect();
    const left = Math.max(4, er.left + er.width / 2 - pr.width / 2);
    pop.style.left = left + 'px';
    pop.style.top = (er.top - pr.height - 8) + 'px';
    pop.classList.add('visible');
  });
}

function hideWsPop() {
  __wsPopTimer = setTimeout(() => {
    const pop = document.getElementById('sb-ws-popover');
    if (pop) { pop.classList.remove('visible'); pop.style.display = 'none'; }
  }, 120);
}

function keepWsPop() {
  clearTimeout(__wsPopTimer);
}

function showWsPopRename() {
  const pop = document.getElementById('sb-ws-popover');
  const wsId = pop && pop.dataset.wsId;
  hideWsPop();
  if (wsId) openWorkspaceModal(wsId);
}

function showWsPopDelete() {
  const pop = document.getElementById('sb-ws-popover');
  const wsId = pop && pop.dataset.wsId;
  hideWsPop();
  if (wsId) deleteWorkspace(null, wsId);
}

function renderTabs() {
  const list = document.getElementById('sb-page');
  const tabs = (state.tabs || []).filter(t => t.workspace_id === state.active_workspace_id);
  if (tabs.length === 0) {
    list.innerHTML = '<div style="text-align:center;padding:20px 8px;color:var(--text-dim);font-size:11px">No tabs open</div>';
    __animateSbPageEntry(list);
    return;
  }
  list.innerHTML = tabs.map(tab => {
    const active = tab.id === state.active_tab_id ? 'active' : '';
    const loading = tab.status === 'loading' ? 'loading' : '';
    const pinned = tab.pinned ? 'pinned' : '';
    const audioPlaying = tab.is_audio_playing ? 'audio-playing' : '';
    const tabMuted = tab.is_muted ? 'tab-muted' : '';
    const icon = tabIconUrl(tab);
    const fallback = tabFallbackIcon(loading, !!icon);
    const faviconEl = icon
      ? `<img class="tab-favicon ${loading}" src="${escAttr(icon)}" alt="" onerror="this.style.display='none';this.nextElementSibling.style.display='block'">${fallback}`
      : fallback;
    const showAudioBtn = tab.is_audio_playing || tab.is_muted;
    const audioTitle = tab.is_muted ? 'Unmute tab' : 'Mute tab';
    const audioSvg = tab.is_muted
      ? `<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><line x1="23" y1="9" x2="17" y2="15"/><line x1="17" y1="9" x2="23" y2="15"/></svg>`
      : `<svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14"/><path d="M15.54 8.46a5 5 0 0 1 0 7.07"/></svg>`;
    const audioBtn = showAudioBtn
      ? `<button class="tab-audio-btn" onclick="muteTab(event,'${tab.id}')" title="${audioTitle}">${audioSvg}</button>`
      : '';
    return `<div class="tab-item ${active} ${loading} ${pinned} ${audioPlaying} ${tabMuted}" onclick="switchTab('${tab.id}')" oncontextmenu="tabContextMenu(event,'${tab.id}')">
      ${faviconEl}
      <div class="tab-info">
        <div class="tab-title">${escHtml(tab.title || 'New Tab')}</div>
        <div class="tab-url">${escHtml(formatDisplayUrl(tab.url))}</div>
      </div>
      ${audioBtn}
      <button class="tab-close" onclick="closeTab(event,'${tab.id}')" title="Close tab">
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>
    </div>`;
  }).join('');
  __animateSbPageEntry(list);
}

function tabIconUrl(tab) {
  if (!tab) return '';
  if (tab.status === 'loading') return '';
  if (tab.favicon) return tab.favicon;
  try {
    const h = new URL(tab.url).hostname;
    return h ? shortcutIconUrl(h) : '';
  } catch {
    return '';
  }
}

function tabFallbackIcon(loading, hidden) {
  const style = hidden ? ' style="display:none"' : '';
  return `<svg class="tab-favicon ${loading}"${style} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 2a10 10 0 0 1 0 20"/></svg>`;
}

function renderAddressBar() {
  const tab = state.tabs && state.tabs.find(t => t.id === state.active_tab_id);
  if (!tab) return;
  const input = document.getElementById('url-input');
  if (document.activeElement !== input) {
    input.value = formatDisplayUrl(tab.url);
  }
  updateLockIcon(tab.url);
  checkNewtabPlaceholder(tab.url);
  renderBookmarkIcon();
}

function renderSearchSettings() {
  const select = document.getElementById('set-search-engine');
  if (!select) return;
  const engines = state.search_engines || [];
  const current = engines.find(e => e.is_default)
    || engines.find(e => e.id === (state.settings && state.settings.search && state.settings.search.default_engine))
    || engines.find(e => e.id === 'google');
  const previous = select.value;
  select.innerHTML = engines.map(e => `<option value="${escAttr(e.id)}">${escHtml(e.name)}${e.shortcut ? ` (${escHtml(e.shortcut)})` : ''}</option>`).join('');
  select.value = current ? current.id : previous;
  select.onchange = () => send('SaveSettings', {key: 'default_engine', value: select.value});
}

function renderNewtabShortcuts() {
  const list = document.getElementById('newtab-shortcuts');
  if (!list) return;
  const nt = newtabSettings();
  if (nt.show_quick_links === false) {
    list.innerHTML = '';
    return;
  }
  const engines = state.search_engines || [];
  const defaults = [
    {id: 'github-home', name: 'GitHub', url: 'https://github.com', domain: 'github.com'},
    {id: 'chatgpt-home', name: 'ChatGPT', url: 'https://chatgpt.com', domain: 'chatgpt.com'},
    {id: 'youtube-home', name: 'YouTube', url: 'https://youtube.com', domain: 'youtube.com'},
    {id: 'gmail-home', name: 'Gmail', url: 'https://mail.google.com', domain: 'mail.google.com'},
    {id: 'neurafeed-home', name: 'Neura Feed', url: 'https://feed.neuraspheres.com', domain: 'feed.neuraspheres.com'},
  ];
  const preferred = ['google', 'perplexity', 'brave'];
  const picked = preferred.map(id => engines.find(e => e.id === id)).filter(Boolean).map(e => ({
    id: e.id,
    name: e.name,
    url: e.url_template.replace('{query}', ''),
    domain: shortcutDomain(e.url_template),
  }));
  const seen = new Set();
  const shortcuts = [...defaults, ...picked].filter(item => {
    const key = item.url.replace(/\/$/, '');
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  }).slice(0, 8);
  if (!shortcuts.length) {
    list.innerHTML = '';
    return;
  }
  list.innerHTML = shortcuts.map(item => `
    <button class="newtab-shortcut" data-nav-url="${escAttr(item.url)}">
      <div class="newtab-shortcut-icon"><img src="${escAttr(shortcutIconUrl(item.domain || item.url))}" alt=""></div>
      <span class="newtab-shortcut-label">${escHtml(item.name)}</span>
    </button>
  `).join('');
}

function newtabSettings() {
  const s = state.settings || {};
  return s.new_tab || {};
}

function applyNewtabSettings() {
  const root = document.getElementById('newtab-placeholder');
  if (!root) return;
  const nt = newtabSettings();
  window.__ntState = nt;
  const layout = nt.feed_layout === 'headlines' || nt.feed_layout === 'compact' ? nt.feed_layout : 'cards';
  const bg = nt.show_background !== false && nt.wallpaper_source !== 'none';
  root.classList.toggle('nt-hide-background', !bg);
  root.classList.toggle('nt-hide-search', nt.show_search === false);
  root.classList.toggle('nt-hide-shortcuts', nt.show_quick_links === false);
  root.classList.toggle('nt-feed-headlines', layout === 'headlines');
  root.classList.toggle('nt-feed-compact', layout === 'compact');
  ['nt-theme-minimal','nt-theme-focus','nt-theme-horizon','nt-theme-informative'].forEach(c => root.classList.remove(c));
  const theme = nt.theme || 'focus';
  root.classList.add('nt-theme-' + theme);
  ['nt-clock-sf','nt-clock-rounded','nt-clock-mono','nt-clock-serif'].forEach(c => root.classList.remove(c));
  root.classList.add('nt-clock-' + clockStyle(nt.clock_style));
  initWallpaper(nt);
}

function shortcutIconUrl(domain) {
  return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(domain)}&sz=64`;
}

function shortcutDomain(url) {
  try {
    return new URL(url.replace('{query}', '')).hostname;
  } catch {
    return url;
  }
}

function requestNeuraFeed(force) {
  if (!newtabShowsFeed()) return;
  if (neuraFeedLoading) return;
  if (neuraFeedLoaded && !force) return;
  neuraFeedLoading = true;
  neuraFeedError = '';
  renderNeuraFeed();
  send('LoadNeuraFeed');
}

function renderNeuraFeed() {
  const main = document.getElementById('newtab-feed-main');
  if (!main) return;
  if (!newtabShowsFeed()) {
    main.innerHTML = '';
    return;
  }
  if (neuraFeedLoading && !neuraFeed.length) {
    main.innerHTML = `<div class="newtab-empty-feed">Loading Neura Feed...</div>`;
    return;
  }
  if (neuraFeedError && !neuraFeed.length) {
    main.innerHTML = `<div class="newtab-empty-feed">Neura Feed is not loading right now.</div>`;
    return;
  }
  const articles = neuraFeed.map(normalizeNewsArticle).filter(a => a.title).slice(0, 15);
  if (!articles.length) {
    main.innerHTML = `<div class="newtab-empty-feed">No stories yet.</div>`;
    return;
  }
  main.innerHTML = articles.map(renderNewsCard).join('');
}

function newtabShowsFeed() {
  return (newtabSettings().theme || 'focus') === 'informative';
}

function normalizeNewsArticle(article) {
  const date = Date.parse(article.createdAt || article.updatedAt || '');
  return {
    title: article.title || '',
    summary: stripTags(article.summary || article.whyItMatters || article.article || ''),
    image: article.coverImage || article.image || '',
    source: article.imageSource || sourceName(article.sources),
    url: article.imageSourceUrl || sourceUrl(article.sources) || 'https://feed.neuraspheres.com',
    time: Number.isNaN(date) ? 0 : date,
  };
}

function renderNewsCard(article, index) {
  const meta = [article.source, article.time ? formatRelativeTime(article.time) : 'Neura Feed'].filter(Boolean).join(' · ');
  const img = article.image ? `<img src="${escAttr(article.image)}" alt="" onerror="this.closest('.news-card').classList.add('no-image');this.remove()">` : '';
  const cls = newsCardClass(index, article.image);
  return `<button class="news-card ${cls}" data-news-url="${escAttr(article.url)}">
    ${img}
    <div class="news-card-body">
      <div class="news-card-meta">${escHtml(meta)}</div>
      <div class="news-card-title">${escHtml(article.title)}</div>
      ${index === 0 || cls.includes('tall') ? `<div class="news-card-summary">${escHtml(article.summary)}</div>` : ''}
    </div>
  </button>`;
}

function newsCardClass(index, image) {
  const parts = [];
  if (!image) parts.push('no-image');
  if (index === 0) parts.push('featured');
  else if (index % 9 === 0) parts.push('wide');
  return parts.join(' ');
}

function sourceName(sources) {
  const first = Array.isArray(sources) ? sources[0] : '';
  if (!first) return '';
  return String(first).split(':')[0].replace(/^\[\d+\]\s*/, '').trim();
}

function sourceUrl(sources) {
  const first = Array.isArray(sources) ? sources.find(Boolean) : '';
  const match = String(first || '').match(/https?:\/\/\S+/);
  return match ? match[0].replace(/[)"',.]+$/, '') : '';
}

function stripTags(text) {
  const div = document.createElement('div');
  div.innerHTML = String(text || '');
  return div.textContent || div.innerText || '';
}

function renderBookmarkIcon() {
  document.getElementById('bm-icon-empty').style.display = state.is_bookmarked ? 'none' : 'block';
  document.getElementById('bm-icon-filled').style.display = state.is_bookmarked ? 'block' : 'none';
}

function applyTheme() {
  const theme = state.settings && state.settings.appearance && state.settings.appearance.theme;
  if (!theme || theme === 'System') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
  } else {
    document.documentElement.setAttribute('data-theme', theme.toLowerCase());
  }
}

function applySidebarMode() {
  const s = state.settings || {};
  const mode = (s.appearance && s.appearance.sidebar_mode) || 'expanded';
  const isAutoHide = mode === 'auto_hide';
  const isCompact = mode === 'compact' || state.sidebar_collapsed;
  const app = document.getElementById('app');
  app.classList.toggle('sidebar-auto-hide', isAutoHide);
  app.classList.toggle('sidebar-collapsed', isCompact && !isAutoHide);
  app.classList.toggle('hide-tab-url', !(s.appearance && s.appearance.show_tab_url !== false));
  app.classList.toggle('compact-tabs', !!(s.tabs && s.tabs.compact_tabs));
  app.classList.toggle('show-bookmarks-bar', !!(s.appearance && s.appearance.show_bookmarks_bar));
  if (!isAutoHide && sidebarPeeking) {
    cancelSidebarHide();
    sidebarPeeking = false;
    sidebarPinned  = false;
    app.classList.remove('sidebar-floating-open');
  }
  _syncSidebarBtnState();
  renderBookmarksBar();
}

// ============================================================
// ACTIONS
// ============================================================
function switchTab(id) { send('SwitchTab', {id}); }

// ── Workspace page-slide animation state ───────────────────────────────────
let __wsSlideDir = 0; // -1 = going left (higher index), 1 = going right (lower index)

function switchWorkspace(id) {
  if (id === state.active_workspace_id) return;
  const ws = state.workspaces || [];
  const curIdx = ws.findIndex(w => w.id === state.active_workspace_id);
  const newIdx = ws.findIndex(w => w.id === id);
  if (curIdx >= 0 && newIdx >= 0) {
    __wsSlideDir = newIdx > curIdx ? -1 : 1;
    const page = document.getElementById('sb-page');
    if (page) {
      // Exit: slide toward target direction + fade
      page.style.transition = 'transform 0.17s cubic-bezier(0.4,0,1,1),opacity 0.14s ease';
      page.style.transform = `translateX(${__wsSlideDir * -28}px)`;
      page.style.opacity = '0';
    }
  }
  send('SwitchWorkspace', {id});
}

// Called after renderTabs() and renderTabs() empty-state branch
function __animateSbPageEntry(page) {
  if (!page || __wsSlideDir === 0) return;
  const inDir = -__wsSlideDir;
  __wsSlideDir = 0;
  // Reset to entry position instantly, then spring to rest
  page.style.transition = 'none';
  page.style.transform = `translateX(${inDir * 32}px)`;
  page.style.opacity = '0';
  requestAnimationFrame(() => requestAnimationFrame(() => {
    page.style.transition = 'transform 0.38s cubic-bezier(0.34,1.15,0.64,1),opacity 0.26s ease';
    page.style.transform = 'translateX(0)';
    page.style.opacity = '1';
  }));
}

function editWorkspaceByActive(ev) {
  if (ev) ev.stopPropagation();
  const id = state.active_workspace_id;
  if (id) openWorkspaceModal(id);
}

function deleteWorkspaceByActive(ev) {
  if (ev) ev.stopPropagation();
  const id = state.active_workspace_id;
  if (id) deleteWorkspace(null, id);
}
function closeTab(ev, id) { ev.stopPropagation(); send('CloseTab', {id}); }
function toggleSidebar() {
  const app = document.getElementById('app');
  const isAutoHide = app.classList.contains('sidebar-auto-hide');
  if (isAutoHide) {
    if (sidebarPeeking) {
      hideFloatingSidebar(true);
    } else {
      showFloatingSidebar(true);
    }
  } else {
    const next = app.classList.contains('sidebar-collapsed') ? 'expanded' : 'compact';
    if (!state.settings) state.settings = {};
    if (!state.settings.appearance) state.settings.appearance = {};
    state.settings.appearance.sidebar_mode = next;
    state.sidebar_collapsed = next === 'compact';
    app.classList.toggle('sidebar-collapsed', state.sidebar_collapsed);
    const sel = document.getElementById('set-sidebar-mode');
    if (sel) sel.value = next;
    send('SidebarToggle');
  }
}
function toggleAi() {
  state.ai_open = !state.ai_open;
  applyAiSidebar();
  send('ToggleAiSidebar');
}

function applyAiSidebar() {
  const app = document.getElementById('app');
  if (app) app.classList.toggle('ai-open', !!state.ai_open);
  const btn = document.getElementById('btn-ai');
  if (btn) btn.classList.toggle('active', !!state.ai_open);
}
function toggleBookmark() {
  if (state.is_bookmarked) {
    const tab = state.tabs && state.tabs.find(t => t.id === state.active_tab_id);
    if (tab) send('BookmarkRemove', {url: tab.url});
  } else {
    send('BookmarkAdd');
  }
}
function addWorkspace() {
  openWorkspaceModal();
}
function togglePageChipCollapse() {
  const qa = document.getElementById('ai-quick-actions');
  if (qa) qa.classList.toggle('qa-collapsed');
}
function openIncognitoWorkspace() {
  openWorkspaceModal();
  const check = document.getElementById('workspace-incognito-check');
  if (check) check.checked = true;
  selectedWsEmoji = '🔐';
  selectedWsColor = '#6b7280';
  wsColorManual = false;
  const preview = document.getElementById('ws-emoji-preview');
  if (preview) preview.textContent = selectedWsEmoji;
  document.querySelectorAll('.ws-emoji-opt').forEach(b => {
    b.classList.toggle('selected', b.dataset.emoji === selectedWsEmoji);
  });
  syncWsColorUi();
}
// Ctrl+Shift+N: switch to existing incognito workspace (opening a new tab there),
// or silently create one named "Incognito" if none exists yet.
function switchToTabIndex(i) {
  const tab = (state.tabs || [])[i];
  if (tab) send('SwitchTab', {id: tab.id});
}
function openWorkspaceModal(wsId = null) {
  const modal = document.getElementById('workspace-modal');
  const input = document.getElementById('workspace-name-input');
  const error = document.getElementById('workspace-name-error');
  const title = document.getElementById('workspace-modal-title');
  const submit = document.getElementById('workspace-submit-label');
  if (!modal || !input) return;
  const ws = wsId ? (state.workspaces || []).find(w => w.id === wsId) : null;
  editingWorkspaceId = ws ? ws.id : null;
  if (error) error.textContent = '';
  if (title) title.textContent = ws ? 'Rename workspace' : 'New workspace';
  if (submit) submit.textContent = ws ? 'Save' : 'Create';
  input.value = ws ? ws.name : '';
  const wsIcon = ws ? (ws.icon || '📁') : '📁';
  selectedWsEmoji = wsIcon;
  selectedWsColor = cleanHex(ws && ws.accent_color) || emojiColor(wsIcon);
  wsColorManual = !!ws;
  const emojiPreview = document.getElementById('ws-emoji-preview');
  if (emojiPreview) emojiPreview.textContent = wsIcon;
  document.querySelectorAll('.ws-emoji-opt').forEach(b => {
    b.classList.toggle('selected', b.dataset.emoji === wsIcon);
  });
  syncWsColorUi();
  const incognitoRow = document.getElementById('workspace-incognito-row');
  const incognitoCheck = document.getElementById('workspace-incognito-check');
  if (incognitoRow) incognitoRow.style.display = ws ? 'none' : 'flex';
  if (incognitoCheck) incognitoCheck.checked = false;
  modal.classList.add('open');
  send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
  setTimeout(() => {
    input.focus();
    input.select();
  }, 40);
}
function closeWorkspaceModal() {
  const modal = document.getElementById('workspace-modal');
  if (modal) modal.classList.remove('open');
  editingWorkspaceId = null;
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
}
function handleWorkspaceModalClick(e) {
  if (e.target.id === 'workspace-modal') closeWorkspaceModal();
}
function submitWorkspaceModal(e) {
  e.preventDefault();
  const input = document.getElementById('workspace-name-input');
  const error = document.getElementById('workspace-name-error');
  const name = input ? input.value.trim().replace(/\s+/g, ' ') : '';
  if (!name) {
    if (error) error.textContent = 'Name is required';
    if (input) input.focus();
    return;
  }
  const editId = editingWorkspaceId;
  const isIncognito = !editId && !!(document.getElementById('workspace-incognito-check') || {}).checked;
  const accentColor = cleanHex(selectedWsColor) || emojiColor(selectedWsEmoji);
  closeWorkspaceModal();
  if (editId) {
    send('RenameWorkspace', {id: editId, name, icon: selectedWsEmoji, accent_color: accentColor});
  } else {
    send('NewWorkspace', {name, is_incognito: isIncognito, icon: selectedWsEmoji, accent_color: accentColor});
  }
}
function selectWsEmoji(el, emoji) {
  selectedWsEmoji = emoji;
  const preview = document.getElementById('ws-emoji-preview');
  if (preview) preview.textContent = emoji;
  document.querySelectorAll('.ws-emoji-opt').forEach(b => b.classList.remove('selected'));
  if (el) el.classList.add('selected');
  if (!wsColorManual) selectWsColor(emojiColor(emoji), false);
}

function selectWsColor(color, manual) {
  const next = cleanHex(color) || emojiColor(selectedWsEmoji);
  selectedWsColor = next;
  if (manual) wsColorManual = true;
  syncWsColorUi();
}

function syncWsColorUi() {
  const color = cleanHex(selectedWsColor) || '#8b5cf6';
  const rgb = hexRgb(color) || [139,92,246];
  const prev = document.getElementById('ws-color-preview');
  const input = document.getElementById('ws-color-input');
  if (prev) prev.style.setProperty('--ws-picker-rgb', rgb.join(','));
  if (input) input.value = color;
  document.querySelectorAll('.ws-color-opt').forEach(btn => {
    btn.classList.toggle('selected', cleanHex(btn.dataset.color) === color);
  });
}

function cleanHex(color) {
  const text = String(color || '').trim();
  return /^#[0-9a-f]{6}$/i.test(text) ? text.toLowerCase() : '';
}

function hexRgb(color) {
  const hex = cleanHex(color);
  if (!hex) return null;
  return [
    parseInt(hex.slice(1, 3), 16),
    parseInt(hex.slice(3, 5), 16),
    parseInt(hex.slice(5, 7), 16)
  ];
}

function emojiColor(emoji) {
  const colors = {
    '🌐':'#3b82f6','💼':'#6366f1','🏠':'#f97316','🔬':'#06b6d4','💬':'#22c55e','🛍️':'#ec4899','📰':'#64748b','💻':'#3b82f6','🎵':'#8b5cf6','🎬':'#ef4444','📁':'#8b5cf6','🎨':'#ec4899','🚀':'#f97316','❤️':'#ef4444','⭐':'#eab308','🔥':'#f97316','💡':'#eab308','🏆':'#eab308','🎯':'#ef4444','📚':'#3b82f6','🌟':'#eab308','🍕':'#f97316','🎮':'#8b5cf6','🌈':'#ec4899','🦋':'#06b6d4','🌸':'#ec4899','🎸':'#ef4444','🌙':'#6366f1','☀️':'#eab308','🌊':'#06b6d4','🧩':'#22c55e','🔐':'#6b7280','📊':'#3b82f6','🎭':'#8b5cf6','🏖️':'#06b6d4','🚗':'#ef4444','✈️':'#3b82f6','🎓':'#6366f1','💎':'#06b6d4','🎪':'#ec4899'
  };
  return colors[emoji] || '#8b5cf6';
}

function editWorkspace(ev, id) {
  if (ev && ev.stopPropagation) ev.stopPropagation();
  openWorkspaceModal(id);
}
function deleteWorkspace(ev, id) {
  if (ev && ev.stopPropagation) ev.stopPropagation();
  if ((state.workspaces || []).length <= 1) {
    toast('Keep at least one workspace', 'error');
    return;
  }
  openWorkspaceDeleteModal(id);
}
function openWorkspaceDeleteModal(id) {
  const ws = (state.workspaces || []).find(w => w.id === id);
  if (!ws) return;
  deletingWorkspaceId = id;
  const modal = document.getElementById('workspace-delete-modal');
  const name = document.getElementById('workspace-delete-name');
  const confirmBtn = document.getElementById('workspace-delete-confirm');
  if (name) name.textContent = ws.name || 'this workspace';
  if (modal) modal.classList.add('open');
  send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
  setTimeout(() => confirmBtn && confirmBtn.focus(), 40);
}
function closeWorkspaceDeleteModal() {
  const modal = document.getElementById('workspace-delete-modal');
  if (modal) modal.classList.remove('open');
  deletingWorkspaceId = null;
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
}
function handleWorkspaceDeleteModalClick(e) {
  if (e.target.id === 'workspace-delete-modal') closeWorkspaceDeleteModal();
}
function confirmWorkspaceDelete() {
  const id = deletingWorkspaceId;
  if (!id) return;
  closeWorkspaceDeleteModal();
  send('DeleteWorkspace', {id});
}

// ============================================================
// WINDOW CONTROLS (frameless window)
// ============================================================
const DRAG_EXEMPT = 'button,input,select,textarea,a,.tb-btn,#address-bar,#win-controls';
function handleToolbarDrag(e) {
  if (e.button !== 0) return;
  if (e.detail > 1) return;
  if (e.target.closest(DRAG_EXEMPT)) return;
  send('WindowDragStart');
}
function handleToolbarDblClick(e) {
  if (e.target.closest(DRAG_EXEMPT)) return;
  send('WindowMaximize');
}

function _syncSidebarBtnState() {
  const btn = document.getElementById('sidebar-toggle-btn');
  const app = document.getElementById('app');
  if (!btn) return;
  const isAutoHide = app.classList.contains('sidebar-auto-hide');
  const isPinned = isAutoHide && sidebarPinned;
  btn.classList.toggle('active', isPinned);
  app.classList.toggle('sidebar-pinned', isPinned);
}

function cancelSidebarHide() {
  if (sidebarHideTimer !== null) {
    clearTimeout(sidebarHideTimer);
    sidebarHideTimer = null;
  }
}

function clearSidebarClipTimer() {
  if (sidebarClipTimer !== null) {
    clearTimeout(sidebarClipTimer);
    sidebarClipTimer = null;
  }
}

function clearSidebarPinTimer() {
  if (sidebarPinTimer !== null) {
    clearTimeout(sidebarPinTimer);
    sidebarPinTimer = null;
  }
}

function scheduleSidebarPin() {
  clearSidebarPinTimer();
  sidebarPinTimer = setTimeout(() => {
    sidebarPinTimer = null;
    if (sidebarPeeking && sidebarPinned) send('SidebarPeek', {visible: true, pinned: true});
  }, 200);
}

function _doHideSidebar() {
  clearSidebarPinTimer();
  sidebarPeeking = false;
  sidebarPinned  = false;
  document.getElementById('app').classList.remove('sidebar-floating-open');
  _syncSidebarBtnState();
  clearSidebarClipTimer();
  sidebarClipTimer = setTimeout(() => {
    sidebarClipTimer = null;
    if (!sidebarPeeking) send('SidebarPeek', {visible: false, pinned: false});
  }, 220);
}

// pin=true: opened via button click — stays open until explicitly closed
// pin=false: hover — never activates the button, never pins
function showFloatingSidebar(pin) {
  cancelSidebarHide();
  clearSidebarClipTimer();
  if (!sidebarPeeking) {
    sidebarPinned = !!pin;
    sidebarPeeking = true;
    document.getElementById('app').classList.add('sidebar-floating-open');
    if (pin) {
      send('SidebarPeek', {visible: true, pinned: false});
      scheduleSidebarPin();
    } else {
      send('SidebarPeek', {visible: true, pinned: false});
    }
  } else if (pin) {
    sidebarPinned = true;
    scheduleSidebarPin();
  }
  _syncSidebarBtnState();
}

// Schedule a close with debounce — allows mouse to travel from trigger to sidebar
function scheduledHide() {
  if (!sidebarPeeking || sidebarPinned) return;
  const pop = document.getElementById('sb-ws-popover');
  if (pop && pop.classList.contains('visible')) return;
  cancelSidebarHide();
  sidebarHideTimer = setTimeout(() => {
    sidebarHideTimer = null;
    if (sidebarPeeking && !sidebarPinned) _doHideSidebar();
  }, sidebarHideDelay);
}

// force=true: close immediately even if pinned (button click or backdrop click)
function hideFloatingSidebar(force) {
  if (!sidebarPeeking) return;
  if (sidebarPinned && !force) return;
  cancelSidebarHide();
  _doHideSidebar();
}

function saveSetting(key, value) {
  value = cleanSettingValue(key, value);
  rememberSetting(key, value);
  send('SaveSettings', {key, value});
  if (key === 'new_tab_feed_layout' || key === 'new_tab_clock_style') {
    if (!state.settings) state.settings = {};
    if (!state.settings.new_tab) state.settings.new_tab = {};
    state.settings.new_tab[key.replace('new_tab_', '')] = value;
    applyNewtabSettings();
    syncNewtabSettingsUI();
  }
  if (key === 'new_tab_wallpaper_url' || key === 'new_tab_wallpaper_color') {
    // wallpaper helper updates --nt-bg-image directly; sync label for color
    if (key === 'new_tab_wallpaper_color') {
      const lbl = document.getElementById('nt-wp-color-label');
      if (lbl) lbl.textContent = value;
    }
  }
  if (key === 'sidebar_mode') {
    if (!state.settings) state.settings = {};
    if (!state.settings.appearance) state.settings.appearance = {};
    state.settings.appearance.sidebar_mode = value;
    state.sidebar_collapsed = value === 'compact';
    cancelSidebarHide();
    clearSidebarClipTimer();
    clearSidebarPinTimer();
    sidebarPeeking = false;
    sidebarPinned = false;
    document.getElementById('app').classList.remove('sidebar-floating-open');
    applySidebarMode();
  }
  updateGeneralSettingsReadout();
}
function rememberSetting(key, value) {
  if (!state.settings) state.settings = {};
  if (key.startsWith('new_tab_')) {
    if (!state.settings.new_tab) state.settings.new_tab = {};
    state.settings.new_tab[key.replace('new_tab_', '')] = value;
  }
  if (key === 'startup_behavior') state.settings.startup_behavior = value;
  if (key === 'homepage') state.settings.homepage = value;
  if (key === 'region') state.settings.region = value;
  if (key.startsWith('secure_dns_')) {
    if (!state.settings.privacy) state.settings.privacy = {};
    state.settings.privacy[key] = value;
    syncSecureDnsSettingsUI();
  }
  if (key === 'download_path') {
    if (!state.settings.downloads) state.settings.downloads = {};
    state.settings.downloads.default_folder = value;
  }
  if (key === 'homepage' || key === 'download_path') delete settingDrafts[key];
  if (key === 'homepage') setInputValue('set-homepage', value);
  if (key === 'download_path') setInputValue('set-download-path', value);
}
function browseDownloadFolder() {
  send('BrowseDownloadFolder');
}
function onZoomSliderInput(v) {
  const el = document.getElementById('zoom-level-display');
  if (el) el.textContent = Math.round(v) + '%';
}
function applyGlobalZoom(v) {
  send('ZoomGlobal', {level: parseFloat(v) / 100});
}

function searchSettings() {
  const s = state.settings || {};
  return s.search || {};
}

function searchSuggestionsEnabled() {
  return searchSettings().suggestions_enabled !== false;
}

function trendingEnabled() {
  return searchSettings().trending_enabled !== false;
}

function populateSettingsPanel() {
  const s = state.settings || {};
  const app = s.appearance || {};
  const priv = s.privacy || {};
  const dl = s.downloads || {};
  const ai = s.ai || {};
  const nt = s.new_tab || {};
  setSelectValue('set-startup', s.startup_behavior || 'new_tab');
  setInputValue('set-homepage', cleanSettingValue('homepage', visibleSetting('homepage', s.homepage)));
  setInputValue('set-download-path', cleanSettingValue('download_path', visibleSetting('download_path', dl.default_folder)));
  setToggleEl('toggle-ask-download', dl.ask_where_to_save !== false);
  updateGeneralSettingsReadout();
  const modeMap = {expanded:'expanded', compact:'compact', auto_hide:'auto_hide'};
  setSelectValue('set-sidebar-mode', modeMap[app.sidebar_mode] || 'expanded');
  const themeMap = {dark:'dark', light:'light', system:'system'};
  const themeKey = themeMap[(app.theme || '').toLowerCase()] || 'dark';
  document.querySelectorAll('.theme-card').forEach(c => c.classList.remove('selected'));
  const tc = document.getElementById('theme-' + themeKey);
  if (tc) tc.classList.add('selected');
  setToggleEl('toggle-show-bookmarks-bar', !!app.show_bookmarks_bar);
  setToggleEl('toggle-show-url', app.show_tab_url !== false);
  setToggleEl('toggle-suggestions', searchSuggestionsEnabled());
  setToggleEl('toggle-trending', trendingEnabled());
  setToggleEl('toggle-history', !priv.disable_history);
  setToggleEl('toggle-ad-blocker-enabled', priv.ad_blocker_enabled !== false);
  setToggleEl('toggle-secure-dns-enabled', !!priv.secure_dns_enabled);
  setSelectValue('set-secure-dns-provider', priv.secure_dns_provider || 'cloudflare');
  setSelectValue('set-secure-dns-mode', priv.secure_dns_mode || 'secure');
  setInputValue('set-secure-dns-template', priv.secure_dns_template || 'https://1.1.1.1/dns-query');
  setToggleEl('toggle-new-tab-show-search', nt.show_search !== false);
  setToggleEl('toggle-new-tab-show-quick-links', nt.show_quick_links !== false);
  syncNewtabSettingsUI();
  syncSecureDnsSettingsUI();
  renderAdBlockExceptions(priv.ad_blocker_exceptions || []);
  updateProviderDdUI(ai.default_provider || 'openai');
  const zoomPct = Math.round((app.zoom_level || 1.0) * 100);
  setInputValue('global-zoom-slider', zoomPct);
  onZoomSliderInput(zoomPct);
  populateRegionSettings();
}
function syncSecureDnsSettingsUI() {
  const priv = ((state.settings || {}).privacy) || {};
  const enabled = !!priv.secure_dns_enabled;
  const provider = priv.secure_dns_provider || 'cloudflare';
  const box = document.getElementById('secure-dns-options');
  if (box) {
    box.style.opacity = enabled ? '1' : '0.58';
    box.querySelectorAll('select,input').forEach(el => { el.disabled = !enabled; });
  }
  const custom = document.getElementById('secure-dns-custom-row');
  if (custom) custom.style.display = provider === 'custom' ? 'block' : 'none';
}
function syncNewtabSettingsUI() {
  const nt = newtabSettings();
  const theme = nt.theme || 'focus';
  document.querySelectorAll('.nt-theme-card').forEach(c => {
    c.classList.toggle('selected', c.dataset.theme === theme);
  });
  const clock = clockStyle(nt.clock_style);
  document.querySelectorAll('.nt-clock-card').forEach(c => {
    c.classList.toggle('selected', c.dataset.clock === clock);
  });
  const src = nt.wallpaper_source || 'nature';
  syncWallpaperSourceUI(src, nt);
}

function setSelectValue(id, value) {
  const el = document.getElementById(id);
  if (el) el.value = value;
}
function setInputValue(id, value) {
  const el = document.getElementById(id);
  if (el && document.activeElement !== el) el.value = value;
}
function visibleSetting(key, value) {
  if (value != null && String(value).length > 0) {
    delete settingDrafts[key];
    return value;
  }
  return settingDrafts[key] || '';
}
function cleanSettingValue(key, value) {
  const text = String(value || '').trim();
  if (key === 'homepage') return normalizeHomepage(text);
  if (key === 'download_path') return text;
  return value;
}
function normalizeHomepage(value) {
  const text = String(value || '').trim();
  if (!text) return 'neura://newtab';
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(text)) return text;
  if (text.startsWith('neura://')) return text;
  if (text.startsWith('localhost') || text.startsWith('127.0.0.1') || text.startsWith('0.0.0.0')) return 'http://' + text;
  if (text.includes('.')) return 'https://' + text;
  return text;
}
function updateGeneralSettingsReadout() {
  const s = state.settings || {};
  const dl = s.downloads || {};
  const startup = s.startup_behavior || 'new_tab';
  const startupNames = {
    new_tab: 'Open a new tab',
    last_session: 'Restore last session',
    home_page: 'Open home page'
  };
  setText('current-startup', 'Saved: ' + (startupNames[startup] || startupNames.new_tab));
  setText('current-homepage', 'Saved: ' + normalizeHomepage(s.homepage));
  const path = String(dl.default_folder || '').trim();
  setText('current-download-path', 'Saved: ' + (path || 'Default downloads folder'));
  const ask = dl.ask_where_to_save !== false;
  setText('current-ask-download', ask ? 'Saved: ask every time before downloading' : 'Saved: download automatically to the location above');
}
function setText(id, value) {
  const el = document.getElementById(id);
  if (el) el.textContent = value;
}
function setToggleEl(id, on) {
  const el = document.getElementById(id) || document.getElementById('toggle-' + id.replace(/_/g,'-'));
  if (el) el.classList.toggle('on', !!on);
}

function startLoadProgress() {
  if (loadProgressTimer && loadProgress > 0 && loadProgress < 1) return;
  clearInterval(loadProgressTimer);
  loadProgress = 0.08;
  setAddressLoadProgress(loadProgress);
  loadProgressTimer = setInterval(() => {
    if (loadProgress < 0.88) {
      loadProgress += Math.max(0.015, (0.9 - loadProgress) * 0.06);
      setAddressLoadProgress(loadProgress);
    }
  }, 350);
}
function setLoadProgress(progress) {
  const next = Math.max(loadProgress || 0, Math.min(progress, 0.98));
  if (next <= 0) return;
  loadProgress = next;
  setAddressLoadProgress(next);
}
function finishLoadProgress() {
  clearInterval(loadProgressTimer);
  loadProgressTimer = null;
  loadProgress = 1;
  setAddressLoadProgress(1);
  const bar = document.getElementById('address-bar');
  if (bar) bar.classList.add('done');
  setTimeout(() => {
    if (bar) {
      bar.classList.remove('loading','done');
    }
    loadProgress = 0;
  }, 420);
}
function setAddressLoadProgress(progress) {
  const bar = document.getElementById('address-bar');
  if (!bar) return;
  bar.classList.add('loading');
  bar.classList.remove('done');
}

// ============================================================
// ADDRESS BAR
// ============================================================
function focusUrl() {
  const input = document.getElementById('url-input');
  input.focus();
  input.select();
}
function handleUrlFocus() {
  const input = document.getElementById('url-input');
  input.select();
  input.dataset.showingCurrent = '1';
  if (!searchSuggestionsEnabled()) return;
  activeSuggestionTarget = 'url';
  activeSuggestionIndex = -1;
  renderSuggestions('url', '');
  send('GetHistory', {q: ''});
}
function handleUrlInput(value) {
  const input = document.getElementById('url-input');
  delete input.dataset.showingCurrent;
  if (!searchSuggestionsEnabled()) {
    hideSuggestions();
    return;
  }
  // Inline completion: find best frecency domain match, show as selected ghost text.
  if (value.length >= 2 && !value.includes(' ')) {
    const completion = findInlineCompletion(value);
    if (completion && completion.length > value.length) {
      input.value = completion;
      input.setSelectionRange(value.length, completion.length);
    }
  }
  renderSuggestions('url', value);
  send('GetHistory', {q: value});
}
function handleUrlBlur() {
  delete document.getElementById('url-input').dataset.showingCurrent;
  scheduleSuggestionClose();
  restoreDisplayUrl();
}
function handleUrlKey(e) {
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    const raw = e.target.value.trim();
    if (!raw) return;
    const url = /^[a-z0-9-]+$/i.test(raw) ? 'https://www.' + raw + '.com' : raw;
    hideSuggestions();
    send('Navigate', {url});
    e.target.blur();
    return;
  }
  if (handleSuggestionKey(e, 'url')) return;
  const input = e.target;
  // Accept inline completion: Tab or ArrowRight when there's a selection (ghost text active).
  if (e.key === 'Tab' || e.key === 'ArrowRight') {
    const selStart = input.selectionStart;
    const selEnd = input.selectionEnd;
    if (selEnd > selStart) {
      e.preventDefault();
      input.setSelectionRange(selEnd, selEnd);
      return;
    }
  }
  if (e.key === 'Enter') {
    const val = input.value.trim();
    if (val) { hideSuggestions(); send('Navigate', {url: val}); input.blur(); }
  } else if (e.key === 'Escape') {
    hideSuggestions();
    input.blur();
    renderAddressBar();
  }
}
function restoreDisplayUrl() {
  const tab = state.tabs && state.tabs.find(t => t.id === state.active_tab_id);
  if (tab) document.getElementById('url-input').value = formatDisplayUrl(tab.url);
}
function formatDisplayUrl(url) {
  if (!url || url === 'about:blank') return '';
  if (url.startsWith('neura://')) return url;
  try {
    const u = new URL(url);
    return u.hostname + (u.pathname !== '/' ? u.pathname : '') + u.search;
  } catch { return url; }
}
function siteDomain(url) {
  if (!url || url.startsWith('neura://')) return url || '';
  try { return new URL(url).hostname.replace(/^www\./, ''); } catch { return url; }
}
function extractSiteNameFromTitle(title) {
  title = cleanSiteText(title);
  if (!title || looksLikeUrl(title)) return null;
  const separators = [' - ', ' | ', ` ${String.fromCharCode(0x2014)} `, ` ${String.fromCharCode(0x2013)} `, ` ${String.fromCharCode(0x00b7)} `, ' :: '];
  for (const sep of separators) {
    const idx = title.lastIndexOf(sep);
    if (idx > 0) {
      const candidate = cleanSiteText(title.slice(idx + sep.length));
      const words = candidate.split(/\s+/).length;
      if (candidate.length >= 2 && candidate.length <= 28 && words <= 3 && !looksLikeUrl(candidate) && !genericSiteSuffix(candidate) && !siteTitleNoise(candidate)) {
        return candidate;
      }
    }
  }
  if (title.length <= 28 && title.split(/\s+/).length <= 3 && !genericSiteSuffix(title) && !siteTitleNoise(title)) return title;
  return null;
}
function friendlySiteName(url, title) {
  const fromTitle = extractSiteNameFromTitle(title);
  if (fromTitle) return fromTitle;
  return siteLabel(url);
}
function bookmarkLabel(url, title) {
  const t = cleanSiteText(title);
  if (t && !looksLikeUrl(t) && !genericSiteSuffix(t)) return t;
  return siteLabel(url);
}
function bookmarkIconUrl(url) {
  const domain = siteDomain(url);
  if (!domain || domain.startsWith('neura://')) return '';
  return shortcutIconUrl(domain);
}
function siteInitial(text) {
  const t = cleanSiteText(text);
  if (!t) return 'B';
  return t.charAt(0).toUpperCase();
}
function cleanSiteText(text) {
  return String(text || '').replace(/\s+/g, ' ').replace(/[\[\]{}()]+$/g, '').trim();
}
function looksLikeUrl(text) {
  return /^(https?:\/\/)?(www\.)?[a-z0-9-]+(\.[a-z0-9-]+)+(\/|$)/i.test(String(text || '').trim());
}
function genericSiteSuffix(text) {
  return /^(home|homepage|official site|broadcast yourself|search|login|sign in|news|latest news)$/i.test(String(text || '').trim());
}
function siteTitleNoise(text) {
  return /[/:?]|\.(com|net|org|io|co|id|uk|app|dev)\b/i.test(String(text || '').trim());
}
function siteLabel(url) {
  const domain = siteDomain(url).toLowerCase();
  if (!domain) return 'Bookmark';
  const exact = {
    'mail.google.com':'Gmail',
    'drive.google.com':'Drive',
    'docs.google.com':'Docs',
    'calendar.google.com':'Calendar',
    'maps.google.com':'Maps',
    'photos.google.com':'Photos',
    'music.youtube.com':'YouTube Music',
    'web.whatsapp.com':'WhatsApp',
    'app.slack.com':'Slack'
  };
  if (exact[domain]) return exact[domain];
  const parts = domain.split('.').filter(Boolean).filter(p => !['www','m','mobile','app'].includes(p));
  if (!parts.length) return domain;
  const slds = ['co','com','net','org','ac','gov'];
  const idx = parts.length > 2 && parts[parts.length - 1].length === 2 && slds.includes(parts[parts.length - 2])
    ? parts.length - 3
    : parts.length - 2;
  const raw = parts[Math.max(0, idx)] || parts[0];
  const known = {youtube:'YouTube',github:'GitHub',gmail:'Gmail',google:'Google',reddit:'Reddit',linkedin:'LinkedIn',instagram:'Instagram',facebook:'Facebook',x:'X',twitter:'Twitter',netflix:'Netflix',spotify:'Spotify',tiktok:'TikTok',whatsapp:'WhatsApp',stackoverflow:'Stack Overflow',bbc:'BBC'};
  return known[raw] || raw.split('-').filter(Boolean).map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' ');
}
function findInlineCompletion(typed) {
  if (!typed || typed.length < 2) return null;
  const strippedTyped = typed.toLowerCase().replace(/^https?:\/\/(www\.)?/, '');
  for (const h of uniqueHistory()) {
    if (!h.url) continue;
    try {
      const domain = new URL(h.url).hostname.replace(/^www\./, '').toLowerCase();
      if (domain.startsWith(strippedTyped) && domain.length > strippedTyped.length) {
        return domain; // e.g. "youtube.com"
      }
    } catch {}
  }
  return null;
}
function updateLockIcon(url) {
  const lock = document.getElementById('lock-icon');
  const warn = document.getElementById('insecure-icon');
  const favicon = document.getElementById('active-favicon');
  const loadingIcon = document.getElementById('active-loading-icon');
  const tab = state.tabs && state.tabs.find(t => t.id === state.active_tab_id);
  if (tab && tab.status === 'loading') {
    loadingIcon.style.display = 'flex';
    favicon.style.display = 'none';
    lock.style.display = 'none';
    warn.style.display = 'none';
  } else if (tab && tab.favicon) {
    favicon.src = tab.favicon;
    favicon.style.display = 'block';
    loadingIcon.style.display = 'none';
    lock.style.display = 'none';
    warn.style.display = 'none';
  } else if (url && url.startsWith('https://')) {
    lock.style.display = 'flex';
    warn.style.display = 'none';
    favicon.style.display = 'none';
    loadingIcon.style.display = 'none';
  } else if (url && url.startsWith('http://')) {
    warn.style.display = 'flex';
    lock.style.display = 'none';
    favicon.style.display = 'none';
    loadingIcon.style.display = 'none';
  } else {
    lock.style.display = 'none';
    warn.style.display = 'none';
    favicon.style.display = 'none';
    loadingIcon.style.display = 'none';
  }
}
function checkNewtabPlaceholder(url) {
  const show = !url || url === 'neura://newtab' || url === 'about:blank';
  document.getElementById('newtab-placeholder').style.display = show ? 'flex' : 'none';
  if (show) {
    updateGreeting();
    updateNewtabDate();
    applyNewtabSettings();
    startNewtabClock();
    requestNeuraFeed(false);
  }
}
function handleNewtabKey(e) {
  if (handleSuggestionKey(e, 'newtab')) return;
  if (e.key === 'Enter') {
    const v = e.target.value.trim();
    if (v) { send('Navigate', {url: v}); e.target.value = ''; hideSuggestions(); }
  }
}
function handleNewtabFocus() {
  if (!searchSuggestionsEnabled()) return;
  openSuggestions('newtab');
  send('GetHistory', {q: ''});
}
function handleNewtabInput(value) {
  if (!searchSuggestionsEnabled()) {
    hideSuggestions();
    return;
  }
  renderSuggestions('newtab', value);
}
function handleNewtabBlur() {
  scheduleSuggestionClose();
}
function updateGreeting() {
  const h = new Date().getHours();
  const greet = h < 12 ? 'Good morning' : h < 17 ? 'Good afternoon' : 'Good evening';
  document.getElementById('newtab-greeting').textContent = greet;
}

function updateNewtabDate() {
  const el = document.getElementById('newtab-date');
  if (!el) return;
  el.textContent = new Date().toLocaleDateString([], {weekday: 'long', month: 'long', day: 'numeric'});
}

function cssBgUrl(url) {
  const safe = String(url || '').replace(/\\/g, '\\\\').replace(/"/g, '\\"');
  return `url("${safe}")`;
}

function setNewtabBg(css) {
  const root = document.getElementById('newtab-placeholder');
  if (!root) return;
  const next = css || 'linear-gradient(transparent,transparent)';
  if (newtabBgCss === next) return;
  newtabBgCss = next;
  root.style.setProperty('--nt-bg-image', next);
}

const NT_NATURE_PHOTOS = [
  'https://picsum.photos/seed/mount1/1920/1080',
  'https://picsum.photos/seed/lake2/1920/1080',
  'https://picsum.photos/seed/forest3/1920/1080',
  'https://picsum.photos/seed/coast4/1920/1080',
  'https://picsum.photos/seed/canyon5/1920/1080',
  'https://picsum.photos/seed/desert6/1920/1080',
  'https://picsum.photos/seed/fjord7/1920/1080',
  'https://picsum.photos/seed/savanna8/1920/1080',
  'https://picsum.photos/seed/tundra9/1920/1080',
  'https://picsum.photos/seed/jungle10/1920/1080',
  'https://picsum.photos/seed/valley11/1920/1080',
  'https://picsum.photos/seed/waterfall12/1920/1080',
  'https://picsum.photos/seed/glacier13/1920/1080',
  'https://picsum.photos/seed/meadow14/1920/1080',
  'https://picsum.photos/seed/aurora15/1920/1080',
  'https://picsum.photos/seed/dunes16/1920/1080',
  'https://picsum.photos/seed/cliffs17/1920/1080',
  'https://picsum.photos/seed/steppe18/1920/1080',
  'https://picsum.photos/seed/reef19/1920/1080',
  'https://picsum.photos/seed/volcano20/1920/1080',
];

function initWallpaper(nt) {
  const src = (nt && nt.wallpaper_source) || 'nature';
  const root = document.getElementById('newtab-placeholder');
  if (!root) return;
  if (src !== 'daily') newtabBgSeed = '';
  function applyBgUrl(url) {
    if (!url) {
      setNewtabBg('');
      return;
    }
    const css = cssBgUrl(url);
    if (newtabBgCss === css) return;
    const img = new Image();
    img.onload = () => setNewtabBg(css);
    img.onerror = () => {};
    img.src = url;
  }
  if (src === 'none') {
    setNewtabBg('');
    return;
  }
  if (src === 'color') {
    const col = (nt && nt.wallpaper_color) || '#141414';
    setNewtabBg(`linear-gradient(${col},${col})`);
    return;
  }
  if (src === 'url') {
    applyBgUrl((nt && nt.wallpaper_url) || '');
    return;
  }
  if (src === 'upload') {
    const dataUrl = newtabWallpaperData || '';
    setUploadPreview(dataUrl);
    setNewtabBg(dataUrl ? cssBgUrl(dataUrl) : '');
    return;
  }
  if (src === 'nature') {
    const idx = (new Date().getDate() + new Date().getMonth() * 31) % NT_NATURE_PHOTOS.length;
    applyBgUrl(NT_NATURE_PHOTOS[idx]);
    return;
  }
  const day = new Date().toISOString().slice(0, 10).replace(/-/g, '');
  if (newtabBgSeed === day && newtabBgCss) return;
  newtabBgSeed = day;
  applyBgUrl(`https://picsum.photos/seed/${day}/1920/1080`);
}

function setNewtabTheme(theme) {
  saveSetting('new_tab_theme', theme);
  if (state.settings && state.settings.new_tab) state.settings.new_tab.theme = theme;
  applyNewtabSettings();
  renderNeuraFeed();
  if (newtabShowsFeed()) requestNeuraFeed(false);
  syncNewtabSettingsUI();
}

function clockStyle(style) {
  return ['sf','rounded','mono','serif'].includes(style) ? style : 'serif';
}

function setClockStyle(style) {
  const next = clockStyle(style);
  saveSetting('new_tab_clock_style', next);
  if (state.settings && state.settings.new_tab) state.settings.new_tab.clock_style = next;
  applyNewtabSettings();
  syncNewtabSettingsUI();
}

function setWallpaperSource(src) {
  if (!state.settings) state.settings = {};
  if (!state.settings.new_tab) state.settings.new_tab = {};
  saveSetting('new_tab_wallpaper_source', src);
  state.settings.new_tab.wallpaper_source = src;
  state.settings.new_tab.show_background = src !== 'none';
  const nt = newtabSettings();
  window.__ntState = nt;
  syncWallpaperSourceUI(src, nt);
  if (src === 'upload' && !newtabWallpaperData) {
    setNewtabBg('');
    ntPickWallpaperFile();
    return;
  }
  applyNewtabSettings();
}

function syncWallpaperSourceUI(src, nt) {
  document.querySelectorAll('.nt-wp-src-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.src === src);
  });
  const urlRow = document.getElementById('nt-wp-url-row');
  const uploadRow = document.getElementById('nt-wp-upload-row');
  const colorRow = document.getElementById('nt-wp-color-row');
  if (urlRow) urlRow.style.display = src === 'url' ? 'block' : 'none';
  if (uploadRow) uploadRow.style.display = src === 'upload' ? 'flex' : 'none';
  if (colorRow) colorRow.style.display = src === 'color' ? 'flex' : 'none';
  setUploadPreview(newtabWallpaperData);
  if (src === 'url' && nt && nt.wallpaper_url) {
    const inp = document.getElementById('nt-wp-url-input');
    if (inp) inp.value = nt.wallpaper_url;
  }
  if (src === 'color' && nt) {
    const col = nt.wallpaper_color || '#141414';
    const inp = document.getElementById('nt-wp-color-input');
    const lbl = document.getElementById('nt-wp-color-label');
    if (inp) inp.value = col;
    if (lbl) lbl.textContent = col;
  }
}

function setUploadPreview(dataUrl) {
  const prev = document.getElementById('nt-wp-upload-preview');
  const use = document.getElementById('nt-wp-upload-use');
  const choose = document.getElementById('nt-wp-upload-choose');
  if (prev) {
    prev.src = dataUrl || '';
    prev.style.display = dataUrl ? 'block' : 'none';
  }
  if (use) use.style.display = dataUrl ? 'inline-block' : 'none';
  if (choose) choose.textContent = dataUrl ? 'Change photo' : 'Choose file';
}

function useUploadedWallpaper(showMsg) {
  if (!state.settings) state.settings = {};
  if (!state.settings.new_tab) state.settings.new_tab = {};
  const dataUrl = newtabWallpaperData || '';
  if (!dataUrl) {
    ntPickWallpaperFile();
    return;
  }
  state.settings.new_tab.wallpaper_source = 'upload';
  state.settings.new_tab.show_background = true;
  saveSetting('new_tab_wallpaper_source', 'upload');
  const nt = newtabSettings();
  window.__ntState = nt;
  syncWallpaperSourceUI('upload', nt);
  applyNewtabSettings();
  if (showMsg) toast('Wallpaper applied', 'success');
}

function ntPickWallpaperFile() {
  const inp = document.createElement('input');
  inp.type = 'file';
  inp.accept = 'image/*';
  inp.onchange = function() {
    const file = inp.files[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = function(e) {
      const dataUrl = e.target.result;
      ntSaveUploadedWallpaper(dataUrl, ok => {
        if (!ok) {
          toast('Could not save wallpaper', 'error');
          return;
        }
        useUploadedWallpaper(false);
        toast('Wallpaper saved', 'success');
      });
    };
    reader.onerror = () => toast('Could not read image', 'error');
    reader.readAsDataURL(file);
  };
  inp.click();
}

function ntSaveUploadedWallpaper(dataUrl, cb) {
  try {
    newtabWallpaperData = dataUrl;
    saveSetting('new_tab_wallpaper_data', dataUrl);
    cb(true);
  } catch(err) { cb(false); }
}

let _ntClockTick = null;
function startNewtabClock() {
  if (_ntClockTick) return;
  function tick() {
    const el = document.getElementById('newtab-clock');
    if (!el) { _ntClockTick = null; return; }
    const now = new Date();
    const h = now.getHours();
    const m = String(now.getMinutes()).padStart(2, '0');
    el.textContent = `${h}:${m}`;
    _ntClockTick = setTimeout(tick, (60 - now.getSeconds()) * 1000);
  }
  tick();
}

function openSuggestions(target) {
  if (!searchSuggestionsEnabled()) return;
  clearTimeout(suggestionHideTimer);
  activeSuggestionTarget = target;
  activeSuggestionIndex = -1;
  renderSuggestions(target, getSuggestionInput(target).value);
}

function scheduleSuggestionClose() {
  clearTimeout(suggestionHideTimer);
  suggestionHideTimer = setTimeout(hideSuggestions, 120);
}

function hideSuggestions() {
  clearTimeout(suggestionHideTimer);
  suggestionHideTimer = null;
  activeSuggestionTarget = null;
  activeSuggestionIndex = -1;
  activeSuggestions = [];
  document.getElementById('url-suggestions').classList.remove('open');
  document.getElementById('newtab-suggestions').classList.remove('open');
  syncSuggestionOverlay(null);
}

function renderSuggestionPanels() {
  if (!activeSuggestionTarget) return;
  const input = getSuggestionInput(activeSuggestionTarget);
  const query = activeSuggestionTarget === 'url' && input && input.dataset.showingCurrent === '1'
    ? ''
    : input.value;
  renderSuggestions(activeSuggestionTarget, query);
}

function renderSuggestions(target, rawQuery) {
  if (!searchSuggestionsEnabled()) {
    hideSuggestions();
    return;
  }
  activeSuggestionTarget = target;
  activeSuggestionIndex = -1;
  const input = getSuggestionInput(target);
  const panel = document.getElementById(target === 'newtab' ? 'newtab-suggestions' : 'url-suggestions');
  if (!input || !panel) return;
  const query = (rawQuery || '').trim();
  activeSuggestions = buildSuggestions(query);
  if (!activeSuggestions.length) {
    panel.classList.remove('open');
    panel.innerHTML = '';
    syncSuggestionOverlay(null);
    return;
  }
  panel.innerHTML = renderSuggestionGroups(activeSuggestions);
  panel.querySelectorAll('.suggestion-item').forEach((el, index) => {
    el.addEventListener('mousedown', ev => {
      ev.preventDefault();
      chooseSuggestion(index);
    });
  });
  if (target === 'url') positionUrlSuggestions(panel);
  panel.classList.add('open');
  syncSuggestionOverlay(panel);
}

function positionUrlSuggestions(panel) {
  const rect = document.getElementById('address-bar').getBoundingClientRect();
  panel.style.left = rect.left + 'px';
  panel.style.width = rect.width + 'px';
}

function syncSuggestionOverlay(panel) {
  if (!panel || !panel.classList.contains('open')) {
    send('SuggestionOverlay', {visible: false, x: 0, y: 0, width: 0, height: 0});
    return;
  }
  const rect = panel.getBoundingClientRect();
  send('SuggestionOverlay', {
    visible: true,
    x: rect.left,
    y: rect.top,
    width: rect.width,
    height: rect.height
  });
}

function refreshSuggestionOverlayBounds() {
  const workspaceModal = document.getElementById('workspace-modal');
  if (workspaceModal && workspaceModal.classList.contains('open')) {
    send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
    return;
  }
  if (!activeSuggestionTarget) return;
  const panel = document.getElementById(activeSuggestionTarget === 'newtab' ? 'newtab-suggestions' : 'url-suggestions');
  if (activeSuggestionTarget === 'url' && panel && panel.classList.contains('open')) positionUrlSuggestions(panel);
  syncSuggestionOverlay(panel);
}

function buildSuggestions(query) {
  const q = query.toLowerCase();
  const items = [];
  const defaultEngine = getDefaultEngine();
  if (query) {
    if (looksLikeNavigableUrl(query)) {
      items.push({
        group: 'Recommendation',
        title: `Go to ${query}`,
        sub: 'Open address',
        url: query,
        icon: 'globe'
      });
    }
    items.push({
      group: 'Recommendation',
      title: `Search ${defaultEngine ? defaultEngine.name : 'the web'} for "${query}"`,
      sub: 'Press Enter to search',
      url: query,
      icon: 'search',
      kbd: 'Enter'
    });
  } else {
    items.push({
      group: 'Recommendation',
      title: 'Search the web',
      sub: defaultEngine ? `${defaultEngine.name} is your default search engine` : 'Type a query and press Enter',
      url: '',
      icon: 'search'
    });
  }

  if (!query && trendingEnabled()) {
    items.push(...trendingSearches.map(term => ({
      group: 'Trending searches',
      title: term,
      sub: defaultEngine ? `Search with ${defaultEngine.name}` : 'Search the web',
      url: term,
      icon: 'search'
    })));
  }

  const recentSearches = uniqueRecentSearches()
    .filter(s => !q || s.query.toLowerCase().includes(q))
    .slice(0, 4)
    .map(s => ({
      group: 'Recent searches',
      title: s.query,
      sub: siteDomain(s.url),
      url: s.query,
      icon: 'clock'
    }));
  items.push(...recentSearches);

  // Split history into prefix matches (URL starts with query) vs. full-text matches.
  // Prefix matches go before "Search for…" and "Go to…" for instant navigation.
  const allHistory = uniqueHistory().filter(h =>
    !q || (h.title || '').toLowerCase().includes(q) || (h.url || '').toLowerCase().includes(q)
  );
  const strippedQ = q.replace(/^https?:\/\/(www\.)?/, '');
  const prefixHits = q ? allHistory.filter(h => {
    const bare = (h.url || '').replace(/^https?:\/\/(www\.)?/, '').toLowerCase();
    return bare.startsWith(strippedQ);
  }).slice(0, 3) : [];
  const prefixUrls = new Set(prefixHits.map(h => h.url));
  const restHits = allHistory.filter(h => !prefixUrls.has(h.url)).slice(0, 5);

  if (prefixHits.length) {
    items.unshift(...prefixHits.map(h => ({
      group: 'Best match',
      title: h.title || friendlySiteName(h.url, h.title),
      sub: siteDomain(h.url),
      url: h.url,
      icon: 'globe'
    })));
  }
  const recentSites = restHits.map(h => ({
    group: recentSearches.length ? 'Recent pages' : 'Recent',
    title: h.title || friendlySiteName(h.url, h.title),
    sub: siteDomain(h.url),
    url: h.url,
    icon: 'clock'
  }));
  items.push(...recentSites);

  if (!query) {
    items.push(...(state.search_engines || []).slice(0, 4).map(engine => ({
      group: 'Search engines',
      title: engine.name,
      sub: engine.shortcut ? `Shortcut ${engine.shortcut}` : 'Search provider',
      url: engineHomeUrl(engine),
      icon: 'search'
    })));
  }
  return items.filter(item => item.url || item.group === 'Recommendation');
}

function renderSuggestionGroups(items) {
  let currentGroup = '';
  return items.map((item, index) => {
    const groupHtml = item.group !== currentGroup
      ? (currentGroup = item.group, `<div class="suggestion-section">${escHtml(item.group)}</div>`)
      : '';
    return `${groupHtml}
      <div class="suggestion-item" data-index="${index}">
        <div class="suggestion-item-icon">${item.icon === 'clock' ? clockIconSvg() : searchIconSvg()}</div>
        <div class="suggestion-item-info">
          <div class="suggestion-item-title">${escHtml(item.title)}</div>
          <div class="suggestion-item-sub">${escHtml(item.sub || '')}</div>
        </div>
        ${item.kbd ? `<div class="suggestion-item-kbd">${escHtml(item.kbd)}</div>` : ''}
      </div>`;
  }).join('');
}

function handleSuggestionKey(e, target) {
  const panel = document.getElementById(target === 'newtab' ? 'newtab-suggestions' : 'url-suggestions');
  if (!panel || !panel.classList.contains('open')) return false;
  if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
    e.preventDefault();
    const max = activeSuggestions.length - 1;
    activeSuggestionIndex = e.key === 'ArrowDown'
      ? Math.min(activeSuggestionIndex + 1, max)
      : Math.max(activeSuggestionIndex - 1, 0);
    updateSuggestionHighlight(panel);
    return true;
  }
  if (e.key === 'Enter' && activeSuggestionIndex >= 0) {
    e.preventDefault();
    chooseSuggestion(activeSuggestionIndex);
    return true;
  }
  if (e.key === 'Escape') {
    hideSuggestions();
    return false;
  }
  return false;
}

function updateSuggestionHighlight(panel) {
  panel.querySelectorAll('.suggestion-item').forEach((el, i) => {
    el.classList.toggle('highlighted', i === activeSuggestionIndex);
  });
}

function chooseSuggestion(index) {
  const item = activeSuggestions[index];
  if (!item) return;
  navigateWithSuggestion(item.url);
}

function navigateWithSuggestion(url) {
  if (!url) {
    getSuggestionInput(activeSuggestionTarget || 'url').focus();
    return;
  }
  send('Navigate', {url});
  const input = getSuggestionInput(activeSuggestionTarget || 'url');
  if (input) input.value = '';
  hideSuggestions();
}

function getSuggestionInput(target) {
  return document.getElementById(target === 'newtab' ? 'newtab-input' : 'url-input');
}

function getDefaultEngine() {
  const engines = state.search_engines || [];
  return engines.find(e => e.is_default)
    || engines.find(e => e.id === (state.settings && state.settings.search && state.settings.search.default_engine))
    || engines[0];
}

function uniqueHistory() {
  const seen = new Set();
  return (state.history || []).filter(h => {
    if (!h || !h.url || h.url.startsWith('neura://')) return false;
    if (seen.has(h.url)) return false;
    seen.add(h.url);
    return true;
  });
}

function uniqueRecentSearches() {
  const seen = new Set();
  const searches = [];
  for (const h of uniqueHistory()) {
    const query = extractSearchQuery(h.url);
    if (!query || seen.has(query.toLowerCase())) continue;
    seen.add(query.toLowerCase());
    searches.push({query, url: h.url});
  }
  return searches;
}

function extractSearchQuery(url) {
  try {
    const u = new URL(url);
    const host = u.hostname.replace(/^www\./, '');
    if (host.includes('google.') || host.includes('bing.') || host.includes('brave.com') || host.includes('duckduckgo.com')) {
      return u.searchParams.get('q') || '';
    }
    if (host.includes('perplexity.ai')) {
      return u.searchParams.get('q') || u.searchParams.get('query') || '';
    }
  } catch {}
  return '';
}

function looksLikeNavigableUrl(value) {
  return /^https?:\/\//i.test(value) || /^localhost(:|\/|$)/i.test(value) || /^\d{1,3}(\.\d{1,3}){3}(:|\/|$)/.test(value) || (!value.includes(' ') && value.includes('.'));
}

function engineHomeUrl(engine) {
  try {
    const u = new URL(engine.url_template.replace('{query}', ''));
    return `${u.protocol}//${u.hostname}/`;
  } catch {
    return engine.url_template.replace('{query}', '');
  }
}

function searchIconSvg() {
  return '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>';
}

function clockIconSvg() {
  return '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/></svg>';
}

// ============================================================
// AI SIDEBAR
// ============================================================
let _providerDdOpen = false;
function toggleProviderDd(e) {
  e.stopPropagation();
  _providerDdOpen = !_providerDdOpen;
  if (_providerDdOpen) {
    // Position the fixed menu directly under the button using bounding-rect coords
    const btn = document.querySelector('.ai-provider-dd-btn');
    const menu = document.getElementById('ai-provider-dd-menu');
    if (btn && menu) {
      const r = btn.getBoundingClientRect();
      menu.style.top = (r.bottom + 6) + 'px';
      menu.style.right = (window.innerWidth - r.right) + 'px';
      menu.style.left = 'auto';
    }
  }
  document.getElementById('ai-provider-dd').classList.toggle('open', _providerDdOpen);
}
function selectProviderDd(provider) {
  _providerDdOpen = false;
  document.getElementById('ai-provider-dd').classList.remove('open');
  aiProviderChange(provider);
}
function updateProviderDdUI(provider) {
  const label = document.getElementById('ai-provider-dd-label');
  if (label) label.textContent = providerLabel(provider);
  document.querySelectorAll('.ai-provider-dd-item').forEach(el => {
    el.classList.toggle('active', el.dataset.provider === provider);
  });
}
document.addEventListener('click', function(e) {
  if (_providerDdOpen && !document.getElementById('ai-provider-dd').contains(e.target)) {
    _providerDdOpen = false;
    document.getElementById('ai-provider-dd').classList.remove('open');
  }
});
function aiProviderChange(v) {
  state.ai_provider = v;
  if (!state.settings) state.settings = {};
  if (!state.settings.ai) state.settings.ai = {};
  state.settings.ai.default_provider = v;
  renderAiSidebar();
  send('AiProviderChange', {provider: v});
}
function aiQuickAction(action) {
  if (aiStreaming) return;
  startAiChat();
  send('AiQuickAction', {action});
  addAiMessage('user', actionLabel(action));
  showAiThinking();
  aiStreaming = true;
  document.getElementById('ai-send-btn').disabled = true;
}
function actionLabel(a) {
  return {summarize:'Create a summary of this page',explain:'Expand on this topic',key_points:'Pull out the key points',ask_anything:'What can you help me with on this page?'}[a]||a;
}
function providerLabel(p) {
  return {anthropic:'Anthropic',openai:'OpenAI',gemini:'Gemini',openrouter:'OpenRouter',ollama:'Ollama'}[p] || 'AI';
}
function renderAiSidebar() {
  const ai = state.settings && state.settings.ai ? state.settings.ai : {};
  const provider = ai.default_provider || state.ai_provider || 'openai';
  const model = ai.default_model || 'Smart';
  const status = state.ai_key_status || {};
  const saved = provider === 'ollama' || !!status[provider];
  state.ai_provider = provider;
  updateProviderDdUI(provider);
  const dot = document.getElementById('ai-key-dot');
  const text = document.getElementById('ai-key-text');
  const pill = document.getElementById('ai-model-pill');
  const page = document.getElementById('ai-page-title');
  if (dot) dot.classList.toggle('ok', saved);
  if (text) text.textContent = saved ? providerLabel(provider) + ' key saved locally' : 'Add ' + providerLabel(provider) + ' key locally';
  if (pill) pill.textContent = model.length > 18 ? model.slice(0, 16) + '..' : model;
  if (page) page.textContent = state.active_title || state.active_url || 'Current page';
}
// ============================================================
// MODEL PICKER MODAL
// ============================================================
const PROVIDER_MODELS = {
  anthropic: [
    {id:'claude-opus-4-7',       name:'Claude 4.7 Opus',      ctx:'1M',   tags:['flagship','tools']},
    {id:'claude-sonnet-4-6',     name:'Claude 4.6 Sonnet',    ctx:'1M',   tags:['tools']},
    {id:'claude-haiku-4-5-20251001', name:'Claude 4.5 Haiku', ctx:'200K', tags:['fast','tools']},
    {id:'claude-3-7-sonnet-20250219', name:'Claude 3.7 Sonnet',ctx:'200K',tags:['tools']},
    {id:'claude-3-5-sonnet-20241022', name:'Claude 3.5 Sonnet',ctx:'200K',tags:['tools']},
    {id:'claude-3-5-haiku-20241022',  name:'Claude 3.5 Haiku', ctx:'200K',tags:['fast','tools']},
    {id:'claude-3-opus-20240229',     name:'Claude 3 Opus',    ctx:'200K', tags:['tools']},
  ],
  openai: [
    {id:'gpt-5.5',     name:'GPT-5.5',          ctx:'1M',   tags:['flagship','tools']},
    {id:'gpt-5.4',     name:'GPT-5.4',          ctx:'1M',   tags:['tools']},
    {id:'gpt-5.4-mini',name:'GPT-5.4 Mini',     ctx:'400K', tags:['fast','tools']},
    {id:'gpt-4.1',     name:'GPT-4.1',          ctx:'1M',   tags:['tools']},
    {id:'gpt-4o',      name:'GPT-4o',           ctx:'128K', tags:['tools']},
    {id:'gpt-4o-mini', name:'GPT-4o Mini',      ctx:'128K', tags:['fast','tools']},
  ],
  gemini: [
    {id:'gemini-3.5-flash',    name:'Gemini 3.5 Flash',     ctx:'1M',  tags:['fast','tools']},
    {id:'gemini-3.1-pro',      name:'Gemini 3.1 Pro',       ctx:'2M',  tags:['flagship','tools']},
    {id:'gemini-3.1-flash-lite',name:'Gemini 3.1 Flash Lite',ctx:'1M', tags:['fast']},
    {id:'gemini-2.5-pro',      name:'Gemini 2.5 Pro',       ctx:'1M',  tags:['tools']},
    {id:'gemini-2.5-flash',    name:'Gemini 2.5 Flash',     ctx:'1M',  tags:['fast','tools']},
    {id:'gemini-1.5-pro',      name:'Gemini 1.5 Pro',       ctx:'2M',  tags:['tools']},
    {id:'gemini-1.5-flash',    name:'Gemini 1.5 Flash',     ctx:'1M',  tags:['fast']},
  ],
  openrouter: [
    {id:'meta-llama/llama-3.3-70b-instruct', name:'Llama 3.3 70B',      ctx:'128K', tags:['tools']},
    {id:'deepseek/deepseek-r1',              name:'DeepSeek R1',         ctx:'64K',  tags:['flagship']},
    {id:'mistralai/mistral-large-2411',      name:'Mistral Large',       ctx:'128K', tags:['tools']},
    {id:'qwen/qwq-32b',                      name:'QwQ 32B',             ctx:'32K',  tags:[]},
    {id:'x-ai/grok-3',                       name:'Grok 3',              ctx:'131K', tags:['tools']},
    {id:'google/gemini-2.5-flash',           name:'Gemini 2.5 Flash (OR)',ctx:'1M',  tags:['fast','tools']},
    {id:'anthropic/claude-3.5-sonnet',       name:'Claude 3.5 Sonnet (OR)',ctx:'200K',tags:['tools']},
    {id:'openai/gpt-4o',                     name:'GPT-4o (OR)',         ctx:'128K', tags:['tools']},
    {id:'liquid/lfm-40b',                    name:'LFM 40B',             ctx:'32K',  tags:[]},
  ],
  ollama: [], // populated dynamically
};
const PROVIDER_LABELS = {
  anthropic:'Anthropic', openai:'OpenAI', gemini:'Gemini',
  openrouter:'OpenRouter', ollama:'Ollama'
};
let mmActiveProvider = null;
let ollamaModelsFetched = false;

function openModelModal(e) {
  if (e) e.stopPropagation();
  const ai = (state.settings && state.settings.ai) || {};
  mmActiveProvider = ai.default_provider || 'anthropic';
  renderModalProviders();
  renderModalModels(mmActiveProvider);
  document.getElementById('mm-custom-input').value = '';
  document.getElementById('model-modal').classList.add('open');
}
function closeModelModal() {
  document.getElementById('model-modal').classList.remove('open');
}
function handleModelModalBg(e) {
  if (e.target.id === 'model-modal') closeModelModal();
}
function renderModalProviders() {
  const container = document.getElementById('mm-providers');
  const providers = ['anthropic','openai','gemini','openrouter','ollama'];
  container.innerHTML = providers.map(p =>
    `<button class="mm-tab${p === mmActiveProvider ? ' active' : ''}" onclick="switchModalProvider('${p}')">${PROVIDER_LABELS[p]}</button>`
  ).join('');
}
function switchModalProvider(provider) {
  mmActiveProvider = provider;
  renderModalProviders();
  renderModalModels(provider);
}
function renderModalModels(provider) {
  const ai = (state.settings && state.settings.ai) || {};
  const currentProvider = ai.default_provider || '';
  const currentModel = ai.default_model || '';
  const listEl = document.getElementById('mm-models');

  if (provider === 'ollama' && !ollamaModelsFetched) {
    listEl.innerHTML = '<div class="mm-loading">Fetching local models...</div>';
    fetchOllamaModels().then(() => {
      ollamaModelsFetched = true;
      if (mmActiveProvider === 'ollama') renderModalModels('ollama');
    }).catch(() => {
      ollamaModelsFetched = true;
      if (mmActiveProvider === 'ollama') renderModalModels('ollama');
    });
    return;
  }

  const models = PROVIDER_MODELS[provider] || [];
  if (!models.length) {
    listEl.innerHTML = '<div class="mm-loading">No models found. Is Ollama running?</div>';
    return;
  }

  listEl.innerHTML = models.map(m => {
    const isSelected = provider === currentProvider && m.id === currentModel;
    const tags = (m.tags || []).map(t => `<span class="mm-tag ${t}">${tagLabel(t)}</span>`).join('');
    return `<div class="mm-model${isSelected ? ' selected' : ''}" onclick="selectModel('${provider}','${m.id.replace(/'/g,"\\'")}','${m.name.replace(/'/g,"\\'")}')">
      <div class="mm-model-dot"></div>
      <div class="mm-model-info">
        <div class="mm-model-name">${m.name}</div>
        <div class="mm-model-meta"><span class="mm-tag">${m.ctx || '?'}</span>${tags}</div>
      </div>
    </div>`;
  }).join('');
}
function tagLabel(t) {
  return {flagship:'★ Flagship', fast:'⚡ Fast', tools:'🔧 Tools'}[t] || t;
}
async function fetchOllamaModels() {
  try {
    const ai = (state.settings && state.settings.ai) || {};
    const base = (ai.ollama_url || 'http://localhost:11434').replace(/\/$/, '');
    const resp = await fetch(base + '/api/tags');
    const data = await resp.json();
    const models = (data.models || []).map(m => ({
      id: m.name,
      name: m.name,
      ctx: m.details && m.details.parameter_size ? m.details.parameter_size : '',
      tags: [],
    }));
    PROVIDER_MODELS.ollama = models;
  } catch(e) {
    PROVIDER_MODELS.ollama = [];
  }
}
function selectModel(provider, modelId, modelName) {
  const ai = (state.settings && state.settings.ai) || {};
  const prevProvider = ai.default_provider || '';
  if (provider !== prevProvider) {
    send('AiProviderChange', {provider});
  }
  send('AiModelChange', {model: modelId});
  // Optimistically update local state for immediate UI feedback
  if (!state.settings) state.settings = {};
  if (!state.settings.ai) state.settings.ai = {};
  state.settings.ai.default_provider = provider;
  state.settings.ai.default_model = modelId;
  const pill = document.getElementById('ai-model-pill');
  if (pill) pill.textContent = modelName.length > 18 ? modelName.slice(0, 16) + '..' : modelName;
  closeModelModal();
}
function applyCustomModel() {
  const input = document.getElementById('mm-custom-input');
  const id = input.value.trim();
  if (!id) return;
  const ai = (state.settings && state.settings.ai) || {};
  send('AiModelChange', {model: id});
  if (!state.settings) state.settings = {};
  if (!state.settings.ai) state.settings.ai = {};
  state.settings.ai.default_model = id;
  const pill = document.getElementById('ai-model-pill');
  if (pill) pill.textContent = id.length > 18 ? id.slice(0, 16) + '..' : id;
  closeModelModal();
}

function startAiChat() {
  const sidebar = document.getElementById('ai-sidebar');
  const empty = document.querySelector('#ai-messages .ai-empty');
  if (sidebar) sidebar.classList.add('ai-chatting');
  if (empty) empty.remove();
}
function handleAiKey(e) {
  if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendAiMessage(); }
}
function sendAiMessage() {
  const input = document.getElementById('ai-input');
  const text = input.value.trim();
  if (!text || aiStreaming) return;
  startAiChat();
  addAiMessage('user', text);
  input.value = '';
  showAiThinking();
  aiStreaming = true;
  document.getElementById('ai-send-btn').disabled = true;
  send('AiMessage', {text});
}
// Minimal markdown renderer for AI responses: bold, italic, code, line breaks
function renderAiMarkdown(text) {
  // Escape HTML entities first
  let s = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  // Code blocks (```...```)
  s = s.replace(/```([\s\S]*?)```/g, (_, code) =>
    '<pre style="background:var(--bg);border:1px solid var(--border);border-radius:6px;padding:8px 10px;font-size:12px;overflow-x:auto;white-space:pre-wrap;margin:4px 0"><code>' + code.trim() + '</code></pre>');
  // Inline code
  s = s.replace(/`([^`\n]+)`/g, '<code style="background:var(--bg);border:1px solid var(--border);border-radius:4px;padding:1px 5px;font-size:12px">$1</code>');
  // Bold **text** or __text__
  s = s.replace(/\*\*([^*\n]+)\*\*/g, '<strong>$1</strong>');
  s = s.replace(/__([^_\n]+)__/g, '<strong>$1</strong>');
  // Italic *text* or _text_
  s = s.replace(/\*([^*\n]+)\*/g, '<em>$1</em>');
  s = s.replace(/_([^_\n]+)_/g, '<em>$1</em>');
  // Bullet lists (lines starting with - or *)
  s = s.replace(/(^|\n)([ \t]*[-*] .+)+/g, (match) => {
    const items = match.trim().split('\n').filter(Boolean);
    return '<ul style="margin:4px 0 4px 16px;padding:0">' +
      items.map(i => '<li>' + i.replace(/^[ \t]*[-*] /, '') + '</li>').join('') +
      '</ul>';
  });
  // Newlines → line breaks (after block elements are handled)
  s = s.replace(/\n\n/g, '<br><br>').replace(/\n/g, '<br>');
  return s;
}
function addAiMessage(role, text) {
  startAiChat();
  const msgs = document.getElementById('ai-messages');
  const el = document.createElement('div');
  el.className = `ai-msg ${role}`;
  if (role === 'assistant') {
    el.innerHTML = renderAiMarkdown(text);
  } else {
    el.textContent = text;
  }
  msgs.appendChild(el);
  scrollAiToBottom();
}
function showAiThinking() {
  startAiChat();
  const msgs = document.getElementById('ai-messages');
  const el = document.createElement('div');
  el.className = 'ai-thinking';
  el.innerHTML = '<div class="ai-dot"></div><div class="ai-dot"></div><div class="ai-dot"></div>';
  msgs.appendChild(el);
  scrollAiToBottom();
}
function finishAiBusy() {
  const thinking = document.querySelector('#ai-messages .ai-thinking');
  if (thinking) thinking.remove();
  aiStreaming = false;
  const btn = document.getElementById('ai-send-btn');
  if (btn) btn.disabled = false;
}
function scrollAiToBottom() {
  const msgs = document.getElementById('ai-messages');
  msgs.scrollTop = msgs.scrollHeight;
}
function aiClear() {
  document.getElementById('ai-messages').innerHTML = '<div class="ai-empty"></div>';
  const sidebar = document.getElementById('ai-sidebar');
  if (sidebar) sidebar.classList.remove('ai-chatting');
  currentStreamEl = null;
  finishAiBusy();
  send('AiClearChat');
}

// ============================================================
// REGION SETTINGS
// ============================================================
function countryFlag(code) {
  if (!code || code.length !== 2) return '';
  const A = 0x1F1E6;
  return String.fromCodePoint(A + code.charCodeAt(0) - 65) +
         String.fromCodePoint(A + code.charCodeAt(1) - 65);
}

function onRegionChange(code) {
  saveSetting('region', code);
}

function populateRegionSettings() {
  const select = document.getElementById('set-region');
  if (!select) return;
  const current = (state.settings && state.settings.region) || '';
  const sorted = Object.entries(_COUNTRY_DATA).sort((a,b) => a[1].name.localeCompare(b[1].name));
  select.innerHTML = '<option value="">Not set</option>' +
    sorted.map(([code, c]) =>
      `<option value="${escAttr(code)}"${code === current ? ' selected' : ''}>${countryFlag(code)} ${escHtml(c.name)}</option>`
    ).join('');
}

// ============================================================
// SETTINGS
// ============================================================
function openSettings(section='general') {
  send('OpenSettings');
  document.getElementById('settings-overlay').classList.add('open');
  populateSettingsPanel();
  switchSettings(section);
}
function closeSettings() {
  send('CloseSettings');
  document.getElementById('settings-overlay').classList.remove('open');
}
function handleSettingsOverlayClick(e) {
  if (e.target.id === 'settings-overlay') closeSettings();
}
function switchSettings(sec) {
  document.querySelectorAll('.settings-section').forEach(el => el.classList.remove('active'));
  document.querySelectorAll('.settings-nav-item').forEach(el => el.classList.remove('active'));
  const s = document.getElementById('section-' + sec);
  if (s) s.classList.add('active');
  const n = document.querySelector(`.settings-nav-item[data-section="${sec}"]`);
  if (n) n.classList.add('active');
  if (sec === 'history') {
    const searchEl = document.getElementById('history-search');
    const q = searchEl ? searchEl.value : '';
    send('GetHistory', {q});
  }
  if (sec === 'bookmarks') renderBookmarks();
  if (sec === 'downloads') renderDownloads();
  if (sec === 'general') populateRegionSettings();
}
function toggleSetting(key) {
  const el = document.getElementById('toggle-' + key.replace(/_/g,'-'));
  if (el) el.classList.toggle('on');
  const val = el ? el.classList.contains('on') : null;
  send('SaveSettings', {key, value: val});
  if (key === 'show_tab_url') {
    if (!state.settings) state.settings = {};
    if (!state.settings.appearance) state.settings.appearance = {};
    state.settings.appearance.show_tab_url = val;
    document.getElementById('app').classList.toggle('hide-tab-url', !val);
  }
  if (key === 'compact_tabs') {
    if (!state.settings) state.settings = {};
    if (!state.settings.tabs) state.settings.tabs = {};
    state.settings.tabs.compact_tabs = val;
    document.getElementById('app').classList.toggle('compact-tabs', !!val);
  }
  if (key === 'ask_download') {
    if (!state.settings) state.settings = {};
    if (!state.settings.downloads) state.settings.downloads = {};
    state.settings.downloads.ask_where_to_save = val;
    updateGeneralSettingsReadout();
  }
  if (key === 'show_bookmarks_bar') {
    if (!state.settings) state.settings = {};
    if (!state.settings.appearance) state.settings.appearance = {};
    state.settings.appearance.show_bookmarks_bar = val;
    applySidebarMode();
  }
  if (key === 'search_suggestions') {
    if (!state.settings) state.settings = {};
    if (!state.settings.search) state.settings.search = {};
    state.settings.search.suggestions_enabled = val;
    if (!val) hideSuggestions();
  }
  if (key === 'trending') {
    if (!state.settings) state.settings = {};
    if (!state.settings.search) state.settings.search = {};
    state.settings.search.trending_enabled = val;
    renderSuggestionPanels();
  }
  if (key === 'secure_dns_enabled') {
    if (!state.settings) state.settings = {};
    if (!state.settings.privacy) state.settings.privacy = {};
    state.settings.privacy.secure_dns_enabled = val;
    syncSecureDnsSettingsUI();
  }
  if (key.startsWith('new_tab_')) {
    if (!state.settings) state.settings = {};
    if (!state.settings.new_tab) state.settings.new_tab = {};
    const name = key.replace('new_tab_', '');
    state.settings.new_tab[name] = val;
    applyNewtabSettings();
    renderNewtabShortcuts();
    renderNeuraFeed();
  }
}
function setTheme(t) {
  obTheme = t;
  document.querySelectorAll('.theme-card').forEach(c => c.classList.remove('selected'));
  const el = document.getElementById('theme-' + t);
  if (el) el.classList.add('selected');
  document.documentElement.setAttribute('data-theme', t === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : t);
  send('SaveSettings', {key: 'theme', value: t});
}
function checkForUpdate() {
  __manualUpdateCheck = true;
  showUpdateModal({status: 'checking'});
  send('CheckForUpdate');
}
function installUpdate() {
  __manualUpdateCheck = true;
  showUpdateModal({status: 'downloading', received: 0, total: 0});
  send('InstallUpdate');
}

let spotlightOpen = false;
let spotlightIdx = -1;
let spotlightUrls = [];
let tspAiMode = false;
let tspAiStreaming = false;
let _tspAiRawText = '';

function hasAiKey() {
  // true if any provider has a key configured, or Ollama is selected (no key needed)
  if (!state) return false;
  const provider = state.settings?.ai?.default_provider || '';
  if (provider === 'ollama') return true;
  const ks = state.ai_key_status || {};
  return !!(ks.openai || ks.anthropic || ks.gemini || ks.openrouter);
}

function tspEnterAiMode(query) {
  if (!hasAiKey()) return;
  tspAiMode = true;
  tspAiStreaming = true;
  _tspAiRawText = '';
  const panel = document.getElementById('tsp-ai-panel');
  const results = document.getElementById('tsp-results');
  const content = document.getElementById('tsp-ai-content');
  if (results) results.style.display = 'none';
  if (panel) panel.classList.add('visible');
  if (content) {
    content.innerHTML = '<div class="tsp-ai-dots"><span></span><span></span><span></span></div>';
  }
  send('SpotlightAiQuery', {text: query});
}

function tspExitAiMode() {
  tspAiMode = false;
  tspAiStreaming = false;
  _tspAiRawText = '';
  const panel = document.getElementById('tsp-ai-panel');
  const results = document.getElementById('tsp-results');
  if (panel) panel.classList.remove('visible');
  if (results) results.style.display = '';
}

function openNewTabSpotlight() {
  if (spotlightOpen) { closeSpotlight(); } else { openSpotlight(); }
}

function showSpotlight() {
  if (spotlightOpen) return;
  spotlightOpen = true;
  document.getElementById('tab-spotlight-overlay').classList.add('open');
  renderTspSuggestions('');
  setTimeout(() => {
    const inp = document.getElementById('tsp-input');
    if (inp) { inp.value = ''; inp.focus(); }
  }, 30);
}

function openSpotlight() {
  if (spotlightOpen) return;
  send('BeginSpotlight');
  showSpotlight();
}

function hideSpotlight() {
  if (!spotlightOpen) return;
  spotlightOpen = false;
  tspExitAiMode();
  document.getElementById('tab-spotlight-overlay').classList.remove('open');
}

function closeSpotlight() {
  if (!spotlightOpen) return;
  send('EndSpotlight');
  hideSpotlight();
}

function spotlightNavigate(url) {
  if (!url) return;
  closeSpotlight();
  send('NewTab');
  send('Navigate', {url});
}

function tspFavicon(url) {
  try {
    const h = new URL(url).hostname;
    return h ? 'https://www.google.com/s2/favicons?domain=' + encodeURIComponent(h) + '&sz=32' : '';
  } catch { return ''; }
}

function tspDisplayUrl(url) {
  try { const u = new URL(url); return u.hostname + (u.pathname.length > 1 ? u.pathname.slice(0, 60) : ''); }
  catch { return url; }
}

function tspRow(title, url, favicon, isSearch) {
  const ico = favicon
    ? `<img src="${escAttr(favicon)}" alt="" width="16" height="16" onerror="this.style.display='none'">`
    : isSearch
      ? `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>`
      : `<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/></svg>`;
  const sub = (url && url !== title) ? `<div class="tsp-row-sub">${escHtml(tspDisplayUrl(url))}</div>` : '';
  return `<div class="tsp-row" onclick="spotlightNavigate(this.dataset.u)" data-u="${escAttr(url)}">
    <div class="tsp-row-icon">${ico}</div>
    <div class="tsp-row-body"><div class="tsp-row-title">${escHtml(title)}</div>${sub}</div>
  </div>`;
}

// ── Spotlight calculator ───────────────────────────────────────────────────────

/** Returns true if the query looks like a pure math expression (no plain words). */
function isMathExpr(q) {
  const s = q.replace(/\s+/g, '').toLowerCase();
  if (!s) return false;
  // Must contain at least one digit
  if (!/[0-9]/.test(s)) return false;
  // Must contain an operator or function name
  if (!/[+\-*\/^%]|sin|cos|tan|sqrt|log|ln|abs|pi/.test(s)) return false;
  // Strip known math tokens; remainder must be only digits/operators/parens/dots
  const stripped = s
    .replace(/(asin|acos|atan|sinh|cosh|tanh|sin|cos|tan|sqrt|log|ln|abs|floor|ceil|round|pi|pow|max|min|e(?=[^a-z]))/g, '')
    .replace(/[0-9+\-*\/^%().]/g, '');
  return stripped.length === 0;
}

/** Safely evaluate a math expression. Returns numeric result or null on error. */
function evaluateMath(q) {
  try {
    let expr = q.replace(/\s+/g, '').toLowerCase();
    // Normalise implicit multiplication: 2(3+4) → 2*(3+4), (3)(4) → (3)*(4)
    expr = expr.replace(/(\d)\(/g, '$1*(').replace(/\)(\d)/g, ')*$1').replace(/\)\(/g, ')*(');
    // Exponentiation: ^ → **
    expr = expr.replace(/\^/g, '**');
    // sin30 / cos45 style (no parens) → sin(30) / cos(45)
    expr = expr.replace(/(sin|cos|tan|asin|acos|atan|sqrt|log|ln|abs|ceil|floor|round)([0-9.]+)/g, '$1($2)');
    // Validate: after substitution only math chars remain
    const check = expr.replace(/[0-9+\-*\/().e%]/g, '').replace(/(sin|cos|tan|asin|acos|atan|sinh|cosh|tanh|sqrt|log|ln|abs|floor|ceil|round|pow|max|min|pi)/g, '');
    if (/[a-df-z_$]/i.test(check)) return null; // unexpected letters
    // Build safe scope: trig functions work in degrees
    const deg = Math.PI / 180;
    const result = new Function(
      'sin','cos','tan','asin','acos','atan',
      'sinh','cosh','tanh',
      'sqrt','log','ln','abs','floor','ceil','round','pow','max','min','pi','PI','E',
      '"use strict"; return (' + expr + ');'
    )(
      (d)=>Math.sin(d*deg), (d)=>Math.cos(d*deg), (d)=>Math.tan(d*deg),
      (d)=>Math.asin(d)/deg, (d)=>Math.acos(d)/deg, (d)=>Math.atan(d)/deg,
      Math.sinh, Math.cosh, Math.tanh,
      Math.sqrt, Math.log10, Math.log, Math.abs,
      Math.floor, Math.ceil, Math.round, Math.pow, Math.max, Math.min,
      Math.PI, Math.PI, Math.E
    );
    if (typeof result !== 'number' || !isFinite(result)) return null;
    // Round to avoid floating-point noise (12 sig figs)
    return parseFloat(result.toPrecision(12));
  } catch(_) { return null; }
}

/** Format a number for display: add thousands commas, trim trailing zeros. */
function fmtCalcResult(n) {
  if (Number.isInteger(n)) {
    return n.toLocaleString('en-US');
  }
  // Up to 10 decimal places, trimmed
  let s = n.toPrecision(10).replace(/\.?0+$/, '');
  const parts = s.split('.');
  parts[0] = parts[0].replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return parts.join('.');
}

// ── Spotlight unit converter ──────────────────────────────────────────────────
// Matches: <number> <from-unit> <bridge> <to-unit>
// Bridge words: to, into, in, as
// Handles plural 's'/'es', abbreviations, and multi-word unit names.

const _CONV_UNITS = (()=>{
  const L='length', W='weight', TM='time', TP='temp',
        V='volume',  A='area',   SP='speed', DT='data',
        EN='energy', PR='pressure', CU='currency';
  // [aliases[], type, factor-to-base]
  // Bases: length=meter, weight=gram, time=second, volume=liter,
  //        area=sq-meter, speed=m/s, data=byte, energy=joule,
  //        pressure=pascal, currency=USD
  // Temperature: factor is scale id string (C/F/K/R), not a numeric factor
  const groups = [
    // ─── Length ─────────────────────────────────────────────────────────────
    [['mm','millimeter','millimetre'],                   L, 0.001],
    [['cm','centimeter','centimetre'],                   L, 0.01],
    [['dm','decimeter','decimetre'],                     L, 0.1],
    [['m','meter','metre'],                              L, 1],
    [['km','kilometer','kilometre'],                     L, 1000],
    [['in','inch','inches'],                             L, 0.0254],
    [['ft','foot','feet'],                               L, 0.3048],
    [['yd','yard'],                                      L, 0.9144],
    [['mi','mile'],                                      L, 1609.344],
    [['nmi','nautical mile'],                            L, 1852],
    [['ly','light year','light-year','lightyear'],       L, 9.461e15],
    [['au','astronomical unit'],                         L, 1.496e11],
    // ─── Weight / Mass ───────────────────────────────────────────────────────
    [['mcg','ug','microgram'],                           W, 1e-6],
    [['mg','milligram'],                                 W, 0.001],
    [['g','gram'],                                       W, 1],
    [['kg','kilogram'],                                  W, 1000],
    [['t','tonne','metric ton','metric ton'],            W, 1e6],
    [['oz','ounce'],                                     W, 28.34952],
    [['lb','lbs','pound'],                               W, 453.59237],
    [['st','stone'],                                     W, 6350.29],
    [['short ton'],                                      W, 907185],
    [['long ton'],                                       W, 1016047],
    // ─── Time ───────────────────────────────────────────────────────────────
    [['ms','millisecond'],                               TM, 0.001],
    [['s','sec','second'],                               TM, 1],
    [['min','minute'],                                   TM, 60],
    [['h','hr','hour'],                                  TM, 3600],
    [['d','day'],                                        TM, 86400],
    [['wk','week'],                                      TM, 604800],
    [['mo','month'],                                     TM, 2592000],
    [['yr','year'],                                      TM, 31536000],
    [['decade'],                                         TM, 315360000],
    [['century','centuries'],                            TM, 3153600000],
    // ─── Temperature ────────────────────────────────────────────────────────
    [['c','celsius','°c','degc','deg c'],                TP, 'C'],
    [['f','fahrenheit','°f','degf','deg f'],             TP, 'F'],
    [['k','kelvin'],                                     TP, 'K'],
    [['r','rankine'],                                    TP, 'R'],
    // ─── Volume ─────────────────────────────────────────────────────────────
    [['ml','milliliter','millilitre'],                   V, 0.001],
    [['cl','centiliter','centilitre'],                   V, 0.01],
    [['dl','deciliter','decilitre'],                     V, 0.1],
    [['l','liter','litre'],                              V, 1],
    [['gal','gallon'],                                   V, 3.78541],
    [['qt','quart'],                                     V, 0.946353],
    [['pt','pint'],                                      V, 0.473176],
    [['cup'],                                            V, 0.236588],
    [['fl oz','fluid ounce'],                            V, 0.0295735],
    [['tbsp','tablespoon'],                              V, 0.0147868],
    [['tsp','teaspoon'],                                 V, 0.00492892],
    [['m3','cubic meter'],                               V, 1000],
    [['cm3','cubic centimeter','cubic centimetre','cc'], V, 0.001],
    [['ft3','cubic foot','cubic feet'],                  V, 28.3168],
    [['in3','cubic inch','cubic inches'],                V, 0.0163871],
    // ─── Area ───────────────────────────────────────────────────────────────
    [['mm2','sq mm','square mm'],                        A, 1e-6],
    [['cm2','sq cm','square cm'],                        A, 1e-4],
    [['m2','sqm','sq m','square meter','square metre'],  A, 1],
    [['km2','sq km','square km','square kilometer'],     A, 1e6],
    [['ha','hectare'],                                   A, 10000],
    [['ac','acre'],                                      A, 4046.856],
    [['sqft','sq ft','square foot','square feet'],       A, 0.092903],
    [['sqin','sq in','square inch','square inches'],     A, 6.4516e-4],
    [['sqmi','sq mi','square mile'],                     A, 2589988.11],
    // ─── Speed ──────────────────────────────────────────────────────────────
    [['m/s','mps','meter per second'],                   SP, 1],
    [['km/h','kph','kmh','kilometer per hour'],          SP, 0.277778],
    [['mph','mile per hour'],                            SP, 0.44704],
    [['knot','kn','knots'],                              SP, 0.514444],
    [['ft/s','fps','foot per second','feet per second'], SP, 0.3048],
    // ─── Digital Storage ────────────────────────────────────────────────────
    [['bit'],                                            DT, 0.125],
    [['b','byte'],                                       DT, 1],
    [['kb','kilobyte'],                                  DT, 1000],
    [['mb','megabyte'],                                  DT, 1e6],
    [['gb','gigabyte'],                                  DT, 1e9],
    [['tb','terabyte'],                                  DT, 1e12],
    [['pb','petabyte'],                                  DT, 1e15],
    [['kib','kibibyte'],                                 DT, 1024],
    [['mib','mebibyte'],                                 DT, 1048576],
    [['gib','gibibyte'],                                 DT, 1073741824],
    [['tib','tebibyte'],                                 DT, 1099511627776],
    // ─── Energy ─────────────────────────────────────────────────────────────
    [['j','joule'],                                      EN, 1],
    [['kj','kilojoule'],                                 EN, 1000],
    [['mj','megajoule'],                                 EN, 1e6],
    [['cal','calorie'],                                  EN, 4.184],
    [['kcal','kilocalorie'],                             EN, 4184],
    [['wh','watt-hour'],                                 EN, 3600],
    [['kwh','kilowatt-hour'],                            EN, 3600000],
    [['btu'],                                            EN, 1055.06],
    [['ev','electronvolt'],                              EN, 1.602e-19],
    // ─── Pressure ───────────────────────────────────────────────────────────
    [['pa','pascal'],                                    PR, 1],
    [['kpa','kilopascal'],                               PR, 1000],
    [['mpa','megapascal'],                               PR, 1e6],
    [['bar'],                                            PR, 100000],
    [['mbar','millibar'],                                PR, 100],
    [['psi'],                                            PR, 6894.76],
    [['atm','atmosphere'],                               PR, 101325],
    [['torr','mmhg'],                                    PR, 133.322],
    // ─── Currency (static approximate rates — base 1 USD) ───────────────────
    [['usd','dollar'],                                   CU, 1],
    [['eur','euro'],                                     CU, 1.087],
    [['gbp','pound sterling'],                           CU, 1.268],
    [['jpy','yen'],                                      CU, 0.00671],
    [['cny','yuan','renminbi','rmb'],                    CU, 0.138],
    [['krw','won'],                                      CU, 0.000755],
    [['aud','australian dollar'],                        CU, 0.654],
    [['cad','canadian dollar'],                          CU, 0.735],
    [['chf','swiss franc'],                              CU, 1.124],
    [['inr','rupee','indian rupee'],                     CU, 0.01204],
    [['idr','rupiah'],                                   CU, 0.0000637],
    [['sgd','singapore dollar'],                         CU, 0.746],
    [['hkd','hong kong dollar'],                         CU, 0.128],
    [['mxn'],                                            CU, 0.0582],
    [['brl','real'],                                     CU, 0.201],
    [['rub','ruble'],                                    CU, 0.01118],
    [['try','turkish lira'],                             CU, 0.0328],
    [['php'],                                            CU, 0.0177],
    [['thb','baht'],                                     CU, 0.0283],
    [['myr','ringgit'],                                  CU, 0.212],
    [['vnd','dong'],                                     CU, 0.0000396],
    [['aed','dirham'],                                   CU, 0.272],
    [['sar','riyal'],                                    CU, 0.267],
    [['nzd','new zealand dollar'],                       CU, 0.607],
    [['zar','rand'],                                     CU, 0.0543],
    [['sek','krona'],                                    CU, 0.0958],
    [['nok','krone'],                                    CU, 0.0951],
    [['pln','zloty'],                                    CU, 0.254],
    [['czk','koruna'],                                   CU, 0.0454],
    [['dkk'],                                            CU, 0.145],
    [['huf','forint'],                                   CU, 0.00284],
    [['twd','new taiwan dollar'],                        CU, 0.0312],
    [['pkr','pakistani rupee'],                          CU, 0.00358],
    [['bdt','taka'],                                     CU, 0.00909],
    [['ngn','naira'],                                    CU, 0.000637],
    [['egp','egyptian pound'],                           CU, 0.0204],
    [['kes','shilling'],                                 CU, 0.00775],
    [['btc','bitcoin'],                                  CU, 97000],
    [['eth','ethereum'],                                 CU, 3400],
  ];
  const map = Object.create(null);
  for (const [aliases, type, factor] of groups) {
    const k = aliases[0]; // canonical key = first alias
    for (const a of aliases) map[a] = {t: type, f: factor, k};
  }
  return map;
})();

/**
 * Look up a unit string, handling:
 *  • exact match   "meter"
 *  • plural -s     "meters" → "meter"
 *  • plural -es    "inches" → check without 'es' (though 'inches' is in map directly)
 *  • normalise whitespace so "sq  ft" → "sq ft"
 */
function _convLookup(raw) {
  const s = raw.toLowerCase().trim().replace(/\s+/g, ' ');
  if (_CONV_UNITS[s]) return _CONV_UNITS[s];
  // Strip trailing 's' (hours→hour, meters→meter, pounds→pound, gallons→gallon…)
  if (s.length > 2 && s.endsWith('s') && _CONV_UNITS[s.slice(0, -1)])
    return _CONV_UNITS[s.slice(0, -1)];
  // Strip trailing 'es' (catches irregular plurals not already in the map)
  if (s.length > 3 && s.endsWith('es') && _CONV_UNITS[s.slice(0, -2)])
    return _CONV_UNITS[s.slice(0, -2)];
  return null;
}

/**
 * Parse a conversion query.
 * Returns {value, fromU, toU, fromStr, toStr} or null.
 *
 * Strategy: extract the leading number, then scan for every bridge word
 * position and stop at the first one that yields two known compatible units.
 * This correctly handles cases like "sq in to sq ft" where the unit itself
 * contains a potential bridge word ("in").
 */
function _parseConversion(q) {
  const s = q.trim();
  // Leading number: integer, decimal, scientific, optional commas as thousands sep
  // \s* (not \s+) allows no space between number and unit: "32dm to cm"
  const numM = s.match(/^([+\-]?[\d,]+(?:\.\d+)?(?:[eE][+\-]?\d+)?)\s*/);
  if (!numM) return null;
  const value = parseFloat(numM[1].replace(/,/g, ''));
  if (!isFinite(value)) return null;
  const rest = s.slice(numM[0].length);
  if (!rest) return null;
  // Scan every bridge position left-to-right; stop at first valid unit pair
  const bridgeRE = /\s+(?:to|into|in|as|→|->)\s+/gi;
  let m;
  while ((m = bridgeRE.exec(rest)) !== null) {
    const fromStr = rest.slice(0, m.index);
    const toStr   = rest.slice(m.index + m[0].length);
    if (!fromStr || !toStr) continue;
    const fromU = _convLookup(fromStr);
    const toU   = _convLookup(toStr);
    if (fromU && toU && fromU.t === toU.t)
      return {value, fromU, toU, fromStr, toStr};
  }
  return null;
}

/** Perform the conversion and return a numeric result. */
function _doConvert({value, fromU, toU}) {
  if (fromU.t === 'temp') {
    // Step 1: convert to Celsius
    const c = fromU.f === 'C' ? value
            : fromU.f === 'F' ? (value - 32) * 5 / 9
            : fromU.f === 'K' ? value - 273.15
            :                   (value - 491.67) * 5 / 9; // Rankine
    // Step 2: Celsius to target
    return toU.f === 'C' ? c
         : toU.f === 'F' ? c * 9 / 5 + 32
         : toU.f === 'K' ? c + 273.15
         :                  (c + 273.15) * 9 / 5; // Rankine
  }
  if (fromU.t === 'currency' && _liveRates) {
    const fk = fromU.k.toUpperCase();
    const tk = toU.k.toUpperCase();
    const fRate = _liveRates[fk];
    const tRate = _liveRates[tk];
    if (fRate && tRate) return value * (tRate / fRate);
  }
  // Linear conversion through shared base unit
  return value * fromU.f / toU.f;
}

const _CONV_LABEL = {
  length:'Length', weight:'Weight', time:'Time', temp:'Temperature',
  volume:'Volume', area:'Area', speed:'Speed', data:'Storage',
  energy:'Energy', pressure:'Pressure', currency:'Currency'
};

// Auto-suggest targets: canonical-key → canonical-key of the best "default" target.
// Keys must be the first alias of their group (the canonical key stored in .k).
const _AUTO_SUGGEST = {
  // Length — suggest adjacent scale or imperial equivalent
  mm:'cm', cm:'m', dm:'m', m:'ft', km:'mi',
  'in':'cm', ft:'m', yd:'m', mi:'km', nmi:'km',
  // Weight
  mcg:'mg', mg:'g', g:'oz', kg:'lb', lb:'kg', oz:'g', t:'lb', st:'kg',
  // Time (h has value-dependent override in _autoSuggestConv)
  ms:'s', s:'min', min:'h', h:'min', d:'h', wk:'d', mo:'wk', yr:'mo',
  decade:'yr', century:'yr',
  // Temperature
  c:'f', f:'c', k:'c', r:'f',
  // Volume
  ml:'fl oz', cl:'l', dl:'l', l:'gal', gal:'l', qt:'l', pt:'ml',
  cup:'ml', 'fl oz':'ml', tbsp:'tsp', tsp:'tbsp',
  m3:'l', cm3:'ml', ft3:'l', in3:'ml',
  // Area
  mm2:'cm2', cm2:'m2', m2:'sqft', km2:'sqmi',
  ha:'ac', ac:'ha', sqft:'m2', sqin:'cm2', sqmi:'km2',
  // Speed
  'm/s':'km/h', 'km/h':'mph', mph:'km/h', knot:'mph', 'ft/s':'m/s',
  // Data — always go up one SI step
  bit:'b', b:'kb', kb:'mb', mb:'gb', gb:'tb', tb:'pb',
  kib:'mib', mib:'gib', gib:'tib', tib:'gib',
  // Energy
  j:'cal', kj:'kcal', mj:'kwh', cal:'j', kcal:'kj',
  wh:'kwh', kwh:'mj', btu:'kj', ev:'j',
  // Pressure
  pa:'atm', kpa:'psi', mpa:'bar', bar:'psi', mbar:'pa',
  psi:'bar', atm:'psi', torr:'pa',
  // Currency — suggest USD (or EUR for USD itself)
  usd:'eur', eur:'usd', gbp:'usd', jpy:'usd', cny:'usd', krw:'usd',
  aud:'usd', cad:'usd', chf:'usd', inr:'usd', idr:'usd', sgd:'usd',
  hkd:'usd', mxn:'usd', brl:'usd', rub:'usd', 'try':'usd', php:'usd',
  thb:'usd', myr:'usd', vnd:'usd', aed:'usd', sar:'usd', nzd:'usd',
  zar:'usd', sek:'usd', nok:'usd', pln:'usd', czk:'usd', dkk:'usd',
  huf:'usd', twd:'usd', pkr:'usd', bdt:'usd', ngn:'usd', egp:'usd',
  kes:'usd', btc:'usd', eth:'usd',
};

/** Parse a bare "number + unit" query (no bridge word). Returns {value,fromU,fromStr} or null. */
function _parseUnitOnly(q) {
  const s = q.trim();
  const numM = s.match(/^([+\-]?[\d,]+(?:\.\d+)?(?:[eE][+\-]?\d+)?)\s*/);
  if (!numM) return null;
  const value = parseFloat(numM[1].replace(/,/g, ''));
  if (!isFinite(value)) return null;
  const rest = s.slice(numM[0].length).trim();
  if (!rest) return null;
  const fromU = _convLookup(rest);
  if (!fromU) return null;
  return {value, fromU, fromStr: rest};
}

// Country registry: ISO-3166-1 alpha-2 → {name, currency (canonical key|null), sys}
// sys: 'metric' | 'imperial' | 'uk'
const _COUNTRY_DATA = {
  AE:{name:'UAE',               currency:'aed', sys:'metric'},
  AR:{name:'Argentina',         currency:null,  sys:'metric'},
  AT:{name:'Austria',           currency:'eur', sys:'metric'},
  AU:{name:'Australia',         currency:'aud', sys:'metric'},
  BD:{name:'Bangladesh',        currency:'bdt', sys:'metric'},
  BE:{name:'Belgium',           currency:'eur', sys:'metric'},
  BR:{name:'Brazil',            currency:'brl', sys:'metric'},
  CA:{name:'Canada',            currency:'cad', sys:'metric'},
  CH:{name:'Switzerland',       currency:'chf', sys:'metric'},
  CL:{name:'Chile',             currency:null,  sys:'metric'},
  CN:{name:'China',             currency:'cny', sys:'metric'},
  CO:{name:'Colombia',          currency:null,  sys:'metric'},
  CZ:{name:'Czech Republic',    currency:'czk', sys:'metric'},
  DE:{name:'Germany',           currency:'eur', sys:'metric'},
  DK:{name:'Denmark',           currency:'dkk', sys:'metric'},
  EG:{name:'Egypt',             currency:'egp', sys:'metric'},
  ES:{name:'Spain',             currency:'eur', sys:'metric'},
  ET:{name:'Ethiopia',          currency:null,  sys:'metric'},
  FI:{name:'Finland',           currency:'eur', sys:'metric'},
  FR:{name:'France',            currency:'eur', sys:'metric'},
  GB:{name:'United Kingdom',    currency:'gbp', sys:'uk'},
  GH:{name:'Ghana',             currency:null,  sys:'metric'},
  GR:{name:'Greece',            currency:'eur', sys:'metric'},
  HK:{name:'Hong Kong',         currency:'hkd', sys:'metric'},
  HU:{name:'Hungary',           currency:'huf', sys:'metric'},
  ID:{name:'Indonesia',         currency:'idr', sys:'metric'},
  IE:{name:'Ireland',           currency:'eur', sys:'metric'},
  IL:{name:'Israel',            currency:null,  sys:'metric'},
  IN:{name:'India',             currency:'inr', sys:'metric'},
  IQ:{name:'Iraq',              currency:null,  sys:'metric'},
  IT:{name:'Italy',             currency:'eur', sys:'metric'},
  JP:{name:'Japan',             currency:'jpy', sys:'metric'},
  KE:{name:'Kenya',             currency:'kes', sys:'metric'},
  KR:{name:'South Korea',       currency:'krw', sys:'metric'},
  LB:{name:'Liberia',           currency:null,  sys:'imperial'},
  MA:{name:'Morocco',           currency:null,  sys:'metric'},
  MM:{name:'Myanmar',           currency:null,  sys:'imperial'},
  MX:{name:'Mexico',            currency:'mxn', sys:'metric'},
  MY:{name:'Malaysia',          currency:'myr', sys:'metric'},
  NG:{name:'Nigeria',           currency:'ngn', sys:'metric'},
  NL:{name:'Netherlands',       currency:'eur', sys:'metric'},
  NO:{name:'Norway',            currency:'nok', sys:'metric'},
  NP:{name:'Nepal',             currency:null,  sys:'metric'},
  NZ:{name:'New Zealand',       currency:'nzd', sys:'metric'},
  PE:{name:'Peru',              currency:null,  sys:'metric'},
  PH:{name:'Philippines',       currency:'php', sys:'metric'},
  PK:{name:'Pakistan',          currency:'pkr', sys:'metric'},
  PL:{name:'Poland',            currency:'pln', sys:'metric'},
  PT:{name:'Portugal',          currency:'eur', sys:'metric'},
  RO:{name:'Romania',           currency:null,  sys:'metric'},
  RU:{name:'Russia',            currency:'rub', sys:'metric'},
  SA:{name:'Saudi Arabia',      currency:'sar', sys:'metric'},
  SE:{name:'Sweden',            currency:'sek', sys:'metric'},
  SG:{name:'Singapore',         currency:'sgd', sys:'metric'},
  TH:{name:'Thailand',          currency:'thb', sys:'metric'},
  TR:{name:'Turkey',            currency:'try', sys:'metric'},
  TW:{name:'Taiwan',            currency:'twd', sys:'metric'},
  TZ:{name:'Tanzania',          currency:null,  sys:'metric'},
  UA:{name:'Ukraine',           currency:null,  sys:'metric'},
  US:{name:'United States',     currency:'usd', sys:'imperial'},
  VE:{name:'Venezuela',         currency:null,  sys:'metric'},
  VN:{name:'Vietnam',           currency:'vnd', sys:'metric'},
  ZA:{name:'South Africa',      currency:'zar', sys:'metric'},
};

// Unit-system overrides for metric countries (keys that _AUTO_SUGGEST biases toward imperial).
const _METRIC_OVERRIDES = {
  m:'km',        // m → km  (not ft)
  km:'m',        // km → m  (not mi)
  kg:'g',        // kg → g  (not lb)
  l:'ml',        // l → ml  (not gal)
  'km/h':'m/s',  // km/h → m/s  (not mph)
  m2:'ha',       // m² → ha  (not sqft)
  ha:'m2',       // ha → m²  (not acre)
};

/** Pick the best auto-suggest target key given the source unit + current country. */
function _getAutoSuggestTarget(fromU, value) {
  const code    = (state.settings && state.settings.region) || '';
  const country = _COUNTRY_DATA[code];

  // Currency: route to local currency if possible
  if (fromU.t === 'currency') {
    const local = country && country.currency;
    if (local) {
      if (fromU.k !== local) return local;            // foreign → local
      return fromU.k === 'usd' ? 'eur' : 'usd';     // local → international reference
    }
    return _AUTO_SUGGEST[fromU.k] || null;
  }

  // Time: hours depend on magnitude (universal rule)
  if (fromU.t === 'time' && fromU.k === 'h') {
    return value < 24 ? 'min' : 'd';
  }

  // Metric-system overrides (only applies when country.sys === 'metric')
  if (country && country.sys === 'metric') {
    const ov = _METRIC_OVERRIDES[fromU.k];
    if (ov !== undefined) return ov;
  }

  return _AUTO_SUGGEST[fromU.k] || null;
}

/**
 * Auto-suggest a conversion when the user only typed a number + unit.
 * Returns the same shape as _parseConversion, or null if no suggestion.
 */
function _autoSuggestConv(q) {
  const parsed = _parseUnitOnly(q);
  if (!parsed) return null;
  const {value, fromU, fromStr} = parsed;
  const toKey = _getAutoSuggestTarget(fromU, value);
  if (!toKey) return null;
  const toU = _CONV_UNITS[toKey];
  if (!toU || toU.t !== fromU.t) return null;
  return {value, fromU, toU, fromStr, toStr: toKey};
}

/** Build and return the HTML for a conversion card, or '' if query doesn't match. */
function renderConvCard(query) {
  const conv = _parseConversion(query) || _autoSuggestConv(query);
  if (!conv) return '';
  const result = _doConvert(conv);
  if (!isFinite(result)) return '';
  // Currency: 6 sig-figs is plenty; avoid trailing decimal noise for e.g. IDR
  const sigFigs = conv.fromU.t === 'currency' ? 6 : 10;
  const rounded  = parseFloat(result.toPrecision(sigFigs));
  const formatted = fmtCalcResult(rounded);
  const copyVal   = String(rounded);
  const expr  = `${conv.value} ${conv.fromStr} → ${conv.toStr}`;
  const label = _CONV_LABEL[conv.fromU.t] || conv.fromU.t;
  let disc = '';
  if (conv.fromU.t === 'currency') {
    const stale = !_liveRates || (Date.now() - _liveRatesTs) > _RATES_TTL;
    if (stale) {
      send('FetchCurrencyRates');
      disc = `<div class="tsp-conv-disclaimer">Updating rates...</div>`;
    } else {
      disc = `<div class="tsp-conv-disclaimer">Live rates</div>`;
    }
  }
  return `<div class="tsp-calc-card" onclick="navigator.clipboard&&navigator.clipboard.writeText('${copyVal}').catch(()=>{})" title="Click to copy">
    <div class="tsp-conv-type">${escHtml(label)}</div>
    <div class="tsp-calc-expr">${escHtml(expr)}</div>
    <div class="tsp-calc-result">${escHtml(formatted)}</div>
    ${disc}
    <div class="tsp-calc-copy-hint">click to copy</div>
  </div>`;
}

function renderTspSuggestions(q) {
  const results = document.getElementById('tsp-results');
  if (!results) return;
  const query = q.trim();
  const ql = query.toLowerCase();
  spotlightUrls = [];
  let html = '';

  // ── Calculator: show result immediately if query is a math expression ──────
  if (query && isMathExpr(query)) {
    const calcResult = evaluateMath(query);
    if (calcResult !== null) {
      html += `<div class="tsp-calc-card" onclick="navigator.clipboard&&navigator.clipboard.writeText('${calcResult}').catch(()=>{})" title="Click to copy">
        <div class="tsp-calc-expr">${escHtml(query)}</div>
        <div class="tsp-calc-result">= ${escHtml(fmtCalcResult(calcResult))}</div>
        <div class="tsp-calc-copy-hint">click to copy</div>
      </div>`;
    }
  }
  // ── Unit Converter ────────────────────────────────────────────────────────
  if (query && !html) {
    html += renderConvCard(query);
  }
  // ─────────────────────────────────────────────────────────────────────────

  if (!query) {
    const quick = [
      {t: 'GitHub', u: 'https://github.com'},
      {t: 'YouTube', u: 'https://youtube.com'},
      {t: 'Gmail', u: 'https://mail.google.com'},
      {t: 'ChatGPT', u: 'https://chatgpt.com'},
      {t: 'Google', u: 'https://google.com'},
    ];
    html += '<div class="tsp-section-lbl">Quick access</div>';
    quick.forEach(s => {
      html += tspRow(s.t, s.u, tspFavicon(s.u), false);
      spotlightUrls.push(s.u);
    });
    const hist = (state.history || []).slice(0, 6);
    if (hist.length) {
      html += '<div class="tsp-section-lbl">Recently visited</div>';
      hist.forEach(h => {
        html += tspRow(h.title || h.url, h.url, tspFavicon(h.url), false);
        spotlightUrls.push(h.url);
      });
    }
  } else {
    const engine = (state.search_engines || []).find(e => e.is_default) || {url_template: 'https://www.google.com/search?q={query}'};
    const searchUrl = engine.url_template.replace('{query}', encodeURIComponent(query));
    html += '<div class="tsp-section-lbl">Search</div>';
    html += tspRow('Search for "' + query + '"', searchUrl, '', true);
    spotlightUrls.push(searchUrl);

    const hist = (state.history || []).filter(h =>
      (h.title || '').toLowerCase().includes(ql) || (h.url || '').toLowerCase().includes(ql)
    ).slice(0, 5);
    if (hist.length) {
      html += '<div class="tsp-section-lbl">History</div>';
      hist.forEach(h => {
        html += tspRow(h.title || h.url, h.url, tspFavicon(h.url), false);
        spotlightUrls.push(h.url);
      });
    }

    if (query.includes('.') || query.startsWith('http')) {
      const direct = query.startsWith('http') ? query : 'https://' + query;
      html += '<div class="tsp-section-lbl">Go to</div>';
      html += tspRow(direct, direct, '', false);
      spotlightUrls.push(direct);
    }
  }

  results.innerHTML = html;
  spotlightIdx = -1;

  // Show "Tab → AI" hint when there is a query and AI is configured
  const hint = document.getElementById('tsp-ai-hint');
  if (hint) {
    if (query && hasAiKey()) hint.classList.add('visible');
    else hint.classList.remove('visible');
  }
}

function tspKeydown(e) {
  if (!spotlightOpen) return;
  // Tab → switch to AI mode (only when query exists and AI is configured)
  if (e.key === 'Tab') {
    e.preventDefault();
    const val = document.getElementById('tsp-input')?.value.trim() || '';
    if (val && hasAiKey() && !tspAiMode) {
      tspEnterAiMode(val);
    } else if (tspAiMode) {
      tspExitAiMode();
    }
    return;
  }
  // Escape in AI mode → exit AI mode rather than closing spotlight
  if (e.key === 'Escape' && tspAiMode) {
    e.preventDefault();
    tspExitAiMode();
    return;
  }
  if (tspAiMode) return; // ignore arrow/enter nav while in AI mode
  const rows = document.querySelectorAll('#tsp-results .tsp-row');
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    spotlightIdx = Math.min(spotlightIdx + 1, rows.length - 1);
    rows.forEach((r, i) => r.classList.toggle('tsp-active', i === spotlightIdx));
    if (rows[spotlightIdx]) rows[spotlightIdx].scrollIntoView({block: 'nearest'});
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    spotlightIdx = Math.max(spotlightIdx - 1, -1);
    rows.forEach((r, i) => r.classList.toggle('tsp-active', i === spotlightIdx));
    if (spotlightIdx >= 0 && rows[spotlightIdx]) rows[spotlightIdx].scrollIntoView({block: 'nearest'});
  } else if (e.key === 'Enter') {
    e.preventDefault();
    if (spotlightIdx >= 0 && spotlightUrls[spotlightIdx]) {
      spotlightNavigate(spotlightUrls[spotlightIdx]);
    } else {
      const val = document.getElementById('tsp-input').value.trim();
      if (val) spotlightNavigate(val);
    }
  }
}

let __updateToastTimer = null;
let __pendingUpdateVersion = null;
let __manualUpdateCheck = false;
let __updateModalVersion = null;
function isUpdateModalOpen() {
  const modal = document.getElementById('update-modal');
  return !!modal && modal.classList.contains('open');
}
function updateModalCopy(status, data) {
  if (status === 'up_to_date') return {title: 'Ventus is up to date', sub: 'You already have the newest version.'};
  if (status === 'available') return {title: 'Update available', sub: data.version ? `Ventus v${data.version} is ready.` : 'A new Ventus update is ready.'};
  if (status === 'downloading') {
    const pct = data.total > 0 ? Math.round((data.received / data.total) * 100) : null;
    return {title: 'Downloading update', sub: pct !== null ? `${pct}% downloaded.` : 'Getting the installer ready.'};
  }
  if (status === 'installing') return {title: 'Installing update', sub: 'Ventus will restart when it is done.'};
  if (status === 'error') return {title: 'Could not check updates', sub: data.error || 'Something went wrong.'};
  return {title: 'Checking for updates', sub: 'This should only take a moment.'};
}
function setUpdateModalIcon(status) {
  const svg = document.getElementById('update-modal-icon-svg');
  if (!svg) return;
  let html = '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>';
  if (status === 'up_to_date') html = '<polyline points="20 6 9 17 4 12"/>';
  if (status === 'available') html = '<path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/>';
  if (status === 'error') html = '<circle cx="12" cy="12" r="9"/><path d="M12 8v5"/><path d="M12 16h.01"/>';
  svg.innerHTML = html;
  svg.classList.toggle('update-spin', status === 'checking' || status === 'downloading' || status === 'installing');
}
function setUpdateModalActions(status) {
  const actions = document.getElementById('update-modal-actions');
  if (!actions) return;
  let html = '';
  if (status === 'available') html = '<button class="ob-btn-secondary" onclick="closeUpdateModal(true)">Later</button><button class="ob-btn-primary" onclick="installUpdate()">Update now</button>';
  if (status === 'up_to_date' || status === 'error') html = '<button class="ob-btn-primary" onclick="closeUpdateModal(false)">Done</button>';
  actions.innerHTML = html;
  actions.style.display = html ? 'flex' : 'none';
}
function setUpdateModalProgress(status, data) {
  const box = document.getElementById('update-modal-progress');
  const bar = document.getElementById('update-modal-bar');
  const label = document.getElementById('update-modal-progress-label');
  if (!box || !bar || !label) return;
  if (status !== 'downloading') {
    box.classList.remove('visible');
    bar.style.width = '0%';
    label.textContent = '';
    return;
  }
  const pct = data.total > 0 ? Math.round((data.received / data.total) * 100) : null;
  box.classList.add('visible');
  bar.style.width = pct !== null ? `${pct}%` : '18%';
  label.textContent = pct !== null ? `Downloading... ${pct}%` : 'Downloading...';
}
function showUpdateModal(data) {
  const modal = document.getElementById('update-modal');
  if (!modal) return;
  const status = (data && data.status) || 'checking';
  const copy = updateModalCopy(status, data || {});
  const title = document.getElementById('update-modal-title');
  const sub = document.getElementById('update-modal-sub');
  const notes = document.getElementById('update-modal-notes');
  if (title) title.textContent = copy.title;
  if (sub) sub.textContent = copy.sub;
  if (notes) {
    notes.textContent = status === 'available' ? ((data && data.notes) || 'No release notes for this update.') : '';
    notes.classList.toggle('visible', status === 'available');
  }
  if (status === 'available') __updateModalVersion = (data && data.version) || null;
  if (status === 'up_to_date' || status === 'error') __updateModalVersion = null;
  setUpdateModalIcon(status);
  setUpdateModalActions(status);
  setUpdateModalProgress(status, data || {});
  modal.classList.add('open');
  syncUpdateModalClip();
}
function closeUpdateModal(dismiss) {
  const modal = document.getElementById('update-modal');
  if (!modal || !modal.classList.contains('open')) return;
  modal.classList.remove('open');
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
  if (dismiss && __updateModalVersion) {
    send('DismissUpdate', {version: __updateModalVersion});
    __updateModalVersion = null;
  }
}
function syncUpdateModalClip() {
  const modal = document.getElementById('update-modal');
  const panel = document.getElementById('update-modal-panel');
  if (!modal || !panel || !modal.classList.contains('open')) return;
  requestAnimationFrame(() => {
    if (!modal.classList.contains('open')) return;
    const rect = panel.getBoundingClientRect();
    send('SuggestionOverlay', {visible:true, x:rect.left - 14, y:rect.top - 14, width:rect.width + 28, height:rect.height + 28});
  });
}
function showUpdateToast(version, notes) {
  const toast = document.getElementById('update-toast');
  if (!toast) return;
  __pendingUpdateVersion = version || null;
  const versionEl = document.getElementById('ut-version-text');
  if (versionEl) versionEl.textContent = version ? 'v' + version + ' is ready to install' : 'A new version is ready';
  toast.classList.remove('hiding');
  toast.classList.add('visible');
  if (__updateToastTimer) clearTimeout(__updateToastTimer);
  __updateToastTimer = setTimeout(dismissUpdateToast, 15000);
  requestAnimationFrame(() => {
    const rect = toast.getBoundingClientRect();
    send('SuggestionOverlay', {visible:true, x:rect.left - 4, y:rect.top - 4, width:rect.width + 8, height:rect.height + 8});
  });
}
function dismissUpdateToast() {
  if (__updateToastTimer) { clearTimeout(__updateToastTimer); __updateToastTimer = null; }
  const toast = document.getElementById('update-toast');
  if (!toast || !toast.classList.contains('visible')) return;
  toast.classList.remove('visible');
  toast.classList.add('hiding');
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
  toast.addEventListener('animationend', () => toast.classList.remove('hiding'), {once: true});
  if (__pendingUpdateVersion !== null) {
    send('DismissUpdate', {version: __pendingUpdateVersion});
    __pendingUpdateVersion = null;
  }
}

function saveAiSettings() {
  const keys = {
    anthropic: document.getElementById('set-anthropic-key').value,
    openai: document.getElementById('set-openai-key').value,
    gemini: document.getElementById('set-gemini-key').value,
    openrouter: document.getElementById('set-openrouter-key').value,
    ollama_url: document.getElementById('set-ollama-url').value,
  };
  send('SaveSettings', {key: 'ai_keys', value: keys});
  toast('API keys saved', 'success');
}

// ============================================================
// BOOKMARKS / HISTORY / DOWNLOADS
// ============================================================
function renderBookmarks() {
  const list = document.getElementById('bookmarks-list');
  if (!list) return;
  const bms = state.bookmarks || [];
  if (!bms.length) {
    list.innerHTML = '<div style="color:var(--text-muted);font-size:12px;text-align:center;padding:24px 0">No bookmarks yet. Hit the bookmark icon in the address bar to save a page.</div>';
    return;
  }
  list.innerHTML = bms.map(b => `
    <div class="bm-item" data-nav-url="${escAttr(b.url)}">
      <div class="bm-item-info">
        <div class="bm-item-title">${escHtml(b.title || b.url)}</div>
        <div class="bm-item-url">${escHtml(b.url)}</div>
      </div>
      <button class="bm-item-del" title="Remove bookmark" data-remove-bookmark-url="${escAttr(b.url)}">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>
    </div>`).join('');
}

function renderHistory() {
  const list = document.getElementById('history-list');
  if (!list) return;
  const hist = state.history || [];
  if (!hist.length) {
    list.innerHTML = '<div style="color:var(--text-muted);font-size:12px;text-align:center;padding:24px 0">No history yet.</div>';
    return;
  }
  list.innerHTML = hist.map(h => `
    <div class="hist-item" data-nav-url="${escAttr(h.url)}">
      <div class="hist-item-info">
        <div class="hist-item-title">${escHtml(h.title || friendlySiteName(h.url, h.title))}</div>
        <div class="hist-item-url">${escHtml(siteDomain(h.url))}</div>
      </div>
      <span class="hist-item-time">${formatRelativeTime(h.visited_at)}</span>
      <button class="hist-item-del" title="Delete" data-delete-history-id="${escAttr(String(h.id))}">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>
      </button>
    </div>`).join('');
}

function renderDownloads() {
  const list = document.getElementById('downloads-list');
  if (!list) return;
  const dls = state.downloads || [];
  if (!dls.length) {
    list.innerHTML = '<div style="color:var(--text-muted);font-size:12px;text-align:center;padding:24px 0">Nothing downloaded yet.</div>';
    return;
  }
  list.innerHTML = dls.slice().reverse().map(d => {
    const isDone = d.status === 'complete';
    const isFail = d.status === 'failed';
    const statusColor = isDone ? 'var(--success)' : isFail ? 'var(--danger)' : 'var(--text-muted)';
    const pct = (d.total_bytes && d.total_bytes > 0)
      ? Math.round((d.received_bytes / d.total_bytes) * 100)
      : null;
    const metaText = isDone
      ? `Done${d.local_path ? ' · ' + escHtml(d.local_path.split(/[\\/]/).pop()) : ''}`
      : isFail ? 'Failed'
      : pct !== null ? `${pct}% of ${formatBytes(d.total_bytes)}`
      : 'Downloading…';
    const progressHtml = (!isDone && !isFail && pct !== null)
      ? `<div class="dl-item-progress"><div class="dl-item-progress-bar" style="width:${pct}%"></div></div>`
      : '';
    const actionBtns = isDone && d.local_path ? `
      <button class="dl-action-btn" title="Open file" data-open-file-path="${escAttr(d.local_path)}">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5z"/><polyline points="14 2 14 8 20 8"/></svg>
      </button>
      <button class="dl-action-btn" title="Show in folder" data-reveal-file-path="${escAttr(d.local_path)}">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
      </button>` : '';
    return `
    <div class="dl-item" data-nav-url="${escAttr(d.url)}">
      <div class="dl-item-icon">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
      </div>
      <div class="dl-item-info">
        <div class="dl-item-name">${escHtml(d.filename)}</div>
        <div class="dl-item-meta" style="color:${statusColor}">${metaText}</div>
        ${progressHtml}
      </div>
      <div class="dl-item-actions">${actionBtns}</div>
    </div>`;
  }).join('');
}

// ============================================================
// CONTEXT MENU
// ============================================================
let __ctxData = null;

function copyToClipboard(text) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text).catch(() => _fallbackCopy(text));
  } else {
    _fallbackCopy(text);
  }
}
function _fallbackCopy(text) {
  const el = document.createElement('textarea');
  el.value = text;
  el.style.cssText = 'position:fixed;opacity:0;top:0;left:0';
  document.body.appendChild(el);
  el.focus(); el.select();
  try { document.execCommand('copy'); } catch(_) {}
  document.body.removeChild(el);
}

function ctxAction(action) {
  const d = __ctxData;
  if (!d) return;
  closeContextMenu();
  switch (action) {
    case 'open_link':       send('Navigate', {url: d.linkUrl}); break;
    case 'open_link_tab':   send('OpenInNewTab', {url: d.linkUrl}); break;
    case 'open_link_win':   send('OpenInNewWindow', {url: d.linkUrl}); break;
    case 'copy_link':       copyToClipboard(d.linkUrl); break;
    case 'open_image_tab':  send('OpenInNewTab', {url: d.imageSrc}); break;
    case 'copy_image_url':  copyToClipboard(d.imageSrc); break;
    case 'save_image':      send('ContextMenuSaveImage', {url: d.imageSrc}); break;
    case 'copy_text':       copyToClipboard(d.selectedText); break;
    case 'search_text': {
      const q = 'https://www.google.com/search?q=' + encodeURIComponent(d.selectedText);
      send('OpenInNewTab', {url: q});
      break;
    }
    case 'back':    send('Back'); break;
    case 'forward': send('Forward'); break;
    case 'reload':  send('Reload'); break;
    case 'copy_page_url': copyToClipboard(d.pageUrl); break;
    case 'view_source': send('OpenInNewTab', {url: 'view-source:' + d.pageUrl}); break;
  }
}

function showBrowserContextMenu(data) {
  __ctxData = data;
  const menu = document.getElementById('context-menu');
  const style = document.documentElement.style;
  const sidebarW  = parseFloat(style.getPropertyValue('--sidebar-w'))  || 0;
  const frameW    = parseFloat(style.getPropertyValue('--frame-side-w')) || 5;
  const topChromeH = parseFloat(style.getPropertyValue('--top-chrome-h')) || 44;
  const winW = window.innerWidth;
  const winH = window.innerHeight;

  // Build sections
  const rows = [];
  const sep = () => rows.push('<div class="ctx-sep"></div>');
  const item = (action, label, cls) =>
    rows.push(`<div class="ctx-item${cls ? ' '+cls : ''}" onclick="ctxAction('${action}')">${escHtml(label)}</div>`);

  if (data.linkUrl) {
    item('open_link',     'Open link');
    item('open_link_tab', 'Open link in new tab');
    item('open_link_win', 'Open link in new window');
    item('copy_link',     'Copy link address');
  }
  if (data.imageSrc) {
    if (data.linkUrl) sep();
    item('open_image_tab', 'Open image in new tab');
    item('copy_image_url', 'Copy image address');
    item('save_image',     'Save image as…');
  }
  if (data.selectedText) {
    if (data.linkUrl || data.imageSrc) sep();
    item('copy_text', 'Copy');
    const shortText = data.selectedText.length > 25
      ? data.selectedText.slice(0, 25) + '…'
      : data.selectedText;
    item('search_text', `Search for “${shortText}”`);
  }
  if (rows.length > 0) sep();
  if (data.canBack)  item('back',    'Back',    data.canBack  ? '' : 'ctx-disabled');
  if (data.canFwd)   item('forward', 'Forward', data.canFwd   ? '' : 'ctx-disabled');
  item('reload', 'Reload');
  sep();
  item('copy_page_url', 'Copy page URL');
  item('view_source',   'View page source');

  menu.innerHTML = rows.join('');

  // Position: offset content viewport coords to chrome window coords
  let x = sidebarW + frameW + data.x;
  let y = topChromeH + data.y;
  menu.style.visibility = 'hidden';
  menu.style.left = x + 'px';
  menu.style.top  = y + 'px';
  menu.classList.add('open');

  requestAnimationFrame(() => {
    const rect = menu.getBoundingClientRect();
    if (x + rect.width  > winW - 8) x = Math.max(0, winW - rect.width - 8);
    if (y + rect.height > winH - 8) y = Math.max(0, y - rect.height);
    menu.style.left = x + 'px';
    menu.style.top  = y + 'px';
    menu.style.visibility = 'visible';
    // Expand chrome clip to full window so ANY click outside the menu is captured
    // by the chrome overlay's mousedown listener and dismisses the menu.
    send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
  });
}

function closeContextMenu() {
  const menu = document.getElementById('context-menu');
  if (!menu.classList.contains('open')) return;
  menu.classList.remove('open');
  __ctxData = null;
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
}

// Close context menu or more menu on click outside
document.addEventListener('mousedown', e => {
  const ctx = document.getElementById('context-menu');
  if (ctx.classList.contains('open') && !ctx.contains(e.target)) {
    closeContextMenu();
  }
  const more = document.getElementById('more-menu');
  const moreBtn = document.getElementById('btn-more');
  if (more && more.classList.contains('open') && !more.contains(e.target) && moreBtn && !moreBtn.contains(e.target)) {
    closeMoreMenu();
  }
}, true);

// Suppress browser's own context menu on the chrome overlay
document.addEventListener('contextmenu', e => e.preventDefault());

// ============================================================
// DOWNLOAD PANEL
// ============================================================
function toggleDownloadPanel(event) {
  if (event) event.stopPropagation();
  const panel = document.getElementById('download-panel');
  if (panel.classList.contains('open')) {
    closeDownloadPanel();
    return;
  }
  const btn = document.getElementById('btn-more');
  const rect = btn ? btn.getBoundingClientRect() : {bottom: 48, right: window.innerWidth - 8};
  panel.style.top = rect.bottom + 4 + 'px';
  panel.style.right = (window.innerWidth - rect.right) + 'px';
  renderDownloadPanel();
  panel.classList.add('open');
  // Full-window clip so clicks anywhere (including outside the panel) reach the chrome WebView
  // and the click-outside handler can close the panel properly.
  requestAnimationFrame(() => {
    send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
  });
}

function closeDownloadPanel() {
  document.getElementById('download-panel').classList.remove('open');
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
}

function dlFileTypeClass(filename) {
  const ext = (filename || '').split('.').pop().toLowerCase();
  if (['jpg','jpeg','png','gif','webp','svg','avif','ico','bmp'].includes(ext)) return 'ft-img';
  if (['mp4','mkv','avi','mov','webm','flv','wmv'].includes(ext)) return 'ft-vid';
  if (['mp3','flac','wav','aac','ogg','m4a'].includes(ext)) return 'ft-aud';
  if (['pdf','doc','docx','xls','xlsx','ppt','pptx','odt','txt','md','csv'].includes(ext)) return 'ft-doc';
  if (['zip','rar','7z','tar','gz','bz2'].includes(ext)) return 'ft-arc';
  if (['exe','msi','dmg','pkg','deb','rpm','appimage'].includes(ext)) return 'ft-exe';
  if (['js','ts','py','rs','go','java','cpp','c','cs','json','html','css','xml','yaml'].includes(ext)) return 'ft-code';
  return 'ft-default';
}
function dlFileTypeIcon(cls) {
  const icons = {
    'ft-img':'IMG','ft-vid':'VID','ft-aud':'AUD','ft-doc':'DOC',
    'ft-arc':'ZIP','ft-exe':'EXE','ft-code':'SRC','ft-default':'FILE',
  };
  return icons[cls] || 'FILE';
}
function renderDownloadPanel() {
  const list = document.getElementById('dl-panel-list');
  const dls = (state.downloads || []).slice().reverse().slice(0, 6);
  if (!dls.length) {
    list.innerHTML = `<div class="dl-panel-empty">
      <div class="dl-panel-empty-icon">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
      </div>
      <span class="dl-panel-empty-label">No downloads yet</span>
      <span class="dl-panel-empty-sub">Files you download will appear here</span>
    </div>`;
    return;
  }
  list.innerHTML = dls.map(d => {
    const isDone = d.status === 'complete';
    const isFail = d.status === 'failed';
    const isActive = !isDone && !isFail;
    const pct = (d.total_bytes && d.total_bytes > 0)
      ? Math.round((d.received_bytes / d.total_bytes) * 100)
      : null;
    const metaText = isDone ? (d.total_bytes ? formatBytes(d.total_bytes) : '')
      : isFail ? ''
      : pct !== null ? `${pct}% of ${formatBytes(d.total_bytes)}`
      : '';
    const badgeClass = isDone ? 'done' : isFail ? 'failed' : 'active';
    const badgeText = isDone ? 'Done' : isFail ? 'Failed' : 'Downloading';
    const progressHtml = isActive && pct !== null
      ? `<div class="dl-pi-bar"><div class="dl-pi-bar-fill" style="width:${pct}%"></div></div>`
      : '';
    const ftClass = isActive ? 'ft-active' : dlFileTypeClass(d.filename);
    const iconContent = isActive
      ? `<span class="dl-spin"></span>`
      : `<span style="font-size:9px;font-weight:800;letter-spacing:0.03em">${dlFileTypeIcon(ftClass)}</span>`;
    const actionsHtml = `
      ${isDone && d.local_path ? `<button class="dl-pi-btn" data-open-file-path="${escAttr(d.local_path)}">Open</button>
      <button class="dl-pi-btn" data-reveal-file-path="${escAttr(d.local_path)}">Show</button>` : ''}
      ${!isActive ? `<button class="dl-pi-btn danger" data-del-dl-id="${escAttr(d.id)}" title="Remove">✕</button>` : ''}`;
    return `<div class="dl-panel-item">
      <div class="dl-pi-icon-wrap ${ftClass}">${iconContent}</div>
      <div class="dl-pi-body">
        <div class="dl-pi-name">${escHtml(d.filename)}</div>
        <div class="dl-pi-row">
          <span class="dl-pi-status-badge ${badgeClass}">${badgeText}</span>
          ${metaText ? `<span class="dl-pi-meta">${escHtml(metaText)}</span>` : ''}
        </div>
        ${progressHtml}
      </div>
      <div class="dl-pi-actions">${actionsHtml}</div>
    </div>`;
  }).join('');
}

// Event delegation for download panel buttons (Open, Folder, Delete)
document.getElementById('download-panel').addEventListener('click', e => {
  const openBtn = e.target.closest('[data-open-file-path]');
  if (openBtn) { send('OpenFile', {path: openBtn.dataset.openFilePath}); return; }
  const revBtn = e.target.closest('[data-reveal-file-path]');
  if (revBtn) { send('RevealFile', {path: revBtn.dataset.revealFilePath}); return; }
  const delBtn = e.target.closest('[data-del-dl-id]');
  if (delBtn) { send('DeleteDownload', {id: delBtn.dataset.delDlId}); return; }
});

// Close download panel on click outside (also on Escape — handled in keydown)
document.addEventListener('click', e => {
  const panel = document.getElementById('download-panel');
  const btn = document.getElementById('btn-more');
  if (panel.classList.contains('open') && !panel.contains(e.target) && !(btn && btn.contains(e.target))) {
    closeDownloadPanel();
  }
  const menu = document.getElementById('more-menu');
  const moreBtn = document.getElementById('btn-more');
  if (menu && menu.classList.contains('open') && !menu.contains(e.target) && !(moreBtn && moreBtn.contains(e.target))) {
    closeMoreMenu();
  }
}, true);

// ============================================================
// MORE MENU
// ============================================================
function toggleMoreMenu(e) {
  if (e) e.stopPropagation();
  const menu = document.getElementById('more-menu');
  if (menu.classList.contains('open')) {
    closeMoreMenu();
  } else {
    openMoreMenu();
  }
}

function openMoreMenu() {
  const menu = document.getElementById('more-menu');
  const btn = document.getElementById('btn-more');
  if (!menu || !btn) return;
  const rect = btn.getBoundingClientRect();
  menu.style.top = (rect.bottom + 4) + 'px';
  menu.style.left = 'auto';
  menu.style.right = (window.innerWidth - rect.right) + 'px';
  updateMoreMenuZoom();
  menu.classList.add('open');
  btn.classList.add('active');
  requestAnimationFrame(() => {
    send('SuggestionOverlay', {visible:true, x:0, y:0, width:window.innerWidth, height:window.innerHeight});
  });
}

function closeMoreMenu() {
  const menu = document.getElementById('more-menu');
  if (!menu || !menu.classList.contains('open')) return;
  menu.classList.remove('open');
  const btn = document.getElementById('btn-more');
  if (btn) btn.classList.remove('active');
  send('SuggestionOverlay', {visible:false, x:0, y:0, width:0, height:0});
}

function updateMoreMenuZoom() {
  const id = state.active_tab_id;
  const level = tabZoomLevels[id] || 1.0;
  const el = document.getElementById('more-zoom-pct');
  if (el) el.textContent = Math.round(level * 100) + '%';
}

function muteTab(e, tabId) {
  e.stopPropagation();
  send('MuteTab', {tab_id: tabId});
}

function renderBookmarksBar() {
  const bar = document.getElementById('bookmarks-bar');
  if (!bar) return;
  const show = document.getElementById('app').classList.contains('show-bookmarks-bar');
  if (!show) return;
  const bms = (state.bookmarks || []).slice(0, 20);
  if (!bms.length) {
    bar.innerHTML = '<span class="bm-bar-empty">No bookmarks yet - save pages with Ctrl+D</span>';
    return;
  }
  bar.innerHTML = bms.map(b => {
    const title = bookmarkLabel(b.url, b.title);
    const tip = [b.title, b.url].filter(Boolean).join(' - ');
    const icon = bookmarkIconUrl(b.url);
    const img = icon ? `<img class="bm-bar-icon" src="${escAttr(icon)}" alt="" onerror="this.style.display='none';this.nextElementSibling.classList.remove('hidden')">` : '';
    const fallback = `<span class="bm-bar-fallback ${icon ? 'hidden' : ''}">${escHtml(siteInitial(title))}</span>`;
    return `<div class="bm-bar-item" title="${escAttr(tip)}" onclick="navigateToUrl('${escAttr(b.url)}')">
      ${img}${fallback}
      <span class="bm-bar-text">${escHtml(title.length > 24 ? title.slice(0,22)+'...' : title)}</span>
    </div>`;
  }).join('');
}

// ============================================================
// ZOOM
// ============================================================
const tabZoomLevels = {};
let zoomToastTimer = null;

function zoomIn() {
  const id = state.active_tab_id;
  if (!id) return;
  const cur = tabZoomLevels[id] || 1.0;
  tabZoomLevels[id] = parseFloat(Math.min(cur + 0.1, 3.0).toFixed(1));
  applyZoom(id);
}

function zoomOut() {
  const id = state.active_tab_id;
  if (!id) return;
  const cur = tabZoomLevels[id] || 1.0;
  tabZoomLevels[id] = parseFloat(Math.max(cur - 0.1, 0.25).toFixed(1));
  applyZoom(id);
}

function zoomReset() {
  const id = state.active_tab_id;
  if (!id) return;
  delete tabZoomLevels[id];
  applyZoom(id);
}

function applyZoom(id) {
  const level = tabZoomLevels[id] || 1.0;
  send('ZoomSet', {level});
  showZoomToast(Math.round(level * 100));
}

function showZoomToast(pct) {
  const el = document.getElementById('zoom-toast');
  el.textContent = 'Zoom: ' + pct + '%';
  el.classList.add('visible');
  if (zoomToastTimer) clearTimeout(zoomToastTimer);
  zoomToastTimer = setTimeout(() => { el.classList.remove('visible'); zoomToastTimer = null; }, 1500);
}

function navigateToUrl(url) {
  if (!url) return;
  hideSuggestions();
  const overlay = document.getElementById('settings-overlay');
  if (overlay) overlay.classList.remove('open');
  send('NavigateFromOverlay', {url});
}

function deleteHistoryEntry(id) {
  send('DeleteHistoryEntry', {id});
}

function removeBookmarkByUrl(url) {
  send('BookmarkRemove', {url});
}

function clearHistory() {
  send('HistoryClear');
}

function searchHistory(q) {
  send('GetHistory', {q});
}

function formatRelativeTime(ms) {
  if (!ms) return '';
  const diff = Date.now() - ms;
  const s = Math.floor(diff / 1000);
  if (s < 60) return 'just now';
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.floor(h / 24);
  if (d < 30) return `${d}d ago`;
  return new Date(ms).toLocaleDateString();
}

function formatBytes(bytes) {
  if (!bytes) return '';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + ' MB';
  return (bytes / 1073741824).toFixed(2) + ' GB';
}

function handleDelegatedListClick(e) {
  const openFile = e.target.closest('[data-open-file-path]');
  if (openFile) {
    e.preventDefault();
    e.stopPropagation();
    send('OpenFile', {path: openFile.dataset.openFilePath});
    return;
  }
  const revealFile = e.target.closest('[data-reveal-file-path]');
  if (revealFile) {
    e.preventDefault();
    e.stopPropagation();
    send('RevealFile', {path: revealFile.dataset.revealFilePath});
    return;
  }
  const removeBookmark = e.target.closest('[data-remove-bookmark-url]');
  if (removeBookmark) {
    e.preventDefault();
    e.stopPropagation();
    removeBookmarkByUrl(removeBookmark.dataset.removeBookmarkUrl);
    return;
  }
  const deleteHistory = e.target.closest('[data-delete-history-id]');
  if (deleteHistory) {
    e.preventDefault();
    e.stopPropagation();
    deleteHistoryEntry(Number(deleteHistory.dataset.deleteHistoryId));
    return;
  }
  const shortcut = e.target.closest('[data-shortcut-url]');
  if (shortcut) {
    e.preventDefault();
    navigateWithSuggestion(shortcut.dataset.shortcutUrl);
    return;
  }
  const refreshFeed = e.target.closest('[data-refresh-feed]');
  if (refreshFeed) {
    e.preventDefault();
    requestNeuraFeed(true);
    return;
  }
  const moreFeed = e.target.closest('[data-more-feed]');
  if (moreFeed) {
    e.preventDefault();
    navigateToUrl('https://feed.neuraspheres.com/articles');
    return;
  }
  const news = e.target.closest('[data-news-url]');
  if (news) {
    e.preventDefault();
    navigateToUrl(news.dataset.newsUrl);
    return;
  }
  const nav = e.target.closest('[data-nav-url]');
  if (nav) {
    e.preventDefault();
    navigateToUrl(nav.dataset.navUrl);
  }
}

// ============================================================
// ONBOARDING
// ============================================================
// ONBOARDING
// ============================================================
function obDetectCountry() {
  try {
    const parts = (navigator.language || '').split('-');
    if (parts.length >= 2) {
      const code = parts[parts.length - 1].toUpperCase();
      if (_COUNTRY_DATA[code]) return code;
    }
  } catch(e) {}
  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
    const TZ_MAP = {
      'Asia/Jakarta':'ID','Asia/Makassar':'ID','Asia/Jayapura':'ID',
      'America/New_York':'US','America/Chicago':'US','America/Denver':'US','America/Los_Angeles':'US',
      'America/Phoenix':'US','America/Anchorage':'US','Pacific/Honolulu':'US',
      'Europe/London':'GB','Europe/Paris':'FR','Europe/Berlin':'DE','Europe/Rome':'IT',
      'Europe/Madrid':'ES','Europe/Amsterdam':'NL','Europe/Brussels':'BE','Europe/Lisbon':'PT',
      'Europe/Warsaw':'PL','Europe/Vienna':'AT','Europe/Stockholm':'SE','Europe/Oslo':'NO',
      'Europe/Copenhagen':'DK','Europe/Helsinki':'FI','Europe/Zurich':'CH','Europe/Prague':'CZ',
      'Europe/Budapest':'HU','Europe/Bucharest':'RO','Europe/Sofia':'BG','Europe/Athens':'GR',
      'Europe/Kiev':'UA','Europe/Moscow':'RU','Europe/Istanbul':'TR',
      'Asia/Tokyo':'JP','Asia/Seoul':'KR','Asia/Shanghai':'CN','Asia/Hong_Kong':'HK',
      'Asia/Singapore':'SG','Asia/Bangkok':'TH','Asia/Ho_Chi_Minh':'VN','Asia/Manila':'PH',
      'Asia/Kuala_Lumpur':'MY','Asia/Kolkata':'IN','Asia/Karachi':'PK','Asia/Dhaka':'BD',
      'Asia/Colombo':'LK','Asia/Riyadh':'SA','Asia/Dubai':'AE','Asia/Tehran':'IR',
      'Asia/Baghdad':'IQ','Asia/Beirut':'LB','Asia/Amman':'JO','Asia/Kuwait':'KW',
      'Africa/Cairo':'EG','Africa/Lagos':'NG','Africa/Nairobi':'KE','Africa/Johannesburg':'ZA',
      'Africa/Accra':'GH','Africa/Casablanca':'MA','Africa/Tunis':'TN','Africa/Addis_Ababa':'ET',
      'America/Sao_Paulo':'BR','America/Argentina/Buenos_Aires':'AR','America/Bogota':'CO',
      'America/Lima':'PE','America/Santiago':'CL','America/Mexico_City':'MX',
      'America/Caracas':'VE','America/Montevideo':'UY',
      'America/Toronto':'CA','America/Vancouver':'CA',
      'Australia/Sydney':'AU','Australia/Melbourne':'AU','Australia/Brisbane':'AU',
      'Australia/Perth':'AU','Pacific/Auckland':'NZ',
    };
    if (TZ_MAP[tz]) return TZ_MAP[tz];
  } catch(e) {}
  return null;
}
function obPopulateRegionSelect(current) {
  const sel = document.getElementById('ob-region-select');
  if (!sel) return;
  const sorted = Object.entries(_COUNTRY_DATA).sort((a,b) => a[1].name.localeCompare(b[1].name));
  sel.innerHTML = '<option value="">— Not set —</option>' +
    sorted.map(([code, c]) =>
      `<option value="${escAttr(code)}"${code === current ? ' selected' : ''}>${countryFlag(code)} ${escHtml(c.name)}</option>`
    ).join('');
}
function obShowDetectBanner(code) {
  const c = _COUNTRY_DATA[code];
  if (!c) return;
  const banner = document.getElementById('ob-detect-banner');
  const flagEl = document.getElementById('ob-detect-flag');
  const nameEl = document.getElementById('ob-detect-name');
  const sel    = document.getElementById('ob-region-select');
  if (banner) banner.style.display = 'flex';
  if (flagEl) flagEl.textContent = countryFlag(code);
  if (nameEl) nameEl.textContent = c.name;
  if (sel)    sel.style.display = 'none';
  obRegion = code;
}
function obClearDetect() {
  const banner = document.getElementById('ob-detect-banner');
  const sel    = document.getElementById('ob-region-select');
  if (banner) banner.style.display = 'none';
  if (sel)    sel.style.display = '';
  obPopulateRegionSelect('');
  obRegion = '';
}
function obSelectRegion(code) { obRegion = code; }
function openOnboarding() {
  document.getElementById('onboarding-overlay').classList.add('open');
  obStep = 0;
  obDirection = 1;
  const detected = obDetectCountry();
  obPopulateRegionSelect(detected || '');
  if (detected) {
    obShowDetectBanner(detected);
  } else {
    const banner = document.getElementById('ob-detect-banner');
    const sel    = document.getElementById('ob-region-select');
    if (banner) banner.style.display = 'none';
    if (sel)    sel.style.display = '';
  }
  renderObStep();
}
function finishOnboarding() {
  document.getElementById('onboarding-overlay').classList.remove('open');
  send('SaveSettings', {key: 'onboarding_done', value: true});
}
function obNext() {
  if (obStep >= OB_STEPS - 1) return;
  obDirection = 1;
  obStep++;
  renderObStep();
}
function obPrev() {
  if (obStep <= 0) return;
  obDirection = -1;
  obStep--;
  renderObStep();
}
function renderObStep() {
  const fill    = document.getElementById('ob-bar-fill');
  const counter = document.getElementById('ob-step-counter');
  const totalContent = OB_STEPS - 2;
  const pct = obStep === 0 ? 0 : obStep >= OB_STEPS - 1 ? 100
    : Math.round(obStep / totalContent * 100);
  if (fill)    fill.style.width = pct + '%';
  if (counter) counter.textContent = (obStep > 0 && obStep < OB_STEPS - 1)
    ? `Step ${obStep} of ${totalContent}` : '';
  const cls = obDirection >= 0 ? 'slide-r' : 'slide-l';
  document.querySelectorAll('.ob-step').forEach((el, i) => {
    el.classList.remove('active', 'slide-r', 'slide-l');
    if (i === obStep) el.classList.add('active', cls);
  });
}
function obSelectTheme(t) {
  obTheme = t;
  document.querySelectorAll('[id^="ob-theme-"]').forEach(el => el.classList.remove('selected'));
  const el = document.getElementById('ob-theme-' + t);
  if (el) el.classList.add('selected');
  document.documentElement.setAttribute('data-theme', t === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : t);
}
function obSelectEngine(id, el) {
  obEngine = id;
  document.querySelectorAll('.ob-engine-btn').forEach(b => b.classList.remove('selected'));
  if (el) el.classList.add('selected');
}
function obSelectSidebar(mode) {
  obSidebarMode = mode;
  document.querySelectorAll('.ob-sidebar-chip').forEach(b => b.classList.remove('selected'));
  const el = document.getElementById('ob-sb-' + mode);
  if (el) el.classList.add('selected');
}
function obSaveAndFinish() {
  const anthropic  = (document.getElementById('ob-anthropic-key')  || {}).value || '';
  const openai     = (document.getElementById('ob-openai-key')      || {}).value || '';
  const gemini     = (document.getElementById('ob-gemini-key')      || {}).value || '';
  const openrouter = (document.getElementById('ob-openrouter-key')  || {}).value || '';
  if (anthropic || openai || gemini || openrouter) {
    send('SaveSettings', {key: 'ai_keys', value: {anthropic, openai, gemini, openrouter}});
  }
  send('SaveSettings', {key: 'theme',          value: obTheme});
  send('SaveSettings', {key: 'sidebar_mode',   value: obSidebarMode});
  send('SaveSettings', {key: 'default_engine', value: obEngine});
  if (obRegion) send('SaveSettings', {key: 'region', value: obRegion});
  obDirection = 1;
  obStep = OB_STEPS - 1;
  renderObStep();
}

// ============================================================
// TAB SEARCH
// ============================================================
let sidebarPeeking = false;
let sidebarPinned  = false;
let sidebarHideTimer = null;
let sidebarClipTimer = null;
let sidebarPinTimer = null;
const sidebarHideDelay = 100;
let tabSearchIdx = -1;
function openTabSearch(fromRust) {
  if (!fromRust) send('OpenSettings');
  const modal = document.getElementById('tab-search-modal');
  modal.classList.add('open');
  document.getElementById('tab-search-input').value = '';
  tabSearchIdx = -1;
  filterTabs('');
  setTimeout(() => document.getElementById('tab-search-input').focus(), 50);
}
function closeTabSearch() {
  send('CloseSettings');
  document.getElementById('tab-search-modal').classList.remove('open');
}
function filterTabs(query) {
  const results = document.getElementById('tab-search-results');
  const q = query.toLowerCase();
  const filtered = (state.tabs || []).filter(t =>
    !q || t.title.toLowerCase().includes(q) || t.url.toLowerCase().includes(q)
  );
  if (!filtered.length) {
    results.innerHTML = '<div style="padding:12px 16px;color:var(--text-muted);font-size:12px">No tabs found</div>';
    return;
  }
  results.innerHTML = filtered.map((t, i) => `
    <div class="tab-search-item" data-id="${t.id}" onclick="switchTab('${t.id}');closeTabSearch()">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="color:var(--text-dim);flex-shrink:0"><circle cx="12" cy="12" r="10"/><path d="M12 2a10 10 0 0 1 0 20"/></svg>
      <div style="min-width:0;flex:1">
        <div class="ts-title">${escHtml(t.title||'New Tab')}</div>
        <div class="ts-url">${escHtml(formatDisplayUrl(t.url))}</div>
      </div>
    </div>`).join('');
  tabSearchIdx = -1;
}
function handleTabSearchKey(e) {
  if (e.key === 'Escape') { closeTabSearch(); return; }
  const items = document.querySelectorAll('.tab-search-item');
  if (e.key === 'ArrowDown') {
    tabSearchIdx = Math.min(tabSearchIdx + 1, items.length - 1);
  } else if (e.key === 'ArrowUp') {
    tabSearchIdx = Math.max(tabSearchIdx - 1, 0);
  } else if (e.key === 'Enter' && tabSearchIdx >= 0) {
    items[tabSearchIdx].click();
  }
  items.forEach((el, i) => el.classList.toggle('highlighted', i === tabSearchIdx));
}

// ============================================================
// CONTEXT MENUS
// ============================================================
let _ctxActions = [];
function tabContextMenu(ev, tabId) {
  ev.preventDefault();
  showContextMenu(ev.clientX, ev.clientY, [
    {label:'Pin tab', action:() => send('PinTab', {id: tabId})},
    {label:'Duplicate', action:() => send('NewTab')},
    {sep:true},
    {label:'Close tab', danger:true, action:() => send('CloseTab', {id: tabId})},
  ]);
}
function wsContextMenu(ev, wsId) {
  ev.preventDefault();
  const items = [{label:'Rename', action:() => openWorkspaceModal(wsId)}];
  if ((state.workspaces || []).length > 1) {
    items.push({sep:true});
    items.push({label:'Delete workspace', danger:true, action:() => deleteWorkspace(ev, wsId)});
  }
  showContextMenu(ev.clientX, ev.clientY, items);
}
function showContextMenu(x, y, items) {
  _ctxActions = [];
  const menu = document.getElementById('ctx-menu');
  let idx = 0;
  menu.innerHTML = items.map(item => {
    if (item.sep) return '<div class="ctx-sep"></div>';
    const i = idx;
    _ctxActions.push(item.action);
    idx++;
    return `<div class="ctx-item ${item.danger?'danger':''}" onclick="document.getElementById('ctx-menu').style.display='none';_ctxActions[${i}]()">
      ${escHtml(item.label)}
    </div>`;
  }).join('');
  menu.style.display = 'block';
  menu.style.left = x + 'px';
  menu.style.top = y + 'px';
  const rect = menu.getBoundingClientRect();
  if (rect.right > window.innerWidth) menu.style.left = (x - rect.width) + 'px';
  if (rect.bottom > window.innerHeight) menu.style.top = (y - rect.height) + 'px';
}
document.addEventListener('click', () => {
  document.getElementById('ctx-menu').style.display = 'none';
});
document.addEventListener('contextmenu', e => {
  if (!e.target.closest('#ctx-menu')) document.getElementById('ctx-menu').style.display = 'none';
});
document.addEventListener('click', handleDelegatedListClick);

// ============================================================
// TOAST
// ============================================================
function toast(msg, type='info') {
  const el = document.createElement('div');
  el.className = `toast ${type}`;
  el.textContent = msg;
  document.getElementById('toast-container').appendChild(el);
  setTimeout(() => el.remove(), 3500);
}

// ============================================================
// KEYBOARD SHORTCUTS
// ============================================================
document.addEventListener('keydown', e => {
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  if (ctrl && e.shiftKey && key === 't') { e.preventDefault(); send('ReopenTab'); }
  else if (ctrl && e.shiftKey && key === 'n') { e.preventDefault(); send('OpenIncognito'); }
  else if (ctrl && key === 'n') { e.preventDefault(); send('OpenInNewWindow', {url: 'neura://newtab'}); }
  else if (ctrl && !e.shiftKey && /^[1-9]$/.test(e.key)) { e.preventDefault(); switchToTabIndex(parseInt(e.key, 10) - 1); }
  else if (ctrl && key === 't') { e.preventDefault(); openNewTabSpotlight(); }
  else if (ctrl && e.key === 'w') { e.preventDefault(); if (state.active_tab_id) send('CloseTab', {id: state.active_tab_id}); }
  else if (ctrl && key === 'l') { e.preventDefault(); focusUrl(); }
  else if (ctrl && key === 'k') { e.preventDefault(); openTabSearch(); }
  else if (ctrl && key === 'h') { e.preventDefault(); openSettings('history'); }
  else if (ctrl && key === 'j') { e.preventDefault(); openSettings('downloads'); }
  else if (ctrl && key === 'd') { e.preventDefault(); toggleBookmark(); }
  else if (ctrl && e.shiftKey && e.key === 'A') { e.preventDefault(); toggleAi(); }
  else if (ctrl && key === 'b') { e.preventDefault(); toggleSidebar(); }
  else if (ctrl && e.key === ',') { e.preventDefault(); openSettings('general'); }
  else if (ctrl && key === 'r') { e.preventDefault(); nav('Reload'); }
  else if (e.key === 'F5') { e.preventDefault(); nav('Reload'); }
  else if (ctrl && (e.key === '+' || e.key === '=')) { e.preventDefault(); zoomIn(); }
  else if (ctrl && e.key === '-') { e.preventDefault(); zoomOut(); }
  else if (ctrl && e.key === '0') { e.preventDefault(); zoomReset(); }
  else if (e.key === 'Escape') {
    if (spotlightOpen) closeSpotlight();
    else if (document.getElementById('workspace-delete-modal').classList.contains('open')) closeWorkspaceDeleteModal();
    else if (document.getElementById('workspace-modal').classList.contains('open')) closeWorkspaceModal();
    else if (document.getElementById('app').classList.contains('content-fullscreen')) { send('ToggleFullscreen'); }
    else if (document.getElementById('context-menu').classList.contains('open')) closeContextMenu();
    else if (document.getElementById('adblock-modal').classList.contains('open')) closeAdBlockModal();
    else if (document.getElementById('more-menu').classList.contains('open')) closeMoreMenu();
    else if (document.getElementById('download-panel').classList.contains('open')) closeDownloadPanel();
    else if (document.getElementById('model-modal').classList.contains('open')) closeModelModal();
    else if (document.getElementById('update-modal').classList.contains('open')) closeUpdateModal(false);
    else if (document.getElementById('settings-overlay').classList.contains('open')) closeSettings();
    else if (document.getElementById('tab-search-modal').classList.contains('open')) closeTabSearch();
    else if (document.getElementById('update-toast').classList.contains('visible')) dismissUpdateToast();
    else if (document.getElementById('onboarding-overlay').classList.contains('open')) { /* don't close onboarding on esc */ }
  }
  else if (e.altKey && e.key === 'ArrowLeft') { e.preventDefault(); nav('Back'); }
  else if (e.altKey && e.key === 'ArrowRight') { e.preventDefault(); nav('Forward'); }
  else if (e.key === 'F11') { e.preventDefault(); send('ToggleFullscreen'); }
  else if (e.key === 'F12') { send('OpenDevtools'); }
});
window.addEventListener('resize', () => {
  refreshSuggestionOverlayBounds();
  syncUpdateModalClip();
}, {passive: true});

// ============================================================
// UTILS
// ============================================================
function escHtml(str) {
  if (!str) return '';
  return String(str).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;');
}
function escAttr(str) {
  return escHtml(str);
}

// ============================================================
// AUTO-HIDE SIDEBAR TRIGGERS
// ============================================================
(function() {
  const trigger = document.getElementById('sidebar-float-trigger');
  const sidebar = document.getElementById('sidebar');

  trigger.addEventListener('mouseenter', () => showFloatingSidebar(false));

  trigger.addEventListener('mouseleave', e => {
    if (!document.getElementById('app').classList.contains('sidebar-auto-hide')) return;
    const to = e.relatedTarget;
    // Moving from trigger directly into the sidebar — cancel hide
    if (to && (to === sidebar || sidebar.contains(to))) return;
    scheduledHide();
  });

  sidebar.addEventListener('mouseenter', () => {
    if (document.getElementById('app').classList.contains('sidebar-auto-hide')) cancelSidebarHide();
  });

  sidebar.addEventListener('mouseleave', function(e) {
    if (!document.getElementById('app').classList.contains('sidebar-auto-hide')) return;
    const to = e.relatedTarget;
    if (to && this.contains(to)) return; // moving to a child element — stay open
    const pop = document.getElementById('sb-ws-popover');
    if (to && pop && (to === pop || pop.contains(to))) return; // moving to popover — keep sidebar open
    scheduledHide();
  });

  // For neura:// pages, chrome owns the full window so WM_MOUSELEAVE still won't
  // fire when cursor leaves the sidebar into the content zone. Detect it via
  // document-level mousemove: if cursor is past the sidebar's right edge, hide.
  document.addEventListener('mousemove', function(e) {
    const app = document.getElementById('app');
    if (!app.classList.contains('sidebar-auto-hide')) return;
    if (!app.classList.contains('sidebar-floating-open')) return;
    const rect = sidebar.getBoundingClientRect();
    if (e.clientX > rect.right) scheduledHide();
  }, {passive: true});
})();

// ============================================================
// INIT
// ============================================================
updateGreeting();
updateNewtabDate();
if (window.__neura_pending_state) {
  window.__neura.setState(window.__neura_pending_state);
}
if (window.__neura_show_onboarding) {
  setTimeout(() => window.__neura.showOnboarding(), 100);
}
</script>
</body>
</html>"##;
    html.replace("__LOGO_URL__", &logo)
        .replace("__APP_VERSION__", version)
}
