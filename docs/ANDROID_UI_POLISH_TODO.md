# Android WebRTC Tunnel UI Polish TODO

## 1. Goal

Polish the Android UI/UX so the app matches the original Material-style design and is usable as an end-user Android app, not just a developer control panel.

This is a UI/UX pass. Do not change tunnel protocol, MQTT wire format, WebRTC behavior, identity format, or desktop compatibility.

## 2. Rules

- [ ] Do not change MQTT signaling wire format.
- [ ] Do not change tunnel frame format.
- [ ] Do not change desktop Rust protocol semantics.
- [ ] Do not add TURN.
- [ ] Do not add VPN/TUN mode.
- [ ] Do not add arbitrary remote host/port selection from Android offer side.
- [ ] Do not weaken encrypted identity-at-rest behavior.
- [ ] Do not weaken network policy behavior.
- [ ] Do not weaken log/diagnostic redaction.
- [ ] Keep cellular/metered blocked by default.
- [ ] Keep `127.0.0.1` as the default local bind host.
- [ ] Use Material 3 Compose components unless there is a clear reason not to.
- [ ] Do not perform disk I/O or native validation directly in Composable bodies.

---

# Phase 1 — Apply explicit visual design system

## 1.1 Replace generic dark theme

Current app should move away from generic dark theme.

Implement a custom light color scheme:

- [ ] App background: `#F6F8FB`
- [ ] Card background: `#FFFFFF`
- [ ] App bar navy: `#061A3D`
- [ ] Primary button navy: `#08245C`
- [ ] Accent blue: `#1D4ED8`
- [ ] Success green: `#2E7D32`
- [ ] Warning orange: `#F59E0B`
- [ ] Error red: `#D32F2F`
- [ ] Border/divider: `#E5E7EB`
- [ ] Primary text: `#111827`
- [ ] Secondary text: `#6B7280`

## 1.2 Typography

Use default Android/Material **Roboto**.

Apply consistent type scale:

- [ ] App bar title: 18sp, medium/semibold.
- [ ] Screen title: 22sp, semibold.
- [ ] Card title: 18sp, semibold.
- [ ] Status title: 20sp, semibold.
- [ ] Body text: 14–16sp.
- [ ] Helper/meta text: 12–13sp.
- [ ] Button text: 14sp, medium.

## 1.3 Shapes and spacing

Implement shared dimensions:

- [ ] screen padding: 16dp;
- [ ] card padding: 16dp;
- [ ] card spacing: 12dp;
- [ ] section spacing: 20dp;
- [ ] card corner radius: 16dp;
- [ ] button minimum height: 48dp;
- [ ] minimum touch target: 48dp.

## 1.4 Reusable components

Create or refactor reusable components:

- [ ] `TunnelTopAppBar`
- [ ] `StatusCard`
- [ ] `NetworkStatusCard`
- [ ] `ForwardSummaryRow`
- [ ] `EmptyStateCard`
- [ ] `ErrorResolutionCard`
- [ ] `WizardStepper`
- [ ] `SectionHeader`
- [ ] `SettingsSection`
- [ ] `DestructiveActionButton`

## 1.5 Acceptance

- [ ] App visually uses navy top bars, light background, white cards.
- [ ] Status states use green/orange/red consistently.
- [ ] Typography and spacing are consistent across screens.
- [ ] UI looks closer to the original mockup image.

---

# Phase 2 — Fix global navigation behavior

## 2.1 Bottom navigation only on main tabs

Main tabs:

```text
Home
Forwards
Logs
Settings
```

Tasks:

- [ ] Show bottom navigation only on main tabs.
- [ ] Hide bottom navigation on Setup Wizard.
- [ ] Hide bottom navigation on Forward Details.
- [ ] Hide bottom navigation on Import / Export.
- [ ] Hide bottom navigation on Network Policy details.
- [ ] Secondary flows use top app bar with back arrow.

## 2.2 Avoid duplicate nav stack entries

Update bottom nav navigation:

```kotlin
navController.navigate(route) {
    popUpTo(navController.graph.findStartDestination().id) {
        saveState = true
    }
    launchSingleTop = true
    restoreState = true
}
```

Tasks:

- [ ] Home tab does not stack duplicate Home screens.
- [ ] Forwards tab does not stack duplicate Forwards screens.
- [ ] Logs tab does not stack duplicate Logs screens.
- [ ] Settings tab does not stack duplicate Settings screens.

## 2.3 Tests / manual checks

- [ ] Navigate between tabs repeatedly; back stack remains sane.
- [ ] Setup Wizard back arrow returns to prior screen.
- [ ] Forward Details back arrow returns to Forwards.
- [ ] Android system back works naturally.

