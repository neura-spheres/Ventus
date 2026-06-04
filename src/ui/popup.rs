// Chrome for a wrapped popup window (OAuth, share, payment dialogs). A slim top bar
// that mirrors the main browser: dark, the live origin with a lock so users can verify
// the site, and a close button. The bar is the window drag region.
pub fn popup_chrome_html() -> &'static str {
    r##"<!doctype html><html><head><meta charset="utf-8"><style>
:root{--bg:#0c0a09;--fg:#e7e5e4;--muted:#a8a29e;--line:#1c1917;}
*{margin:0;padding:0;box-sizing:border-box;}
html,body{height:100%;}
body{background:var(--bg);color:var(--fg);font-family:'Segoe UI',system-ui,-apple-system,sans-serif;overflow:hidden;-webkit-user-select:none;user-select:none;cursor:default;}
.bar{display:flex;align-items:center;height:100%;padding:0 6px 0 12px;gap:8px;border-bottom:1px solid var(--line);}
.left{display:flex;align-items:center;gap:7px;min-width:0;flex:1;}
.lock{width:13px;height:13px;color:var(--muted);flex:none;}
.origin{font-size:12.5px;color:var(--fg);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;letter-spacing:.1px;}
.close{width:26px;height:26px;border:none;background:transparent;color:var(--muted);border-radius:7px;display:flex;align-items:center;justify-content:center;cursor:pointer;flex:none;}
.close:hover{background:#e0303018;color:#f87171;}
.close svg{width:13px;height:13px;}
</style></head><body>
<div class="bar" id="bar">
  <div class="left">
    <svg class="lock" id="lock" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
    <span class="origin" id="origin">Loading…</span>
  </div>
  <button class="close" id="close" title="Close"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
</div>
<script>
function send(cmd){try{window.ipc.postMessage(JSON.stringify({cmd:cmd}));}catch(_){}}
document.getElementById('bar').addEventListener('mousedown',function(e){
  if(e.button!==0)return;
  if(e.target.closest('#close'))return;
  send('popup_drag');
});
document.getElementById('close').addEventListener('click',function(){send('popup_close');});
window.__popup={setOrigin:function(host,secure){
  var o=document.getElementById('origin');if(o)o.textContent=host||'';
  var l=document.getElementById('lock');if(l)l.style.color=secure?'#34d399':'#f59e0b';
}};
</script></body></html>"##
}
