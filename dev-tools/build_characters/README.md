# Build Characters — Dev Tools

Phase 1 CLI tools for skill tree data generation and validation. See the wiki for full design, usage, and workflow documentation:

| Tool | Description | Wiki |
|---|---|---|
| `scripts/generate.sh` | SQL template generator for new skill trees | [TDD: Skill Designer Templating → Architecture](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TDD_Skill-Designer-Templating#architecture) |
| `scripts/validate.sh` | SQL syntax and pattern validator | same doc as above |
| `scripts/generate_checksums.sh` | Schema checksum generator | [TDD: Schema Versioning → `generate_checksums.sh`](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TDD_Schema-Versioning#bash-script-generate_checksumssh) |
| `scripts/validate_schema.sh` | Schema diff comparison tool | [TDD: Schema Versioning → `validate_schema.sh`](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TDD_Schema-Versioning#bash-script-validate_schemash) |

All tools require `sqlite3` and `bash`. Run any script with `--help` (or no args) for usage.
