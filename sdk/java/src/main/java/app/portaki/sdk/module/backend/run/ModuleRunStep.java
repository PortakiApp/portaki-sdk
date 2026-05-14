package app.portaki.sdk.module.backend.run;

import app.portaki.sdk.module.backend.ModuleBackendException;

/**
 * Une étape nommée d’une pipeline module. Reçoit un état mutable partagé {@code C} (carry) typé par le backend.
 */
public interface ModuleRunStep<C> {

    String id();

    /**
     * Exécute l’étape. Peut lever {@link ModuleBackendException} pour erreurs métier fatales (la pipeline
     * propage sans les convertir en simple {@link ModuleRunStepResult#failure}).
     */
    ModuleRunStepResult run(ModuleRunContext ctx, C carry) throws ModuleBackendException;
}
