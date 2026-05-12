package app.portaki.sdk.module.backend;

/**
 * Résultat générique d'une action backend (compteurs agrégés + résumé lisible humain / logs).
 */
public record HostModuleRunResult(
        boolean ok,
        int succeeded,
        int failed,
        int itemsTotal,
        String summary
) {

    public static HostModuleRunResult emptySuccess() {
        return new HostModuleRunResult(true, 0, 0, 0, "");
    }
}
