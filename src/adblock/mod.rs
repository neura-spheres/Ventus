mod blocklist;
pub mod cosmetic;
pub mod filter_loader;

use std::collections::HashSet;

// The `adblock` crate is re-aliased here so it doesn't shadow this module itself.
use adblock as adblock_crate;

pub use blocklist::BLOCKED_DOMAINS;

/// The live ad-block engine.
///
/// Two-phase startup:
///   Phase 1 — `AdBlockEngine::new()` builds from the ~180-domain fallback list instantly.
///              The browser is usable immediately, blocking major ad networks.
///   Phase 2 — A background task downloads EasyList/EasyPrivacy/uBlock filters and calls
///              `engine.rebuild(...)`.  From that point on, network-level blocking uses the
///              full adblock-rust engine (~300k+ rules) and the JS domain list grows to
///              several thousand entries for fetch/XHR interception.
pub struct AdBlockEngine {
    /// Full adblock-rust engine (None until filter lists are ready).
    inner: Option<adblock_crate::Engine>,
    /// Domains fed to the JS fetch/XHR intercept (updated when filter lists load).
    js_domains: Vec<String>,
    pub exceptions: Vec<String>,
    pub enabled: bool,
    /// Cached JS init script regenerated whenever any field above changes.
    init_script: String,
}

impl AdBlockEngine {
    /// Phase-1 constructor: uses the bundled fallback domain list.
    pub fn new(enabled: bool, exceptions: &[String]) -> Self {
        let js_domains: Vec<String> = BLOCKED_DOMAINS.iter().map(|s| s.to_string()).collect();
        let mut engine = Self {
            inner: None,
            js_domains,
            exceptions: exceptions.to_vec(),
            enabled,
            init_script: String::new(),
        };
        engine.init_script = engine.build_script();
        engine
    }

    /// Phase-2 upgrade: swap in the full adblock-rust engine and expanded domain list.
    /// Called from the main thread when the background filter-loader task completes.
    pub fn rebuild(&mut self, inner: adblock_crate::Engine, js_domains: Vec<String>) {
        self.inner = Some(inner);
        self.js_domains = js_domains;
        self.init_script = self.build_script();
        tracing::info!(
            "adblock: engine rebuilt with {} JS domains",
            self.js_domains.len()
        );
    }

