from __future__ import annotations

import hashlib
import json
import math
import os
import re
from collections import Counter
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Iterable

import yaml

from .models import AVAILABLE_VIEWS, CachePayload, FileFingerprint, LoadRequest, SearchRequest, SectionRecord, SkillRecord

CACHE_VERSION = 3
TOKEN_RE = re.compile(r"[a-zA-Z0-9_]+|[\u4e00-\u9fff]+")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$")
SAFE_ID_RE = re.compile(r"^[A-Za-z0-9_.:@+-]+$")

STOPWORDS = {
    "the", "a", "an", "and", "or", "to", "of", "in", "on", "for", "with", "when", "this", "that",
    "use", "using", "user", "asks", "ask", "please", "need", "needs", "be", "is", "are", "by", "as",
    "then", "after", "before", "into", "from", "it", "its", "only", "do", "not", "you", "your",
}

ACTION_TOKENS = {
    "review", "inspect", "implement", "fix", "debug", "create", "open", "push", "merge", "commit", "delete",
    "remove", "deploy", "write", "edit", "modify", "generate", "download", "install", "run", "execute",
}

RISK_PATTERNS = [
    ("NETWORK", ("http", "https", "api", "github", "download", "web", "network", "curl", "wget", "gh ")),
    ("GIT", ("git", "github", "pull request", "pr", "branch", "commit", "diff")),
    ("PROCESS", ("run ", "execute", "command", "terminal", "pytest", "npm", "uv ", "python -m", "build", "test")),
    ("LOCAL_FS_READ", ("read", "inspect", "file", "diff", "repository", "repo", "open")),
    ("LOCAL_FS_WRITE", ("write", "edit", "patch", "create", "update", "commit", "save", "delete", "remove")),
    ("DESTRUCTIVE_POSSIBLE", ("delete", "remove", "drop", "overwrite", "push", "merge", "deploy")),
]

VIEW_SECTIONS = {
    "runtime": ["required-inputs", "workflow", "process", "steps", "verification", "pitfalls", "common-pitfalls"],
    "risk": ["risk", "security", "pitfalls", "do-not-use-when", "required-inputs"],
}


def default_roots() -> list[Path]:
    raw = os.environ.get("SRM_SKILL_ROOTS", "").strip()
    if raw:
        return [Path(part).expanduser().resolve() for part in raw.split(":") if part.strip()]
    return [Path.home() / ".hermes" / "skills"]


def default_cache_path() -> Path:
    raw = os.environ.get("SRM_CACHE_PATH", "").strip()
    if raw:
        return Path(raw).expanduser().resolve()
    return Path.home() / ".cache" / "skill-retrieval-mcp" / "index.json"


def tokenize(text: str) -> list[str]:
    tokens: list[str] = []
    for match in TOKEN_RE.finditer(text.lower()):
        tok = match.group(0).strip("_")
        if not tok or tok in STOPWORDS:
            continue
        if len(tok) > 3 and tok.endswith("ing"):
            tok = tok[:-3]
        elif len(tok) > 3 and tok.endswith("ed"):
            tok = tok[:-2]
        elif len(tok) > 3 and tok.endswith("s"):
            tok = tok[:-1]
        tokens.append(tok)
    return tokens


def slugify(text: str) -> str:
    # Slugs must preserve structural words in headings such as
    # "When to Use" and "Do Not Use When"; scoring tokenization may drop
    # stopwords, but ids must remain stable and human-recognisable.
    raw_tokens = [m.group(0).strip("_").lower() for m in TOKEN_RE.finditer(text) if m.group(0).strip("_")]
    slug = "-".join(raw_tokens)
    return slug or "section"


def estimate_tokens(text: str) -> int:
    return max(1, math.ceil(len(text) / 4))


def trim_to_token_budget(text: str, max_tokens: int) -> tuple[str, bool, int]:
    if estimate_tokens(text) <= max_tokens:
        return text, False, estimate_tokens(text)
    suffix = "\n[TRUNCATED]"
    max_chars = max(0, max_tokens * 4 - len(suffix))
    trimmed = text[:max_chars].rstrip() + suffix
    return trimmed, True, estimate_tokens(trimmed)


def _sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _query_id(data: dict[str, Any]) -> str:
    raw = json.dumps(data, sort_keys=True, ensure_ascii=False)
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()[:16]


def _load_frontmatter(text: str) -> tuple[dict[str, Any], str]:
    if not text.startswith("---\n"):
        return {}, text
    try:
        _, front, body = text.split("---\n", 2)
    except ValueError:
        return {}, text
    try:
        data = yaml.safe_load(front) or {}
        if not isinstance(data, dict):
            data = {}
    except yaml.YAMLError:
        data = {}
    return data, body


