pub fn popup_chrome_html() -> &'static str {
    r##"<!doctype html><html><head><meta charset="utf-8"><style>
:root{--bg:#0c0a09;--bg2:#141110;--fg:#f5f5f4;--muted:#a8a29e;--line:#262220;--accent:#8b5cf6;--ok:#34d399;--warn:#f59e0b;}
*{margin:0;padding:0;box-sizing:border-box;}
html,body{height:100%;}
body{background:linear-gradient(180deg,var(--bg2),var(--bg));color:var(--fg);font-family:'Segoe UI',system-ui,-apple-system,sans-serif;overflow:hidden;-webkit-user-select:none;user-select:none;cursor:default;}
.bar{display:flex;align-items:center;height:100%;padding:0 8px 0 11px;gap:9px;border-bottom:1px solid var(--line);}
.badge{width:24px;height:24px;border-radius:7px;background:#ffffff0a;border:1px solid var(--line);display:flex;align-items:center;justify-content:center;flex:none;}
.lock{width:12px;height:12px;color:var(--ok);}
.left{display:flex;flex-direction:column;justify-content:center;min-width:0;flex:1;line-height:1.15;}
.origin{font-size:12.5px;font-weight:600;color:var(--fg);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;letter-spacing:.1px;}
.tag{font-size:10px;color:var(--muted);letter-spacing:.3px;}
.close{width:28px;height:28px;border:none;background:transparent;color:var(--muted);border-radius:8px;display:flex;align-items:center;justify-content:center;cursor:pointer;flex:none;transition:background .12s,color .12s;}
.close:hover{background:#ef444420;color:#f87171;}
.close svg{width:13px;height:13px;}
</style></head><body>
<div class="bar" id="bar">
  <div class="badge"><svg class="lock" id="lock" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg></div>
  <div class="left">
    <span class="origin" id="origin">Loading…</span>
    <span class="tag">Opened by a site</span>
  </div>
  <button class="close" id="close" title="Close"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
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
  var l=document.getElementById('lock');if(l)l.style.color=secure?'var(--ok)':'var(--warn)';
}};
</script></body></html>"##
}
