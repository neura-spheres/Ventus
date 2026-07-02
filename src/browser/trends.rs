use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::browser::omnibox::Trend;

pub async fn fetch(region: &str) -> Result<Vec<Trend>> {
    let region = clean_region(region);
    let url = format!("https://trends.google.com/trending/rss?geo={region}");
    let text = reqwest::Client::builder()
        .user_agent(crate::version::USER_AGENT)
        .timeout(std::time::Duration::from_secs(8))
        .build()?
        .get(url)
        .send()
        .await?
        .text()
        .await?;
    Ok(parse(&text))
}

pub fn clean_region(region: &str) -> String {
    let r = region.trim().to_uppercase();
    if r.len() == 2 && r.chars().all(|c| c.is_ascii_uppercase()) {
        return r;
    }
    "US".to_string()
}

fn parse(xml: &str) -> Vec<Trend> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in doc.descendants().filter(|n| n.has_tag_name("item")) {
        let title = child_text(item, "title");
        if title.is_empty() {
            continue;
        }
        let traffic_label = child_text(item, "approx_traffic");
        let traffic = traffic_count(&traffic_label);
        let source = item
            .descendants()
            .find(|n| n.has_tag_name("news_item_source"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim()
            .to_string();
        let context = item
            .descendants()
            .find(|n| n.has_tag_name("news_item_title"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim()
            .to_string();
        let published = child_text(item, "pubDate")
            .parse::<DateTime<Utc>>()
            .map(|d| d.timestamp_millis())
            .or_else(|_| {
                DateTime::parse_from_rfc2822(&child_text(item, "pubDate"))
                    .map(|d| d.timestamp_millis())
            })
            .unwrap_or_else(|_| Utc::now().timestamp_millis());
        out.push(Trend {
            title,
            traffic,
            traffic_label,
            source,
            context,
            published,
        });
    }
    out
}

fn child_text(node: roxmltree::Node, name: &str) -> String {
    node.children()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn traffic_count(s: &str) -> i64 {
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trends_rss() {
        let xml = r#"
        <rss xmlns:ht="https://trends.google.com/trending/rss">
          <channel>
            <item>
              <title>AI news</title>
              <ht:approx_traffic>2,000+</ht:approx_traffic>
              <pubDate>Fri, 12 Jun 2026 21:40:00 -0700</pubDate>
              <ht:news_item>
                <ht:news_item_source>Example News</ht:news_item_source>
                <ht:news_item_title>Why AI is moving fast</ht:news_item_title>
              </ht:news_item>
            </item>
          </channel>
        </rss>
        "#;
        let trends = parse(xml);
        assert_eq!(trends.len(), 1);
        assert_eq!(trends[0].title, "AI news");
        assert_eq!(trends[0].traffic, 2000);
        assert_eq!(trends[0].traffic_label, "2,000+");
        assert_eq!(trends[0].source, "Example News");
        assert_eq!(trends[0].context, "Why AI is moving fast");
    }

    #[test]
    fn region_is_strict() {
        assert_eq!(clean_region("id"), "ID");
        assert_eq!(clean_region(" usa "), "US");
        assert_eq!(clean_region("1D"), "US");
    }
}
