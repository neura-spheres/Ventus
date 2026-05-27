/// Fallback CSS injected at document-start before filter lists are downloaded.
/// Once the full adblock-rust engine is loaded, per-page cosmetic rules from
/// `engine.url_cosmetic_resources()` are injected additionally via evaluate_script.
pub static FALLBACK_CSS: &str = concat!(
    // Google AdSense
    ".adsbygoogle,ins.adsbygoogle{display:none!important}",
    "[data-ad-client],[data-ad-slot],[data-google-query-id]{display:none!important}",
    ".google-auto-placed,.GoogleActiveViewElement{display:none!important}",
    // Google Ad Manager / DFP
    "[id^='div-gpt-ad'],[id*='google_ads_iframe'],[id*='google_ads_frame']{display:none!important}",
    // Carbon Ads
    "#carbonads,.carbon-ads,#carbon-block,.carbon-ad{display:none!important}",
    // DFP / GAM
    ".dfp-ad,[id*='dfp-ad'],[class*='dfp-ad']{display:none!important}",
    // Taboola
    "[id*='taboola'],[class*='taboola'],.trc_related_container,.taboola-recommended{display:none!important}",
    // Outbrain
    "[id*='outbrain'],[class*='outbrain'],.ob_widget,.OUTBRAIN{display:none!important}",
    // Criteo
    ".criteo-collapse,.criteo-placeholder,[id*='criteo']{display:none!important}",
    // AdSense patterns
    "[class*='adsense']:not(script){display:none!important}",
    "[id*='adsense']:not(script):not(meta){display:none!important}",
    // Generic ad wrappers
    "div[class='ad'],div[id='ad'],div[class='ads'],div[id='ads']{display:none!important}",
    "#ad-container,#ad-wrapper,.ad-container,.ad-wrapper,.ad-slot,.ad-unit,.ad-space,.ad-area,.ad-box{display:none!important}",
    ".ads-container,.ads-wrapper,.ads-area,.ads-slot,.ads-unit{display:none!important}",
    "[class*='advertisement']:not(article):not(section):not(main):not(p){display:none!important}",
    "[id*='advertisement']:not(article):not(section):not(main){display:none!important}",
    // Indonesian
    "[class*='iklan'],[id*='iklan']{display:none!important}",
    // Banner / sticky / floating
    "[class*='banner-ad'],[class*='ad-banner'],[id*='banner-ad'],[id*='ad-banner']{display:none!important}",
    "[class*='adslot'],[id*='adslot']{display:none!important}",
    "[data-ad],[data-ads],[data-advertisement],[data-ad-unit]{display:none!important}",
    "#advertisement,.advertisement:not(article):not(section):not(p){display:none!important}",
    ".sponsored-content:not(article),.sponsored-label,.sponsored-wrapper{display:none!important}",
    "[id*='sticky-ad'],[class*='sticky-ad'],[id*='floating-ad'],[class*='floating-ad']{display:none!important}",
    "[id*='interstitial'],[class*='interstitial-ad']{display:none!important}",
    // iFrame ad units by src domain
    "iframe[src*='doubleclick.net'],iframe[src*='googlesyndication.com'],",
    "iframe[src*='adnxs.com'],iframe[src*='rubiconproject.com'],",
    "iframe[src*='openx.net'],iframe[src*='pubmatic.com'],",
    "iframe[src*='taboola.com'],iframe[src*='outbrain.com']{display:none!important}",
);

/// Fallback selector list used by the MutationObserver before the full engine is ready.
pub static FALLBACK_SELECTORS: &[&str] = &[
    ".adsbygoogle",
    "ins.adsbygoogle",
    "[data-ad-client]",
    "[data-ad-slot]",
    "[data-google-query-id]",
    ".google-auto-placed",
    "[id^='div-gpt-ad']",
    "[id*='google_ads_iframe']",
    "#carbonads",
    ".carbon-ads",
    "[id*='taboola']",
    "[class*='taboola']",
    ".trc_related_container",
    "[id*='outbrain']",
    "[class*='outbrain']",
    ".ob_widget",
    ".OUTBRAIN",
    ".criteo-placeholder",
    "[id*='criteo']",
    ".ad-slot",
    ".ad-unit",
    ".ad-container",
    ".ad-wrapper",
    ".ad-space",
    ".ad-area",
    ".ad-box",
    ".ads-container",
    ".ads-wrapper",
    "#advertisement",
    "[class*='advertisement']:not(article):not(section):not(main):not(p)",
    "[id*='advertisement']:not(article):not(section):not(main)",
    "[class*='iklan']",
    "[id*='iklan']",
    "[class*='banner-ad']",
    "[class*='ad-banner']",
    "[class*='adslot']",
    "[data-ad]",
    "[data-advertisement]",
    "[class*='adsense']:not(script)",
    ".sponsored-label",
    ".sponsored-wrapper",
    "[class*='sticky-ad']",
    "[class*='floating-ad']",
];
