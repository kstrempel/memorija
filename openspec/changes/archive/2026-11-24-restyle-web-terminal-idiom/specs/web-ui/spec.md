# Spec Delta

## ADDED Requirements

### Requirement: Path filters focus with a slash shortcut

The Changes and Files screens SHALL focus their path filter when the user presses `/` while no text field, select, or editable element has focus, and SHALL ignore the shortcut while Control, Alt, or Meta is held. The Changes screen SHALL show a visible hint that `/` focuses the filter, and pressing Escape in the focused filter SHALL release focus without changing the filter.

#### Scenario: Slash focuses the filter on Changes

- **WHEN** the Changes screen is open, no editable element has focus, and the user presses `/`
- **THEN** the path filter gains focus and subsequent typing enters text into it, while the slash itself is not entered

#### Scenario: Slash focuses the filter on Files

- **WHEN** the Files screen is open, no editable element has focus, and the user presses `/`
- **THEN** the Files path filter gains focus and subsequent typing narrows the list

#### Scenario: Typing a slash inside a field is untouched

- **WHEN** any text field, including the path filter itself, has focus and the user types `/`
- **THEN** the character is entered in that field and the shortcut does not fire

#### Scenario: Escape releases the focused filter

- **WHEN** the path filter has focus and the user presses Escape
- **THEN** the filter loses focus and its current text and the listed results are unchanged

#### Scenario: The hint names the shortcut

- **WHEN** the Changes screen is shown
- **THEN** a visible hint tells the user that pressing `/` focuses the filter

## MODIFIED Requirements

### Requirement: Appearance control offers Light, Dark, and System

The shared header SHALL present the three appearance choices — Light, Dark, and System — as icon controls (sun, moon, and monitor) with accessible names and a pressed state, and SHALL show a visible status readout naming the appearance in effect. The control SHALL be available on every screen.

#### Scenario: Control is present on every screen

- **WHEN** the user opens the Changes, Files, Reader, or not-found screen
- **THEN** the three icon controls and the status readout are part of the shared header, each icon exposes the name of its choice to assistive technology, and the active choice is marked

#### Scenario: Choosing an appearance applies immediately

- **WHEN** the user chooses Light or Dark
- **THEN** the visible screen switches to that appearance at once, that icon is marked as active, and the status readout names the appearance in effect

#### Scenario: Choosing System returns to the OS preference

- **WHEN** the user chooses System after having chosen Light or Dark
- **THEN** the viewer matches the browser's current preferred appearance, follows it from then on, marks System as active, and the status readout names the resulting appearance
