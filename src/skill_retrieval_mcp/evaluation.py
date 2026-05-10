from __future__ import annotations

import json
import math
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

from .core import tokenize
from .models import SearchRequest


@dataclass(frozen=True)
class RetrievalCase:
    """A fully judged retrieval-quality case.

    relevant_skill_ids are acceptable retrieval targets. expected_top1 is stricter
    and is used for top-1 accuracy when present. forbidden_skill_ids are known
    false positives for this case and drive judged precision checks.
    """

    case_id: str
    intent: str
    difficulty: str
    language: str
    relevant_skill_ids: tuple[str, ...]
    expected_top1: str | None = None
    forbidden_skill_ids: tuple[str, ...] = ()
    raw_user_request: str = ""
    description_query: str = ""
    workflow_query: str = ""
    must_have: tuple[str, ...] = ()
    nice_to_have: tuple[str, ...] = ()
    must_not: tuple[str, ...] = ()
    environment: tuple[str, ...] = ()
    category: str | None = None
    k: int = 3
    max_tokens: int = 1200
    mmr_lambda: float = 0.70
    notes: str = ""

    def to_search_request(self) -> SearchRequest:
        return SearchRequest(
            raw_user_request=self.raw_user_request,
            description_query=self.description_query,
            workflow_query=self.workflow_query,
            must_have=list(self.must_have),
            nice_to_have=list(self.nice_to_have),
            must_not=list(self.must_not),
            environment=list(self.environment),
            category=self.category,
            k=self.k,
            max_tokens=self.max_tokens,
            mmr_lambda=self.mmr_lambda,
        )


def _tuple_str(value: Any, field_name: str) -> tuple[str, ...]:
    if value is None:
        return ()
    if not isinstance(value, list):
        raise ValueError(f"{field_name} must be a list of strings")
    result = tuple(str(item).strip() for item in value if str(item).strip())
    return result


def _required_str(data: dict[str, Any], field_name: str) -> str:
    value = data.get(field_name)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{field_name} must be a non-empty string")
    return value.strip()


