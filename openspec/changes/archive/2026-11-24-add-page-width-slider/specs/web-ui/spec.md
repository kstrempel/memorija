# Spec Delta

## ADDED Requirements

### Requirement: Settings dialog holds the viewer's settings

The shared header SHALL present a settings control on every screen that opens a modal dialog containing the viewer's settings. The dialog SHALL dismiss on its close control, on Escape, or on a click outside it, and SHALL return focus to the control that opened it.

#### Scenario: Settings are reachable from every screen

- **WHEN** the user opens the Changes, Files, Reader, empty, or not-found screen
- **THEN** the settings control is part of the shared header, names what it opens for assistive technology, and opens the same dialog

#### Scenario: The dialog dismisses without changing anything

- **WHEN** the dialog is open and the user activates Close, presses Escape, or clicks outside the dialog
- **THEN** the dialog closes and the visible screen is unchanged except for the settings that were already applied

#### Scenario: Focus returns to the control that opened the dialog

- **WHEN** the user closes the dialog with the keyboard
- **THEN** focus is on the settings control in the header, not on the page body

### Requirement: Page width is adjustable

The settings dialog SHALL offer a page-width control that sets the width of the content shell as a continuous value between a comfortable minimum and the full window width, defaulting to a mid-range width. The chosen width SHALL be remembered per browser and SHALL never be sent to the server or written to the memory root or the version store.

#### Scenario: Adjusting the width applies at once

- **WHEN** the user moves the page-width control
- **THEN** the shell widens or narrows immediately, the dialog stays open, and the visible screen keeps its filters, scroll position, and selected file without a reload

#### Scenario: Width is remembered per browser

- **WHEN** the user sets a width and then reloads the page or restarts the viewer
- **THEN** the page opens at that width without first painting at another one

#### Scenario: Another browser starts from the default

- **WHEN** the viewer is opened in a browser or profile with no stored width
- **THEN** it uses the default mid-range width rather than a width chosen elsewhere

#### Scenario: Smallest setting stops at the layout minimum

- **WHEN** the user chooses the smallest width
- **THEN** the shell stops at a width where the change and file rows still keep their columns instead of narrowing further

#### Scenario: Rendered files keep a readable measure

- **WHEN** the user widens the shell and opens a file in the Reader
- **THEN** prose lines stop growing at a readable measure, while tables, code blocks, and the list screens use the available width

#### Scenario: The viewer stays read-only

- **WHEN** the user changes the width
- **THEN** no memory file or version record is created, changed, or deleted

## MODIFIED Requirements

### Requirement: Appearance control offers Light, Dark, and System

The settings dialog SHALL present the three appearance choices — Light, Dark, and System — as labelled controls with accessible names and a pressed state. The control SHALL be available on every screen, and no separate status readout of the appearance in effect SHALL be shown.

#### Scenario: Control is present on every screen

- **WHEN** the user opens the Changes, Files, Reader, or not-found screen and then the settings dialog
- **THEN** the three labelled appearance choices are part of the dialog, each names its choice to assistive technology, and the active choice is marked

#### Scenario: Choosing an appearance applies immediately

- **WHEN** the user chooses Light or Dark in the dialog
- **THEN** the visible screen switches to that appearance at once, that choice is marked as active, and the dialog stays open

#### Scenario: Choosing System returns to the OS preference

- **WHEN** the user chooses System after having chosen Light or Dark
- **THEN** the viewer matches the browser's current preferred appearance, follows it from then on, and marks System as active
