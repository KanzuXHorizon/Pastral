# Pastral Manager Brand and Localization Design

## Goal

Turn the existing WinUI 3 manager into a recognizable, modern Windows productivity utility without weakening native behavior, accessibility, privacy boundaries, or release honesty.

## Scope

This pass covers the existing manager shell, Home, History, shared design tokens, localization resources, and brand mark presentation. It does not fabricate backend capabilities for Quick Paste, Profiles, Rules, or Settings that are not yet implemented.

## Visual direction

Pastral uses a restrained signal-trail identity: two linked geometric planes imply a continuous copy-to-paste path and a compact letter P. Aurora Violet and Signal Cyan are limited to the mark and rare brand moments. Ordinary interaction uses Windows system and accent brushes.

The manager remains an Operate-mode Windows app. The visual hierarchy prioritizes task, current state, recent content, and available action. Technical implementation language moves out of primary reading paths unless it is required to explain degraded state.

## Shell

The title bar presents a compact Pastral mark, product name, and local-private positioning. Navigation labels are localized and use familiar Fluent icons. Existing Home and History routes remain the only active destinations in this pass.

## Home

Home becomes a concise command center. It leads with a welcome/task statement, verified operational status, privacy/profile summary, and recent clips. Technical provider-boundary detail remains available as secondary copy, not the main headline.

## History

History remains search-first. Search, result count, empty/loading/degraded states, result rows, and detail pane use clearer hierarchy and less card nesting. Rows expose preview, source, type, time, profile, and representations without turning every fact into a badge.

## Localization

All visible static XAML copy touched in this pass uses x:Uid resources. English remains the neutral fallback and Vietnamese is added as a complete matching resource set for the shell and the current Home/History surfaces. Dynamic runtime messages remain in code until their state model can be localized safely in a later vertical slice.

## Accessibility and adaptation

The design preserves system text styles, Mica fallback behavior, keyboard navigation, UI Automation names, high-contrast compatibility, 100–300% DPI support, and responsive History layout. Brand color is never the only carrier of state.

## Acceptance criteria

- Shell visibly carries the Pastral identity without a custom inaccessible title-bar implementation.
- Home and History have clearer task hierarchy and contemporary spacing.
- Vietnamese and English resource files contain matching keys for the touched static surfaces.
- No UI bypasses the manager provider or reads storage directly.
- Existing unrelated PowerShell changes remain untouched.
- Manager build and repository verification commands complete successfully, or any environment blocker is reported exactly.
