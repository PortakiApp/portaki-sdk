package app.portaki.sdk.module.backend;

import java.lang.annotation.Documented;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Marque une classe comme implémentation backend d'un module hôte (découverte DI, registre, …).
 */
@Documented
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.TYPE)
public @interface PortakiHostModule {

    /** Identifiant du module, aligné sur {@code portaki.module.json} (ex. {@code checklist}). */
    String value();
}
