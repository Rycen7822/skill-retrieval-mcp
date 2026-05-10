---
name: academic-plotting
description: Generate publication-quality plots for ML papers from experiment CSV/JSON logs, including ablations, confidence intervals, and camera-ready styling.
tags: [plotting, matplotlib, figures, ablation, ml, paper]
---
# Academic Plotting
## When to Use
Use when the user needs polished research figures, ablation plots, learning curves, confidence intervals, or camera-ready chart styling from local experiment data.
## Do Not Use When
Do not use for taxonomy CSV consolidation, document OCR conversion, or text humanization.
## Required Inputs
Data files, metric names, grouping variables, target venue style, figure size, and output path.
## Workflow
1. Load experiment logs and validate metric columns.
2. Aggregate means, standard deviations, and confidence intervals.
3. Render publication-quality plots with labels and legends.
4. Save figure files and reproducible plotting scripts.