    /// JS initialization script injected into every non-incognito content WebView.
    /// Returns an empty string when disabled (caller also passes empty for incognito).
    pub fn init_script(&self) -> &str {
        if self.enabled {
            &self.init_script
        } else {
            ""
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.init_script = self.build_script();
    }

    /// Toggle a site exception.  Returns `true` if the site is now excepted.
    pub fn toggle_exception(&mut self, host: &str) -> bool {
        let host = normalize_host(host);
        if let Some(pos) = self.exceptions.iter().position(|e| e == &host) {
            self.exceptions.remove(pos);
            self.init_script = self.build_script();
            false
        } else {
            self.exceptions.push(host);
            self.init_script = self.build_script();
            true
        }
    }

    pub fn set_exceptions(&mut self, exceptions: Vec<String>) {
        self.exceptions = exceptions;
        self.init_script = self.build_script();
    }

    pub fn exceptions(&self) -> &[String] {
        &self.exceptions
    }

    pub fn is_site_excepted(&self, url: &str) -> bool {
        if let Ok(u) = url::Url::parse(url) {
            if let Some(host) = u.host_str() {
                return host_matches_list(host, &self.exceptions);
            }
        }
        false
    }

    /// Rust-side network check using the full adblock-rust engine (when loaded).
    /// Falls back to simple domain matching against the JS domain list.
    /// Used by the navigation handler for top-level page requests.
    pub fn should_block_url(&self, url: &str, source: &str) -> bool {
        if !self.enabled {
            return false;
        }
        // Check against the full engine first.
        if let Some(engine) = &self.inner {
            if let Ok(req) = adblock_crate::request::Request::new(url, source, "document") {
                return engine.check_network_request(&req).matched;
            }
        }
        // Fallback: domain list check.
        if let Ok(u) = url::Url::parse(url) {
            if let Some(host) = u.host_str() {
                return host_matches_list(host, &self.js_domains);
            }
        }
        false
    }

    /// Cosmetic rules (CSS selectors + optional scriptlet) for a given page URL.
    /// Called after navigation to inject page-specific hiding rules.
    pub fn cosmetic_for_url(&self, url: &str) -> (Vec<String>, String) {
        let Some(engine) = &self.inner else {
            return (vec![], String::new());
        };
        if !self.enabled || self.is_site_excepted(url) {
            return (vec![], String::new());
        }
        if skip_site(url) {
            return (vec![], String::new());
        }
        let res = engine.url_cosmetic_resources(url);
        (
            res.hide_selectors.into_iter().collect(),
            res.injected_script,
        )
    }

    fn build_script(&self) -> String {
        let domains_json =
            serde_json::to_string(&self.js_domains).unwrap_or_else(|_| "[]".to_string());
        let exceptions_json =
            serde_json::to_string(&self.exceptions).unwrap_or_else(|_| "[]".to_string());
        let css_json =
            serde_json::to_string(cosmetic::FALLBACK_CSS).unwrap_or_else(|_| r#""""#.to_string());
        let sels_json = serde_json::to_string(cosmetic::FALLBACK_SELECTORS)
            .unwrap_or_else(|_| "[]".to_string());

        format!(
            r#"(function(){{
"use strict";
if(window.__nab)return;
window.__nab=1;
var _bd=new Set({domains});
var _ex=new Set({exceptions});
var _en={enabled};
window.__nabSet=function(e){{_en=e?1:0;}};
window.__nabUpdateDomains=function(arr){{_bd=new Set(arr);}};
function _th(h){{var p=h.replace(/^www\./,"").split(".");for(var i=0;i<p.length-1;i++){{if(_bd.has(p.slice(i).join(".")))return 1;}}return 0;}}
function _gh(){{try{{return new URL(top.location.href).hostname.toLowerCase();}}catch(e){{}}try{{return new URL(location.href).hostname.toLowerCase();}}catch(e){{}}return"";}}
function _rd(h){{var p=h.replace(/^www\./,"").split(".");if(p.length<2)return h;var s=p.slice(-2).join(".");if(/^(co|com|net|org|ac|go|or|ne)\.[a-z][a-z]$/.test(s)&&p.length>2)return p.slice(-3).join(".");return s;}}
function _fp(h){{var p=_gh();if(!p)return 0;h=h.replace(/^www\./,"");p=p.replace(/^www\./,"");return h===p||h.endsWith("."+p)||p.endsWith("."+h)||_rd(h)===_rd(p);}}
function _safe(h){{var r=_rd(h);return /^(apple\.com|github\.com|githubassets\.com|githubusercontent\.com|google\.com|googleapis\.com|googleusercontent\.com|gstatic\.com|live\.com|microsoft\.com|microsoftonline\.com)$/.test(r);}}
function _exc(){{var h=_gh(),p=h.replace(/^www\./,"").split(".");for(var i=0;i<p.length-1;i++){{if(_ex.has(p.slice(i).join(".")))return 1;}}return 0;}}
function _skip(){{var h=_gh();return /(^|\.)youtube\.com$|(^|\.)youtu\.be$/.test(h);}}
function _blk(url){{if(!_en||_exc()||_skip())return 0;try{{var h=new URL(url,location.href).hostname.toLowerCase();if(_fp(h)||_safe(h))return 0;return _th(h);}}catch(e){{}}return 0;}}
function _inc(n){{_kc+=Math.max(0,n|0);_rpt();}}
var _of=window.fetch;
window.fetch=function(r,i){{var u=typeof r==="string"?r:(r&&r.url)||"";if(u&&_blk(u)){{_inc(1);return Promise.reject(new TypeError("blocked"));}}return _of.apply(this,arguments);}};
var _ox=XMLHttpRequest.prototype.open;
XMLHttpRequest.prototype.open=function(m,u){{if(u&&_blk(String(u))){{this.__nb=1;_inc(1);}}return _ox.apply(this,arguments);}};
var _sx=XMLHttpRequest.prototype.send;
XMLHttpRequest.prototype.send=function(){{if(this.__nb){{try{{this.abort();}}catch(e){{}}return;}}return _sx.apply(this,arguments);}};
var _sl={sels};
var _kc=0;var _kr=0;
window.__nabCount=function(){{return _kc;}};
window.__nabAddCount=function(n){{_inc(n);}};
function _hc(root){{
  if(!_en||_exc()||_skip()||!root||root.nodeType!==1)return;
  var k=0;
  _sl.forEach(function(sel){{try{{if(root.matches&&root.matches(sel)&&!root.__nh){{root.__nh=1;k++;}}root.querySelectorAll&&root.querySelectorAll(sel).forEach(function(el){{if(!el.__nh){{el.__nh=1;k++;}}}});}}catch(e){{}}}});
  if(k)_inc(k);
}}
function _ic(){{if(_skip()||document.getElementById("__nabc"))return;var s=document.createElement("style");s.id="__nabc";s.textContent={css};(document.head||document.documentElement).appendChild(s);_hc(document.documentElement);}}
if(document.readyState!=="loading"){{_ic();}}else{{document.addEventListener("DOMContentLoaded",_ic,{{once:true}});}}
function _kill(el){{
  if(!el||el.__nk||el===document.body||el===document.documentElement)return;
  el.__nk=1;
  try{{el.remove();}}catch(e){{return;}}
  if(!el.__nh)_kc++;
}}
function _hv(root){{
  if(!_en||_exc()||_skip()||!root||root.nodeType!==1)return;
  _sl.forEach(function(sel){{try{{if(root.matches&&root.matches(sel))_kill(root);root.querySelectorAll&&root.querySelectorAll(sel).forEach(function(el){{_kill(el);}});}}catch(e){{}}}});
  try{{root.querySelectorAll&&root.querySelectorAll('iframe').forEach(function(fr){{var s=fr.getAttribute('src')||fr.getAttribute('data-src')||'';if(s&&_blk(s))_kill(fr);}});}}catch(e){{}}
}}
function _rpt(){{if(_kc!==_kr){{_kr=_kc;try{{if(window.ipc)window.ipc.postMessage(JSON.stringify({{cmd:"ad_block_stats",killed:_kc}}));}}catch(e){{}}}}}}
_hv(document.documentElement);_rpt();
var _raf=null;
var _nodes=[];
var _ob=new MutationObserver(function(ms){{for(var i=0;i<ms.length;i++){{ms[i].addedNodes&&ms[i].addedNodes.forEach(function(n){{if(n&&n.nodeType===1)_nodes.push(n);}});}}if(!_raf)_raf=requestAnimationFrame(function(){{_raf=null;var ns=_nodes.splice(0,200);for(var i=0;i<ns.length;i++)_hv(ns[i]);_rpt();}});}});
_ob.observe(document.documentElement,{{childList:true,subtree:true}});
}})();"#,
            domains = domains_json,
            exceptions = exceptions_json,
            enabled = if self.enabled { 1 } else { 0 },
            css = css_json,
            sels = sels_json,
        )
    }
}

fn normalize_host(host: &str) -> String {
    host.to_lowercase().trim_start_matches("www.").to_string()
}

fn host_matches_list<S: AsRef<str>>(host: &str, list: &[S]) -> bool {
    let host = normalize_host(host);
    let set: HashSet<&str> = list.iter().map(|s| s.as_ref()).collect();
    let parts: Vec<&str> = host.split('.').collect();
    for i in 0..parts.len().saturating_sub(1) {
        if set.contains(parts[i..].join(".").as_str()) {
            return true;
        }
    }
    false
}

fn skip_site(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = normalize_host(host);
    host == "youtube.com" || host.ends_with(".youtube.com") || host == "youtu.be"
}
