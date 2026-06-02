pub fn new_tab_html() -> String {
    let logo = crate::ui::assets::logo_data_url();
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>New Tab</title>
<style>
*{box-sizing:border-box;margin:0;padding:0}
:root{
  --bg:#0d0f1a;--text:#e8eaf6;--text-muted:#8890b5;--text-dim:#484e72;
  --accent:#6366f1;--border:#2a2e54;--bg-elevated:#13162a;--bg-hover:#1a1d36;
  --accent-dim:rgba(99,102,241,0.15);--accent-glow:rgba(99,102,241,0.32);
  --font:'Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',system-ui,sans-serif;
}
[data-theme="light"]{
  --bg:#f7f8ff;--text:#1a1b2e;--text-muted:#5c6082;--text-dim:#7a7fa0;
  --accent:#5557e8;--border:#d1d5f0;--bg-elevated:#ffffff;--bg-hover:#eef0fd;
  --accent-dim:rgba(85,87,232,0.10);--accent-glow:rgba(85,87,232,0.22);
}
html,body{height:100%;background:var(--bg);color:var(--text);font-family:var(--font);overflow:hidden}
body{display:flex;flex-direction:column;align-items:center;justify-content:center;gap:24px;padding:32px;animation:nt-fade-in 0.4s ease}
@keyframes nt-fade-in{from{opacity:0;transform:translateY(8px)}to{opacity:1;transform:translateY(0)}}

.greeting{font-size:32px;font-weight:700;letter-spacing:-0.5px}
.sub{font-size:14px;color:var(--text-muted)}

.search-box{
  display:flex;align-items:center;gap:10px;
  background:var(--bg-elevated);border:1px solid var(--border);
  border-radius:999px;padding:10px 14px 10px 18px;width:520px;max-width:90vw;
  transition:border-color 0.2s,box-shadow 0.2s;
}
.search-box:focus-within{border-color:var(--accent);box-shadow:0 0 0 3px var(--accent-dim),0 0 12px var(--accent-glow)}
.search-box input{
  flex:1;border:none;background:transparent;color:var(--text);
  font-size:15px;outline:none;font-family:var(--font);
}
.search-box input::placeholder{color:var(--text-dim)}
.search-btn{
  background:linear-gradient(135deg,#3b82f6,#6366f1,#8b5cf6);
  border:none;border-radius:999px;
  padding:6px 16px;color:#fff;font-size:12px;font-weight:600;cursor:pointer;
  transition:opacity 0.15s,transform 0.15s;letter-spacing:0.02em;
}
.search-btn:hover{opacity:0.88;transform:scale(1.03)}

.shortcuts{display:flex;gap:12px;flex-wrap:wrap;justify-content:center;max-width:560px}
.shortcut{
  display:flex;flex-direction:column;align-items:center;gap:6px;
  padding:12px 14px;border-radius:12px;background:transparent;
  border:1px solid transparent;cursor:pointer;width:80px;
  transition:transform 0.15s,border-color 0.2s,box-shadow 0.2s;text-decoration:none;
}
.shortcut:hover{transform:translateY(-3px);border-color:var(--accent);box-shadow:0 4px 16px var(--accent-dim)}
.shortcut-icon{
  width:38px;height:38px;border-radius:9px;background:var(--bg-hover);
  display:flex;align-items:center;justify-content:center;
}
.shortcut-label{font-size:10px;color:var(--text-muted);text-align:center;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;width:100%}

.tip{font-size:11px;color:var(--text-dim);text-align:center;max-width:380px;line-height:1.7}
</style>
</head>
<body>
<img src="__LOGO_URL__" style="width:52px;height:52px;object-fit:contain;margin-bottom:-4px" alt="">
<div class="greeting" id="greeting">Good afternoon</div>
<div class="sub">What are you searching for?</div>

<div class="search-box">
  <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--text-dim)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
  <input id="search" placeholder="Search or enter a URL" autofocus onkeydown="handleKey(event)">
  <button class="search-btn" onclick="go()">Go</button>
</div>

<div class="shortcuts" id="shortcuts"></div>

<div class="tip">Tip: use Ctrl+T for a new tab, Ctrl+K to search your open tabs, and Ctrl+Shift+A to open the AI sidebar.</div>

<script>
const shortcuts=[
  {label:'GitHub',url:'https://github.com',color:'#24292e'},
  {label:'YouTube',url:'https://youtube.com',color:'#ff0000'},
  {label:'Reddit',url:'https://reddit.com',color:'#ff4500'},
  {label:'Wikipedia',url:'https://wikipedia.org',color:'#3366cc'},
  {label:'MDN',url:'https://developer.mozilla.org',color:'#0066cc'},
];

function init(){
  const h=new Date().getHours();
  document.getElementById('greeting').textContent=h<12?'Good morning':h<17?'Good afternoon':'Good evening';
  const sc=document.getElementById('shortcuts');
  sc.innerHTML=shortcuts.map(s=>`
    <a class="shortcut" onclick="navigate('${s.url}')">
      <div class="shortcut-icon" style="background:${s.color}22">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="${s.color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/></svg>
      </div>
      <span class="shortcut-label">${s.label}</span>
    </a>`).join('');
}
function handleKey(e){if(e.key==='Enter')go();}
function go(){
  const v=document.getElementById('search').value.trim();
  if(v)navigate(v);
}
function send(cmd,data={}){
  const sc=cmd.replace(/[A-Z]/g,m=>'_'+m.toLowerCase()).replace(/^_/,'');
  if(window.ipc)window.ipc.postMessage(JSON.stringify({cmd:sc,...data}));
}
function navigate(url){
  send('Navigate',{url});
}
init();
</script>
</body>
</html>"#;
    html.replace("__LOGO_URL__", &logo)
}
