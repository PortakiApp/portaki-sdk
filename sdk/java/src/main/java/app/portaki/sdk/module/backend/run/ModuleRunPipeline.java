package app.portaki.sdk.module.backend.run;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

import app.portaki.sdk.module.backend.ModuleBackendException;

/**
 * Enchaîne des {@link ModuleRunStep} sur un {@link ModuleRunContext} et un carry mutable {@code C}.
 *
 * <p>Orchestration <strong>in-process</strong> uniquement : pas de persistance saga, pas d’appel au cœur
 * Portaki, pas d’Axon. Les {@link ModuleRunListener} servent à l’observabilité.
 */
public final class ModuleRunPipeline<C> {

    private final List<ModuleRunStep<C>> steps;
    private final List<ModuleRunListener> listeners;

    private ModuleRunPipeline(List<ModuleRunStep<C>> steps, List<ModuleRunListener> listeners) {
        this.steps = List.copyOf(steps);
        this.listeners = List.copyOf(listeners);
    }

    @SafeVarargs
    public static <C> ModuleRunPipeline<C> of(ModuleRunStep<C>... steps) {
        Objects.requireNonNull(steps, "steps");
        if (steps.length == 0) {
            throw new IllegalArgumentException("steps must not be empty");
        }
        return new ModuleRunPipeline<>(List.of(steps), List.of());
    }

    public ModuleRunPipeline<C> withListeners(ModuleRunListener... listeners) {
        Objects.requireNonNull(listeners, "listeners");
        return new ModuleRunPipeline<>(steps, List.of(listeners));
    }

    /**
     * Exécute toutes les étapes dans l’ordre. Propage {@link ModuleBackendException} telle quelle.
     */
    public ModuleRunReport execute(ModuleRunContext ctx, C carry) throws ModuleBackendException {
        Objects.requireNonNull(ctx, "ctx");
        Objects.requireNonNull(carry, "carry");
        for (ModuleRunListener listener : listeners) {
            listener.onRunStarted(ctx);
        }
        List<ModuleRunStepResult> results = new ArrayList<>();
        for (ModuleRunStep<C> step : steps) {
            for (ModuleRunListener listener : listeners) {
                listener.onStepStarted(ctx, step.id());
            }
            ModuleRunStepResult result = step.run(ctx, carry);
            Objects.requireNonNull(result, "step result");
            results.add(result);
            for (ModuleRunListener listener : listeners) {
                listener.onStepFinished(ctx, step.id(), result);
            }
        }
        ModuleRunReport report = new ModuleRunReport(ctx, Collections.unmodifiableList(results));
        for (ModuleRunListener listener : listeners) {
            listener.onRunFinished(ctx, report);
        }
        return report;
    }
}
