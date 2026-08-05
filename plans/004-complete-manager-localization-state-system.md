# Plan 004: Complete manager localization and state-copy infrastructure

> **Executor instructions:** Use an isolated worktree. Resource all user-visible runtime text without changing factual product behavior. Do not translate identifiers, file paths, protocol names, hashes, or diagnostic codes.
>
> **Drift check:** `git diff --stat 49be43c..HEAD -- apps/manager/Pastral.Manager/Strings apps/manager/Pastral.Manager/Pages apps/manager/Pastral.Manager/Services apps/manager/Pastral.Manager/ViewModels eng/verify-native-manager.ps1`

## Status

- **Priority:** P1
- **Effort:** M
- **Risk:** MED
- **Depends on:** Plan 002
- **Category:** localization, accessibility, UI architecture, tests
- **Planned at:** commit `49be43c`, 2026-08-05

## Why this matters

Pastral advertises English and Vietnamese resources, but localization currently covers only prominent static headings. Connection states, errors, empty states, counts, source/type/profile labels, representation summaries, relative times and accessible names are assembled from English literals in C++. This produces mixed-language workflows and makes future pages multiply inconsistent copy.

## Current state

- `Strings/en-US/Resources.resw` and `Strings/vi-VN/Resources.resw` each contain 25 entries.
- Static controls with `x:Uid` localize some headings and placeholders.
- `HistoryPage.xaml.cpp` hard-codes loading/searching/empty/no-results/available/unavailable states and result counts.
- `HomePage.xaml.cpp` hard-codes capture/profile/empty states and `N items`.
- `ManagerDataProvider.cpp` hard-codes connection errors, type/profile labels, representation summaries, relative time and automation names.
- Presentation models currently carry already-formatted strings, mixing provider truth with locale-specific wording.
- The repository’s supported locales are `en-US` and `vi-VN`; long-localization and RTL reflow are required by design even when RTL is not a shipped locale.

## Target architecture

- Provider/domain snapshots carry semantic enums, values and timestamps, not localized sentences where practical.
- A manager localization service loads PRI resources and formats state on the UI/presentation boundary.
- Resource keys are stable semantic names, grouped by shell/home/history/common/status/type/profile/action/accessibility.
- Counts and relative times use locale-aware plural/format rules, with deterministic test clocks.
- UI Automation names use the same localized semantic data and never expose hidden content.

## Commands

| Purpose | Command | Expected |
|---|---|---|
| Resource parity | project verifier script added in this plan | same key set, no blank values |
| Native build/UIA | `powershell -File eng/verify-native-manager.ps1 -Mode All` | English and Vietnamese scenarios PASS |
| Bridge live mode | `powershell -File eng/verify-manager-ipc-bridge.ps1 -Mode All` | PASS |
| Full baseline | `powershell -File eng/build.ps1 -Task Full` | exit 0 |

## Scope

**In scope**

- manager `Strings/en-US`, `Strings/vi-VN`
- a focused `Services/ManagerStrings.*` or equivalent localization boundary
- presentation state types needed to move semantic values out of formatted provider strings
- Home/History XAML and code-behind
- manager static/UIA verifier and locale launch support

**Out of scope**

- Adding more shipping languages.
- Translating raw clipboard content, app names, paths, URLs, protocol/status codes or developer diagnostics.
- Replacing Windows resource infrastructure with JSON or a third-party localization library.
- Changing privacy or connection semantics.

## Steps

### Step 1: Add a resource completeness gate

Create a verifier that parses all manager `.resw` files and checks:

- exact key parity between `en-US` and `vi-VN`;
- no blank values or duplicate keys;
- every `x:Uid` used in XAML has a corresponding resource property;
- no user-visible literal remains in page/provider code except an explicit allowlist for product name, stable identifiers and test-only synthetic payload content;
- format placeholders match by name/count across locales.

Seed the allowlist narrowly and store rationale beside each entry. The current code must fail this gate with evidence listing file/line/string.

**Verify:** run the new static gate → FAIL on known runtime English literals.

### Step 2: Introduce a typed resource access boundary

Add a small manager localization service using WinRT resource APIs. Expose semantic methods rather than arbitrary string-key lookup at call sites, for example:

```cpp
hstring ConnectionTitle(ConnectionState state);
hstring ResultCount(uint32_t count, bool partial);
hstring RelativeTime(ObservedAt value, ClockSnapshot now);
hstring ClipAutomationName(ClipPresentationFacts const& facts);
```

Keep the service UI-process local and immutable/thread-safe. Failed required resource lookup should return a safe English fallback and emit only content-free diagnostics in Debug; tests must detect missing keys before release.

