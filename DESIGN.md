# Z-Split visual system

The seven images in wireframe/ are the visual authority. Preserve their layouts and exact product voice.
White paper inside a dark charcoal frame; black ink, acid lime #d7fa3c, mint green #68bf95, pale yellow #f2e97b. Anton headings, Patrick Hand prose, compact bold sans-serif controls. All fonts are locally hosted.
Headings are oversized, condensed, uppercase with tight line height. Outlined pills, round black action caps, hand-shaped SVG graphics, dotted dividers, QR tickets. Preserve the supplied illustrations using custom lightweight SVG components. No stock icon library.
Organizer screens use a wide canvas, generous whitespace, and a two-column form/dashboard. Recipient screens preserve the narrow phone composition on large devices. At smaller sizes organizer views stack and claim cards reduce from six to three to two columns.
Motion: spring page entrances and selectable cards, 40ms QR staggering, counter transitions, brief press feedback, one-time illustration entrance. Only transform and opacity animate. Reduced motion disables positional motion. No idle animation in task UI.

## November 4 refinement

The latest user request supersedes the original full-screen magnification: remove viewport-driven root scaling, cap desktop workspace widths, use a 14px desktop rem baseline with smaller headings and consistent outlined controls. Mobile remains at16px for comfortable input sizing.
The landing page is now a complete scrolling product explanation, followed by payout options, privacy, use cases, FAQ, and a final CTA. Preserve the reference palette, lettering, and custom SVG identity. Each of the twelve bottom-hero illustrations is a focusable draggable toy with hover tilt and interruptible spring return, implemented with the existing Motion dependency. Marketing sections reveal once on scroll with native IntersectionObserver. Honor reduced motion and keep task screens still.
Additional frontend routes: /rooms (current room overview), /claim (paste a claim link), /how-it-works, and /privacy. The frontend currently stores one room per browser; do not imply secure multi-user or real-payment behavior.
