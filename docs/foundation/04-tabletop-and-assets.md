# Tabletop and assets

## Table and interaction

Netrunner's rules establish zones, server relationships, and ICE order; they do not mandate one universal playmat. Use a familiar physical arrangement adapted for legibility. Put the human's area nearest the camera whether playing Runner or Corp. Each player's ordinary cards face their seat; opposing cards rotate 180 degrees. ICE retains its sideways orientation and ordered protective column. Inspection panels present authorized card faces upright.

Corp has stable HQ, R&D, and Archives locations, plus expandable remote-server columns. Root cards sit behind their ICE, which extends toward the Runner. Keep faceup and facedown Archives distinguishable without revealing the latter. Group the Runner's rig by programs, hardware, and resources; provide grip, stack, heap, identity, score area, and relevant counters. No layout-imposed server or card cap may reject a legal position; pan, zoom, and expand spacing as needed.

An unrezzed agenda, asset, and upgrade in a remote must not be distinguishable by reserved slots, card spacing, outlines, names, hover data, or action hints. Use opaque backs uniformly within each side. The Corp may inspect its own installed hidden cards; opposing hidden faces remain unavailable. Run highlighting shows the current server/ICE/access stage without exposing future information.

Clicking selects a card or zone and shows currently available choices, costs, targets, and consequences known to that player. Selecting an ability can lead to explicit target, amount, payment-source, or ordering choices. Never make an optional decision for the human merely to simplify the UI. Skip only genuinely forced bookkeeping; automatic passing must not skip a meaningful legal opportunity. Contextual help explains timing and rejection reasons using the current rules basis, without claiming what unknown cards contain.

Credit/click pools and tokens hosted on cards are visually distinct. Rule changes happen in the core; animations explain them and can be shortened. Modal choice input must not also activate the underlying table. Bot thinking leaves camera, inspection, and UI responsive. Native checks on Jon's Ubuntu system matter: screenshots alone do not establish readable text, comfortable play, correct picking, or acceptable frame pacing.

## Save, undo, and history

Save complete current game state at stable decision boundaries, including a suspended run/effect, actor, pending choices, private knowledge, RNG state, deck/content/rules identities, and continuity needed by bots. Save UI preferences separately. Do not require reconstructing state from a log of every past action. Validate versions and content availability before loading; unsupported saves fail clearly without replacing a good save. A tested migration is explicit, never a best-effort reinterpretation of rules.

Use safe replacement with a recoverable previous record; [Arkham's candidate](05-shared-code-and-development.md) needs admission checks owned by Netrunner. Verify interrupted/corrupt writes and recovery. A save survives application restart and offline operation, including mid-run decisions. Restarting may restart bot computation, but must not change committed chance outcomes or reveal information.

Undo is a rollback of a complete reversible choice, including counters, RNG state, pending effects, and history presentation. It is available only before new information or an opponent response; a draw, reveal, access, random result, or opponent choice seals the boundary as applicable. Cancel stale bot work on rollback. Choice previews can be edited before commitment. Explain why undo is unavailable rather than silently rewinding across a knowledge boundary or allowing rerolls.

The action history records resolved effects, costs, and causes in readable terms. Redact according to the viewer's knowledge at the event; do not retrospectively reveal hidden draws or internal bot beliefs. Contextual explanations are deterministic application content, not LLM narration. History and diagnostic traces do not add a completed-game replay requirement.

## External assets

Use the supplied [System Gateway A4 3× PDF](https://access.nullsignal.games/Gateway/English/English/SystemGatewayEnglish-A4%20Printable%20Sheets%203x.pdf) as the initial art source. The inspected file is rasterized by sheet; extraction needs explicit page/rectangle mappings. A reviewed one-card crop demonstrates a viable seam, not a finished pipeline. [Evidence](../research/evidence.md#pdf-extraction-evidence) records its hash and probe.

Maintain a Rust preparation command that verifies the PDF revision, renders/crops with an existing PDF backend, associates each image with a stable printing ID, removes repeated copies, and checks output. Calling installed Poppler from Rust is compatible with the language policy; a maintained Python preparation pipeline is not. Do not infer semantics from OCR, print position, or duplicate artwork. Review all 45 required faces for complete bounds, correct identity, orientation, and readable detail before claiming extraction complete.

PDFs, images, model weights, logs, and saves stay outside Git. Use configurable external roots with sensible XDG defaults; checked-in configuration contains source references, crop recipes, expected hashes, and relative asset keys, not machine-specific absolute paths. Missing files produce an actionable preparation error. Fully prepared play requires no downloads. Text placeholders are useful diagnostics but do not satisfy the finished physical-table experience.

The supplied sheets omit card backs. Start with original neutral opaque sleeves and geometric tokens; official imagery can be added only with its applicable terms. MIT code licensing does not cover card art. Null Signal's [visual-assets license](https://nullsignal.games/about/nsg-visual-assets/) covers the included symbols/glyphs, not all artwork, frames, or backs. Preserve attribution and notices; do not publish a mirror of extracted artwork merely because it is stored outside the source repository. Show current effective card text alongside older printed wording when they differ.
