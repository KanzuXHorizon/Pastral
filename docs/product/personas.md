# Personas and usage contexts

Personas represent distinct constraints, not demographic stereotypes. Product decisions must work across these contexts without forcing users into separate editions.

## 1. Keyboard-first developer

**Primary jobs**

- recover commands, paths, errors, code fragments, links, and configuration values;
- search by project, source editor, terminal, error code, or code language;
- paste rich code, plain text, code blocks, file paths, or filenames intentionally;
- avoid repeated URL cleanup and format stripping.

**Constraints**

- very low tolerance for hotkey conflicts, focus loss, input interception, or latency;
- may copy secrets accidentally;
- uses terminals, IDEs, browsers, Win32, WinForms, WPF, and custom controls;
- expects deterministic behavior and detailed diagnostics.

**Success condition**

Pastral feels invisible during copy, opens instantly on demand, protects likely secrets, and can explain every automatic transformation.

## 2. Researcher or student

**Primary jobs**

- recover quotations, citations, URLs, screenshots, OCR text, and notes;
- group clips by course, research topic, website, profile, or collection;
- retain source context and distinguish original from cleaned/derived text.

**Constraints**

- long-lived history can become noisy;
- source title/domain may itself be private;
- needs exact phrase and date filtering;
- may work across different DPI displays and laptop battery modes.

**Success condition**

A copied item from weeks earlier can be found by remembered words, source, time, type, or project without exposing unrelated sensitive history.

## 3. Privacy-sensitive professional

**Primary jobs**

- keep ordinary work history while excluding password managers, private browser windows, banking tools, confidential apps, and secret-like content;
- inspect storage, retention, encryption, and deletion behavior;
- pause capture predictably.

**Constraints**

- clipboard content may include credentials, customer data, legal material, or internal URLs;
- false negatives can expose data; false positives must not silently destroy ordinary work;
- network silence and content-free diagnostics are required.

**Success condition**

Defaults are conservative, exceptions are explicit and narrow, and the user can prove what is stored and delete it without hidden copies.

## 4. Designer or content worker

**Primary jobs**

- retain images, formatted text, HTML, links, file lists, color values, and multiple representations;
- choose original image, encoded format, OCR text, filename, or compressed derived copy;
- organize clips by project and source tool.

**Constraints**

- large payloads must not freeze copy or UI;
- previews must be lazy and accurate;
- color-only state and tiny targets are unacceptable;
- monitor DPI, color mode, and transparency settings vary.

**Success condition**

Pastral preserves originals without recompression, presents useful representations clearly, and remains responsive with image-heavy history.

## 5. Assistive-technology and reduced-distraction user

**Primary jobs**

- navigate Quick Paste and manager entirely with keyboard, screen reader, magnification, touch, switch access, or high contrast;
- receive confirmation without disruptive motion or repeated announcements;
- understand focus and selection state reliably.

**Constraints**

- passive overlay must not enter the focus order or create unsolicited verbose announcements;
- standard editing shortcuts must remain standard;
- text scaling, RTL, long localization, and 100–300% DPI can alter layout substantially;
- motion and transparency may be disabled.

**Success condition**

Every core action is discoverable, operable, understandable, and robust without pointer precision, color perception, animation, transparency, or sight.

## Shared environmental contexts

All personas may encounter:

- multiple monitors with different DPI and work areas;
- RDP or virtual machines;
- fullscreen games, video, presentations, and screen sharing;
- battery saver and GPU device loss;
- low disk space or database recovery;
- rapid copy bursts;
- nonstandard destination controls;
- Windows session lock, switch user, suspend, resume, shutdown, update, and crash.

These contexts belong in acceptance tests rather than being treated as exceptional support cases.
