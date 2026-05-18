package app.portaki.sdk.module.backend.run;

import java.util.Objects;

/**
 * Résultat d’une étape de pipeline (succès ou échec avec message court pour logs / UI technique).
 */
public record ModuleRunStepResult(String stepId, boolean ok, String message) {

    public ModuleRunStepResult {
        Objects.requireNonNull(stepId, "stepId");
        Objects.requireNonNull(message, "message");
    }

    public static ModuleRunStepResult ok(String stepId, String message) {
        return new ModuleRunStepResult(stepId, true, message);
    }

    public static ModuleRunStepResult failure(String stepId, String message) {
        return new ModuleRunStepResult(stepId, false, message);
    }
}
