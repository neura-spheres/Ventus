pub struct Shortcut {
    pub key: &'static str,
    pub description: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        key: "Ctrl+T",
        description: "New tab",
    },
    Shortcut {
        key: "Ctrl+W",
        description: "Close tab",
    },
    Shortcut {
        key: "Ctrl+L",
        description: "Focus address bar",
    },
    Shortcut {
        key: "Ctrl+K",
        description: "Search tabs",
    },
    Shortcut {
        key: "Ctrl+Shift+A",
        description: "Toggle AI sidebar",
    },
    Shortcut {
        key: "Ctrl+,",
        description: "Settings",
    },
    Shortcut {
        key: "Ctrl+R / F5",
        description: "Reload",
    },
    Shortcut {
        key: "Alt+Left",
        description: "Back",
    },
    Shortcut {
        key: "Alt+Right",
        description: "Forward",
    },
    Shortcut {
        key: "Ctrl+Shift+T",
        description: "Reopen closed tab",
    },
    Shortcut {
        key: "F12",
        description: "DevTools",
    },
];
