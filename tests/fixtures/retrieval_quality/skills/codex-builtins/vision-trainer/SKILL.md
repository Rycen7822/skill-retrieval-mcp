---
name: "huggingface-vision-trainer"
description: Codex curated Hugging Face skill for training or fine-tuning vision models with image datasets, transforms, metrics, checkpoints, and Hub integration.
tags: [codex-builtin, huggingface, vision, training, image-classification]
---
# huggingface-vision-trainer
## When to Use
Use for image classification or vision model training pipelines, dataset transforms, evaluation metrics, and checkpoint publishing.
## Do Not Use When
Do not use for text-only LLM fine-tuning or Gradio UI-only changes.
## Required Inputs
Image dataset, model checkpoint, transforms, metrics, hardware constraints, and output repo or checkpoint path.
## Workflow
1. Load and inspect image dataset splits.
2. Define transforms and trainer config.
3. Run a small smoke train/eval.
4. Save and optionally push the checkpoint.
