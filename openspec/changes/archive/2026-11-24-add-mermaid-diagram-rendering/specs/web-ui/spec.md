# Spec Delta

## ADDED Requirements

### Requirement: Reader draws Mermaid diagrams

The Reader SHALL draw a fenced code block whose info string is `mermaid` as an SVG diagram wherever it renders a memory file's markdown — in the current content and in every recorded version. Every other code block SHALL render exactly as before, as escaped source text.

#### Scenario: A diagram in the current content

- **WHEN** the Reader renders a file containing a ` ```mermaid ` block with a valid flowchart
- **THEN** the block is shown as a drawn diagram, in place of its source text, inside the rendered file

#### Scenario: A diagram in a past version

- **WHEN** the user selects a recorded version whose content contains a Mermaid block
- **THEN** that version's diagram is drawn, and the screen still marks the version as historical

#### Scenario: Other code blocks are untouched

- **WHEN** a file contains a fenced block with a language other than `mermaid`, or a plain code block
- **THEN** it stays a code block with its language class and source text, and no diagram is drawn for it

#### Scenario: Inline code and prose are untouched

- **WHEN** a file mentions the word `mermaid` in prose or in inline code without a `mermaid` fence
- **THEN** nothing on the page is turned into a diagram

#### Scenario: A file with several diagrams

- **WHEN** one file contains two or more `mermaid` blocks with different diagram types
- **THEN** each block is drawn as its own diagram, in the order the blocks appear

### Requirement: An undrawable diagram keeps its source

When a Mermaid block's content is invalid, unsupported, or too large to draw, the Reader SHALL keep that block's source visible and readable and SHALL mark it as not drawn. A failed diagram SHALL NOT become an error page, SHALL NOT hide the rest of the file, and SHALL NOT stop the file's other diagrams from being drawn.

#### Scenario: Invalid diagram source

- **WHEN** the Reader renders a `mermaid` block whose source does not parse
- **THEN** the block's source remains visible with a note that the diagram could not be drawn, and the rest of the file still renders

#### Scenario: One bad diagram among good ones

- **WHEN** a file contains one invalid and one valid `mermaid` block
- **THEN** the valid block is drawn and the invalid one keeps its source with the note

#### Scenario: The fallback is readable

- **WHEN** a diagram has fallen back to its source
- **THEN** the source keeps the formatting of a code block in both appearances, rather than appearing as unstyled text

### Requirement: Diagrams follow the viewer appearance

A drawn diagram SHALL use colours and type legible in the appearance in effect, and SHALL be redrawn in the matching appearance when the user changes it, without reloading, rebuilding the Reader, or losing the shown file, version, scroll position, or the settings dialog's open state.

#### Scenario: Diagram in the light appearance

- **WHEN** the viewer is in the light appearance and the Reader shows a diagram
- **THEN** its text and shapes are drawn against light-appropriate colours

#### Scenario: Diagram in the dark appearance

- **WHEN** the viewer is in the dark appearance and the Reader shows a diagram
- **THEN** its text and shapes are drawn against dark-appropriate colours, not the light palette

#### Scenario: Appearance changes while a diagram is shown

- **WHEN** the user chooses the other appearance while the Reader shows a diagram
- **THEN** the diagram is redrawn to match, the shown version and scroll position stay as they were, and the settings dialog stays open

#### Scenario: System preference changes while a diagram is shown

- **WHEN** the viewer follows the system preference and that preference flips while a diagram is shown
- **THEN** the diagram is redrawn to match without a reload

### Requirement: Diagrams stay inside the page

A diagram SHALL NOT widen the content shell, push the page sideways, or overflow outside its file content area. A diagram wider than the available measure SHALL scale down to it and SHALL remain reachable at its full size inside its own scrolling frame. A diagram smaller than the measure SHALL be drawn at its natural size rather than stretched.

#### Scenario: A wide diagram at the smallest page width

- **WHEN** the user selects the smallest page width and opens a file with a wide diagram
- **THEN** the page keeps that width, no horizontal page scrollbar appears, and the diagram can be inspected by scrolling within its own frame

#### Scenario: A small diagram is not stretched

- **WHEN** a file's diagram is narrower than the reading measure
- **THEN** it is drawn at its natural size and left-aligned, not scaled up

#### Scenario: Diagram text stays legible

- **WHEN** a diagram is scaled down to fit the measure
- **THEN** it is not shrunk below the point where its labels become unreadable, and the frame offers the full-size view instead

### Requirement: Diagram support loads only when a diagram is present

The viewer SHALL request the diagram renderer only after it renders a file that actually contains a `mermaid` block, and SHALL take that renderer from its own origin. A screen or file without a diagram SHALL make no diagram-renderer request. If the renderer cannot be fetched or started, diagrams SHALL fall back to their source as a failed diagram does, and the rest of the page SHALL keep working.

#### Scenario: A file without a diagram

- **WHEN** the Reader opens a file with no `mermaid` block
- **THEN** no diagram-renderer request is made and the page behaves exactly as before

#### Scenario: The renderer comes from the viewer's origin

- **WHEN** a file with a diagram is opened
- **THEN** the renderer is fetched from the viewer's own address and nothing is requested from a third-party host

#### Scenario: The renderer is unavailable

- **WHEN** the renderer cannot be fetched or fails to start
- **THEN** every diagram on the page shows its source with the not-drawn note, and the rest of the file renders normally

#### Scenario: The viewer stays offline-capable

- **WHEN** the machine has no network access and a file with a diagram is opened
- **THEN** the diagram is drawn as usual

### Requirement: Diagram content cannot run code or reach the network

A diagram's source SHALL be treated as data, never as page markup or script: labels, titles, links, and directives in it SHALL NOT execute script, open a page, or navigate the viewer. Drawing a diagram SHALL NOT make any network request beyond the viewer's own renderer asset. The page's Content-Security-Policy SHALL continue to forbid inline and third-party script.

#### Scenario: Script in a diagram label

- **WHEN** a `mermaid` block contains markup such as `<script>` or an event-handler attribute in a node label
- **THEN** it is drawn as literal text or the diagram falls back to source, and no script runs

#### Scenario: Click directives and links

- **WHEN** a `mermaid` block contains `click` directives or `href` links in nodes
- **THEN** nodes are not clickable and nothing navigates away, opens a window, or loads a URL

#### Scenario: Remote resources in a diagram

- **WHEN** a `mermaid` block tries to reference a remote image, font, or stylesheet
- **THEN** nothing is fetched over the network and the diagram renders without it

#### Scenario: The policy stays script-strict

- **WHEN** any screen address is requested
- **THEN** the response's policy allows script only from the viewer's own origin, forbids inline script and inline event handlers, and permits inline styles only for drawing

## MODIFIED Requirements

### Requirement: Rendered markdown cannot execute scripts

When rendering memory file content as HTML, the viewer SHALL escape embedded HTML so that content in a memory file cannot execute script in the browser. A drawn diagram SHALL inherit that guarantee even though drawing may add inline style to the page.

#### Scenario: Memory file contains raw HTML

- **WHEN** a memory file contains markup such as a `<script>` tag
- **THEN** the page displays it as text and no script runs

#### Scenario: Inline styles do not open a script path

- **WHEN** the page carries the inline style a drawn diagram produced
- **THEN** no inline script or event handler in the page can run, and memory content still cannot introduce markup of its own
