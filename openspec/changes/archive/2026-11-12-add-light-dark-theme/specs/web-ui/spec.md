# Spec Delta

## ADDED Requirements

### Requirement: Viewer offers light and dark appearances

The viewer SHALL render every screen in either a light or a dark appearance. Text, backgrounds, borders, links, muted text, and the change-size colours SHALL stay legible and distinguishable in both appearances.

#### Scenario: Each screen renders in both appearances

- **WHEN** the viewer shows the Changes, Files, Reader, not-found, or empty state in the light appearance and again in the dark appearance
- **THEN** every screen is fully styled in both, with no unstyled or light-only region

#### Scenario: Rendered markdown follows the appearance

- **WHEN** the Reader shows markdown that contains a heading, a code block, and a table in the dark appearance
- **THEN** those elements use dark-appropriate text, background, and border colours rather than the light palette

#### Scenario: Change-size colours stay distinguishable

- **WHEN** the Changes screen lists an added and a removed line count in either appearance
- **THEN** both counts are visible against the row background and remain distinguishable from each other

### Requirement: Appearance follows the system preference by default

Without an explicit choice, the viewer SHALL match the operating system's or browser's light or dark preference, and SHALL follow a change to that preference while a screen is open without a reload.

#### Scenario: First visit on a dark system

- **WHEN** the viewer is opened in a browser whose preferred appearance is dark and no appearance was chosen in that browser
- **THEN** the viewer is shown in the dark appearance

#### Scenario: First visit on a light system

- **WHEN** the viewer is opened in a browser whose preferred appearance is light and no appearance was chosen in that browser
- **THEN** the viewer is shown in the light appearance

#### Scenario: System preference changes while the page is open

- **WHEN** the system preference switches from light to dark while a screen is open and no explicit choice was made in that browser
- **THEN** the open screen switches to the dark appearance without a reload

### Requirement: Appearance control offers Light, Dark, and System

The shared header SHALL offer the three choices Light, Dark, and System, SHALL mark which one is active, and SHALL make the control available on every screen.

#### Scenario: Control is present on every screen

- **WHEN** the user opens the Changes, Files, Reader, or not-found screen
- **THEN** the appearance control is part of the shared header and names the current choice

#### Scenario: Choosing an appearance applies immediately

- **WHEN** the user chooses Light or Dark
- **THEN** the visible screen switches to that appearance at once and the control marks that choice as active

#### Scenario: Choosing System returns to the OS preference

- **WHEN** the user chooses System after having chosen Light or Dark
- **THEN** the viewer matches the browser's current preferred appearance and follows it from then on

### Requirement: Explicit appearance choice is remembered per browser

An explicit appearance choice SHALL persist across reloads and later visits in the same browser. It SHALL be stored in the browser only and SHALL NOT be written to the server, the memory root, or the version store.

#### Scenario: Reload keeps the chosen appearance

- **WHEN** the user chooses Dark and then reloads the page or restarts the viewer
- **THEN** the viewer opens in the dark appearance without briefly using the other one

#### Scenario: Another browser starts from the system preference

- **WHEN** the viewer is opened in a browser or profile that has no stored choice
- **THEN** it follows the system preference rather than a choice made elsewhere

#### Scenario: Browser storage is unavailable

- **WHEN** the browser refuses or discards the stored choice
- **THEN** the viewer still offers the control, applies choices for the current page, and falls back to the system preference on the next visit

#### Scenario: The viewer stays read-only

- **WHEN** the user changes the appearance
- **THEN** no memory file or version record is created, changed, or deleted
