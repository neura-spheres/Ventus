use serde_json::{json, Value};
use super::provider::Tool;

pub fn browser_tool_definitions() -> Vec<Tool> {
    vec![
        // Read tools
        Tool::function(
            "get_page_text",
            "Extract the readable text content of the current page. Use this first to understand what is on the page before clicking or typing.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        Tool::function(
            "get_page_interactive_elements",
            "Get all clickable / interactive elements on the current page, each labeled with an `element_id` you can pass to click_element or type_text.",
            json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        ),
        Tool::function(
            "click_element",
            "Click an interactive element on the page. Call get_page_interactive_elements first to obtain element IDs.",
            json!({
                "type": "object",
                "properties": {
                    "element_id": {
                        "type": "string",
                        "description": "The element_id returned by get_page_interactive_elements"
                    }
                },
                "required": ["element_id"]
            }),
        ),
        Tool::function(
            "type_text",
            "Type text into an input field or textarea. Call get_page_interactive_elements first to obtain element IDs.",
            json!({
                "type": "object",
                "properties": {
                    "element_id": {
                        "type": "string",
                        "description": "The element_id of the input element"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type into the field"
                    }
                },
                "required": ["element_id", "text"]
            }),
        ),
        Tool::function(
            "scroll_page",
            "Scroll the current page.",
            json!({
                "type": "object",
                "properties": {
                    "direction": {
                        "type": "string",
                        "enum": ["up", "down", "top", "bottom"],
                        "description": "Scroll direction. 'top'/'bottom' jump to page start/end."
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Pixels to scroll for 'up'/'down'. Default: 400.",
                        "default": 400
                    }
                },
                "required": ["direction"]
            }),
        ),
        // Navigation tools
        Tool::function(
            "navigate",
            "Navigate the current tab to a URL. Use a full URL (https://...) or a search query.",
            json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Full URL to navigate to (e.g. https://example.com)"
                    }
                },
                "required": ["url"]
            }),
        ),
        Tool::function(
            "go_back",
            "Go back to the previous page in the current tab's history.",
            json!({"type": "object", "properties": {}, "required": []}),
        ),
        Tool::function(
            "go_forward",
            "Go forward in the current tab's history.",
            json!({"type": "object", "properties": {}, "required": []}),
        ),
        Tool::function(
            "new_tab",
            "Open a new browser tab, optionally navigating to a URL.",
            json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "Optional URL to open in the new tab"
                    }
                },
                "required": []
            }),
        ),
        // Tab / state tools
        Tool::function(
            "list_tabs",
            "List all open tabs in the current workspace.",
            json!({"type": "object", "properties": {}, "required": []}),
        ),
        Tool::function(
            "switch_tab",
            "Switch to a different open tab.",
            json!({
                "type": "object",
                "properties": {
                    "tab_id": {
                        "type": "string",
                        "description": "The tab_id from list_tabs"
                    }
                },
                "required": ["tab_id"]
            }),
        ),
        Tool::function(
            "get_current_url",
            "Get the URL and title of the currently active tab.",
            json!({"type": "object", "properties": {}, "required": []}),
        ),
        Tool::function(
            "search_history",
            "Search the browser history for previously visited pages.",
            json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for in URLs and page titles"
                    }
                },
                "required": ["query"]
            }),
        ),
    ]
}

// ── JS builders for page tools ─────────────────────────────────────────────────
//
// Each function returns a JS snippet that, when eval'd in the content WebView,
// computes its result and posts it back via:
//   window.ipc.postMessage(JSON.stringify({cmd:"ai_tool_result", call_id:"...", result:"..."}))

pub fn js_get_page_text(call_id: &str) -> String {
    let cid = js_escape(call_id);
    format!(
        r#"(function(){{
  try {{
    var walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT, null, false);
    var lines = [];
    var node;
    while ((node = walker.nextNode())) {{
      var p = node.parentElement;
      if (!p) continue;
      var tag = p.tagName ? p.tagName.toUpperCase() : '';
      if (['SCRIPT','STYLE','NOSCRIPT','HEAD','SVG'].indexOf(tag) >= 0) continue;
      var style = window.getComputedStyle ? window.getComputedStyle(p) : null;
      if (style && (style.display === 'none' || style.visibility === 'hidden')) continue;
      var txt = node.textContent.replace(/\s+/g,' ').trim();
      if (txt.length > 1) lines.push(txt);
    }}
    var text = lines.join('\n').replace(/\n{{3,}}/g,'\n\n').substring(0, 8000);
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify(text)}}));
  }} catch(e) {{
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify("Error: "+e.message)}}));
  }}
}})()"#
    )
}