---

# Phase 3 — Polish Home / Status screen

## 3.1 Friendly status labels

Replace raw enum names with user-facing labels.

Map examples:

- [ ] `Stopped` -> `Stopped`
- [ ] `Starting` -> `Starting`
- [ ] `Connected` -> `Connected`
- [ ] `Listening` -> `Listening`
- [ ] `PausedMeteredBlocked` -> `Paused`
- [ ] `NoNetwork` -> `No network`
- [ ] `Error` -> `Error`
- [ ] `ConfigInvalid` -> `Configuration needs attention`
- [ ] `Stopping` -> `Stopping`

Add friendly descriptions:

- [ ] Connected: `Tunnel is active and ready to use.`
- [ ] Paused: `Cellular/metered network blocked.`
- [ ] Stopped: `Tunnel service is not running.`
- [ ] No network: `Connect to Wi-Fi to start the tunnel.`
- [ ] Config invalid: `Open setup to fix configuration.`

## 3.2 State-aware action row

Do not always show both Start and Stop.

Implement:

- [ ] Stopped: `Start Tunnel`, `Setup`
- [ ] Starting: `Stop`, `View Logs`, spinner
- [ ] Connected/Listening: `Stop Tunnel`, `View Logs`, optional `Open URL`
- [ ] PausedMeteredBlocked: `Settings`, `Stop`, optional `Allow Temporarily`
- [ ] NoNetwork: `Retry`, `Settings`
- [ ] Error: `Retry`, `View Logs`, contextual fix action
- [ ] ConfigInvalid: `Open Setup`, `View Logs`

## 3.3 Improve cards

Status card:

- [ ] large icon;
- [ ] friendly title;
- [ ] description;
- [ ] mode;
- [ ] remote peer;
- [ ] active sessions;
- [ ] uptime;
- [ ] last error if present with friendly fix.

Network card:

- [ ] Wi-Fi/cellular/no-network icon;
- [ ] network type;
- [ ] metered/unmetered;
- [ ] tunnel allowed/blocked;
- [ ] blocked reason.

Forwards summary:

- [ ] show configured forwards;
- [ ] status dot/icon per row;
- [ ] `127.0.0.1:<port> -> <forward_id>`;
- [ ] add forward action;
- [ ] empty state when none.

## 3.4 Error resolution

Add `ErrorResolutionCard`.

Tasks:

- [ ] friendly error summary;
- [ ] suggested fix;
- [ ] technical details collapsed by default;
- [ ] action button: Retry / Edit Forward / Open Setup / View Logs.

## 3.5 Acceptance

- [ ] No raw enum names are visible on Home.
- [ ] Home actions match current state.
- [ ] Home looks like a dashboard, not a debug dump.
- [ ] Error state gives next-step guidance.

---

# Phase 4 — Rebuild Setup Wizard UX

## 4.1 Wizard shell

Tasks:

- [ ] Make Setup Wizard a secondary flow with back arrow top app bar.
- [ ] Hide bottom navigation during wizard.
- [ ] Add numbered horizontal `WizardStepper`.
- [ ] Show current step number and title.
- [ ] Add Cancel action.
- [ ] Use Back/Next bottom row.
- [ ] Disable Next until current step is valid when practical.
- [ ] Review step uses Back / Save / Start Tunnel.

## 4.2 Step 1 — Choose Mode

Implement selectable cards:

- [ ] Offer/client card with icon and description.
- [ ] Answer/server card marked Advanced or Not available yet.
- [ ] Offer selected by default.
- [ ] If answer unsupported, answer card disabled with explanation.
- [ ] Do not show only plain text.

## 4.3 Step 2 — Identity

Local identity only.

Tasks:

- [ ] Generate new identity action.
- [ ] Import existing identity using Android file picker if possible.
- [ ] Hide raw path import behind Advanced/debug.
- [ ] Show local peer ID.
- [ ] Show public identity.
- [ ] Copy Public Key action.
- [ ] Share Public Key action.
- [ ] Do not show remote public identity here.
- [ ] Do not validate identity file on every keystroke.
- [ ] Private identity export warning remains intact.

## 4.4 Step 3 — MQTT Broker

Tasks:

- [ ] Broker host field.
- [ ] Port field.
- [ ] TLS enabled switch.
- [ ] Username optional field.
- [ ] Password field or password-file-path field clearly labeled.
- [ ] Topic prefix optional field.
- [ ] Test Connection action.
- [ ] Password hidden if actual password.
- [ ] No password/secrets in logs.

