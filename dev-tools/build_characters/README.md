# Build Characters — Dev Tools

Phase 1 CLI tools for skill tree data generation and validation. See the wiki for full design, usage, and workflow documentation:

| Tool | Description | Wiki |
|---|---|---|
| `scripts/generate.sh` | SQL template generator for new skill trees | [TDD: Skill Designer Templating](https://github.com/HidekiAI/sidescroll-towerdefense.wiki/blob/master/TechnicalDesign/TDD_Skill-Designer-Templating.md) |
| `scripts/validate.sh` | SQL syntax and pattern validator | same doc as above |
| `scripts/generate_checksums.sh` | Schema checksum generator | [TDD: Schema Versioning](https://github.com/HidekiAI/sidescroll-towerdefense.wiki/blob/master/TechnicalDesign/TDD_Schema-Versioning.md) |
| `scripts/validate_schema.sh` | Schema diff comparison tool | same doc as above |

All tools require `sqlite3` and `bash`. Run any script with `--help` (or no args) for usage.
