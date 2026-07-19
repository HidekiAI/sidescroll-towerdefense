test_character Skill Tree Template
===========================

Generated: Thu Jul 16 08:44:47 CDT 2026

Files to edit:
1. 00_entity_setup.sql    - Entity and category setup
2. 01_skill_nodes.sql     - Skill node definitions  
3. 02_prerequisites.sql   - Prerequisite chains

How to use:
1. Edit the SQL files (replace placeholder names and costs)
2. Validate: sqlite3 game.db "BEGIN; .read 00_entity_setup.sql; ROLLBACK;"
3. Apply: sqlite3 game.db ".read 00_entity_setup.sql"
            sqlite3 game.db ".read 01_skill_nodes.sql"
            sqlite3 game.db ".read 02_prerequisites.sql"
4. Verify: sqlite3 game.db ".read 03_verification.sql"

Template structure:
- 3 branches with A->B->C progression
- Foundation skills (A nodes): sort_order 10-19
- Intermediate skills (B nodes): sort_order 20-29  
- Advanced skills (C nodes): sort_order 30-39

Replace placeholder names:
  test_character.basic_foundation -> Your actual skill name
  test_character.intermediate_skill -> Your actual skill name
  test_character.advanced_skill -> Your actual skill name

Update localization keys:
  skill.test_character.skill_name.name -> Display name
  skill.test_character.skill_name.desc -> Description
