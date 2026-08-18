# LinkedIn Project Copy

## Recommended title

Ventus - AI-Powered Desktop Browser for Windows

## Description

Ventus is a native Windows desktop browser I designed and built with Rust, Tao, Wry, and Microsoft Edge WebView2. Instead of using a standard browser window, it has a custom frameless interface with layered WebViews, native window management, and a Rust-owned application runtime.

The browser includes workspaces, tabs and pinned tabs, bookmarks, history, downloads, search shortcuts, a personalized new-tab experience, privacy controls, built-in ad blocking, and an AI sidebar that supports OpenAI, OpenRouter, Anthropic, and local Ollama models.

I also built the local data layer with SQLite, protected API credentials with the Windows keychain, handled async services with Tokio, and created the Windows installer and update workflow. The most interesting engineering challenge was coordinating Rust, JavaScript, WebView2, and Win32 clipping and z-order so the custom browser chrome and web content stay fast, responsive, and fully clickable.

Ventus is an active open-source project focused on creating a clean, practical browser that keeps browsing, organization, and AI assistance in one desktop experience.

## Short description

A native Windows browser built with Rust, Tao, Wry, WebView2, and SQLite. Ventus combines a custom frameless interface, workspaces, local-first browser data, built-in privacy tools, and a multi-provider AI sidebar in one focused desktop experience.

## Skills

Rust · Desktop Application Development · WebView2 · Win32 · Tao · Wry · SQLite · Tokio · JavaScript · HTML/CSS · API Integration · Software Architecture · UI/UX Design

## Project URL

https://github.com/neura-spheres/Ventus

## Suggested post caption

I have been building Ventus, a native Windows desktop browser with AI built in.

The project started as an experiment in creating a cleaner browser experience, but it quickly became a deep engineering challenge across Rust, WebView2, Win32 window management, JavaScript, local persistence, downloads, privacy controls, and AI provider integrations.

Ventus now has a custom frameless browser shell, workspaces, tabs, bookmarks, history, downloads, built-in ad blocking, and an AI sidebar that works with OpenAI, OpenRouter, Anthropic, and Ollama.

One of my favorite parts of the project is its layered architecture: Rust owns the native runtime and application state, a transparent chrome WebView renders the interface, and separate WebView2 instances render tab content. Making those layers feel like one responsive browser taught me a lot about desktop UI architecture and the edge cases behind software people use every day.

The project is still evolving, but I am proud of how far it has come.

https://github.com/neura-spheres/Ventus

#Rust #WindowsDevelopment #WebView2 #DesktopApp #OpenSource #AI #SoftwareEngineering