## 4.5 Step 4 — Remote Peer

Tasks:

- [ ] Remote peer ID field.
- [ ] Remote public identity field.
- [ ] Paste from Clipboard button.
- [ ] Import File button.
- [ ] Validate peer ID/public identity match.
- [ ] Reject local identity as remote peer.
- [ ] Helper text explaining answer side must authorize this phone.

## 4.6 Step 5 — Forwards

The wizard must support forward editing directly.

Tasks:

- [ ] List current forwards inside wizard.
- [ ] Add Forward button.
- [ ] Edit Forward action.
- [ ] Delete Forward action.
- [ ] Enable/disable forward.
- [ ] Inline forward editor or dialog.
- [ ] Validate name required.
- [ ] Validate local port 1-65535.
- [ ] Reject duplicate enabled local ports.
- [ ] Validate remote forward_id required.
- [ ] Reject duplicate enabled remote forward_id.
- [ ] Hide non-localhost bind behind Advanced warning.
- [ ] User does not need to leave wizard to configure forwards.

## 4.7 Step 6 — Network Policy

Tasks:

- [ ] Show current network type.
- [ ] Show metered/unmetered.
- [ ] Show tunnel allowed/blocked.
- [ ] Show blocked reason.
- [ ] Allow cellular/metered toggle.
- [ ] Show warning before enabling cellular/metered.
- [ ] Resume when Wi-Fi returns toggle.
- [ ] Explain Unknown network is blocked.

## 4.8 Step 7 — Review

Tasks:

- [ ] Summary card for Mode.
- [ ] Summary card for Local Identity.
- [ ] Summary card for Remote Peer.
- [ ] Summary card for Broker.
- [ ] Summary card for Network Policy.
- [ ] Summary card for Forwards.
- [ ] Start Tunnel disabled if previous steps invalid.
- [ ] Start Tunnel saves, validates, checks identity/network, and starts service.
- [ ] Errors shown inline and actionably.

## 4.9 Acceptance

- [ ] Setup Wizard visually matches original seven-step design.
- [ ] Wizard can complete first-run setup without leaving wizard.
- [ ] Wizard does not require TOML editing or raw path typing for normal flow.
- [ ] Wizard has a real progress indicator.

---

# Phase 5 — Refactor setup data loading and validation

## 5.1 Remove disk I/O from composition

Fix any code like:

```kotlin
val forwards = vm.loadSavedForwards()
```

inside Composables.

Tasks:

- [ ] Move forwards loading into `SetupViewModel`.
- [ ] Expose forwards as `StateFlow`.
- [ ] Use `collectAsStateWithLifecycle()`.
- [ ] No file I/O from Composable body.

## 5.2 Stop validating files on every keystroke

Tasks:

- [ ] Text field changes update only text state.
- [ ] Import Identity button performs file read/validation.
- [ ] Import Public Identity button performs validation.
- [ ] Paste action validates pasted text.
- [ ] Next button validates final values.
- [ ] Native validation is not called on every keystroke.

## 5.3 Tests

- [ ] Composable does not trigger file load during recomposition.
- [ ] identity path typing does not call file read each character.
- [ ] import button calls validation exactly once.
- [ ] pasted public identity validates on paste/import/Next.

## 5.4 Acceptance

- [ ] Setup Wizard is responsive.
- [ ] No disk/native work happens directly in composition.
- [ ] No expensive validation on every keystroke.

---

# Phase 6 — Implement Forward Details screen

## 6.1 Add route

Add route:

```text
forwardDetails/{forwardId}
```

Tasks:

- [ ] Forwards row tap navigates to details.
- [ ] Details screen has top app bar with back arrow.
- [ ] Bottom nav hidden on details screen.

## 6.2 Details layout

Show:

- [ ] forward name;
- [ ] status;
- [ ] local address;
- [ ] local URL;
- [ ] remote forward_id;
- [ ] bytes sent if available;
- [ ] bytes received if available;
- [ ] open connections if available;
- [ ] last error.

## 6.3 Actions

Implement:

- [ ] Copy URL.
- [ ] Open Browser.
- [ ] Test Local Port.
- [ ] Edit.
- [ ] Disable/Enable.
- [ ] Delete with confirmation.

## 6.4 Forwards list cleanup

List row should be concise:

- [ ] status dot/icon;
- [ ] name;
- [ ] local address -> remote ID;
- [ ] status text;
- [ ] chevron.

Do not cram all details/actions into the list row.

## 6.5 Acceptance

- [ ] Dedicated Forward Details screen exists.
- [ ] Forwards list is clean and scannable.
- [ ] Details screen matches original mockup concept.

