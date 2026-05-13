# DDL checklist (référence)

Les fichiers **canoniques** décrivant le schéma métier du module vivent ici (`portaki-sdk/packages/checklist/db/`).

**Exécution Flyway** : une seule instance Flyway tourne dans **`portaki-api`** (base unique, ordre des versions, checksums). Les migrations appliquées sont donc **dupliquées** sous  
`portaki-api/infrastructure/src/main/resources/db/migration/` — elles doivent rester **alignées** avec ce dossier lors des PR (ou via script de sync à terme).

**Nommage tables** (convention Portaki) :

- Entités : `t_e_*` (ex. `t_e_module_checklist_items`)
- Jointures / tables de lien : `t_j_*` (ex. `t_j_module_checklist_completions`)

Voir aussi `portaki-api/.cursor/rules/portaki-module-db-naming.mdc`.
