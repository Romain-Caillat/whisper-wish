"""NLLB-200 Translation Server for SubsForge."""

import os
from contextlib import asynccontextmanager

import torch
from fastapi import FastAPI
from pydantic import BaseModel
from transformers import AutoModelForSeq2SeqLM, AutoTokenizer

MODEL_NAME = os.environ.get("MODEL_NAME", "facebook/nllb-200-distilled-600M")
DEVICE = "mps" if torch.backends.mps.is_available() else "cpu"
MAX_BATCH = 128

model = None
tokenizer = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    global model, tokenizer
    print(f"Loading model {MODEL_NAME} on {DEVICE}...")
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
    model = AutoModelForSeq2SeqLM.from_pretrained(MODEL_NAME).to(DEVICE)
    model.eval()
    print(f"Model loaded. Ready to translate.")
    yield
    del model, tokenizer


app = FastAPI(title="SubsForge Translator", lifespan=lifespan)


class TranslateRequest(BaseModel):
    text: list[str]
    source_lang: str  # e.g. "eng_Latn"
    target_lang: str  # e.g. "fra_Latn"


class TranslateResponse(BaseModel):
    translations: list[str]


@app.post("/translate")
async def translate(req: TranslateRequest) -> TranslateResponse:
    tokenizer.src_lang = req.source_lang
    target_id = tokenizer.convert_tokens_to_ids(req.target_lang)

    all_translations = []

    # Process in batches to avoid OOM
    for i in range(0, len(req.text), MAX_BATCH):
        batch = req.text[i : i + MAX_BATCH]
        inputs = tokenizer(
            batch,
            return_tensors="pt",
            padding=True,
            truncation=True,
            max_length=512,
        ).to(DEVICE)

        with torch.no_grad():
            outputs = model.generate(
                **inputs,
                forced_bos_token_id=target_id,
                max_new_tokens=512,
            )

        decoded = tokenizer.batch_decode(outputs, skip_special_tokens=True)
        all_translations.extend(decoded)

    return TranslateResponse(translations=all_translations)


@app.get("/health")
async def health():
    return {"status": "ok", "model": MODEL_NAME, "device": str(DEVICE)}
