/// Curated list of ad-serving and tracker domains.
/// Domain matching is hierarchical: blocking "doubleclick.net" also blocks
/// "ad.doubleclick.net", "cm.g.doubleclick.net", etc.
pub static BLOCKED_DOMAINS: &[&str] = &[
    // ── Google Ad Network ──────────────────────────────────────────────────
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "2mdn.net",
    // ── Amazon Advertising ────────────────────────────────────────────────
    "amazon-adsystem.com",
    "advertising.amazon.com",
    "fls-na.amazon.com",
    "fls-eu.amazon.com",
    // ── AppNexus / Xandr (Microsoft) ──────────────────────────────────────
    "adnxs.com",
    "appnexus.com",
    // ── The Trade Desk ────────────────────────────────────────────────────
    "adsrvr.org",
    "thetradedesk.com",
    // ── Criteo ────────────────────────────────────────────────────────────
    "criteo.com",
    "criteo.net",
    "hlserve.com",
    // ── OpenX ─────────────────────────────────────────────────────────────
    "openx.net",
    "openx.com",
    // ── Magnite / Rubicon Project ─────────────────────────────────────────
    "rubiconproject.com",
    "magnite.com",
    // ── PubMatic ──────────────────────────────────────────────────────────
    "pubmatic.com",
    // ── Index Exchange ────────────────────────────────────────────────────
    "casalemedia.com",
    "indexww.com",
    // ── Outbrain ──────────────────────────────────────────────────────────
    "outbrain.com",
    "outbrainimg.com",
    // ── Taboola ───────────────────────────────────────────────────────────
    "taboola.com",
    // ── Media.net (Yahoo-Bing Network) ────────────────────────────────────
    "media.net",
    // ── ShareThrough ──────────────────────────────────────────────────────
    "sharethrough.com",
    // ── TripleLift ────────────────────────────────────────────────────────
    "triplelift.com",
    // ── BidSwitch ─────────────────────────────────────────────────────────
    "bidswitch.net",
    // ── Sovrn ─────────────────────────────────────────────────────────────
    "sovrn.com",
    "lijit.com",
    // ── 33Across ──────────────────────────────────────────────────────────
    "33across.com",
    "tynt.com",
    // ── YieldMo ───────────────────────────────────────────────────────────
    "yieldmo.com",
    // ── ID5 (Identity) ────────────────────────────────────────────────────
    "id5-sync.com",
    // ── LiveIntent ────────────────────────────────────────────────────────
    "liveintent.com",
    // ── LiveRamp ──────────────────────────────────────────────────────────
    "liveramp.com",
    "liveramp.net",
    // ── SpotX ─────────────────────────────────────────────────────────────
    "spotxchange.com",
    "spotx.tv",
    // ── Connatix ──────────────────────────────────────────────────────────
    "connatix.com",
    // ── Smart AdServer ────────────────────────────────────────────────────
    "smartadserver.com",
    // ── SmartClip ─────────────────────────────────────────────────────────
    "smartclip.net",
    // ── Yieldlab ──────────────────────────────────────────────────────────
    "yieldlab.de",
    "yieldlab.net",
    // ── AdYouLike ─────────────────────────────────────────────────────────
    "adyoulike.com",
    // ── InMobi ────────────────────────────────────────────────────────────
    "inmobi.com",
    "inmobicdn.net",
    // ── MOAT Analytics (Oracle) ───────────────────────────────────────────
    "moat.com",
    "moatads.com",
    // ── Integral Ad Science (IAS) ─────────────────────────────────────────
    "adsafeprotected.com",
    "iasds01.com",
    // ── DoubleVerify ──────────────────────────────────────────────────────
    "doubleverify.com",
    "dvtps.com",
    // ── Lotame ────────────────────────────────────────────────────────────
    "crwdcntrl.net",
    "lotame.com",
    // ── BlueKai (Oracle DMP) ──────────────────────────────────────────────
    "bluekai.com",
    "bkrtx.com",
    // ── Krux (Salesforce DMP) ─────────────────────────────────────────────
    "krxd.net",
    // ── Demdex / Adobe Audience Manager ──────────────────────────────────
    "demdex.net",
    "omtrdc.net",
    // ── Amobee (formerly Turn) ────────────────────────────────────────────
    "turn.com",
    "amobee.com",
    // ── AdForm ────────────────────────────────────────────────────────────
    "adform.net",
    "adform.com",
    // ── Flashtalking / Sizmek ─────────────────────────────────────────────
    "serving-sys.com",
    "flashtalking.com",
    "mediamind.com",
    "sizmek.com",
    // ── Verizon / Oath / AOL Advertising ─────────────────────────────────
    "advertising.com",
    "adtech.de",
    "tacoda.net",
    "adsonar.com",
    "adap.tv",
    // ── Quantcast ─────────────────────────────────────────────────────────
    "quantserve.com",
    "quantcount.com",
    // ── Comscore / Scorecard Research ─────────────────────────────────────
    "scorecardresearch.com",
    "comscore.com",
    "imrworldwide.com",
    // ── AddThis / ShareThis ───────────────────────────────────────────────
    "addthis.com",
    "addthisedge.com",
    "sharethis.com",
    // ── Conversant / Commission Junction ─────────────────────────────────
    "rfihub.com",
    "rfihub.net",
    // ── LiveRamp (pixel tracking) ─────────────────────────────────────────
    "rlcdn.com",
    // ── AdRoll ────────────────────────────────────────────────────────────
    "adroll.com",
    "d.adroll.com",
    // ── Tapad ─────────────────────────────────────────────────────────────
    "tapad.com",
    // ── Acuity Ads ────────────────────────────────────────────────────────
    "acuityads.com",
    // ── Telaria / Tremor Video ────────────────────────────────────────────
    "telaria.com",
    "tremorvideo.com",
    "tremorhub.com",
    // ── Primis (formerly YuMe) ────────────────────────────────────────────
    "primis.tech",
    // ── Engageya ──────────────────────────────────────────────────────────
    "engageya.com",
    // ── MediaMath ─────────────────────────────────────────────────────────
    "mediamath.com",
    // ── Semasio ───────────────────────────────────────────────────────────
    "semasio.net",
    "semasio.com",
    // ── Eyeota ────────────────────────────────────────────────────────────
    "eyeota.net",
    "eyeota.com",
    // ── Ogury ─────────────────────────────────────────────────────────────
    "ogury.com",
    // ── Zeotap ────────────────────────────────────────────────────────────
    "zeotap.com",
    // ── Yahoo Display Ads ─────────────────────────────────────────────────
    "yimg.com",
    // ── GumGum ────────────────────────────────────────────────────────────
    "gumgum.com",
    // ── Nativo ────────────────────────────────────────────────────────────
    "nativo.com",
    // ── Kargo ─────────────────────────────────────────────────────────────
    "kargo.com",
    // ── PulsePoint / Context Web ──────────────────────────────────────────
    "contextweb.com",
    // ── Unruly ────────────────────────────────────────────────────────────
    "unrulymedia.com",
    // ── Beachfront ────────────────────────────────────────────────────────
    "beachfront.com",
    "beachfront.io",
    // ── 4INFO ─────────────────────────────────────────────────────────────
    "4info.com",
    // ── Samba TV ──────────────────────────────────────────────────────────
    "samba.tv",
    // ── IPONWEB ───────────────────────────────────────────────────────────
    "iponweb.net",
    // ── EMX Digital ───────────────────────────────────────────────────────
    "emxdigital.com",
    // ── RhythmOne ─────────────────────────────────────────────────────────
    "rhythmone.com",
    // ── Dataxu (Roku) ─────────────────────────────────────────────────────
    "dataxu.com",
    // ── Peer39 ────────────────────────────────────────────────────────────
    "peer39.net",
    // ── AdKernel ──────────────────────────────────────────────────────────
    "adkernel.com",
    // ── Native/Content Recommendation Networks ────────────────────────────
    "revcontent.com",
    "mgid.com",
    "zergnet.com",
    "vidoomy.com",
    "nativeads.com",
    // ── Mobile Ad Networks ────────────────────────────────────────────────
    "mopub.com",
    "vungle.com",
    "chartboost.com",
    "smaato.com",
    "smaato.net",
    "loopme.com",
    // ── Social Tracking Pixels ────────────────────────────────────────────
    "connect.facebook.net",
    "ads-twitter.com",
    // ── Cross-site Fingerprinting ─────────────────────────────────────────
    "ioam.de",
    "2o7.net",
    "mxptint.net",
    // ── Popup/Popunder Networks ───────────────────────────────────────────
    "popads.net",
    "popcash.net",
    "adsterra.com",
    "propellerads.com",
    "propellerclick.com",
    "hilltopads.net",
    "trafficjunky.com",
    "trafficjunky.net",
    "trafficstars.com",
    "exoclick.com",
    "clickadu.com",
    "ero-advertising.com",
    "juicyadscpm.com",
    // ── SpringServe (CTV/OTT ads) ─────────────────────────────────────────
    "springserve.com",
    // ── Vidoomy ───────────────────────────────────────────────────────────
    "vidoomy.com",
    // ── Ad Maven ──────────────────────────────────────────────────────────
    "ad-maven.com",
    "admaven.com",
    // ── Neustar ───────────────────────────────────────────────────────────
    "neustar.biz",
];