**Verify:** focused unit/probe tests cover every enum value and resource key in both locales.

### Step 3: Move semantic state out of provider-formatted sentences

Refactor `ManagerSnapshot` and `ClipPreviewData` incrementally so provider code returns facts:

- connection/error enum and bounded technical reason;
- capture/profile/storage semantic state;
- observed timestamp instead of preformatted relative time;
- clip kind, availability/truncation/pin flags;
- source label as data;
- representation identifiers/summaries as semantic values.

Keep compatibility helpers only while migrating all call sites in the same plan. Do not localize in the Rust bridge or persist localized strings.

**Verify:** provider tests assert semantic state independent of locale; manager builds after each additive migration.

### Step 4: Resource all static and dynamic Home/History copy

Add keys for:

- connection/loading/retry/protocol/auth/configuration states;
- Home capture/profile/storage/recent/empty states;
- History loading/searching/refreshing/partial/empty/no-results/detail states;
- common actions, field labels, type/profile/state/representation labels;
- unavailable/truncated/pinned descriptions;
- automation names/help text/live announcements;
- singular/plural result counts and first-page/partial qualifiers.

Use `x:Uid` for static XAML properties and typed service methods for runtime text. Remove duplicate fallback English literals from XAML when resource resolution is authoritative, except safe design-time defaults if the build requires them and the verifier allows them explicitly.

**Verify:** literal scan returns only reviewed allowlist entries.

### Step 5: Implement locale-aware count and time formatting

Use Windows globalization APIs where appropriate. Define deterministic buckets for relative time and fall back to an absolute localized date after the documented horizon. Avoid assembling grammar with string concatenation.

Tests must cover zero, one, two, large counts; seconds/minutes/hours/yesterday/days/absolute date; clock rollback/future timestamps; Vietnamese and English output; truncation/overflow.

**Verify:** focused formatter tests pass under both language profiles.

### Step 6: Localize accessibility output and verify language consistency

Launch the manager under explicit `en-US` and `vi-VN` resource contexts using synthetic and live seeded modes. UIA assertions must verify localized:

- navigation names;
- page headings and search placeholder/name;
- result count/live announcements;
- row automation names and unavailable state;
- detail field labels/actions/help text;
- loading/error/retry/empty/no-result states.

Raw clip preview/source names remain unchanged unless they are synthetic localized fixtures.

**Verify:** no English-only state sentence appears in the Vietnamese UIA tree outside the reviewed raw-data allowlist.

### Step 7: Stress long strings, scaling and fallback

Add pseudo-long fixtures or an internal diagnostic locale mode that expands labels without shipping a fake locale. Test 300% text scaling, narrow History, long app/source names, multiline error messages and missing optional resources. Controls with consequential meaning must wrap/reflow rather than truncate ambiguously.

**Verify:** UIA can still reach every action and screenshot review shows no overlap/clipping.

### Step 8: Run aggregate gates and update localization documentation

Document key naming, semantic-state ownership, translator constraints and how to add a locale. Run all manager/full gates and inspect the final resource parity report.

## Test plan

- resource-key parity and placeholder parity;
- literal user-copy scanner;
- every connection/error/clip enum maps in both locales;
- count and relative-time edge cases;
- English/Vietnamese Debug synthetic UIA flows;
- English/Vietnamese live agent connected/disconnected flow;
- unavailable/partial/no-results and long-copy states;
- missing-resource safe fallback in Debug test only.

## Done criteria

- [ ] English and Vietnamese resources have exact key parity and no blank values.
- [ ] Runtime operational copy is resource-backed; reviewed literals are data/identifiers only.
- [ ] Provider state is semantic and locale-independent.
- [ ] Counts/time/type/profile/state/accessibility strings format correctly in both locales.
- [ ] Full Home/History UIA flows pass in English and Vietnamese.
- [ ] Long strings and 300% scaling preserve consequential controls.
- [ ] Full baseline exits 0.

## STOP conditions

Stop and report if:

- A proposed refactor would place locale-specific strings in persisted storage or IPC DTOs.
- Windows resource lookup is not safe from the provider worker thread; keep formatting on the UI boundary.
- Product terminology in Vietnamese is ambiguous and changes security/privacy meaning; request owner review for those exact terms.
- Pseudo-localization requires adding a shipping locale; use diagnostic test expansion instead.

## Maintenance notes

Every future manager feature must add semantic presentation facts first, then resources for all supported locales and UIA tests. Static copy scans should remain required so mixed-language regressions do not reappear.
