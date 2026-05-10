from __future__ import annotations

from enum import Enum
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator


AVAILABLE_VIEWS = ["card", "preview", "runtime", "risk", "sections", "full"]


class OutputFormat(str, Enum):
    JSON = "json"
    MARKDOWN = "markdown"


class SearchRequest(BaseModel):
    """Validated input for dual-query skill retrieval."""

    model_config = ConfigDict(str_strip_whitespace=True, extra="forbid")

    raw_user_request: str = Field(default="", description="Original user request for intent sanity checks", max_length=8000)
    description_query: str = Field(default="", description="Intent/type query for skill card retrieval", max_length=4000)
    workflow_query: str = Field(default="", description="Procedure query for workflow-summary reranking", max_length=4000)
    must_have: list[str] = Field(default_factory=list, description="Hard requirement cues; missing cues lower confidence", max_length=20)
    nice_to_have: list[str] = Field(default_factory=list, description="Soft preference cues", max_length=20)
    must_not: list[str] = Field(default_factory=list, description="Actions or properties the selected skill must not positively require", max_length=20)
    environment: list[str] = Field(default_factory=list, description="Runtime/environment cues such as git repo or Windows", max_length=20)
    k: int = Field(default=3, ge=1, le=10, description="Number of skill-level candidates to return")
    max_tokens: int = Field(default=1200, ge=200, le=4000, description="Approximate maximum response budget")
    mmr_lambda: float = Field(default=0.70, ge=0.0, le=1.0, description="MMR relevance/diversity tradeoff")
    trusted_only: bool = Field(default=False, description="If true, only return skills from trusted roots")
    category: str | None = Field(default=None, max_length=100, description="Optional category filter")

    @field_validator("must_have", "nice_to_have", "must_not", "environment")
    @classmethod
    def _strip_lists(cls, values: list[str]) -> list[str]:
        return [item.strip() for item in values if item and item.strip()]

    @model_validator(mode="after")
    def _need_some_query(self) -> "SearchRequest":
        if not (self.raw_user_request or self.description_query or self.workflow_query or self.must_have):
            raise ValueError("At least one of raw_user_request, description_query, workflow_query, or must_have is required")
        return self


class LoadRequest(BaseModel):
    """Validated input for loading selected skill views."""

    model_config = ConfigDict(str_strip_whitespace=True, extra="forbid")

    skill_id_or_handle: str = Field(..., min_length=1, max_length=300, description="Canonical skill_id or handle returned by skill_search")
    view: Literal["card", "preview", "runtime", "risk", "sections", "full"] = Field(default="preview", description="View to load")
    section_ids: list[str] = Field(default_factory=list, description="Section ids to load when view='sections'", max_length=20)
    max_tokens: int = Field(default=1200, ge=80, le=8000, description="Approximate maximum tokens in returned content")

    @field_validator("section_ids")
    @classmethod
    def _strip_section_ids(cls, values: list[str]) -> list[str]:
        return [item.strip() for item in values if item and item.strip()]


class SectionRecord(BaseModel):
    model_config = ConfigDict(extra="forbid")

    section_id: str
    title: str
    level: int
    content: str
    start_line: int
    end_line: int


class SkillRecord(BaseModel):
    model_config = ConfigDict(extra="forbid")

    skill_id: str
    name: str
    description: str
    category: str
    tags: list[str]
    skill_card: str
    workflow_summary: str
    use_when: str
    do_not_use_when: str
    required_inputs: str
    risk_flags: list[str]
    sections: dict[str, SectionRecord]
    source_path: str
    source_sha256: str
    mtime_ns: int
    size: int
    trust_level: str
    content: str

    @property
    def positive_text(self) -> str:
        return "\n".join([
            self.name,
            self.description,
            " ".join(self.tags),
            self.category,
            self.skill_card,
            self.workflow_summary,
            self.use_when,
            self.required_inputs,
        ])

    @property
    def all_search_text(self) -> str:
        return self.positive_text + "\n" + self.do_not_use_when + "\n" + "\n".join(s.title for s in self.sections.values())


class FileFingerprint(BaseModel):
    model_config = ConfigDict(extra="forbid")

    path: str
    mtime_ns: int
    size: int


class CachePayload(BaseModel):
    model_config = ConfigDict(extra="forbid")

    version: int
    root_paths: list[str]
    files: list[FileFingerprint]
    records: list[SkillRecord]
