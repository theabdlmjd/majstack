---
name: visual-parity
group: build
mode: write
summary: Make one UI match a reference pixel for pixel.
---
Capture the reference and the current UI with `majstack browse --shot`. Compare them region by region (layout, spacing, type, color). Fix one difference at a time, recapture, and repeat until they match at the target viewport widths. List any intentional differences.
