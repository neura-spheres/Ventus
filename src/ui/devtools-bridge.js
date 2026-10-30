(() => {
  const NativeWebSocket = window.WebSocket;
  const sockets = new Set();
  const queued = [];
  class InspectorSocket extends EventTarget {
    static CONNECTING = 0;
    static OPEN = 1;
    static CLOSING = 2;
    static CLOSED = 3;
    constructor(url) {
      super();
      this.url = String(url);
      this.readyState = 0;
      this.bufferedAmount = 0;
      this.extensions = '';
      this.protocol = '';
      this.binaryType = 'blob';
      sockets.add(this);
      queueMicrotask(() => {
        this.readyState = 1;
        this.emit('open', new Event('open'));
        for (const data of queued.splice(0)) this.emit('message', new MessageEvent('message', {data}));
      });
    }
    emit(name, event) {
      this.dispatchEvent(event);
      if (typeof this['on' + name] === 'function') this['on' + name](event);
    }
    send(data) {
      if (this.readyState !== 1) throw new DOMException('Inspector connection is closed', 'InvalidStateError');
      window.ipc.postMessage(JSON.stringify({type: 'protocol', message: String(data)}));
    }
    close() {
      this.readyState = 3;
      sockets.delete(this);
      this.emit('close', new CloseEvent('close', {wasClean: true, code: 1000}));
    }
  }
  window.WebSocket = new Proxy(NativeWebSocket, {
    construct(Target, args) {
      const url = new URL(String(args[0]), location.href);
      return url.hostname === 'ventus-devtools' ? new InspectorSocket(url.href) : new Target(...args);
    },
  });
  window.__ventusDevtoolsReceive = message => {
    const data = typeof message === 'string' ? message : JSON.stringify(message);
    if (!sockets.size) { queued.push(data); return; }
    for (const socket of sockets) {
      if (socket.readyState === 1) socket.emit('message', new MessageEvent('message', {data}));
      else queued.push(data);
    }
  };
  window.addEventListener('DOMContentLoaded', async () => {
    try {
      const UI = await import('./ui/legacy/legacy.js');
      const SDK = await import('./core/sdk/sdk.js');
      const Common = await import('./core/common/common.js');
      const ready = async () => {
        try {
          // Frontend singletons are initialized after DOMContentLoaded.
          const target = SDK.TargetManager.TargetManager.instance().primaryPageTarget();
          if (!target) throw new Error('Waiting for inspected page');
          Common.Settings.Settings.instance().createSetting('screencast-enabled', false).set(false);
          UI.DockController.DockController.instance().setDockSide('undocked');
          await UI.InspectorView.InspectorView.instance().showPanel('elements');
          await target.model(SDK.DOMModel.DOMModel).requestDocument();
          window.ipc.postMessage(JSON.stringify({type: 'ready'}));
        } catch { setTimeout(ready, 100); }
      };
      ready();
    } catch (error) { console.error('Ventus inspector initialization:', error); }
  }, {once: true});
})();
