# Accessibility Checklist

Use this checklist for every UI change before merge.

## Keyboard And Focus

- Every interactive control is reachable by keyboard only.
- Focus order matches visual reading order.
- Focus is visible on all controls, including icon-only buttons.
- Escape closes transient surfaces (launcher/dialogs) and returns focus predictably.
- Global shortcuts do not block normal text input behaviors.

## Semantics And Labels

- Every icon-only action has a readable text label/tooltip equivalent.
- Form controls have clear labels and helper/error text.
- Button text describes the action, not just "OK" or "Click".
- Notification actions use explicit labels.

## Color And Contrast

- Body text meets WCAG AA contrast against background.
- Muted text remains readable in light and dark themes.
- State is not communicated by color alone; add text/icon cues.

## Motion And Feedback

- Time-based transitions are subtle and do not block interaction.
- Loading/progress states are visible for async work.
- Background task completion/failure is visible in status/notifications.

## Screen Reader / Platform Bridge

The bridge is gpui's native AccessKit integration: macOS NSAccessibility,
Windows UIA, Linux X11/Wayland AT-SPI. The wasm build ships no bridge, so
accessibility is inactive there by design.

Verified practice for every UI change:

- Announced elements carry an `.id(...)` plus a role; focusable elements also
  track focus.
- Plain text children produce no accessibility nodes. Give text-bearing
  containers a label, or render the text as a labeled node (heading,
  paragraph, list item).
- Icon-only controls carry an explicit accessibility label.
- Shell landmarks exist: navigation (sidebar), main content, page title
  heading, status bar.
- State changes a user must hear go through polite live regions: command
  palette selection and result count, form results, errors, notification
  counts. Nothing that changes per frame is live, or it would spam the reader.
- App init registers the `accessibility` capability; the Diagnostics page
  renders the registry and its read-outs as labeled list items.

The snapshot global tracks bridge state and active-window counts and is
refreshed on window open/close and when assistive technology connects or
disconnects mid-session (`Window::is_a11y_active`).

Known gaps live in gpui-component/gpui and the Linux AT-SPI bridge; closing
them needs upstream fixes or app-side contract changes:

- Toast notifications render no accessibility nodes.
- Title bar window controls (minimize/maximize/close) and resizable-panel
  drag handles carry no accessibility semantics.
- Sidebar items are not keyboard-focusable: assistive-tech activation works,
  Tab order does not reach them. The kit's item is unfocusable; a tab stop
  on the app-side wrapper would change mouse-focus transfer and Tab-order
  contracts.
- Form fields have no programmatic label-to-input association; inputs carry
  explicit `aria_label`s and error text is a nearby labeled alert.
- Popup menu items with a checkmark expose the checked state only visually;
  `menu_with_check` has no `aria_toggled`.
- A button's attached dropdown menu cannot expose `aria_expanded`; the popup
  state is internal to the component.
- The query devtools registry header row and its sort/filter captions are
  visual-only; row labels embed the column values and the button groups carry
  group labels instead.
- Leaf text inside labeled containers (markdown chat bodies, response bodies)
  is summarized by the container label, not exposed verbatim.
- The Linux AT-SPI bridge exposes no heading-level attribute (`aria_level`
  is set in code but not announced) and maps Navigation/Main to a generic
  "Landmark" role; the landmark's name carries the distinction.

## Manual QA Passes

These need a human session per OS; automated tests cover none of them.

- macOS: keyboard-only walkthrough of launcher, sidebar, settings,
  notifications, plus a VoiceOver pass over the same surfaces.
- Windows: keyboard-only walkthrough and menu traversal, plus Narrator.
- Linux: keyboard-only walkthrough and menu traversal, plus Orca.

## Done Criteria

- All checklist items pass or have an explicit tracked exception.
- Any exception includes owner, scope, and follow-up milestone.
