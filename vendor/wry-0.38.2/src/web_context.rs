// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

#[cfg(gtk)]
use crate::webkitgtk::WebContextImpl;

use std::path::{Path, PathBuf};

/// A context that is shared between multiple [`WebView`]s.
///
/// A browser would have a context for all the normal tabs and a different context for all the
/// private/incognito tabs.
///
/// # Warning
/// If [`WebView`] is created by a WebContext. Dropping `WebContext` will cause [`WebView`] lose
/// some actions like custom protocol on Mac. Please keep both instances when you still wish to
/// interact with them.
///
/// [`WebView`]: crate::WebView
#[derive(Debug)]
pub struct WebContext {
  data: WebContextData,
  #[allow(dead_code)] // It's not needed on Windows and macOS.
  pub(crate) os: WebContextImpl,
}

impl WebContext {
  /// Create a new [`WebContext`].
  ///
  /// `data_directory`:
  /// * Whether the WebView window should have a custom user data path. This is useful in Windows
  ///   when a bundled application can't have the webview data inside `Program Files`.
  pub fn new(data_directory: Option<PathBuf>) -> Self {
    let data = WebContextData { data_directory };
    let os = WebContextImpl::new(&data);
    Self { data, os }
  }

  #[cfg(gtk)]
  pub(crate) fn new_ephemeral() -> Self {
    let data = WebContextData::default();
    let os = WebContextImpl::new_ephemeral();
    Self { data, os }
  }

  /// A reference to the data directory the context was created with.
  pub fn data_directory(&self) -> Option<&Path> {
    self.data.data_directory()
  }

  /// Set if this context allows automation.
  ///
  /// **Note:** This is currently only enforced on Linux, and has the stipulation that
  /// only 1 context allows automation at a time.
  pub fn set_allows_automation(&mut self, flag: bool) {
    self.os.set_allows_automation(flag);
  }
}

impl Default for WebContext {
  fn default() -> Self {
    let data = WebContextData::default();
    let os = WebContextImpl::new(&data);
    Self { data, os }
  }
}

/// Data that all [`WebContext`] share regardless of platform.
#[derive(Default, Debug)]
pub struct WebContextData {
  data_directory: Option<PathBuf>,
}

impl WebContextData {
  /// A reference to the data directory the context was created with.
  pub fn data_directory(&self) -> Option<&Path> {
    self.data_directory.as_deref()
  }
}

#[cfg(not(gtk))]
#[derive(Debug, Default)]
pub(crate) struct WebContextImpl {
  // Windows: the single `CoreWebView2Environment` shared by every WebView created with
  // this context. WebView2's supported (and Microsoft-recommended) model is ONE
  // environment with many controllers. The previous behaviour created a brand-new
  // environment for every WebView against the same user-data folder, which causes random
  // navigation / render failures (blank "black screen" tabs that never load) that get
  // worse as more tabs are opened. Caching it here ties its lifetime to the WebContext:
  // dropping or recreating the context (e.g. to apply new browser args) discards it, and
  // the next WebView build lazily creates a fresh one.
  #[cfg(windows)]
  pub(crate) env:
    Option<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment>,
}

#[cfg(not(gtk))]
impl WebContextImpl {
  fn new(_data: &WebContextData) -> Self {
    Self::default()
  }

  fn set_allows_automation(&mut self, _flag: bool) {}
}