pub fn js_get_interactive_elements(call_id: &str) -> String {
    let cid = js_escape(call_id);
    format!(
        r#"(function(){{
  try {{
    var sel = 'a[href], button, input:not([type="hidden"]), select, textarea, [onclick], [role="button"], [role="link"], [role="menuitem"], [role="tab"], [tabindex]:not([tabindex="-1"])';
    var els = Array.from(document.querySelectorAll(sel));
    var result = [];
    els.slice(0, 120).forEach(function(el, i) {{
      var id = 'ai-el-' + i;
      el.setAttribute('data-ai-id', id);
      var rect = el.getBoundingClientRect();
      var visible = rect.width > 0 && rect.height > 0 && rect.top < window.innerHeight && rect.bottom > 0;
      var text = (el.textContent || el.value || el.placeholder || el.getAttribute('aria-label') || el.tagName || '').replace(/\s+/g,' ').trim().substring(0, 80);
      var type = el.tagName.toLowerCase();
      var href = el.href ? el.href.substring(0, 100) : '';
      if (visible || result.length < 30) {{
        result.push({{element_id: id, type: type, text: text, href: href, visible: visible}});
      }}
    }});
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify(result)}}));
  }} catch(e) {{
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{error:e.message}})}}));
  }}
}})()"#
    )
}

pub fn js_click_element(call_id: &str, element_id: &str) -> String {
    let cid = js_escape(call_id);
    let eid = js_escape(element_id);
    format!(
        r#"(function(){{
  try {{
    var el = document.querySelector('[data-ai-id="{eid}"]');
    if (!el) {{
      window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{error:"Element not found: {eid}"}})}}));
      return;
    }}
    el.focus();
    el.click();
    var tag = el.tagName.toLowerCase();
    var text = (el.textContent || el.value || '').replace(/\s+/g,' ').trim().substring(0,60);
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{success:true,clicked:tag,text:text}})}}));
  }} catch(e) {{
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{error:e.message}})}}));
  }}
}})()"#
    )
}

pub fn js_type_text(call_id: &str, element_id: &str, text: &str) -> String {
    let cid = js_escape(call_id);
    let eid = js_escape(element_id);
    let txt = js_escape(text);
    format!(
        r#"(function(){{
  try {{
    var el = document.querySelector('[data-ai-id="{eid}"]');
    if (!el) {{
      window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{error:"Element not found: {eid}"}})}}));
      return;
    }}
    el.focus();
    var nativeInputValueSetter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value');
    if (nativeInputValueSetter && el.tagName === 'INPUT') {{
      nativeInputValueSetter.set.call(el, "{txt}");
    }} else {{
      el.value = "{txt}";
    }}
    el.dispatchEvent(new Event('input', {{bubbles:true}}));
    el.dispatchEvent(new Event('change', {{bubbles:true}}));
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{success:true,typed:"{txt}"}})}}));
  }} catch(e) {{
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{error:e.message}})}}));
  }}
}})()"#
    )
}

pub fn js_scroll_page(call_id: &str, direction: &str, amount: i64) -> String {
    let cid = js_escape(call_id);
    let scroll_cmd = match direction {
        "top" => "window.scrollTo(0, 0);".to_string(),
        "bottom" => "window.scrollTo(0, document.body.scrollHeight);".to_string(),
        "up" => format!("window.scrollBy(0, -{});", amount),
        _ => format!("window.scrollBy(0, {});", amount), // "down"
    };
    format!(
        r#"(function(){{
  try {{
    {scroll_cmd}
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{success:true,scrollY:window.scrollY}})}}));
  }} catch(e) {{
    window.ipc.postMessage(JSON.stringify({{cmd:"ai_tool_result",call_id:"{cid}",result:JSON.stringify({{error:e.message}})}}));
  }}
}})()"#
    )
}

// ── Helper ─────────────────────────────────────────────────────────────────────

/// Minimal JSON-string escape for values we embed directly into JS string literals.
pub fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Human-readable label for a tool call (shown in the AI sidebar).
pub fn tool_call_label(name: &str, args_json: &str) -> String {
    let args: Value = serde_json::from_str(args_json).unwrap_or(json!({}));
    match name {
        "get_page_text" => "Reading page text…".to_string(),
        "get_page_interactive_elements" => "Scanning interactive elements…".to_string(),
        "click_element" => format!(
            "Clicking element {}…",
            args["element_id"].as_str().unwrap_or("?")
        ),
        "type_text" => format!(
            "Typing '{}' into {}...",
            args["text"].as_str().unwrap_or("?"),
            args["element_id"].as_str().unwrap_or("?")
        ),
        "scroll_page" => format!("Scrolling {}…", args["direction"].as_str().unwrap_or("?")),
        "navigate" => format!("Navigating to {}…", args["url"].as_str().unwrap_or("?")),
        "go_back" => "Going back…".to_string(),
        "go_forward" => "Going forward…".to_string(),
        "new_tab" => format!(
            "Opening new tab{}…",
            args["url"]
                .as_str()
                .map(|u| format!(" → {}", u))
                .unwrap_or_default()
        ),
        "list_tabs" => "Listing tabs…".to_string(),
        "switch_tab" => format!(
            "Switching to tab {}…",
            args["tab_id"].as_str().unwrap_or("?")
        ),
        "get_current_url" => "Getting current URL…".to_string(),
        "search_history" => format!(
            "Searching history for '{}'...",
            args["query"].as_str().unwrap_or("?")
        ),
        other => format!("{}…", other),
    }
}
