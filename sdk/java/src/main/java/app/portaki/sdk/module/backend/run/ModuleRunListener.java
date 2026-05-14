package app.portaki.sdk.module.backend.run;

/**
 * Observateur optionnel d’une exécution {@link ModuleRunPipeline} (logs structurés, métriques, traces).
 *
 * <p>Implémentations typiques : SLF4J, OpenTelemetry. Ne doit pas introduire de dépendance vers le bus
 * d’événements applicatif (Axon, etc.) dans le SDK.
 */
public interface ModuleRunListener {

    default void onRunStarted(ModuleRunContext ctx) {}

    default void onStepStarted(ModuleRunContext ctx, String stepId) {}

    default void onStepFinished(ModuleRunContext ctx, String stepId, ModuleRunStepResult result) {}

    default void onRunFinished(ModuleRunContext ctx, ModuleRunReport report) {}
}
