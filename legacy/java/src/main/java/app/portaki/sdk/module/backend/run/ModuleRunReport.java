package app.portaki.sdk.module.backend.run;

import java.util.List;
import java.util.Objects;

/**
 * Synthèse d’une exécution pipeline : contexte + résultats par étape, dans l’ordre d’exécution.
 */
public record ModuleRunReport(ModuleRunContext context, List<ModuleRunStepResult> stepResults) {

    public ModuleRunReport {
        Objects.requireNonNull(context, "context");
        Objects.requireNonNull(stepResults, "stepResults");
    }

    public boolean allStepsSucceeded() {
        return stepResults.stream().allMatch(ModuleRunStepResult::ok);
    }
}
