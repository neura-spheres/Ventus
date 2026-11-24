(() => {
  const waitForReady = async () => {
    try {
      // uBOL answers this only after its startup tasks and script registration finish.
      const config = await chrome.runtime.sendMessage({what: 'getOptionsPageData'});
      if (!config) throw new Error('Background not ready');
      // Leave excepted sites alone; filtering stays on everywhere else.
      const want = __UBOL_SITES__;
      const {none} = await chrome.runtime.sendMessage({what: 'getFilteringModeDetails'});
      for (const hostname of none.filter(host => host !== 'all-urls' && !want.includes(host))) {
        await chrome.runtime.sendMessage({what: 'setFilteringMode', hostname, level: config.defaultFilteringMode});
      }
      for (const hostname of want) {
        await chrome.runtime.sendMessage({what: 'setFilteringMode', hostname, level: 0});
      }
      const scripts = await chrome.scripting.getRegisteredContentScripts();
      const rulesets = await chrome.declarativeNetRequest.getEnabledRulesets();
      const ready = config.hasOmnipotence && config.defaultFilteringMode >= 2 &&
        rulesets.includes('ublock-filters') &&
        scripts.some(script => script.id === 'scriptlets-main' && script.runAt === 'document_start');
      window.ipc.postMessage(JSON.stringify({ready: !!ready, mode: config.defaultFilteringMode,
        broadPermissions: config.hasOmnipotence, rulesets: rulesets.length,
        scripts: scripts.map(script => ({id: script.id, runAt: script.runAt}))}));
    } catch {
      setTimeout(waitForReady, 100);
    }
  };
  window.addEventListener('DOMContentLoaded', waitForReady, {once: true});
})();
