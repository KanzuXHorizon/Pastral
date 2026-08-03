# Product

<!-- impeccable:product-schema 1 -->

## Platform

windows

The design-skill schema does not define a Windows value; this repository extends it explicitly because Pastral is a Windows 11-native desktop product, not a web, iOS, Android, or cross-platform adaptive product.

## Users

Primary users are Windows power users who repeatedly copy text, code, links, images, files, and formatted content and need to recover or paste it later without disrupting their current work.

Important contexts include:

- keyboard-first developers using IDEs, terminals, browsers, Win32, WinForms, WPF, and custom controls;
- researchers and students organizing long-lived source-aware material;
- privacy-sensitive professionals handling credentials and confidential content;
- designers/content workers preserving rich and image representations;
- users relying on keyboard navigation, screen readers, magnification, contrast themes, text scaling, touch, or reduced motion.

## Product Purpose

Pastral is a private, searchable, source-aware memory of what the user copies, combined with a high-fidelity paste engine, profiles, deterministic rules, filters, and calm native Windows interaction.

Success means the user can forget Pastral is running, find a clip copied weeks earlier, understand how it was handled, and paste the best safe representation the destination accepts.

## Positioning

Pastral's distinctive mechanism is one immutable logical clip event containing multiple preserved representations, source/profile context, honest fidelity, and reversible deterministic behavior—implemented as a small Windows-native agent rather than a generic cross-platform clipboard list or AI transformer.

## Operating Context

Pastral runs in the interactive Windows user session and observes clipboard-change notifications. It must coexist with text selection, menus, typing, fullscreen applications, games, presentations, screen sharing, remote desktop, multiple monitors, mixed DPI, battery saver, session lock, suspend/resume, low disk, and application-specific clipboard behavior.

Primary surfaces:

- passive copy confirmation overlay;
- explicitly invoked Quick Paste;
- on-demand manager;
- tray and global hotkeys;
- CLI and diagnostics.

## Capabilities and Constraints

Confirmed capabilities and constraints:

- Windows 11 only; x64 first;
- Rust resident agent with a responsive control/overlay message thread and a dedicated clipboard-platform STA for foreign Win32/OLE capture objects/media and Pastral replay-object publication/lifetime;
- C++/WinRT and WinUI 3 manager;
- event-driven capture with no clipboard polling;
- immutable originals and derived representations;
- SQLite/FTS5 plus one recoverable content-addressed blob contract whose internal/external physical placement is benchmark-selected;
- profiles and explainable deterministic rules;
- local-first, network-silent core;
- no mandatory AI, OCR, semantic search, account, or cloud sync;
- no primary WebView/Electron/Tauri UI;
- passive overlay must never steal focus or intercept ordinary typing;
- highly confident secrets are not stored by default;
- default retention is 90 days with a 5 GB automatic-cleanup target for ordinary unpinned history; pinned/protected clips may exceed it with visible warnings;
- source-owned history hard deny creates no durable record; high-confidence secret skips create a hidden content-free 24-hour audit by default;
- the built-in Private profile is unavailable until mandatory encrypted storage, random blob identity, non-indexing, lock, and recovery gates pass;
- local IPC strongly isolates users/logon sessions but is not represented as a secure enclave against fully compromised same-user code;
- Quick Paste is hosted by the single-instance on-demand manager, with cold and warm activation measured separately;
- all speed, fidelity, security, and footprint claims require measurement.

## Brand Commitments

- Internal provisional codename: Pastral; confusing similarity with the clipboard product `Pastry` requires formal review before public branding.
- Tagline: “Copy once. Find anything. Paste perfectly.” The last phrase is aspirational and must not be presented as universal losslessness.
- Brand concept: a reliable trail connecting copy, preservation, search, transformation, paste, and recall.
- Core colors: Aurora Violet `#725CFF`, Signal Cyan `#2ED3FF`, Night Ink `#0B1020`, Frost `#F7F8FC`, Success Mint `#35D49A`, Warning Amber `#F2B84B`.
- The violet-to-cyan gradient is reserved for the mark and rare brand moments; system colors and user accent govern most interaction.
- Logo should combine two geometric ribbon/document shapes, imply `P` and continuity, use negative space for a sheet/cursor, and remain recognizable in monochrome and at tray size.
- Avoid literal clipboard boards, paperclips, lightning bolts, AI sparkles, excessive gradients, and generic rounded SaaS marks.
- Use Segoe UI Variable from Windows; do not redistribute font files.

## Evidence on Hand

- A detailed product and engineering specification approved by the project owner.
- Three supplied concept boards showing a dark Windows productivity utility, compact copy confirmation, Quick Paste/history manager, profiles, and violet/cyan branding.
- The concept boards are visual references only. Their generated labels, icons, and technical text are not factual product copy or component specifications.
- No validated production UI, usability study, customer evidence, benchmark, final logo, signed package, Store identity, or legal trademark clearance exists yet. Future work must not fabricate these.

## Product Principles

1. Preserve original data and provenance.
2. Never interrupt copy, focus, selection, or typing.
3. Keep core behavior local, deterministic, inspectable, and reversible.
4. Remain effectively idle until an event or explicit task occurs.
5. Be honest about fidelity, privacy, compatibility, and measured performance.

## Accessibility & Inclusion

Core flows must support keyboard-only use, UI Automation and screen readers, high contrast/contrast themes, text scaling, 100–300% DPI, reduced motion, transparency disabled, RTL/long localization, touch targets, RDP, and users who need quiet/non-disruptive confirmation. Color and motion never carry meaning alone.