---

# Phase 7 — Polish Logs screen

## 7.1 Layout

Tasks:

- [ ] Top app bar title `Logs`.
- [ ] Filter chips: All / Info / Warn / Error / Debug.
- [ ] Log rows with timestamp and message.
- [ ] Action row: Copy Logs / Clear Logs / Export Diagnostics / Pause Logs.
- [ ] Empty state when no logs.

## 7.2 Presentation

Tasks:

- [ ] Info logs use default text.
- [ ] Warn logs use orange indicator.
- [ ] Error logs use red indicator.
- [ ] Debug logs use muted style.
- [ ] Long messages wrap cleanly.
- [ ] Raw JSON hidden unless debug mode is enabled.

## 7.3 Redaction

Confirm:

- [ ] displayed logs are redacted;
- [ ] copied logs are redacted;
- [ ] exported diagnostics are redacted;
- [ ] secrets do not appear in UI.

## 7.4 Acceptance

- [ ] Logs screen is readable for normal users.
- [ ] Debug details are available without overwhelming default view.

---

# Phase 8 — Rebuild Settings screen sections

## 8.1 Section structure

Implement sections:

- [ ] Tunnel
- [ ] Network Policy
- [ ] Identity
- [ ] Configuration
- [ ] Diagnostics
- [ ] Advanced
- [ ] About

## 8.2 Tunnel section

Include:

- [ ] Start tunnel automatically when app opens.
- [ ] Resume tunnel when Wi-Fi returns.
- [ ] Run setup wizard again.

## 8.3 Network Policy section

Include:

- [ ] Allow cellular / metered data.
- [ ] Show warning before allowing cellular / metered data.
- [ ] Open Network Policy details.

## 8.4 Identity section

Include:

- [ ] View public identity.
- [ ] Copy public identity.
- [ ] Share public identity.
- [ ] Import identity.
- [ ] Export public identity.
- [ ] Export private identity with warning.

## 8.5 Configuration section

Include:

- [ ] Import configuration.
- [ ] Export configuration with warning.
- [ ] Validate configuration.
- [ ] Reset configuration.

## 8.6 Diagnostics section

Include:

- [ ] Export diagnostics.
- [ ] Share diagnostics.
- [ ] Copy status JSON.
- [ ] Copy redacted config.

## 8.7 Advanced section

Collapsed by default.

Include:

- [ ] Debug logs.
- [ ] Developer/debug raw path import/export.
- [ ] Custom topic prefix if supported.
- [ ] Non-localhost bind controls, if supported.
- [ ] Answer mode, if present.

## 8.8 Acceptance

- [ ] Settings is not just a list of navigation links.
- [ ] Settings matches the original sectioned spec.
- [ ] Dangerous/debug items are hidden behind Advanced.

---

# Phase 9 — Improve Import / Export UX

## 9.1 Primary actions

Use Android-safe flows as the primary UI:

- [ ] Import config: document picker.
- [ ] Export config: create document with warning.
- [ ] Import identity: document picker.
- [ ] Export public identity: create document/share.
- [ ] Export private identity: create document with private identity warning.
- [ ] Import remote public identity: document picker/paste.
- [ ] Export/share diagnostics: create document/share.

## 9.2 Hide raw paths

Tasks:

- [ ] Move raw path fields to Advanced / Developer fallback.
- [ ] Collapse Advanced by default.
- [ ] Label raw path fallback as developer/debug only.
- [ ] Do not show raw path fields in normal first-run setup.

## 9.3 Acceptance

- [ ] Normal user can import/export without typing filesystem paths.
- [ ] Developer raw path fallback exists only behind Advanced.
- [ ] Sensitive export warnings remain.

---

# Phase 10 — Notification permission UX

## 10.1 Android 13+ permission prompt

Implement runtime notification permission flow for Android 13+.

Tasks:

- [ ] Detect if `POST_NOTIFICATIONS` permission is needed.
- [ ] Show explanation before request.
- [ ] Request permission.
- [ ] Handle denied state.
- [ ] Show Settings action if permission denied.

Explanation text:

```text
Rust WebRTC Tunnel needs notifications so Android can keep the tunnel service visible while it is running in the background.
```

## 10.2 Tests/manual checks

- [ ] Fresh install on Android 13+ shows explanation.
- [ ] Allow path works.
- [ ] Deny path shows warning/action.
- [ ] Tunnel behavior remains correct if permission is denied.

## 10.3 Acceptance

- [ ] Notification permission UX exists.
- [ ] User understands why notifications are needed.

---

# Phase 11 — Accessibility pass