def _normalise_tags(value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [str(item) for item in value if str(item).strip()]
    return []


def _extract_sections(body: str) -> dict[str, SectionRecord]:
    lines = body.splitlines()
    headings: list[tuple[int, int, str]] = []
    for idx, line in enumerate(lines):
        match = HEADING_RE.match(line)
        if match:
            headings.append((idx, len(match.group(1)), match.group(2).strip()))
    sections: dict[str, SectionRecord] = {}
    for pos, (start, level, title) in enumerate(headings):
        end = len(lines)
        for nxt_start, nxt_level, _ in headings[pos + 1:]:
            if nxt_level <= level:
                end = nxt_start
                break
        section_id = slugify(title)
        if section_id in sections:
            section_id = f"{section_id}-{pos + 1}"
        content = "\n".join(lines[start:end]).strip()
        sections[section_id] = SectionRecord(
            section_id=section_id,
            title=title,
            level=level,
            content=content,
            start_line=start + 1,
            end_line=end,
        )
    return sections


def _section_text(sections: dict[str, SectionRecord], *ids: str) -> str:
    parts = []
    for sid in ids:
        if sid in sections:
            # Remove the heading line for compact summaries.
            lines = sections[sid].content.splitlines()
            if lines and HEADING_RE.match(lines[0]):
                lines = lines[1:]
            parts.append("\n".join(lines).strip())
    return "\n\n".join(part for part in parts if part)


def _extract_by_aliases(sections: dict[str, SectionRecord], aliases: Iterable[str]) -> str:
    for alias in aliases:
        if alias in sections:
            return _section_text(sections, alias)
    return ""


def _risk_flags(text: str) -> list[str]:
    low = text.lower()
    flags = []
    for flag, patterns in RISK_PATTERNS:
        if any(pattern in low for pattern in patterns):
            flags.append(flag)
    return sorted(set(flags))


def _canonical_id(frontmatter: dict[str, Any], path: Path) -> str:
    raw = str(frontmatter.get("name") or path.parent.name).strip()
    return slugify(raw) or path.parent.name


def _category_for(root: Path, path: Path) -> str:
    try:
        rel = path.parent.relative_to(root)
    except ValueError:
        return "local"
    return rel.parts[0] if len(rel.parts) > 1 else "uncategorized"


def _trust_level(path: Path, roots: list[Path]) -> str:
    home_skills = (Path.home() / ".hermes" / "skills").resolve()
    resolved = path.resolve()
    try:
        resolved.relative_to(home_skills)
        return "user-hermes"
    except ValueError:
        pass
    for root in roots:
        try:
            resolved.relative_to(root.resolve())
            return "local-root"
        except ValueError:
            continue
    return "unknown"


def _make_card(name: str, description: str, tags: list[str], use_when: str, required_inputs: str) -> str:
    parts = [f"{name}: {description}".strip()]
    if tags:
        parts.append("Tags: " + ", ".join(tags[:8]))
    if use_when:
        parts.append("Use when: " + " ".join(use_when.split())[:500])
    if required_inputs:
        parts.append("Required: " + " ".join(required_inputs.split())[:300])
    return "\n".join(part for part in parts if part).strip()


def _workflow_summary(sections: dict[str, SectionRecord], description: str) -> str:
    text = _extract_by_aliases(sections, ("workflow", "process", "steps", "procedure", "core-principle"))
    if not text:
        text = description
    lines = []
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or HEADING_RE.match(stripped):
            continue
        lines.append(stripped)
        if len(" ".join(lines)) > 1000:
            break
    return "\n".join(lines).strip()


def _parse_skill(path: Path, root: Path, roots: list[Path]) -> SkillRecord:
    content = path.read_text(encoding="utf-8")
    frontmatter, body = _load_frontmatter(content)
    sections = _extract_sections(body)
    name = str(frontmatter.get("name") or path.parent.name).strip()
    description = str(frontmatter.get("description") or "").strip()
    tags = _normalise_tags(frontmatter.get("tags"))
    if not tags:
        meta = frontmatter.get("metadata")
        if isinstance(meta, dict):
            hermes = meta.get("hermes")
            if isinstance(hermes, dict):
                tags = _normalise_tags(hermes.get("tags"))
    use_when = _extract_by_aliases(sections, ("when-to-use", "use-when", "overview"))
    do_not = _extract_by_aliases(sections, ("do-not-use-when", "when-not-to-use", "dont-use-when"))
    required = _extract_by_aliases(sections, ("required-inputs", "requirements", "setup", "required-environment-variables"))
    workflow = _workflow_summary(sections, description)
    card = _make_card(name, description, tags, use_when, required)
    stat = path.stat()
    return SkillRecord(
        skill_id=_canonical_id(frontmatter, path),
        name=name,
        description=description,
        category=_category_for(root, path),
        tags=tags,
        skill_card=card,
        workflow_summary=workflow,
        use_when=use_when,
        do_not_use_when=do_not,
        required_inputs=required,
        risk_flags=_risk_flags(content),
        sections=sections,
        source_path=str(path),
        source_sha256=_sha256(content),
        mtime_ns=stat.st_mtime_ns,
        size=stat.st_size,
        trust_level=_trust_level(path, roots),
        content=content,
    )


def _discover_files(roots: list[Path]) -> list[Path]:
    files: list[Path] = []
    for root in roots:
        if not root.exists():
            continue
        files.extend(path.resolve() for path in root.rglob("SKILL.md") if path.is_file())
    return sorted(files, key=lambda p: str(p))


def _fingerprints(files: list[Path]) -> list[FileFingerprint]:
    result = []
    for path in files:
        stat = path.stat()
        result.append(FileFingerprint(path=str(path), mtime_ns=stat.st_mtime_ns, size=stat.st_size))
    return result


@dataclass(frozen=True)
class PreparedText:
    text: str
    lower: str
    counts: Counter[str]
    tokens: frozenset[str]


@dataclass(frozen=True)
class IdentityAlias:
    kind: str
    alias: str
    tokens: tuple[str, ...]
    pattern: re.Pattern[str]


@dataclass(frozen=True)
class PreparedRecord:
    description: PreparedText
    workflow: PreparedText
    raw: PreparedText
    metadata: PreparedText
    positive: PreparedText
    negative: PreparedText
    identity_aliases: tuple[IdentityAlias, ...]


def _prepare_text(text: str) -> PreparedText:
    toks = tokenize(text)
    return PreparedText(text=text, lower=text.lower(), counts=Counter(toks), tokens=frozenset(toks))


def _prepare_identity_aliases(record: SkillRecord) -> tuple[IdentityAlias, ...]:
    aliases: list[IdentityAlias] = []
    seen: set[tuple[str, str]] = set()
    for kind, value in (("skill_id", record.skill_id), ("name", record.name)):
        value = value.strip()
        values = [value]
        phrase = " ".join(tokenize(value))
        if phrase and phrase != value.lower():
            values.append(phrase)
        for idx, alias in enumerate(values):
            alias_kind = kind if idx == 0 else kind + "_phrase"
            key = (alias_kind, alias)
            if not alias or key in seen:
                continue
            seen.add(key)
            pattern = re.compile(rf"(?<![A-Za-z0-9_:@+.-]){re.escape(alias.lower())}(?![A-Za-z0-9_:@+.-])")
            aliases.append(IdentityAlias(alias_kind, alias, tuple(tokenize(alias)), pattern))
    return tuple(aliases)


def _prepare_record(record: SkillRecord) -> PreparedRecord:
    desc_text = "\n".join([record.description, record.skill_card, record.use_when, " ".join(record.tags)])
    workflow_text = "\n".join([record.workflow_summary, "\n".join(s.title for s in record.sections.values())])
    metadata_text = " ".join([record.category, " ".join(record.tags), " ".join(record.risk_flags), record.required_inputs])
    return PreparedRecord(
        description=_prepare_text(desc_text),
        workflow=_prepare_text(workflow_text),
        raw=_prepare_text(record.positive_text),
        metadata=_prepare_text(metadata_text),
        positive=_prepare_text(record.positive_text),
        negative=_prepare_text(record.do_not_use_when),
        identity_aliases=_prepare_identity_aliases(record),
    )


def _prepare_query_text(text: str) -> dict[str, Any]:
    return {"raw": text, "tokens": tokenize(text), "phrases": _important_phrases(text)}


def _prepare_query(request: SearchRequest) -> dict[str, Any]:
    raw = _prepare_query_text(request.raw_user_request)
    description = _prepare_query_text(request.description_query)
    workflow = _prepare_query_text(request.workflow_query)
    return {
        "description": description,
        "workflow": workflow,
        "raw": raw,
        "metadata": _prepare_query_text(" ".join(request.environment + request.nice_to_have)),
        "identity_text": "\n".join([request.raw_user_request, request.description_query, request.workflow_query]).lower(),
        "identity_tokens": raw["tokens"] + description["tokens"] + workflow["tokens"],
        "must_have": [(cue, tokenize(cue), " ".join(tokenize(cue))) for cue in request.must_have],
        "must_not": [(cue, tokenize(cue), " ".join(tokenize(cue))) for cue in request.must_not],
    }


def _score_prepared(query: dict[str, Any], prepared: PreparedText) -> tuple[float, list[str]]:
    q_tokens: list[str] = query["tokens"]
    if not q_tokens or not prepared.counts:
        return 0.0, []
    matched = []
    weighted_hits = 0.0
    for tok in q_tokens:
        if tok in prepared.counts:
            matched.append(tok)
            weighted_hits += min(2.0, 1.0 + math.log1p(prepared.counts[tok]) / 3.0)
    overlap = weighted_hits / max(1, len(q_tokens))
    phrase_bonus = 0.0
    for phrase in query["phrases"]:
        if phrase in prepared.lower:
            phrase_bonus += 0.08
    return min(1.0, overlap + phrase_bonus), sorted(set(matched))


def _score_text(query: str, text: str) -> tuple[float, list[str]]:
    return _score_prepared(_prepare_query_text(query), _prepare_text(text))


def _contains_token_phrase(tokens: list[str], phrase_tokens: list[str]) -> bool:
    if not tokens or not phrase_tokens or len(phrase_tokens) > len(tokens):
        return False
    width = len(phrase_tokens)
    return any(tokens[idx:idx + width] == phrase_tokens for idx in range(len(tokens) - width + 1))


def _identity_match(prep: PreparedRecord, qprep: dict[str, Any]) -> tuple[float, list[str]]:
    """Return an exact skill-identity boost and human-readable matches.

    Stress testing showed that rich neighboring skills such as github-issues can
    outrank github-repo-management even when the query explicitly names the
    target skill. This boost is deliberately exact and bounded: full multi-token
    ids/names receive a stronger boost; single-token router names receive only a
    small boost so they do not dominate specific hyphenated skill ids.
    """
    query_text = qprep["identity_text"]
    query_tokens = qprep["identity_tokens"]
    best = 0.0
    matches: list[str] = []
    for alias in prep.identity_aliases:
        if not alias.tokens:
            continue
        exact_text = bool(alias.pattern.search(query_text))
        phrase_text = _contains_token_phrase(query_tokens, list(alias.tokens))
        if not (exact_text or phrase_text):
            continue
        if len(alias.tokens) > 1:
            bonus = 0.24 if alias.kind in {"skill_id", "name"} and exact_text else 0.18
        else:
            bonus = 0.08 if exact_text else 0.04
        if bonus > best:
            best = bonus
        matches.append(f"{alias.kind}={alias.alias}")
    return best, matches[:4]


def _important_phrases(text: str) -> list[str]:
    # Keep 2-5 token phrases; useful for hard cues like "code review" or "git diff".
    toks = tokenize(text)
    phrases = []
    for n in range(2, min(5, len(toks)) + 1):
        for idx in range(len(toks) - n + 1):
            phrase = " ".join(toks[idx:idx + n])
            if len(phrase) > 4:
                phrases.append(phrase)
    return phrases[:80]


def _cue_present(cue: str, text: str) -> bool:
    cue_tokens = tokenize(cue)
    return _cue_tokens_present(cue_tokens, " ".join(cue_tokens), _prepare_text(text))


def _cue_tokens_present(cue_tokens: list[str], phrase: str, prepared: PreparedText) -> bool:
    if not cue_tokens:
        return False
    if all(tok in prepared.tokens for tok in cue_tokens):
        return True
    return bool(phrase and phrase in prepared.lower)


def _jaccard(a: str, b: str) -> float:
    return _jaccard_sets(set(tokenize(a)), set(tokenize(b)))


def _jaccard_sets(ta: set[str] | frozenset[str], tb: set[str] | frozenset[str]) -> float:
    if not ta or not tb:
        return 0.0
    return len(ta & tb) / len(ta | tb)


def _negative_conflict(raw_query: dict[str, Any], negative_text: str) -> tuple[float, list[str]]:
    """Return a do-not-use conflict score with action-sensitive matching.

    A negative sentence such as "do not use when creating a pull request" must
    not fire merely because the raw request says "review this pull request".
    We therefore require overlap on at least one action verb plus enough
    supporting tokens from the same negative sentence.
    """
    raw_tokens = set(raw_query["tokens"])
    if not raw_tokens or not negative_text.strip():
        return 0.0, []
    matches: list[str] = []
    best = 0.0
    for sentence in re.split(r"[\n.;]+", negative_text):
        sent_tokens = set(tokenize(sentence))
        if not sent_tokens:
            continue
        action_overlap = sorted(raw_tokens & sent_tokens & ACTION_TOKENS)
        if not action_overlap:
            continue
        overlap = sorted(raw_tokens & sent_tokens)
        # Require an action plus either a direct object/context token or a strong
        # action-only match (e.g. "push" is often decisive by itself).
        non_action_overlap = [tok for tok in overlap if tok not in ACTION_TOKENS]
        if not non_action_overlap and action_overlap[0] not in {"push", "merge", "delete", "remove", "deploy"}:
            continue
        score = min(1.0, 0.20 * len(action_overlap) + 0.08 * len(non_action_overlap))
        if score > best:
            best = score
            matches = overlap[:8]
    return best, matches


def _fit_search_budget(response: dict[str, Any], max_tokens: int) -> dict[str, Any]:
    """Shrink search response while preserving valid JSON and decision fields.

    ``tokens_estimate`` is part of the returned JSON, so it must be included in
    the final size calculation. The stress test uses the minimum accepted budget
    (200), which requires an explicit final minimal-provenance shape rather than
    just shortening cards/reasons.
    """
    original_count = len(response.get("results", []))
    response.setdefault("omitted_results", 0)

    def stable_token_count(payload: dict[str, Any]) -> int:
        for _ in range(8):
            count = estimate_tokens(json.dumps(payload, ensure_ascii=False, separators=(",", ":")))
            if payload.get("tokens_estimate") == count:
                return count
            payload["tokens_estimate"] = count
        return estimate_tokens(json.dumps(payload, ensure_ascii=False, separators=(",", ":")))

    def fits(payload: dict[str, Any]) -> bool:
        return stable_token_count(payload) <= max_tokens

    response["truncated"] = False
    if fits(response):
        return response

    response["truncated"] = True
    # Phase 1: shrink cards aggressively but keep enough to decide.
    for card_len in (320, 220, 140, 80):
        for item in response.get("results", []):
            card = item.get("card", "")
            if len(card) > card_len:
                item["card"] = card[: max(0, card_len - 3)].rstrip() + "..."
        if fits(response):
            return response

    # Phase 2: keep only the most actionable reasons.
    for item in response.get("results", []):
        item["why_match"] = item.get("why_match", [])[:2]
        item["why_maybe_not"] = item.get("why_maybe_not", [])[:2]
    if fits(response):
        return response

    # Phase 3: reduce candidate count. Token budget has priority over requested k.
    while len(response.get("results", [])) > 1 and not fits(response):
        response["results"].pop()
        response["omitted_results"] = original_count - len(response["results"])
    if fits(response):
        return response

    # Phase 4: ultra-compact single-candidate mode. Keep full source_sha256;
    # dropping or renaming provenance is worse than dropping verbose card/reasons.
    for item in response.get("results", []):
        item.pop("card", None)
        item.pop("source_path", None)
        item.pop("risk_flags", None)
        item.pop("why_match", None)
        item.pop("why_maybe_not", None)
        item.pop("matched_fields", None)
        if not item.get("missing_requirements"):
            item.pop("missing_requirements", None)
        item["provenance_truncated"] = True
    if "ambiguity" in response:
        response["ambiguity"].pop("reason", None)
    if fits(response):
        return response

    # Phase 5: hard-budget fallback. Preserve one loadable candidate and full
    # source hash, then omit nonessential top-level diagnostics.
    compact_results: list[dict[str, Any]] = []
    for item in response.get("results", [])[:1]:
        compact_item = {
            "handle": item.get("handle"),
            "skill_id": item.get("skill_id"),
            "score": item.get("score"),
            "confidence": item.get("confidence"),
            "load_decision": item.get("load_decision"),
            "recommended_view": item.get("recommended_view"),
            "source_sha256": item.get("source_sha256"),
        }
        if item.get("missing_requirements"):
            compact_item["missing_requirements"] = item["missing_requirements"][:2]
        compact_results.append({k: v for k, v in compact_item.items() if v not in (None, [], "")})
    compact_response = {
        "query_id": response.get("query_id"),
        "confidence": response.get("confidence"),
        "results": compact_results,
        "truncated": True,
        "omitted_results": original_count - len(compact_results),
        "total_indexed": response.get("total_indexed"),
    }
    if fits(compact_response):
        return compact_response

    # Last resort for pathological long ids/handles: canonical skill_id remains
    # enough for skill_load, so handle can be omitted to keep the response valid.
    for item in compact_response.get("results", []):
        item.pop("handle", None)
    fits(compact_response)
    return compact_response


class SkillRetrievalEngine:
    """In-memory high-performance skill retrieval engine.

    The engine scans SKILL.md files once, caches parsed records, and serves search/load
    from memory. It never loads arbitrary paths from tool inputs; load is restricted to
    indexed skill ids or handles returned by search.
    """

    def __init__(self, roots: list[str | Path] | None = None, cache_path: str | Path | None = None) -> None:
        self.roots = [Path(p).expanduser().resolve() for p in (roots or default_roots())]
        self.cache_path = Path(cache_path).expanduser().resolve() if cache_path else default_cache_path()
        self.cache_status = "unknown"
        self._handles: dict[str, str] = {}
        self.records = self._load_or_rebuild()
        self._by_id: dict[str, SkillRecord] = {record.skill_id: record for record in self.records}
        self._prepared: dict[str, PreparedRecord] = {record.skill_id: _prepare_record(record) for record in self.records}

    def _load_or_rebuild(self) -> list[SkillRecord]:
        files = _discover_files(self.roots)
        fingerprints = _fingerprints(files)
        root_paths = [str(root) for root in self.roots]
        if self.cache_path.exists():
            try:
                raw = json.loads(self.cache_path.read_text(encoding="utf-8"))
                payload = CachePayload.model_validate(raw)
                if payload.version == CACHE_VERSION and payload.root_paths == root_paths and payload.files == fingerprints:
                    self.cache_status = "loaded"
                    return payload.records
            except Exception:
                # Bad cache should never break the server; rebuild below.
                pass
        records: list[SkillRecord] = []
        for path in files:
            root = next((r for r in self.roots if _is_relative_to(path, r)), self.roots[0] if self.roots else path.parent)
            try:
                records.append(_parse_skill(path, root, self.roots))
            except UnicodeDecodeError:
                continue
        records = _dedupe_records(records)
        payload = CachePayload(version=CACHE_VERSION, root_paths=root_paths, files=fingerprints, records=records)
        try:
            self.cache_path.parent.mkdir(parents=True, exist_ok=True)
            self.cache_path.write_text(json.dumps(payload.model_dump(), ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
        except OSError:
            pass
        self.cache_status = "rebuilt"
        return records

    def search(self, request: SearchRequest) -> dict[str, Any]:
        qprep = _prepare_query(request)
        scored = []
        for record in self.records:
            if request.trusted_only and record.trust_level not in {"user-hermes", "local-root"}:
                continue
            if request.category and record.category != request.category:
                continue
            scored.append(self._score_record(record, request, qprep, self._prepared[record.skill_id]))
        scored.sort(key=lambda item: item["score"], reverse=True)
        selected = self._mmr_select(scored, request.k, request.mmr_lambda)
        query_id = _query_id(request.model_dump())
        selected = selected[:request.k]
        top_gap = selected[0]["score"] - selected[1]["score"] if len(selected) > 1 else selected[0]["score"] if selected else 0.0
        ambiguous = bool(len(selected) > 1 and (top_gap < 0.055 or _jaccard_sets(self._prepared[selected[0]["record"].skill_id].positive.tokens, self._prepared[selected[1]["record"].skill_id].positive.tokens) > 0.92))
        overall_conf = self._overall_confidence(selected[0]["score"] if selected else 0.0, top_gap, ambiguous)
        results = []
        for rank, item in enumerate(selected, start=1):
            record: SkillRecord = item["record"]
            handle = f"search:{query_id}:{rank}:{record.skill_id}"
            self._handles[handle] = record.skill_id
            confidence = self._candidate_confidence(item["score"], top_gap if rank == 1 else 0.0, ambiguous)
            load_decision = self._load_decision(item, confidence, ambiguous)
            result = {
                "handle": handle,
                "skill_id": record.skill_id,
                "score": round(float(item["score"]), 4),
                "confidence": confidence,
                "load_decision": load_decision,
                "recommended_view": "runtime" if load_decision == "safe_to_load" else "preview",
                "why_match": item["why_match"][:5],
                "why_maybe_not": item["why_maybe_not"][:5],
                "missing_requirements": item["missing_requirements"],
                "matched_fields": sorted(item["matched_fields"]),
                "risk_flags": record.risk_flags,
                "trust_level": record.trust_level,
                "source_path": record.source_path,
                "source_sha256": record.source_sha256,
                "card": record.skill_card[:900],
            }
            results.append(result)
        response = {
            "query_id": query_id,
            "confidence": overall_conf,
            "results": results,
            "ambiguity": {
                "is_ambiguous": ambiguous,
                "reason": "top candidates are close or near-duplicates; preview before runtime load" if ambiguous else "top candidate is clearly separated",
                "top_gap": round(float(top_gap), 4),
            },
            "cache_status": self.cache_status,
            "total_indexed": len(self.records),
        }
        return _fit_search_budget(response, request.max_tokens)

    def load(self, request: LoadRequest) -> dict[str, Any]:
        skill_id = self._resolve_skill_id(request.skill_id_or_handle)
        record = self._by_id[skill_id]
        content = self._render_view(record, request)
        content, truncated, tokens = trim_to_token_budget(content, request.max_tokens)
        return {
            "skill_id": record.skill_id,
            "view": request.view,
            "content": content,
            "tokens_estimate": tokens,
            "truncated": truncated,
            "available_views": AVAILABLE_VIEWS,
            "source_path": record.source_path,
            "source_sha256": record.source_sha256,
            "updated_at": datetime.fromtimestamp(record.mtime_ns / 1_000_000_000).isoformat(timespec="seconds"),
            "trust_level": record.trust_level,
            "risk_flags": record.risk_flags,
        }

    def _resolve_skill_id(self, raw: str) -> str:
        if raw in self._handles:
            return self._handles[raw]
        if raw.startswith("search:"):
            raise KeyError(f"Unknown or expired skill search handle: {raw!r}. Use a handle returned by this skill_search session or pass a canonical skill_id.")
        if not SAFE_ID_RE.match(raw):
            raise KeyError(f"Unknown skill handle/id: {raw!r}. Use a skill_id or handle returned by skill_search.")
        if raw not in self._by_id:
            raise KeyError(f"Unknown skill handle/id: {raw!r}. Use skill_search first or pass a canonical skill_id.")
        return raw

    def _score_record(self, record: SkillRecord, request: SearchRequest, qprep: dict[str, Any], prep: PreparedRecord) -> dict[str, Any]:
        desc_score, desc_matches = _score_prepared(qprep["description"], prep.description)
        workflow_score, workflow_matches = _score_prepared(qprep["workflow"], prep.workflow)
        raw_score, raw_matches = _score_prepared(qprep["raw"], prep.raw)
        metadata_score, meta_matches = _score_prepared(qprep["metadata"], prep.metadata)
        trust_score = 1.0 if record.trust_level in {"user-hermes", "local-root"} else 0.5
        score = 0.40 * desc_score + 0.30 * workflow_score + 0.15 * raw_score + 0.10 * metadata_score + 0.05 * trust_score
        identity_bonus, identity_matches = _identity_match(prep, qprep)
        score += identity_bonus
        why_match: list[str] = []
        matched_fields: set[str] = set()
        if identity_matches:
            matched_fields.add("identity")
            why_match.append("identity matched: " + ", ".join(identity_matches))
        if desc_matches:
            matched_fields.add("description")
            why_match.append("description/card matched: " + ", ".join(desc_matches[:8]))
        if workflow_matches:
            matched_fields.add("workflow_summary")
            why_match.append("workflow matched: " + ", ".join(workflow_matches[:8]))
        if raw_matches:
            matched_fields.add("raw_user_request")
            why_match.append("raw request sanity matched: " + ", ".join(raw_matches[:8]))
        if meta_matches:
            matched_fields.add("metadata")
            why_match.append("metadata matched: " + ", ".join(meta_matches[:8]))
        missing = [cue for cue, toks, phrase in qprep["must_have"] if not _cue_tokens_present(toks, phrase, prep.positive)]
        if request.must_have:
            matched_have = [cue for cue, toks, phrase in qprep["must_have"] if cue not in missing]
            if matched_have:
                score += min(0.16, 0.04 * len(matched_have))
                why_match.append("must_have satisfied: " + ", ".join(matched_have[:5]))
                matched_fields.add("must_have")
            if missing:
                score -= min(0.22, 0.055 * len(missing))
        positive_conflicts = [cue for cue, toks, phrase in qprep["must_not"] if _cue_tokens_present(toks, phrase, prep.positive)]
        why_maybe_not: list[str] = []
        if positive_conflicts:
            score -= min(0.35, 0.09 * len(positive_conflicts))
            why_maybe_not.append("must_not_positive_conflict: " + ", ".join(positive_conflicts[:5]))
        if qprep["raw"]["tokens"] and record.do_not_use_when:
            neg_score, neg_matches = _negative_conflict(qprep["raw"], record.do_not_use_when)
            if neg_score >= 0.20:
                score -= min(0.45, 0.18 + neg_score * 0.30)
                why_maybe_not.append("do_not_use_when matched raw request: " + ", ".join(neg_matches[:8]))
        if not why_match:
            why_match.append("weak lexical fallback match")
        return {
            "record": record,
            "score": max(0.0, min(1.0, score)),
            "why_match": why_match,
            "why_maybe_not": why_maybe_not,
            "missing_requirements": missing,
            "matched_fields": matched_fields,
            "positive_conflicts": positive_conflicts,
        }

    def _mmr_select(self, scored: list[dict[str, Any]], k: int, lambda_: float) -> list[dict[str, Any]]:
        if not scored or k <= 0:
            return []
        selected = [scored[0]]
        remaining = scored[1:]
        while remaining and len(selected) < k:
            best_idx = 0
            best_value = -1e9
            for idx, item in enumerate(remaining):
                item_tokens = self._prepared[item["record"].skill_id].positive.tokens
                similarity = max(_jaccard_sets(item_tokens, self._prepared[chosen["record"].skill_id].positive.tokens) for chosen in selected)
                value = lambda_ * item["score"] - (1.0 - lambda_) * similarity
                if value > best_value:
                    best_value = value
                    best_idx = idx
            selected.append(remaining.pop(best_idx))
        return selected

    @staticmethod
    def _overall_confidence(top_score: float, gap: float, ambiguous: bool) -> str:
        if ambiguous:
            return "ambiguous"
        if top_score >= 0.42 and gap >= 0.055:
            return "high"
        if top_score >= 0.22:
            return "medium"
        return "low"

    @staticmethod
    def _candidate_confidence(score: float, gap: float, ambiguous: bool) -> str:
        if ambiguous:
            return "ambiguous"
        if score >= 0.42 and (gap >= 0.055 or gap == 0.0):
            return "high"
        if score >= 0.22:
            return "medium"
        return "low"

    @staticmethod
    def _load_decision(item: dict[str, Any], confidence: str, ambiguous: bool) -> str:
        if item["why_maybe_not"] or item["positive_conflicts"]:
            return "do_not_auto_load"
        if item["missing_requirements"]:
            return "preview_first"
        if ambiguous or confidence in {"medium", "low", "ambiguous"}:
            return "preview_first"
        return "safe_to_load"

    def _render_view(self, record: SkillRecord, request: LoadRequest) -> str:
        applicability = self._applicability(record)
        if request.view == "card":
            return record.skill_card
        if request.view == "preview":
            return "\n\n".join([
                applicability,
                "Workflow summary:\n" + (record.workflow_summary or "[not documented]"),
                "Risk flags: " + (", ".join(record.risk_flags) if record.risk_flags else "none detected"),
                "Source: " + record.source_path,
            ])
        if request.view == "runtime":
            section_texts = []
            for sid in VIEW_SECTIONS["runtime"]:
                if sid in record.sections:
                    section_texts.append(record.sections[sid].content)
            if not section_texts:
                section_texts.append(record.workflow_summary or record.content)
            return applicability + "\n\n" + "\n\n".join(section_texts)
        if request.view == "risk":
            parts = [
                applicability,
                "Risk flags: " + (", ".join(record.risk_flags) if record.risk_flags else "none detected"),
                "Trust level: " + record.trust_level,
                "Source SHA256: " + record.source_sha256,
            ]
            for sid in VIEW_SECTIONS["risk"]:
                if sid in record.sections:
                    parts.append(record.sections[sid].content)
            return "\n\n".join(parts)
        if request.view == "sections":
            if not request.section_ids:
                available = ", ".join(record.sections)
                raise KeyError(f"section_ids required for sections view. Available sections: {available}")
            parts = [applicability]
            for sid in request.section_ids:
                if sid not in record.sections:
                    available = ", ".join(record.sections)
                    raise KeyError(f"Unknown section_id {sid!r}. Available sections: {available}")
                parts.append(record.sections[sid].content)
            return "\n\n".join(parts)
        if request.view == "full":
            return applicability + "\n\n" + record.content
        raise ValueError(f"Unsupported view: {request.view}")

    @staticmethod
    def _applicability(record: SkillRecord) -> str:
        use = record.use_when.strip() or record.description or "[not documented]"
        do_not = record.do_not_use_when.strip() or "[not documented]"
        required = record.required_inputs.strip() or "[not documented]"
        return "\n".join([
            "Applicability check:",
            "Use this skill only if:",
            use,
            "Do not use if:",
            do_not,
            "Required context:",
            required,
            "If this does not match the current task, stop and call skill_search again before applying the workflow.",
        ])


def _is_relative_to(path: Path, root: Path) -> bool:
    try:
        path.resolve().relative_to(root.resolve())
        return True
    except ValueError:
        return False


def _dedupe_records(records: list[SkillRecord]) -> list[SkillRecord]:
    seen: dict[str, int] = {}
    deduped: list[SkillRecord] = []
    for record in records:
        count = seen.get(record.skill_id, 0)
        seen[record.skill_id] = count + 1
        if count:
            data = record.model_dump()
            data["skill_id"] = f"{record.category}-{record.skill_id}-{count + 1}"
            record = SkillRecord.model_validate(data)
        deduped.append(record)
    return deduped