def load_retrieval_cases(path: str | Path) -> list[RetrievalCase]:
    """Load JSONL retrieval-quality cases and validate judgement fields."""
    case_path = Path(path)
    cases: list[RetrievalCase] = []
    seen: set[str] = set()
    for line_no, line in enumerate(case_path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        data = json.loads(stripped)
        case_id = _required_str(data, "case_id")
        if case_id in seen:
            raise ValueError(f"duplicate case_id {case_id!r} at {case_path}:{line_no}")
        seen.add(case_id)
        relevant = _tuple_str(data.get("relevant_skill_ids"), "relevant_skill_ids")
        if not relevant:
            raise ValueError(f"{case_id}: relevant_skill_ids must not be empty")
        expected_top1 = data.get("expected_top1")
        if expected_top1 is not None:
            expected_top1 = str(expected_top1).strip()
            if expected_top1 and expected_top1 not in relevant:
                raise ValueError(f"{case_id}: expected_top1 must be listed in relevant_skill_ids")
        forbidden = _tuple_str(data.get("forbidden_skill_ids", []), "forbidden_skill_ids")
        overlap = set(relevant) & set(forbidden)
        if overlap:
            raise ValueError(f"{case_id}: relevant and forbidden overlap: {sorted(overlap)}")
        cases.append(RetrievalCase(
            case_id=case_id,
            intent=_required_str(data, "intent"),
            difficulty=_required_str(data, "difficulty"),
            language=_required_str(data, "language"),
            relevant_skill_ids=relevant,
            expected_top1=expected_top1 or None,
            forbidden_skill_ids=forbidden,
            raw_user_request=str(data.get("raw_user_request", "")),
            description_query=str(data.get("description_query", "")),
            workflow_query=str(data.get("workflow_query", "")),
            must_have=_tuple_str(data.get("must_have", []), "must_have"),
            nice_to_have=_tuple_str(data.get("nice_to_have", []), "nice_to_have"),
            must_not=_tuple_str(data.get("must_not", []), "must_not"),
            environment=_tuple_str(data.get("environment", []), "environment"),
            category=data.get("category"),
            k=int(data.get("k", 3)),
            max_tokens=int(data.get("max_tokens", 1200)),
            mmr_lambda=float(data.get("mmr_lambda", 0.70)),
            notes=str(data.get("notes", "")),
        ))
    return cases


def _average_precision_at_k(ranked_ids: list[str], relevant: set[str], k: int) -> float:
    if not relevant:
        return 0.0
    hits = 0
    score = 0.0
    for idx, skill_id in enumerate(ranked_ids[:k], start=1):
        if skill_id in relevant:
            hits += 1
            score += hits / idx
    return score / min(len(relevant), k)


def _judged_precision_at_k(ranked_ids: list[str], relevant: set[str], forbidden: set[str], k: int) -> float:
    judged = [skill_id for skill_id in ranked_ids[:k] if skill_id in relevant or skill_id in forbidden]
    if not judged:
        return 1.0
    return sum(1 for skill_id in judged if skill_id in relevant) / len(judged)


def _bucket_rates(outcomes: list[dict[str, Any]], field_name: str) -> dict[str, dict[str, float | int]]:
    buckets: dict[str, list[dict[str, Any]]] = {}
    for row in outcomes:
        buckets.setdefault(str(row[field_name]), []).append(row)
    result: dict[str, dict[str, float | int]] = {}
    for key, rows in sorted(buckets.items()):
        result[key] = {
            "case_count": len(rows),
            "top1_accuracy": round(sum(1 for row in rows if row["top1_hit"]) / len(rows), 4),
            "hit_rate_at_k": round(sum(1 for row in rows if row["hit_at_k"]) / len(rows), 4),
            "mean_average_precision_at_k": round(sum(float(row["average_precision_at_k"]) for row in rows) / len(rows), 4),
            "mean_judged_precision_at_k": round(sum(float(row["judged_precision_at_k"]) for row in rows) / len(rows), 4),
        }
    return result


def _normalised_entropy(labels: Iterable[str]) -> float:
    counts = Counter(label for label in labels if label)
    total = sum(counts.values())
    if total == 0 or len(counts) <= 1:
        return 0.0
    entropy = -sum((count / total) * math.log(count / total) for count in counts.values())
    return round(entropy / math.log(len(counts)), 4)


def _case_diversity_text(case: RetrievalCase) -> str:
    """Return the lexical surface used for dataset-diversity metrics.

    The text intentionally excludes gold labels so the metric measures request
    diversity rather than simply rewarding different `relevant_skill_ids`.
    Structured query fields are included because they are part of the actual
    SearchRequest surface used by this MCP.
    """
    return "\n".join([
        case.raw_user_request,
        case.description_query,
        case.workflow_query,
        " ".join(case.must_have),
        " ".join(case.nice_to_have),
        " ".join(case.must_not),
        " ".join(case.environment),
        case.category or "",
        case.intent,
        case.language,
    ])


def _tfidf_vectors(cases: list[RetrievalCase]) -> list[dict[str, float]]:
    counters = [Counter(tokenize(_case_diversity_text(case))) for case in cases]
    df: Counter[str] = Counter()
    for counts in counters:
        df.update(counts.keys())
    total_docs = len(cases)
    vectors: list[dict[str, float]] = []
    for counts in counters:
        vector: dict[str, float] = {}
        for token, count in counts.items():
            # Smooth IDF and log TF keep repeated boilerplate from dominating.
            tf = 1.0 + math.log(count)
            idf = math.log((total_docs + 1.0) / (df[token] + 1.0)) + 1.0
            vector[token] = tf * idf
        norm = math.sqrt(sum(weight * weight for weight in vector.values()))
        if norm:
            vector = {token: weight / norm for token, weight in vector.items()}
        vectors.append(vector)
    return vectors


def _cosine_sparse(left: dict[str, float], right: dict[str, float]) -> float:
    if not left or not right:
        return 0.0
    if len(left) > len(right):
        left, right = right, left
    return sum(weight * right.get(token, 0.0) for token, weight in left.items())


def _percentile(values: list[float], percentile: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    rank = (len(ordered) - 1) * percentile
    lower = math.floor(rank)
    upper = math.ceil(rank)
    if lower == upper:
        return ordered[int(rank)]
    weight = rank - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def _pair_record(case_a: RetrievalCase, case_b: RetrievalCase, cosine: float) -> dict[str, Any]:
    return {
        "case_id_a": case_a.case_id,
        "case_id_b": case_b.case_id,
        "cosine": round(cosine, 4),
        "intent_a": case_a.intent,
        "intent_b": case_b.intent,
        "language_a": case_a.language,
        "language_b": case_b.language,
        "gold_overlap": sorted(set(case_a.relevant_skill_ids) & set(case_b.relevant_skill_ids)),
    }


def _mmr_novelty_order(
    cases: list[RetrievalCase],
    vectors: list[dict[str, float]],
    similarities: dict[tuple[int, int], float],
    mmr_lambda: float,
) -> tuple[list[int], dict[int, float]]:
    if not cases:
        return [], {}
    # Start with the case that has the lowest average similarity to the rest;
    # this makes the novelty order deterministic and diversity-seeking even
    # before the first MMR step has any selected item to compare with.
    avg_similarities = []
    for idx in range(len(cases)):
        sims = [similarities[(min(idx, other), max(idx, other))] for other in range(len(cases)) if other != idx]
        avg_similarities.append(sum(sims) / len(sims) if sims else 0.0)
    first = min(range(len(cases)), key=lambda idx: (avg_similarities[idx], cases[idx].case_id))
    selected = [first]
    remaining = set(range(len(cases))) - {first}
    novelty_by_index: dict[int, float] = {first: 1.0}
    while remaining:
        best_idx = min(remaining)
        best_value = -1e9
        best_novelty = 0.0
        for idx in sorted(remaining):
            max_similarity = max(similarities[(min(idx, chosen), max(idx, chosen))] for chosen in selected)
            novelty = 1.0 - max_similarity
            # Relevance is constant for dataset curation; the MMR penalty is the
            # signal. Tiny lexical breadth bonus breaks ties in favour of richer
            # requests without overwhelming the diversity term.
            lexical_breadth = min(1.0, len(vectors[idx]) / 40.0)
            value = mmr_lambda * (1.0 + 0.05 * lexical_breadth) - (1.0 - mmr_lambda) * max_similarity
            if value > best_value:
                best_value = value
                best_idx = idx
                best_novelty = novelty
        selected.append(best_idx)
        novelty_by_index[best_idx] = best_novelty
        remaining.remove(best_idx)
    return selected, novelty_by_index


def evaluate_case_diversity(
    cases: Iterable[RetrievalCase],
    *,
    near_duplicate_threshold: float = 0.86,
    mmr_lambda: float = 0.65,
    low_novelty_threshold: float = 0.28,
    top_n_pairs: int = 20,
) -> dict[str, Any]:
    """Measure algorithmic diversity of labelled retrieval cases.

    Metrics combine TF-IDF cosine similarity (duplicate/cluster detection), MMR
    novelty ordering (how much each additional case contributes beyond already
    selected cases), and normalised entropy over judgement labels.
    """
    case_list = list(cases)
    vectors = _tfidf_vectors(case_list)
    similarities: dict[tuple[int, int], float] = {}
    pair_values: list[float] = []
    pair_records: list[dict[str, Any]] = []
    for idx, left in enumerate(vectors):
        similarities[(idx, idx)] = 1.0
        for jdx in range(idx + 1, len(vectors)):
            cosine = _cosine_sparse(left, vectors[jdx])
            similarities[(idx, jdx)] = cosine
            pair_values.append(cosine)
            pair_records.append(_pair_record(case_list[idx], case_list[jdx], cosine))
    pair_records.sort(key=lambda row: (-float(row["cosine"]), row["case_id_a"], row["case_id_b"]))
    near_duplicates = [row for row in pair_records if float(row["cosine"]) >= near_duplicate_threshold]
    mmr_order, novelty_by_index = _mmr_novelty_order(case_list, vectors, similarities, mmr_lambda)
    low_novelty = []
    for idx in mmr_order[1:]:
        novelty = novelty_by_index.get(idx, 0.0)
        if novelty < low_novelty_threshold:
            low_novelty.append({
                "case_id": case_list[idx].case_id,
                "mmr_novelty": round(novelty, 4),
                "max_prior_similarity": round(1.0 - novelty, 4),
            })
    gold_labels = [skill_id for case in case_list for skill_id in case.relevant_skill_ids]
    category_labels = [case.category or "uncategorized" for case in case_list]
    return {
        "case_count": len(case_list),
        "intent_entropy_norm": _normalised_entropy(case.intent for case in case_list),
        "language_entropy_norm": _normalised_entropy(case.language for case in case_list),
        "category_entropy_norm": _normalised_entropy(category_labels),
        "gold_skill_entropy_norm": _normalised_entropy(gold_labels),
        "mean_pairwise_cosine": round(sum(pair_values) / len(pair_values), 4) if pair_values else 0.0,
        "p95_pairwise_cosine": round(_percentile(pair_values, 0.95), 4),
        "max_pairwise_cosine": round(max(pair_values), 4) if pair_values else 0.0,
        "near_duplicate_threshold": near_duplicate_threshold,
        "near_duplicate_pairs": near_duplicates[:top_n_pairs],
        "top_similar_pairs": pair_records[:top_n_pairs],
        "mmr_lambda": mmr_lambda,
        "mmr_mean_novelty": round(sum(novelty_by_index.values()) / len(novelty_by_index), 4) if novelty_by_index else 0.0,
        "mmr_min_novelty": round(min(novelty_by_index.values()), 4) if novelty_by_index else 0.0,
        "mmr_low_novelty_threshold": low_novelty_threshold,
        "mmr_low_novelty_cases": low_novelty[:top_n_pairs],
        "mmr_selected_case_ids_head": [case_list[idx].case_id for idx in mmr_order[: min(12, len(mmr_order))]],
    }


def evaluate_retrieval_cases(engine: Any, cases: Iterable[RetrievalCase]) -> dict[str, Any]:
    """Run labelled retrieval cases and return accuracy/precision metrics."""
    case_list = list(cases)
    outcomes: list[dict[str, Any]] = []
    forbidden_topk_violations: list[dict[str, Any]] = []
    missing_expected_topk: list[dict[str, Any]] = []
    top1_misses: list[dict[str, Any]] = []

    for case in case_list:
        response = engine.search(case.to_search_request())
        ranked_ids = [str(item.get("skill_id")) for item in response.get("results", [])]
        relevant = set(case.relevant_skill_ids)
        forbidden = set(case.forbidden_skill_ids)
        expected_top1 = case.expected_top1
        top1 = ranked_ids[0] if ranked_ids else None
        top1_hit = top1 == expected_top1 if expected_top1 else bool(top1 in relevant if top1 else False)
        hit_at_k = bool(relevant & set(ranked_ids[:case.k]))
        average_precision = _average_precision_at_k(ranked_ids, relevant, case.k)
        judged_precision = _judged_precision_at_k(ranked_ids, relevant, forbidden, case.k)
        forbidden_hits = [skill_id for skill_id in ranked_ids[:case.k] if skill_id in forbidden]
        if forbidden_hits:
            forbidden_topk_violations.append({
                "case_id": case.case_id,
                "expected": sorted(relevant),
                "forbidden_hits": forbidden_hits,
                "ranked_ids": ranked_ids[:case.k],
            })
        if not hit_at_k:
            missing_expected_topk.append({
                "case_id": case.case_id,
                "expected": sorted(relevant),
                "ranked_ids": ranked_ids[:case.k],
            })
        if not top1_hit:
            top1_misses.append({
                "case_id": case.case_id,
                "expected_top1": expected_top1,
                "relevant": sorted(relevant),
                "top1": top1,
                "ranked_ids": ranked_ids[:case.k],
            })
        outcomes.append({
            "case_id": case.case_id,
            "intent": case.intent,
            "difficulty": case.difficulty,
            "language": case.language,
            "top1": top1,
            "ranked_ids": ranked_ids[:case.k],
            "top1_hit": top1_hit,
            "hit_at_k": hit_at_k,
            "average_precision_at_k": round(average_precision, 6),
            "judged_precision_at_k": round(judged_precision, 6),
            "forbidden_hits": forbidden_hits,
            "tokens_estimate": response.get("tokens_estimate"),
            "confidence": response.get("confidence"),
        })

    count = len(outcomes) or 1
    return {
        "case_count": len(outcomes),
        "top1_accuracy": round(sum(1 for row in outcomes if row["top1_hit"]) / count, 4),
        "hit_rate_at_k": round(sum(1 for row in outcomes if row["hit_at_k"]) / count, 4),
        "mean_average_precision_at_k": round(sum(float(row["average_precision_at_k"]) for row in outcomes) / count, 4),
        "mean_judged_precision_at_k": round(sum(float(row["judged_precision_at_k"]) for row in outcomes) / count, 4),
        "forbidden_topk_violations": forbidden_topk_violations,
        "missing_expected_topk": missing_expected_topk,
        "top1_misses": top1_misses,
        "by_intent": _bucket_rates(outcomes, "intent"),
        "by_difficulty": _bucket_rates(outcomes, "difficulty"),
        "by_language": _bucket_rates(outcomes, "language"),
        "outcomes": outcomes,
    }
