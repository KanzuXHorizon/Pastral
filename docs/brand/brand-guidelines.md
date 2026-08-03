# Pastral brand guidelines

**Status:** Provisional brand system. Final public assets require name clearance and trademark review.

## 1. Brand idea

Pastral represents a reliable trail between copy and paste:

- Copy
- Preserve
- Find
- Transform
- Paste
- Recall

The product should feel quiet, exact, native, private, and dependable—not playful automation, generic AI, or a decorative clipboard toy.

## 2. Name and tagline

Working name: **Pastral**
Tagline: **Copy once. Find anything. Paste perfectly.**

Usage rules:

- Treat `Pastral` as provisional until `docs/brand/name-clearance.md` reaches an approved legal gate.
- Do not claim the name is unique or legally cleared.
- “Paste perfectly” describes the product ambition; product copy must pair it with honest fidelity limitations and never promise universal format preservation.
- Use title case for the product name and sentence case for the tagline.

## 3. Logo concept

The symbol combines two interlocking rounded ribbon/document planes.

Required readings:

- an abstract `P`;
- a continuous handoff between copy and paste;
- negative space suggesting a sheet, insertion point, or cursor;
- forward movement without a lightning bolt or arrow badge;
- recognizable silhouette in one color.

The symbol must work independently from the wordmark.

## 4. Construction constraints

- Use a small number of geometric paths.
- Avoid hairline details and internal holes that close at 16–20 px.
- Keep optical balance centered when used in square app-icon masks.
- Test filled, outline, and single-color reductions.
- Do not encode tiny text, literal clipboard clips, paper sheets with lines, checkmarks as the main silhouette, AI sparkles, or complex perspective.
- Negative space remains open at the smallest size.
- The tray glyph is a separate simplification, not a downscaled full-color logo.

## 5. Required asset set

Source assets:

- `assets/brand/logo-symbol.svg`
- `assets/brand/logo-wordmark.svg`
- `assets/brand/logo-horizontal.svg`
- `assets/brand/logo-monochrome.svg`
- `assets/brand/tray-glyph.svg`

Generated/validated exports:

- app icon `.ico` containing 16, 20/24 where tooling supports, 32, 48, 64, 128, 256, and 512-source raster masters as required by package pipeline;
- light-theme, dark-theme, monochrome, and high-contrast variants;
- symbolic tray assets for normal, paused, private/locked, warning, and error states only when shape remains distinguishable;
- MSIX/Store assets generated from vector/source masters according to current Store requirements.

Final file sizes and package scale targets are determined from the pinned Windows packaging documentation during asset implementation.

## 6. Clear space and minimum size

- Clear space around the primary symbol is at least the width of its main internal ribbon stroke.
- Wordmark clear space is at least the cap-height of the `P` around the combined lockup.
- At 16–20 px, use the tray/symbolic glyph with no gradient detail or wordmark.
- If the internal negative space becomes ambiguous, switch to the simplified glyph rather than sharpening or adding contrast effects.

## 7. Color

Core palette:

| Name | Hex | Role |
|---|---|---|
| Aurora Violet | `#725CFF` | Primary brand endpoint |
| Signal Cyan | `#2ED3FF` | Secondary brand endpoint/continuity |
| Night Ink | `#0B1020` | Dark brand artwork field |
| Frost | `#F7F8FC` | Light brand artwork field |
| Success Mint | `#35D49A` | Semantic success only |
| Warning Amber | `#F2B84B` | Semantic warning only |

### Gradient

- Primary direction follows the visual flow of the ribbon from Aurora Violet to Signal Cyan.
- Use only in the full-color logo and rare provenance/brand moments.
- Do not place body text on the gradient without tested contrast.
- Do not use the gradient for standard buttons, every selection, every card, or status meaning.
- Monochrome/high-contrast versions communicate the same silhouette without gradient.

### System UI

Most product interaction uses Windows system colors and user accent. Success Mint and Warning Amber never communicate state alone; include icon and text.

## 8. Typography

- Product UI: OS-provided Segoe UI Variable.
- Wordmark: create a custom-drawn or carefully adjusted geometric wordmark only after name clearance; do not redistribute or outline an unlicensed font source.
- Documentation/marketing artifacts may use the system font unless a separate licensed brand typeface is approved.
- Avoid rounded generic SaaS lettering, exaggerated gradient outlines, glow, bevel, and 3D extrusion.

## 9. Icon and tray behavior

Tray is functional before expressive:

- one-color symbolic glyph;
- 16/20/24 px optical masters;
- readable in light/dark taskbar and high contrast;
- paused state uses a distinct pause cut/overlay with accessible tooltip text;
- warning/error state follows Windows semantic icon behavior and tooltip—not color-only tint;
- no animation in tray;
- no tiny profile colors that become indistinguishable.

## 10. Voice

Brand voice:

- calm;
- direct;
- technically honest;
- protective without alarmism;
- confident without hype.

Preferred:

- `Copied · Text · VS Code`
- `Original preserved`
- `Sensitive content not saved`
- `Paste sent; destination response unknown`
- `Rule paused for this profile`

Avoid:

- “Magic paste”;
- “AI-powered everything”;
- “Never lose anything ever”;
- “100% secure/lossless”;
- celebratory copy confetti;
- productivity scores and shame-oriented warnings.

## 11. Supplied concept boards

The three concept boards supplied by the project owner contribute:

- compact dark Windows utility atmosphere;
- visible copy confirmation and fast actions;
- source/profile organization;
- violet/cyan or cyan-forward mark exploration;
- preference for native, fast, secure positioning.

They do not contribute final:

- UI labels or spelling;
- technical stack text;
- icon set;
- component dimensions;
- screenshots or product claims;
- logo geometry.

Generated-text artifacts and excessive glow/glass are explicitly excluded from final implementation.

## 12. Asset production and review

Before accepting final brand assets:

1. complete formal name-clearance gate;
2. create monochrome geometry first;
3. inspect at all required pixel sizes on light/dark/high-contrast backgrounds;
4. test square app-icon masks and tray placement;
5. validate SVG accessibility/metadata and remove editor cruft;
6. generate ICO/MSIX assets reproducibly;
7. compare raster edges at 100%, 125%, 150%, 200%, and 300% scaling;
8. verify no font files are included;
9. document source, license, generator command, and checksums;
10. obtain owner approval before Store/package identity is finalized.

## 13. Rebrand readiness

- Keep wordmark and symbol independent.
- Centralize display name, package identity, app-data path labels, pipe name, executable display strings, URI schemes, and registry labels.
- Avoid branded database table/type names.
- Preserve migration aliases for data paths and protocol identity.
- Do not publish internal crates/packages under the provisional name until authorized.
