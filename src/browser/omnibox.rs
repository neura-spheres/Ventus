use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::storage::{repositories, settings_store};

const MODEL_KEY: &str = "omnibox_model";
const NUM_FEATURES: usize = 6;
const LEARN_RATE: f64 = 0.18;
const WEIGHT_CLAMP: f64 = 6.0;
const ASSOC_LIMIT: i64 = 800;
const CAND_POOL: usize = 60;
const TREND_LIMIT: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub w: Vec<f64>,
    pub b: f64,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            w: vec![1.4, 2.2, 1.0, 2.6, 1.6, 0.8],
            b: -1.2,
        }
    }
}

impl Model {
    fn ready(&mut self) {
        if self.w.len() != NUM_FEATURES {
            *self = Model::default();
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub url: String,
    pub title: String,
    pub kind: String,
    pub score: f64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub sub: String,
}

#[derive(Debug, Clone, Default)]
pub struct Trend {
    pub title: String,
    pub traffic: i64,
    pub traffic_label: String,
    pub source: String,
    pub context: String,
    pub published: i64,
}

pub fn load(conn: &Connection) -> Model {
    let mut model: Model = settings_store::get(conn, MODEL_KEY)
        .ok()
        .flatten()
        .unwrap_or_default();
    model.ready();
    model
}

fn save(conn: &Connection, model: &Model) {
    let _ = settings_store::set(conn, MODEL_KEY, model);
}

struct Assoc {
    prefix: String,
    url: String,
    picks: i64,
}

fn load_assoc(conn: &Connection) -> Vec<Assoc> {
    let Ok(mut stmt) = conn.prepare("SELECT prefix, url, picks FROM omnibox_learn") else {
        return Vec::new();
    };
    let rows = stmt.query_map([], |row| {
        Ok(Assoc {
            prefix: row.get(0)?,
            url: row.get(1)?,
            picks: row.get(2)?,
        })
    });
    let Ok(rows) = rows else {
        return Vec::new();
    };
    rows.filter_map(|r| r.ok()).collect()
}

struct Learned {
    q_assoc: std::collections::HashMap<String, i64>,
    host_picks: std::collections::HashMap<String, i64>,
    total_picks: i64,
}

fn digest(assoc: &[Assoc], q: &str) -> Learned {
    let mut q_assoc = std::collections::HashMap::new();
    let mut host_picks = std::collections::HashMap::new();
    let mut total_picks = 0;
    for a in assoc {
        total_picks += a.picks;
        let host = host_of(&a.url);
        if !host.is_empty() {
            *host_picks.entry(host).or_insert(0) += a.picks;
        }
        if !q.is_empty() && a.prefix == q {
            *q_assoc.entry(a.url.clone()).or_insert(0) += a.picks;
        }
    }
    Learned {
        q_assoc,
        host_picks,
        total_picks,
    }
}

struct Cand {
    url: String,
    title: String,
    visits: i64,
    last: i64,
    kind: &'static str,
    traffic: i64,
    rank: usize,
    sub: String,
}

pub fn suggest(
    conn: &Connection,
    model: &Model,
    raw_q: &str,
    trends: &[Trend],
    limit: usize,
) -> Vec<Suggestion> {
    let q = raw_q.trim().to_lowercase();
    let assoc = load_assoc(conn);
    let learned = digest(&assoc, &q);

    let mut cands: Vec<Cand> = repositories::history_candidates(conn, &q, CAND_POOL)
        .unwrap_or_default()
        .into_iter()
        .map(|h| Cand {
            url: h.url,
            title: h.title,
            visits: h.visits,
            last: h.last,
            kind: "site",
            traffic: 0,
            rank: 0,
            sub: String::new(),
        })
        .collect();
    add_trends(&mut cands, trends, &q);

    let mut seen: HashSet<String> = cands.iter().map(|c| url_key(&c.url)).collect();
    let now = now_ms();
    for url in learned.q_assoc.keys() {
        let key = url_key(url);
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        cands.push(cand_for(conn, url, now));
    }

    let mut out: Vec<Suggestion> = cands
        .iter()
        .map(|c| {
            let f = features(&q, c, &learned, now);
            let base = sigmoid(dot(&model.w, &f) + model.b);
            Suggestion {
                url: c.url.clone(),
                title: c.title.clone(),
                kind: c.kind.to_string(),
                score: (base + trend_boost(&q, c, now)).min(0.999),
                sub: c.sub.clone(),
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    uniq(out, limit)
}

pub fn learn(conn: &Connection, model: &mut Model, raw_q: &str, chosen: &str, shown: &[String]) {
    let q = raw_q.trim().to_lowercase();
    if q.is_empty() || chosen.is_empty() {
        return;
    }
    model.ready();
    let assoc = load_assoc(conn);
    let learned = digest(&assoc, &q);
    let now = now_ms();

    step(model, &q, chosen, 1.0, &learned, conn, now);
    for url in shown {
        if url == chosen {
            continue;
        }
        step(model, &q, url, 0.0, &learned, conn, now);
    }
    save(conn, model);
    record_pick(conn, &q, chosen, now);
}

fn step(
    model: &mut Model,
    q: &str,
    url: &str,
    label: f64,
    learned: &Learned,
    conn: &Connection,
    now: i64,
) {
    let cand = cand_for(conn, url, now);
    let f = features(q, &cand, learned, now);
    let p = sigmoid(dot(&model.w, &f) + model.b);
    let g = LEARN_RATE * (label - p);
    for i in 0..NUM_FEATURES {
        model.w[i] = (model.w[i] + g * f[i]).clamp(-WEIGHT_CLAMP, WEIGHT_CLAMP);
    }
    model.b = (model.b + g).clamp(-WEIGHT_CLAMP, WEIGHT_CLAMP);
}

fn cand_for(conn: &Connection, url: &str, now: i64) -> Cand {
    if let Ok(Some(h)) = repositories::history_stats(conn, url) {
        return Cand {
            url: h.url,
            title: h.title,
            visits: h.visits,
            last: h.last,
            kind: "site",
            traffic: 0,
            rank: 0,
            sub: String::new(),
        };
    }
    Cand {
        url: url.to_string(),
        title: url.to_string(),
        visits: 0,
        last: now,
        kind: if looks_like_url(url) {
            "site"
        } else {
            "search"
        },
        traffic: 0,
        rank: 0,
        sub: String::new(),
    }
}

fn add_trends(cands: &mut Vec<Cand>, trends: &[Trend], q: &str) {
    let mut seen = HashSet::new();
    let mut added = 0;
    for (i, t) in trends.iter().enumerate() {
        if added >= TREND_LIMIT {
            return;
        }
        let title = t.title.trim();
        if title.is_empty() || !trend_matches(q, title, &t.context, &t.source) {
            continue;
        }
        let key = title.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        let mut sub = if !t.context.is_empty() {
            t.context.clone()
        } else if t.traffic_label.is_empty() {
            "People are searching this".to_string()
        } else {
            format!("{} searches", t.traffic_label)
        };
        if !t.source.is_empty() && !sub.contains(&t.source) {
            sub = format!("{sub} · {}", t.source);
        }
        cands.push(Cand {
            url: title.to_string(),
            title: title.to_string(),
            visits: 0,
            last: t.published,
            kind: "trend",
            traffic: t.traffic,
            rank: i,
            sub,
        });
        added += 1;
    }
}

fn trend_matches(q: &str, title: &str, sub: &str, source: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let text = format!("{title} {sub} {source}").to_lowercase();
    let parts: Vec<&str> = q.split_whitespace().collect();
    if parts.is_empty() {
        return true;
    }
    parts.iter().all(|p| text.contains(p))
}

fn record_pick(conn: &Connection, q: &str, url: &str, now: i64) {
    let _ = conn.execute(
        "INSERT INTO omnibox_learn(prefix, url, picks, last_pick) VALUES(?1, ?2, 1, ?3)
         ON CONFLICT(prefix, url) DO UPDATE SET picks = picks + 1, last_pick = ?3",
        params![q, url, now],
    );
    let _ = conn.execute(
        "DELETE FROM omnibox_learn WHERE rowid NOT IN (
            SELECT rowid FROM omnibox_learn ORDER BY picks DESC, last_pick DESC LIMIT ?1
        )",
        params![ASSOC_LIMIT],
    );
}

fn features(q: &str, c: &Cand, learned: &Learned, now: i64) -> [f64; NUM_FEATURES] {
    let bare = strip_scheme(&c.url);
    let host = host_of(&c.url);
    let title_l = c.title.to_lowercase();
    let age_days = ((now - c.last).max(0) as f64) / 86_400_000.0;

    let prefix_hit = !q.is_empty() && (bare.starts_with(q) || host.starts_with(q));
    let substr_hit = !q.is_empty() && !prefix_hit && (bare.contains(q) || title_l.contains(q));
    let host_affinity = if learned.total_picks > 0 && !host.is_empty() {
        *learned.host_picks.get(&host).unwrap_or(&0) as f64 / learned.total_picks as f64
    } else {
        0.0
    };

    [
        norm(c.visits as f64 * 0.5),
        if prefix_hit { 1.0 } else { 0.0 },
        if substr_hit { 1.0 } else { 0.0 },
        norm(*learned.q_assoc.get(&c.url).unwrap_or(&0) as f64 * 1.5),
        host_affinity,
        (-age_days / 7.0).exp(),
    ]
}

fn dot(w: &[f64], f: &[f64; NUM_FEATURES]) -> f64 {
    w.iter().zip(f.iter()).map(|(a, b)| a * b).sum()
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn norm(x: f64) -> f64 {
    x / (1.0 + x)
}

fn trend_boost(q: &str, c: &Cand, now: i64) -> f64 {
    if c.kind != "trend" {
        return 0.0;
    }
    let title = c.title.to_lowercase();
    let traffic = norm((c.traffic.max(0) as f64).ln_1p() * 0.8);
    let age_days = ((now - c.last).max(0) as f64) / 86_400_000.0;
    let fresh = (-age_days / 2.0).exp();
    let rank = 1.0 / (1.0 + c.rank as f64 * 0.16);
    let hit = if q.is_empty() {
        0.08
    } else if title.starts_with(q) {
        0.36
    } else if trend_matches(q, &title, &c.sub, "") {
        0.28
    } else {
        0.0
    };
    hit + traffic * 0.1 + fresh * 0.08 + rank * 0.08
}

fn uniq(items: Vec<Suggestion>, limit: usize) -> Vec<Suggestion> {
    let mut out = Vec::new();
    let mut urls = HashSet::new();
    let mut rows = HashSet::new();
    for item in items {
        let uk = url_key(&item.url);
        let rk = row_key(&item);
        if urls.contains(&uk) || rows.contains(&rk) {
            continue;
        }
        urls.insert(uk);
        rows.insert(rk);
        out.push(item);
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn row_key(s: &Suggestion) -> String {
    if s.kind == "search" || s.kind == "trend" {
        return format!("search:{}", s.url.trim().to_lowercase());
    }
    let title = s.title.trim().to_lowercase();
    let host = host_of(&s.url);
    if !title.is_empty() && !host.is_empty() {
        return format!("site:{host}:{title}");
    }
    url_key(&s.url)
}

fn url_key(raw: &str) -> String {
    let trimmed = raw.trim();
    let Ok(url) = url::Url::parse(trimmed) else {
        let lower = trimmed.trim_end_matches('/').to_lowercase();
        return lower.trim_start_matches("www.").to_string();
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        let lower = trimmed.trim_end_matches('/').to_lowercase();
        return lower.trim_start_matches("www.").to_string();
    }
    let host = url.host_str().unwrap_or("").to_lowercase();
    let host = host.trim_start_matches("www.");
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    let path = url.path().trim_end_matches('/');
    let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
    format!("{host}{port}{path}{query}")
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn looks_like_url(s: &str) -> bool {
    s.contains("://") || (!s.contains(' ') && s.contains('.'))
}

fn strip_scheme(url: &str) -> String {
    let lower = url.to_lowercase();
    let no_scheme = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .unwrap_or(&lower);
    no_scheme
        .strip_prefix("www.")
        .unwrap_or(no_scheme)
        .to_string()
}

fn host_of(url: &str) -> String {
    if !looks_like_url(url) {
        return String::new();
    }
    let bare = strip_scheme(url);
    bare.split(['/', '?', '#']).next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_hides_same_visible_site() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                workspace_id TEXT,
                visited_at INTEGER NOT NULL
            );",
        )
        .unwrap();
        let now = now_ms();
        conn.execute(
            "INSERT INTO history(url, title, workspace_id, visited_at) VALUES(?1, ?2, NULL, ?3)",
            params!["https://claude.ai/new", "New chat - Claude", now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO history(url, title, workspace_id, visited_at) VALUES(?1, ?2, NULL, ?3)",
            params![
                "https://claude.ai/chat/abc",
                "New chat - Claude",
                now - 1000
            ],
        )
        .unwrap();

        let items = suggest(&conn, &Model::default(), "claude", &[], 8);
        let count = items
            .iter()
            .filter(|s| s.title == "New chat - Claude" && host_of(&s.url) == "claude.ai")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn suggest_uses_matching_trends() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                workspace_id TEXT,
                visited_at INTEGER NOT NULL
            );",
        )
        .unwrap();
        let trends = vec![Trend {
            title: "AI news".to_string(),
            traffic: 2000,
            traffic_label: "2,000+".to_string(),
            source: "Example News".to_string(),
            context: "Why AI is moving fast".to_string(),
            published: now_ms(),
        }];
        let items = suggest(&conn, &Model::default(), "ai", &trends, 8);
        assert!(items
            .iter()
            .any(|s| s.kind == "trend" && s.title == "AI news"));
        assert!(items
            .iter()
            .any(|s| s.kind == "trend" && s.sub.contains("Why AI is moving fast")));
    }

    #[test]
    fn suggest_limits_trends() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                workspace_id TEXT,
                visited_at INTEGER NOT NULL
            );",
        )
        .unwrap();
        let trends: Vec<Trend> = (0..6)
            .map(|i| Trend {
                title: format!("Trend {i}"),
                traffic: 1000,
                traffic_label: "1,000+".to_string(),
                source: String::new(),
                context: String::new(),
                published: now_ms(),
            })
            .collect();
        let items = suggest(&conn, &Model::default(), "", &trends, 8);
        let count = items.iter().filter(|s| s.kind == "trend").count();
        assert_eq!(count, TREND_LIMIT);
    }

    #[test]
    fn suggest_matches_trend_context() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                title TEXT NOT NULL DEFAULT '',
                workspace_id TEXT,
                visited_at INTEGER NOT NULL
            );",
        )
        .unwrap();
        let trends = vec![Trend {
            title: "AI news".to_string(),
            traffic: 2000,
            traffic_label: "2,000+".to_string(),
            source: "Example News".to_string(),
            context: "Why AI is moving fast".to_string(),
            published: now_ms(),
        }];
        let items = suggest(&conn, &Model::default(), "moving fast", &trends, 8);
        assert!(items
            .iter()
            .any(|s| s.kind == "trend" && s.title == "AI news"));
    }
}
