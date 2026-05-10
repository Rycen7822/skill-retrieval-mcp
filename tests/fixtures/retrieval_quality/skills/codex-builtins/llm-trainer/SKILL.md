---
name: "huggingface-llm-trainer"
description: Codex curated Hugging Face skill for training or fine-tuning language models with datasets, tokenizers, trainers, LoRA, metrics, and Hub checkpoints.
tags: [codex-builtin, huggingface, llm, training, fine-tuning]
---
# huggingface-llm-trainer
## When to Use
Use for LLM fine-tuning, tokenizer setup, training arguments, LoRA adapters, evaluation metrics, or Hub checkpoint publishing.
## Do Not Use When
Do not use for vision-only training or simple Gradio UI work.
## Required Inputs
Base model, dataset, tokenizer strategy, hardware budget, training configuration, and checkpoint destination.
## Workflow
1. Inspect model and dataset sizes.
2. Configure tokenizer and trainer.
3. Run a tiny smoke training step.
4. Evaluate and save/push outputs.