## 11.1 Content descriptions

Add content descriptions for actionable icons:

- [ ] Home tab icon.
- [ ] Forwards tab icon.
- [ ] Logs tab icon.
- [ ] Settings tab icon.
- [ ] Add forward icon.
- [ ] Delete icon.
- [ ] Copy icon.
- [ ] Share icon.
- [ ] Open browser icon.
- [ ] Status icons where needed.

## 11.2 Touch targets

Ensure minimum 48dp touch target for:

- [ ] buttons;
- [ ] icon buttons;
- [ ] switches;
- [ ] bottom nav items;
- [ ] list rows.

## 11.3 Color and text

- [ ] Color is not the only state indicator.
- [ ] Status labels are text-visible.
- [ ] Error/warning text is readable.
- [ ] Text scales with system font size.
- [ ] Dialogs are screen-reader friendly.

## 11.4 Acceptance

- [ ] Basic accessibility requirements are implemented.
- [ ] App is usable without relying only on color.

---

# Phase 12 — Tests and validation

## 12.1 UI tests / ViewModel tests

Add or update tests for:

- [ ] friendly status label mapping;
- [ ] state-aware Home actions;
- [ ] wizard step validation;
- [ ] wizard forwards add/edit/delete;
- [ ] remote peer validation;
- [ ] settings section visibility;
- [ ] raw path fields hidden behind Advanced;
- [ ] Forward Details route/actions;
- [ ] no validation on every keystroke;
- [ ] no disk I/O in composition path where testable.

## 12.2 Manual UI checklist

Manually verify:

- [ ] Home connected state matches mockup concept.
- [ ] Home paused cellular state matches mockup concept.
- [ ] Setup Wizard stepper appears.
- [ ] Identity step is local identity only.
- [ ] Remote Peer step contains remote identity.
- [ ] Forwards step allows add/edit/delete.
- [ ] Forward Details screen exists.
- [ ] Logs screen is readable.
- [ ] Settings has required sections.
- [ ] Import/export primary flow uses SAF/share.
- [ ] Advanced/debug fields are collapsed.
- [ ] Android 13 notification permission explanation appears.

## 12.3 Regression validation

Run existing validation:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo ndk \
  -t arm64-v8a \
  -t x86_64 \
  -o android/app/src/main/jniLibs \
  build -p p2p-mobile --release

cd android
./gradlew assembleDebug
./gradlew testDebugUnitTest
```

If device/emulator available:

```bash
./gradlew connectedDebugAndroidTest
```

## 12.4 Acceptance

- [ ] UI polish does not break runtime/config/security tests.
- [ ] Existing Android validation still passes.
- [ ] Manual UI checklist passes.
- [ ] Any intentionally deferred UI items are documented.

---

# Phase 13 — Final UI acceptance checklist

Do not check until complete.

## 13.1 Visual design

- [ ] Light card-based theme implemented.
- [ ] Navy app bar implemented.
- [ ] Explicit color palette used.
- [ ] Roboto/Material typography used consistently.
- [ ] Cards/buttons/spacing match spec.
- [ ] Status colors are consistent.

## 13.2 Home

- [ ] Friendly labels, no raw enum names.
- [ ] State-aware actions.
- [ ] Error resolution card.
- [ ] Network card clear.
- [ ] Forwards summary clear.

## 13.3 Setup Wizard

- [ ] Secondary flow without bottom nav.
- [ ] Progress stepper.
- [ ] Mode cards.
- [ ] Local identity step only.
- [ ] Remote Peer step contains remote identity.
- [ ] MQTT step polished.
- [ ] Forwards can be edited inside wizard.
- [ ] Network Policy step shows real state and controls.
- [ ] Review step clear.

## 13.4 Forwards

- [ ] Clean list rows.
- [ ] Dedicated details screen.
- [ ] Copy/Open/Test/Edit/Disable/Delete actions.
- [ ] Delete confirmation.

## 13.5 Logs / Settings / Import Export

- [ ] Logs readable and redacted.
- [ ] Settings sectioned.
- [ ] Import/export uses SAF/share as primary UX.
- [ ] Raw path fallback hidden behind Advanced.
- [ ] Notification permission UX implemented.

## 13.6 Accessibility/performance

- [ ] Content descriptions for icons.
- [ ] 48dp touch targets.
- [ ] Color not sole state indicator.
- [ ] No disk/native work in Composable body.
- [ ] No expensive validation on every keystroke.

## 13.7 Regression

- [ ] Existing runtime/security/build validation still passes.
- [ ] No protocol behavior changed.
- [ ] E2E compatibility status remains honest.
